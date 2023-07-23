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
        # Meta
        pname = "tg-maid";
        version = "unstable-2023-07-14";

        # Rust overlays for the Nixpkgs
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        fonts = import ./nix/quote-font.nix { inherit pkgs; };

        # Override the nixpkgs
        rust-toolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };
        rust = pkgs.makeRustPlatform {
          cargo = rust-toolchain;
          rustc = rust-toolchain;
        };
      in {
        # nix build
        packages.default = rust.buildRustPackage {
          src = ./.;

          # Build time & Runtime dependencies
          nativeBuildInputs = [ pkgs.pkg-config ];
          # Link time dependencies
          buildInputs = [ pkgs.openssl ];

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          # Some test require proper env, which is not available during build
          doCheck = false;

          # Export font path
          QUOTE_TEXT_FONT_PATH = fonts.bold;
          QUOTE_USERNAME_FONT_PATH = fonts.light;

          inherit pname version;
        };

        # nix develop
        devShells.default =
          import ./nix/devshell.nix { inherit pkgs fonts rust-toolchain; };

        # nix build .#docker
        packages.docker = import ./nix/docker-image.nix {
          name = "ghcr.io/avimitin/${pname}";
          tag = version;
          executable = "${self.packages."${system}".default}/bin/tgbot";

          inherit pkgs;
        };

        # Generate script for GitHub Action to run
        packages.ci-script = import ./nix/finalize-image.nix {
          name = "ghcr.io/avimitin/${pname}";
          tag = version;

          # Do docker push
          do_push = true;
          # Also tag image as latest
          is_latest = true;

          inherit pkgs;
        };

        # nix run .#ci
        apps.ci = {
          type = "app";
          program = "${self.packages."${system}".ci-script}";
        };
      });
}
