{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixpkgs-master.url = "github:NixOS/nixpkgs/master";
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
    nixpkgs-master,
    flake-utils,
    fenix,
    crane,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            fenix.overlays.default
            # nixpkgs-unstable carries supernovas 1.5.1; master has 1.6.0.
            # Take the master derivation without the C++ wrapper but with
            # CALCEPH support, so libsolsys-calceph is available when the
            # `calceph` cargo feature is enabled.
            (_: _: {
              supernovas =
                (nixpkgs-master.legacyPackages.${system}.supernovas).override {
                  cppSupport = false;
                  withCalceph = true;
                };
            })
          ];
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

        # Crane source filter: vendor/ is excluded automatically (C files);
        # that's fine because we use --no-default-features below.
        # wrapper.h is included so bindgen can generate the FFI bindings.
        src = lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (lib.hasSuffix ".h" path) || (craneLib.filterCargoSources path type);
        };

        # All crane builds use the nixpkgs supernovas library via pkg-config;
        # cmake is not needed in the sandbox.
        commonArgs = {
          inherit src;
          pname = "supernovas";
          strictDeps = true;
          cargoExtraArgs = "--workspace --no-default-features";
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustPlatform.bindgenHook
          ];
          buildInputs = with pkgs; [
            supernovas
            calceph
          ];
        };

        # Shared pre-built dependency artifacts; amortises compile time across checks.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Pinned MSRV toolchain (1.88.0 — established by `cargo msrv`).
        # To update: set sha256 = lib.fakeSha256, run `nix flake check`, replace
        # with the hash reported in the error.
        msrvToolchain = (pkgs.fenix.toolchainOf {
          channel = "1.88.0";
          sha256 = "sha256-Qxt8XAuaUR2OMdKbN4u8dBJOhSHxS+uS06Wl9+flVEk=";
        }).minimalToolchain;

        craneLibMsrv = (crane.mkLib pkgs).overrideToolchain msrvToolchain;
      in {
        checks = {
          # cargo nextest run --workspace --no-default-features
          test = craneLib.cargoNextest (commonArgs
            // {
              inherit cargoArtifacts;
              nativeBuildInputs = commonArgs.nativeBuildInputs ++ [pkgs.cargo-nextest];
              # libsupernovas is a shared library; tell the dynamic linker where
              # to find it when the test binaries run inside the Nix sandbox.
              preCheck = ''
                export LD_LIBRARY_PATH=${lib.makeLibraryPath [pkgs.supernovas]}
              '';
            });

          # cargo clippy --workspace --no-default-features -- -D warnings
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

          # cargo doc --workspace --no-default-features --no-deps
          doc = craneLib.cargoDoc (commonArgs
            // {
              inherit cargoArtifacts;
              cargoDocExtraArgs = "--no-deps";
              RUSTDOCFLAGS = "-D warnings";
            });

          # Compile check against the declared MSRV (1.88.0).
          msrv = craneLibMsrv.cargoBuild (commonArgs
            // {
              cargoArtifacts = craneLibMsrv.buildDepsOnly commonArgs;
            });
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            cmake # for local vendored builds
            rust-analyzer-nightly
            cargo-msrv
          ];

          # Test binaries link against shared libsupernovas (and libcalceph
          # when the `calceph` cargo feature is enabled); Nix doesn't set
          # RPATH on cargo outputs so expose the lib dirs explicitly.
          LD_LIBRARY_PATH = lib.makeLibraryPath [pkgs.supernovas pkgs.calceph];
          RUST_BACKTRACE = 1;
        };
      }
    );
}
