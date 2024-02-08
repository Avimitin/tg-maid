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
        myOverlay = import ./overlay.nix;
        overlays = [ (import rust-overlay) myOverlay ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        # nix run .#ci
        apps.build-docker-image = flake-utils.lib.mkApp {
          drv = pkgs.docker-image.finalizer;
        };

        # nix develop
        devShells.default = pkgs.tg-maid.bot.devShell;

        # nix fmt
        formatter = pkgs.nixpkgs-fmt;


        legacyPackages = pkgs;
        overlays.default = myOverlay;
      });
}
