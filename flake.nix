{
  description = "Flakes for Rust development";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        myOverlay = import ./overlay.nix;
        overlays = [ myOverlay ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        # nix develop
        devShells.default = pkgs.tg-maid.bot.devShell;

        # nix fmt
        formatter = pkgs.nixpkgs-fmt;

        legacyPackages = pkgs;
        overlays.default = myOverlay;
      });
}
