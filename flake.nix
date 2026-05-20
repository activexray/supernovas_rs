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

        # Nightly toolchain derived from rust-toolchain.toml. The sha256 covers
        # the channel manifest fetched from static.rust-lang.org; it changes
        # with each nightly so we use lib.fakeSha256 here and let the magic-nix
        # cache keep builds reproducible in CI.
        toolchain = pkgs.fenix.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = lib.fakeSha256;
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        # Crane source filter: vendor/ is excluded automatically (C files);
        # that's fine because we use --no-default-features below.
        src = craneLib.cleanCargoSource ./.;

        # All crane builds use the nixpkgs supernovas library so cmake is not
        # needed in the sandbox. The vendored feature remains the default for
        # non-Nix users; in Nix we take the pkg-config path.
        commonArgs = {
          inherit src;
          strictDeps = true;
          cargoExtraArgs = "--workspace --no-default-features";
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.rustPlatform.bindgenHook
          ];
          buildInputs = [pkgs.supernovas];
        };

        # Shared pre-built dependency artifacts; amortises compile time across checks.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Pinned MSRV toolchain (1.88.0 — established by `cargo msrv`).
        # If this hash ever needs updating: temporarily set sha256 = lib.fakeSha256,
        # run `nix flake check`, and replace with the hash in the error output.
        msrvToolchain = (pkgs.fenix.toolchainOf {
          channel = "1.88.0";
          sha256 = lib.fakeSha256;
        }).minimalToolchain;

        craneLibMsrv = (crane.mkLib pkgs).overrideToolchain msrvToolchain;
      in {
        checks = {
          # cargo test --workspace --no-default-features
          test = craneLib.cargoTest (commonArgs
            // {
              inherit cargoArtifacts;
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
          fmt = craneLib.cargoFmt {inherit src;};

          # cargo doc --workspace --no-default-features --no-deps
          doc = craneLib.cargoDoc (commonArgs
            // {
              inherit cargoArtifacts;
              cargoDocExtraArgs = "--no-deps";
              RUSTDOCFLAGS = "-D warnings";
            });

          # Build check against the declared MSRV (1.88.0).
          msrv = craneLibMsrv.cargoCheck (commonArgs
            // {
              cargoArtifacts = craneLibMsrv.buildDepsOnly commonArgs;
            });
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.pkg-config
            pkgs.cmake # for local vendored builds (cargo build default)
            toolchain
            pkgs.rustPlatform.bindgenHook
            pkgs.rust-analyzer-nightly
            pkgs.supernovas
            pkgs.cargo-nextest
            pkgs.cargo-msrv
          ];

          # Test binaries link against shared libsupernovas; Nix doesn't set
          # RPATH on cargo outputs so expose the lib dir explicitly.
          LD_LIBRARY_PATH = lib.makeLibraryPath [pkgs.supernovas];
          RUST_BACKTRACE = 1;
        };
      }
    );
}
