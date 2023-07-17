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
        pname = "tg-maid";
        docker_img_name = "ghcr.io/avimitin/${pname}";
        version = "unstable-2023-07-14";

        # Rust overlays for the Nixpkgs
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        #
        # Dependencies
        #
        # Custom Rust toolchains.
        # Default toolchains includes latest cargo,clippy,cargo-fmt..., 
        rs-toolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };
        # Font data dependencies
        noto-fonts-cjk = pkgs.fetchFromGitHub {
          owner = "googlefonts";
          repo = "noto-cjk";
          rev = "1c7ca85cb5195a3332e18c2b5cfe196ffb084e72";
          sha256 = "sha256-541hsYHqjBYTBEg7ooGfX1+hJLo4QouQnVOIq8UzN7Y=";
          sparseCheckout = [ "Sans/OTC" ];
        };
        QUOTE_TEXT_FONT_PATH =
          "${noto-fonts-cjk}/Sans/OTC/NotoSansCJK-Black.ttc";
        QUOTE_USERNAME_FONT_PATH =
          "${noto-fonts-cjk}/Sans/OTC/NotoSansCJK-Light.ttc";

        # Build time & Runtime dependencies
        nativeBuildInputs = with pkgs; [ pkg-config ];
        # Link time dependencies
        buildInputs = with pkgs; [ openssl ];

        # Default build target
        rs-platform = pkgs.makeRustPlatform {
          cargo = rs-toolchain;
          rustc = rs-toolchain;
        };
        tg-maid = rs-platform.buildRustPackage {
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          # Some test require proper env, which is not available during build
          doCheck = false;

          inherit pname version nativeBuildInputs buildInputs
            QUOTE_TEXT_FONT_PATH QUOTE_USERNAME_FONT_PATH;
        };
      in {
        # nix develop
        devShells.default = with pkgs;
          mkShell {
            nativeBuildInputs = nativeBuildInputs ++ [
              rs-toolchain
              # rust-analyzer comes from nixpkgs toolchain, I want the unwrapped version
              rust-analyzer-unwrapped
              yt-dlp
              # Dependency for yt-dlp
              ffmpeg
              # A temporary DB
              redis
              # In case someone want to commit inside the nix shell but got a version mismatch openssl
              git
            ];

            # To make rust-analyzer work correctly (The path prefix issue)
            RUST_SRC_PATH = "${rs-toolchain}/lib/rustlib/src/rust/library";

            inherit buildInputs QUOTE_TEXT_FONT_PATH QUOTE_USERNAME_FONT_PATH;
          };
        # nix build
        packages.default = tg-maid;

        # nix build .#docker
        packages.docker = import ./nix/docker-image.nix {
          name = docker_img_name;
          tag = version;

          inherit pkgs tg-maid;
        };

        # nix run .#build-push-docker-img
        apps.build-push-docker-img = let
          script = import ./nix/finalize-image.nix {
            name = docker_img_name;
            tag = version;

            do_push = true;
            is_latest = true;

            inherit pkgs;
          };
        in {
          type = "app";
          program = "${script}";
        };
      });
}
