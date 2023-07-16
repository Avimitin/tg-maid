{ pkgs, version, tg-maid }:
let
  # Bot links to openssl at build time
  LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [ openssl ];
in
pkgs.dockerTools.streamLayeredImage {
  name = "ghcr.io/Avimitin/tg-maid";
  tag = version;

  contents = with pkgs; [
     cacert
     redis
     yt-dlp
     ffmpeg
     my-maid-pkg
  ];

  config = {
    env = [ ''LD_LIBRARY_PATH=${LD_LIBRARY_PATH}'' ];
    cmd = [ "${tg-maid}/bin/tgbot" ];
    healthcheck = {
      test = [
        "CMD-SHELL"
        "${pkgs.netcat-openbsd}/bin/nc -z 127.0.0.1 11451 || exit 1"
      ];
    };
  };
}
