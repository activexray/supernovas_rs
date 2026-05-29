{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    crane,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [fenix.overlays.default];
        };
        inherit (pkgs) lib;

        # Nightly toolchain derived from rust-toolchain.toml. The sha256 is the
        # hash of the channel manifest for the pinned date; update it whenever
        # rust-toolchain.toml is bumped to a new nightly.
        toolchain = pkgs.fenix.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-T9bAi9MXgNomhnt7+2UwSQY9YWyYFZx6ZsxnU3KLEjI=";
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        # The SuperNOVAS C library, fetched at the exact commit recorded by the
        # git submodule.  Nix's flake source copy excludes git-submodule working
        # trees, so we bring the content in separately and graft it in postPatch.
        # Update `rev` + `hash` whenever supernovas-ffi/vendor/supernovas moves.
        supernovasC = pkgs.fetchFromGitHub {
          owner = "sigmyne";
          repo = "supernovas";
          rev = "119e5e1276312a0ed69163c0e8dc2059b49259de";
          hash = "sha256-+Tw7Zd4fmq6fw+1T1Abj51eJqy32Lwfuf8HMW+VbpY0=";
        };

        # Include Rust/Cargo sources and headers (for bindgen).  The vendor
        # directory skeleton is included so the path exists, but its contents
        # are populated by postPatch (see commonArgs below).
        src = lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (lib.hasSuffix ".h" path)
            || (lib.hasInfix "/vendor/" path)
            || (craneLib.filterCargoSources path type);
        };

        # SuperNOVAS is built from the vendored submodule (no system lib needed).
        # calceph is still system-provided (not vendored); it is only required
        # when the `calceph` cargo feature is enabled.
        commonArgs = {
          inherit src;
          # Graft the SuperNOVAS C source into the vendor directory.  Nix's
          # flake source machinery copies the git-tracked skeleton but leaves
          # the submodule directory empty; postPatch fills it in from the
          # separately-fetched derivation above.
          postPatch = ''
            rm -rf supernovas-ffi/vendor/supernovas
            cp -r "${supernovasC}/." supernovas-ffi/vendor/supernovas
            chmod -R u+w supernovas-ffi/vendor/supernovas
          '';
          pname = "supernovas";
          strictDeps = true;
          # Default features: vendored (SuperNOVAS via CMake) + anise (implies std).
          cargoExtraArgs = "--workspace";
          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            rustPlatform.bindgenHook
          ];
          buildInputs = with pkgs; [
            calceph
          ];
        };

        # Shared pre-built dependency artifacts; amortises compile time across checks.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Pinned MSRV toolchain — version tracks rust-toolchain.toml's `channel`.
        # To update: bump `channel`, set sha256 = lib.fakeSha256, run `nix flake check`,
        # then replace sha256 with the hash reported in the error.
        msrvToolchain =
          (pkgs.fenix.toolchainOf {
            channel = "1.89.0";
            sha256 = "sha256-+9FmLhAOezBZCOziO0Qct1NOrfpjNsXxc/8I0c7BdKE=";
          }).minimalToolchain;

        craneLibMsrv = (crane.mkLib pkgs).overrideToolchain msrvToolchain;
      in {
        checks = {
          # cargo nextest run --workspace
          test = craneLib.cargoNextest (commonArgs
            // {
              inherit cargoArtifacts;
              nativeBuildInputs = commonArgs.nativeBuildInputs ++ [pkgs.cargo-nextest];
            });

          # cargo clippy --workspace -- -D warnings
          clippy = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "-- -D warnings";
            });

          # cargo fmt --check  (nightly, for rustfmt.toml unstable features)
          fmt = craneLib.cargoFmt {
            inherit src;
            pname = "supernovas";
          };

          # cargo doc --workspace --no-deps
          doc = craneLib.cargoDoc (commonArgs
            // {
              inherit cargoArtifacts;
              cargoDocExtraArgs = "--no-deps";
              RUSTDOCFLAGS = "-D warnings";
            });

          # Compile check against the declared MSRV
          msrv = craneLibMsrv.cargoBuild (commonArgs
            // {
              cargoArtifacts = craneLibMsrv.buildDepsOnly commonArgs;
            });

          cov = craneLib.cargoLlvmCov (commonArgs
            // {
              inherit cargoArtifacts;
            });
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            cmake # for local vendored builds
            rust-analyzer-nightly
            cargo-msrv
          ];

          # calceph is a shared library; expose it for the `calceph` cargo feature.
          LD_LIBRARY_PATH = lib.makeLibraryPath [pkgs.calceph];
          RUST_BACKTRACE = 1;
        };
      }
    );
}
