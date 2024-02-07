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
        version = "unstable-2023-08-30";
        dockerScripts = import ./nix/docker-scripts.nix { inherit version; };
      in
      {
        # nix build
        packages.default = pkgs.callPackage ./nix/tg-maid.nix { inherit version; };

        # nix build .#docker
        packages.docker-builder = pkgs.callPackage dockerScripts.builder { tg-maid = self.packages.${system}.default; };
        # nix run .#ci
        apps.ci = flake-utils.lib.mkApp {
          drv = pkgs.callPackage dockerScripts.finalizer { doPush = true; isLatest = true; };
        };

        # nix develop
        devShells.default = pkgs.callPackage ./nix/devshell.nix { };

        formatter = pkgs.nixpkgs-fmt;
        legacyPackages = pkgs;
      });
}
