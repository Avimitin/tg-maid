{ makeRustPlatform
, myRustToolchain
, pkg-config
, mold
, openssl
, myFont
, version

  # dev deps
, rust-analyzer-unwrapped
, yt-dlp
, ffmpeg
, redis
, git
}:
let
  myRustPlatform = makeRustPlatform {
    cargo = myRustToolchain;
    rustc = myRustToolchain;
  };

  self = myRustPlatform.buildRustPackage
    {
      pname = "tg-maid";
      inherit version;

      src = ../.;

      # Build time & Runtime dependencies
      nativeBuildInputs = [ pkg-config mold ];
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

      passthru.rustPlatform = myRustPlatform;
      passthru.devShell = self.overrideAttrs (old: {

        nativeBuildInputs = old.nativeBuildInputs ++ [
          rust-analyzer-unwrapped
          yt-dlp
          ffmpeg
          redis
          git
        ];

        env = {
          # To make rust-analyzer work correctly (The path prefix issue)
          RUST_SRC_PATH = "${myRustToolchain}/lib/rustlib/src/rust/library";
        };

        shellHook = ''
          redisWorkDir="./.cache/redis"
          redisServerPort="16379"
          redisPidFile="$redisWorkDir/redis_$redisServerPort.pid";

          mkdir -p $redisWorkDir
          echo "
          daemonize yes
          dir $redisWorkDir
          bind 127.0.0.1 -::1
          port $redisServerPort
          pidfile $redisPidFile
          " | redis-server -

          shutdown() {
            redis-cli -h 127.0.0.1 -p $redisServerPort shutdown
            [[ -f $redisPidFile ]] && kill $(cat $redisPidFile)
            [[ -d $redisWorkDir ]] && rm -r $redisWorkDir
            echo "Redis server shutdown"
          }
          trap shutdown EXIT

          alias redis-cli="redis-cli -h 127.0.0.1 -p $redisServerPort"

          echo "[nix-shell] redis server listen at $redisServerPort"
          echo "[nix-shell] redis-cli alias to 'redis-cli -h 127.0.0.1 -p $redisServerPort'"
        '';
      });
    };
in
self
