{ pkgs, name, tag, executable }:
let
  # Specify a dir for user to easily mount volume
  workdir = "/app";
in
pkgs.dockerTools.streamLayeredImage {
  inherit name tag;

  contents = with pkgs; [ cacert yt-dlp ffmpeg ];

  fakeRootCommands = ''
    mkdir -p ${workdir} /tmp
  '';
  enableFakechroot = true;

  maxLayers = 25;

  config = {
    env = [ "TG_MAID_CFG_PATH=${workdir}/config.toml" ];
    cmd = [ executable ];
    healthcheck = {
      test = [
        "CMD-SHELL"
        "${pkgs.netcat-openbsd}/bin/nc -z 127.0.0.1 11451 || exit 1"
      ];
    };
  };
}
