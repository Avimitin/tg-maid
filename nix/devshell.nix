{ mkShell, writeShellScriptBin, rust-analyzer-unwrapped, yt-dlp, ffmpeg, redis, git, openssl, llvmPackages_16, myFont, myRustToolchain }:
let
  redisWorkDir = "./.cache/redis";
  redisServerPort = "16379";
  redisPidFile = "${redisWorkDir}/redis_${redisServerPort}.pid";

  startUpRedisScript = writeShellScriptBin "startup-redis" ''
    # Initialize redis-server
    mkdir -p ${redisWorkDir}
    echo \
    "
    daemonize yes
    dir ${redisWorkDir}
    bind 127.0.0.1 -::1
    port ${redisServerPort}
    pidfile ${redisPidFile}
    " | redis-server -
  '';
  shutdownRedisScript = writeShellScriptBin "shutdown-redis" ''
    redis-cli -h 127.0.0.1 -p ${redisServerPort} shutdown
    [[ -f ${redisPidFile} ]] && kill $(cat ${redisPidFile})
    [[ -d ${redisWorkDir} ]] && rm -r ${redisWorkDir}
  '';
  redisCliWrapper = writeShellScriptBin "rcli" ''
    [[ -f ${redisPidFile} ]] || \
      (echo "Run startup-redis to start a redis server" && exit 1)
    redis-cli -h 127.0.0.1 -p ${redisServerPort}
  '';
in
mkShell {
  nativeBuildInputs = [
    myRustToolchain
    # rust-analyzer comes from nixpkgs toolchain, I want the unwrapped version
    rust-analyzer-unwrapped
    yt-dlp
    # Dependency for yt-dlp
    ffmpeg
    # Redis instance for debugging
    redis
    # In case someone want to commit inside the nix shell but got a version mismatch openssl
    git
    # Use LLD
    llvmPackages_16.bintools

    # Helper script
    startUpRedisScript
    shutdownRedisScript
    redisCliWrapper
  ];

  buildInputs = [ openssl ];

  env = {
    # To make rust-analyzer work correctly (The path prefix issue)
    RUST_SRC_PATH = "${myRustToolchain}/lib/rustlib/src/rust/library";
    # Export font path to be include
    QUOTE_TEXT_FONT_PATH = "${myFont.bold}";
    QUOTE_USERNAME_FONT_PATH = "${myFont.light}";
  };
}
