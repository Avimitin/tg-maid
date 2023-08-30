{ myRustPlatform, pkg-config, llvmPackages_16, openssl, myFont, version }:
myRustPlatform.buildRustPackage {
  pname = "tg-maid";
  inherit version;

  src = ../.;

  # Build time & Runtime dependencies
  nativeBuildInputs = [ pkg-config llvmPackages_16.bintools ];
  # Link time dependencies
  buildInputs = [ openssl ];

  cargoLock = {
    lockFile = ../Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  # Some test require proper env, which is not available during build
  doCheck = false;

  # Export font path
  QUOTE_TEXT_FONT_PATH = myFont.bold;
  QUOTE_USERNAME_FONT_PATH = myFont.light;
}

