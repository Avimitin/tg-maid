final: prev:
rec {
  myRustToolchain = final.rust-bin.stable.latest.default.override {
    extensions = [ "rust-src" ];
  };
  myRustPlatform = final.makeRustPlatform {
    cargo = myRustToolchain;
    rustc = myRustToolchain;
  };
  myFont = final.callPackages ./nix/quote-font.nix { };
}
