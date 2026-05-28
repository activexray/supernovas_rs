use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=SUPERNOVAS_NO_PKG_CONFIG");
    println!("cargo:rerun-if-env-changed=SUPERNOVAS_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=SUPERNOVAS_LIB_DIR");
    println!("cargo:rerun-if-env-changed=CALCEPH_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=CALCEPH_LIB_DIR");

    let lib = find_library();
    #[cfg(feature = "calceph")]
    let calceph = find_calceph();

    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_arg("-D_EXCLUDE_DEPRECATED")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .generate_comments(true)
        .derive_default(true)
        .derive_debug(true)
        .derive_copy(true);

    for path in &lib.include_paths {
        builder = builder.clang_arg(format!("-I{}", path.display()));
        // Only emit bindings for symbols declared in SuperNOVAS headers,
        // not transitively-included system headers (libc, math.h, etc.).
        let pattern = format!("{}/.*\\.h", path.display());
        builder = builder.allowlist_file(pattern);
    }

    #[cfg(feature = "calceph")]
    {
        // Make wrapper.h pick up <novas-calceph.h> + <calceph.h>.
        builder = builder.clang_arg("-DSUPERNOVAS_FFI_WITH_CALCEPH");
        for path in &calceph.include_paths {
            builder = builder.clang_arg(format!("-I{}", path.display()));
            let pattern = format!("{}/.*\\.h", path.display());
            builder = builder.allowlist_file(pattern);
        }
        // Explicit name-based allowlist for the calceph symbols we wrap.
        // This ensures they get emitted even when the calceph header is
        // pulled in via a toolchain-managed include path (e.g. Nix's
        // bindgenHook) that we don't surface in `calceph.include_paths`.
        builder = builder
            .allowlist_function("calceph_open")
            .allowlist_function("calceph_open_array")
            .allowlist_function("calceph_close")
            .allowlist_type("calcephbin");
    }

    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}

#[cfg(feature = "calceph")]
struct Calceph {
    include_paths: Vec<PathBuf>,
}

#[cfg(feature = "calceph")]
fn find_calceph() -> Calceph {
    // calceph itself is never vendored — it has to be system-installed.
    // Three search tiers:
    //
    // 1. Explicit override via CALCEPH_INCLUDE_DIR / CALCEPH_LIB_DIR.
    // 2. pkg-config (works on distros that ship a calceph.pc; not nixpkgs as
    //    of calceph 4.0.5 — it's CMake-only there).
    // 3. Implicit: emit `-lcalceph` and trust the toolchain's native
    //    include path to find calceph.h. In the project's Nix dev shell
    //    that's populated by rustPlatform.bindgenHook from buildInputs.
    let include_dir = env::var_os("CALCEPH_INCLUDE_DIR").map(PathBuf::from);
    let lib_dir = env::var_os("CALCEPH_LIB_DIR").map(PathBuf::from);
    if include_dir.is_some() || lib_dir.is_some() {
        if let Some(dir) = &lib_dir {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
        println!("cargo:rustc-link-lib=calceph");
        return Calceph {
            include_paths: include_dir.into_iter().collect(),
        };
    }

    if let Ok(lib) = pkg_config::Config::new().probe("calceph") {
        return Calceph {
            include_paths: lib.include_paths,
        };
    }

    println!("cargo:rustc-link-lib=calceph");
    Calceph {
        include_paths: vec![],
    }
}

struct Library {
    include_paths: Vec<PathBuf>,
}

#[cfg(feature = "vendored")]
fn find_library() -> Library {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = manifest_dir.join("vendor").join("supernovas");
    assert!(
        vendor.join("CMakeLists.txt").exists(),
        "vendor/supernovas is empty — run `git submodule update --init --recursive`"
    );

    let mut config = cmake::Config::new(&vendor);
    config
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("BUILD_TESTING", "OFF")
        .define("BUILD_EXAMPLES", "OFF")
        .define("BUILD_BENCHMARK", "OFF")
        .define("BUILD_DOC", "OFF")
        // Disable the optional libcurl dep added in v1.7 (EOP fetching from
        // IERS); not needed for offline astrometry and avoids a C dependency.
        .define("WITHOUT_CURL", "ON");
    // Without the `libc` feature (implied by `std`), build a freestanding
    // SuperNOVAS with no libc calls inside the C library.
    #[cfg(not(feature = "libc"))]
    config.define("WITHOUT_LIBC", "ON");
    #[cfg(feature = "calceph")]
    config.define("ENABLE_CALCEPH", "ON");
    let dst = config.build();

    // GNUInstallDirs picks lib64 on RHEL-family / Nix, lib elsewhere.
    let lib_dir = if dst.join("lib64").is_dir() {
        dst.join("lib64")
    } else {
        dst.join("lib")
    };

    // Upstream sets CMAKE_DEBUG_POSTFIX=d via a non-cache `set()`, so we can't
    // suppress it from the command line. Probe both names instead.
    let lib_name = ["supernovas", "supernovasd"]
        .into_iter()
        .find(|name| lib_dir.join(format!("lib{name}.a")).exists())
        .expect("static libsupernovas[d].a not produced by cmake build");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static={lib_name}");
    // SuperNOVAS pulls in libm; static linking needs it spelled out explicitly.
    println!("cargo:rustc-link-lib=m");

    #[cfg(feature = "calceph")]
    {
        // The CALCEPH plugin is a separate static lib in the vendored build.
        let plugin_name = ["solsys-calceph", "solsys-calcephd"]
            .into_iter()
            .find(|name| lib_dir.join(format!("lib{name}.a")).exists())
            .expect("static libsolsys-calceph[d].a not produced by cmake build");
        println!("cargo:rustc-link-lib=static={plugin_name}");
        // calceph itself is system-installed regardless of vendored mode.
        // `find_calceph` (called from main) already emits the link directives
        // for libcalceph via pkg-config; nothing more to do here.
    }

    Library {
        include_paths: vec![dst.join("include")],
    }
}

#[cfg(not(feature = "vendored"))]
fn find_library() -> Library {
    let include_dir = env::var_os("SUPERNOVAS_INCLUDE_DIR").map(PathBuf::from);
    let lib_dir = env::var_os("SUPERNOVAS_LIB_DIR").map(PathBuf::from);

    // Explicit override via env vars wins, regardless of pkg-config.
    if include_dir.is_some() || lib_dir.is_some() {
        if let Some(dir) = &lib_dir {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
        println!("cargo:rustc-link-lib=supernovas");
        return Library {
            include_paths: include_dir.into_iter().collect(),
        };
    }

    if env::var_os("SUPERNOVAS_NO_PKG_CONFIG").is_none() {
        match pkg_config::Config::new()
            .atleast_version("1.7.0")
            .probe("supernovas")
        {
            Ok(lib) => {
                return Library {
                    include_paths: lib.include_paths,
                };
            }
            Err(e) => panic!(
                "pkg-config could not locate supernovas >= 1.7.0: {e}\n\
                 Options:\n\
                 - enable the `vendored` feature to build the bundled v1.7 statically, or\n\
                 - install supernovas >= 1.7.0 system-wide, or\n\
                 - set SUPERNOVAS_INCLUDE_DIR / SUPERNOVAS_LIB_DIR to point at a local install, or\n\
                 - set SUPERNOVAS_NO_PKG_CONFIG=1 with the above env vars to skip pkg-config entirely."
            ),
        }
    }

    panic!(
        "SUPERNOVAS_NO_PKG_CONFIG is set, but neither SUPERNOVAS_INCLUDE_DIR nor SUPERNOVAS_LIB_DIR \
         was provided to locate the C library."
    );
}
