{
  description = "Flakes for Rust development";

  inputs = {
    # The nixpkgs
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Utility functions
    flake-utils.url = "github:numtide/flake-utils";

    # An nixpkgs overlay for overriding the nixpkgs to get declarative Rust toolchain specification.
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rs-toolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

        noto-fonts-cjk = pkgs.fetchFromGitHub {
          owner = "googlefonts";
          repo = "noto-cjk";
          rev = "1c7ca85cb5195a3332e18c2b5cfe196ffb084e72";
          sha256 = "sha256-541hsYHqjBYTBEg7ooGfX1+hJLo4QouQnVOIq8UzN7Y=";
          sparseCheckout = [ "Sans/OTC" ];
        };
      in {
        devShells.default = with pkgs; mkShell {
          nativeBuildInputs = [
            pkg-config
          ];
          buildInputs = [
            # Including latest cargo,clippy,cargo-fmt
            rs-toolchain
            # rust-analyzer comes from nixpkgs toolchain, I want the unwrapped version
            rust-analyzer-unwrapped
            openssl
            noto-fonts-cjk-sans
          ];

          # To make rust-analyzer work correctly (The path prefix issue)
          RUST_SRC_PATH = "${rs-toolchain}/lib/rustlib/src/rust/library";
          # To make sure cargo test run correctly
          LD_LIBRARY_PATH = lib.makeLibraryPath [ openssl ];

          QUOTE_TEXT_FONT_PATH = "${noto-fonts-cjk}/Sans/OTC/NotoSansCJK-Black.ttc";
          QUOTE_USERNAME_FONT_PATH = "${noto-fonts-cjk}/Sans/OTC/NotoSansCJK-Light.ttc";
        };
      });
}