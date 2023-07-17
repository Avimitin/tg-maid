{ pkgs, version, tg-maid }:
let
  # Specify a dir for user to easily mount volume
  workdir = "/app";
in
pkgs.dockerTools.streamLayeredImage {
  name = "ghcr.io/Avimitin/tg-maid";
  tag = version;

  contents = with pkgs; [
     cacert
     yt-dlp
     ffmpeg
  ];

  fakeRootCommands = ''
    mkdir -p ${workdir} /tmp
  '';
  enableFakechroot = true;

  config = {
    env = [
      "TG_MAID_CFG_PATH=${workdir}/config.toml"
    ];
    cmd = [ "${tg-maid}/bin/tgbot" ];
    healthcheck = {
      test = [
        "CMD-SHELL"
        "${pkgs.netcat-openbsd}/bin/nc -z 127.0.0.1 11451 || exit 1"
      ];
    };
  };
}
