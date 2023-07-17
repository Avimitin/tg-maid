{ pkgs, fonts, rust-toolchain }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rust-toolchain
    # rust-analyzer comes from nixpkgs toolchain, I want the unwrapped version
    rust-analyzer-unwrapped
    yt-dlp
    # Dependency for yt-dlp
    ffmpeg
    # Redis instance for debugging
    redis
    # In case someone want to commit inside the nix shell but got a version mismatch openssl
    git
  ];

  buildInputs = [ pkgs.openssl ];

  shellHook = ''
    #
    # ENV
    #
    # To make rust-analyzer work correctly (The path prefix issue)
    export RUST_SRC_PATH="${rust-toolchain}/lib/rustlib/src/rust/library"

    # Export font path to be include
    export QUOTE_TEXT_FONT_PATH=${fonts.bold}
    export QUOTE_USERNAME_FONT_PATH=${fonts.light}

    # Initialize redis-server
    workdir=$PWD/.cache/redis
    mkdir -p $workdir
    port=16379
    pidfile="$workdir/redis_$port.pid"

    echo \
    "
    daemonize yes
    dir $workdir
    bind 127.0.0.1 -::1
    port $port
    pidfile $pidfile
    " | redis-server -

    # Kill redis-server on exit
    trap \
      "
        redis-cli -h 127.0.0.1 -p 16379 shutdown
        [[ -f $pidfile ]] && kill $(cat $pidfile)
        [[ -d $workdir ]] && rm -r $workdir
      " \
    EXIT
  '';
}
