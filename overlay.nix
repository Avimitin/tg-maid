final: prev:

{
  myRustToolchain = final.rust-bin.stable.latest.default.override {
    extensions = [ "rust-src" ];
  };
  myFont = final.callPackages ./nix/quote-font.nix { };

  tg-maid = final.lib.makeScope final.newScope (self: {
    version = "unstable-2024-02-08";
    docker-image = self.callPackage ./nix/docker-image.nix { };
    bot = self.callPackage ./nix/tg-maid.nix { };
  });
}
