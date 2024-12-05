{ rustPlatform
, pkg-config
, mold
, openssl
, quote-fonts

  # dev deps
, rust-analyzer-unwrapped
, yt-dlp
, ffmpeg
, redis
, git
}:
let
  self = rustPlatform.buildRustPackage
    {
      name = "tg-maid";

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

      env = {
        QUOTE_TEXT_FONT_PATH = quote-fonts.bold;
        QUOTE_USERNAME_FONT_PATH = quote-fonts.light;
      };

      passthru.devShell = self.overrideAttrs (old: {

        nativeBuildInputs = old.nativeBuildInputs ++ [
          rust-analyzer-unwrapped
          yt-dlp
          ffmpeg
          redis
          git
        ];

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
