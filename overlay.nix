final: prev:

{
  tg-maid = final.lib.makeScope final.newScope (self: {
    quote-fonts = final.callPackages ./nix/quote-font.nix { };
    bot = self.callPackage ./nix/tg-maid.nix { };
  });
}
