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
        version = "unstable-2023-07-14";

        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rs-toolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

        minimal-toolchain = pkgs.rust-bin.stable.latest.minimal;
        rs-env = pkgs.makeRustPlatform {
          cargo = minimal-toolchain;
          rustc = minimal-toolchain;
        };

        noto-fonts-cjk = pkgs.fetchFromGitHub {
          owner = "googlefonts";
          repo = "noto-cjk";
          rev = "1c7ca85cb5195a3332e18c2b5cfe196ffb084e72";
          sha256 = "sha256-541hsYHqjBYTBEg7ooGfX1+hJLo4QouQnVOIq8UzN7Y=";
          sparseCheckout = [ "Sans/OTC" ];
        };

        # Compile time dependecies
        nativeBuildInputs = with pkgs; [ pkg-config mold ];
        # Build time dependecies
        buildInputs = with pkgs; [ openssl ffmpeg yt-dlp ];

        QUOTE_TEXT_FONT_PATH =
          "${noto-fonts-cjk}/Sans/OTC/NotoSansCJK-Black.ttc";
        QUOTE_USERNAME_FONT_PATH =
          "${noto-fonts-cjk}/Sans/OTC/NotoSansCJK-Light.ttc";

        defaultBuildTarget = rs-env.buildRustPackage {
          pname = "tg-maid";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          # Some test require proper env, which is not available during build
          doCheck = false;

          inherit version nativeBuildInputs buildInputs QUOTE_TEXT_FONT_PATH
            QUOTE_USERNAME_FONT_PATH;
        };
      in {
        # nix develop
        devShells.default = with pkgs;
          mkShell {
            buildInputs = buildInputs ++ [
              # Including latest cargo,clippy,cargo-fmt
              rs-toolchain
              # rust-analyzer comes from nixpkgs toolchain, I want the unwrapped version
              rust-analyzer-unwrapped
            ];

            # To make rust-analyzer work correctly (The path prefix issue)
            RUST_SRC_PATH = "${rs-toolchain}/lib/rustlib/src/rust/library";
            # To make sure cargo test run correctly
            LD_LIBRARY_PATH = lib.makeLibraryPath [ openssl ];

            inherit nativeBuildInputs QUOTE_TEXT_FONT_PATH
              QUOTE_USERNAME_FONT_PATH;
          };
        # nix build
        packages.default = defaultBuildTarget;

        # nix build .#docker
        packages.docker = pkgs.dockerTools.buildImage {
          name = "ghcr.io/Avimitin/tg-maid";
          tag = version;

          config = {
            cmd = [ "${defaultBuildTarget}/bin/tgbot" ];
            healthcheck = {
              test = [
                "CMD-SHELL"
                "${pkgs.netcat-openbsd}/bin/nc -z 127.0.0.1 11451 || exit 1"
              ];
            };
          };
        };
      });
}
