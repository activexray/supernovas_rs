{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    fenix,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [fenix.overlays.default];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.fenix.complete.withComponents [
          "cargo"
          "clippy"
          "rust-src"
          "rustc"
          "rustfmt"
        ];
      in
        with pkgs; {
          devShells.default = mkShell {
            buildInputs = [
              pkg-config
              cmake
              rust
              rustPlatform.bindgenHook
              rust-analyzer-nightly
              supernovas
              cargo-nextest
	      cargo-msrv
            ];

            # Test binaries linked against shared libsupernovas need the
            # runtime loader to find it; Nix doesn't set RPATH for cargo
            # outputs, so expose the lib dir on LD_LIBRARY_PATH.
            LD_LIBRARY_PATH = lib.makeLibraryPath [supernovas];
            RUST_BACKTRACE = 1;
          };
        }
    );
}
