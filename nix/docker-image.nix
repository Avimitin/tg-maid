{ pkgs, version, tg-maid }:
let
  # Bot links to openssl at build time
  LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [ openssl ];

  bot_config = pkgs.writeTextFile {
    name = "config.toml";
    text = ''
      bot_token = "abcd"
      redis_addr = "redis://localhost:6379"
      log_level = "INFO"
      health_check_port = 11451

      [deepl]
      api_key = "abcd"

      [osu]
      client_id = 0
      client_secret = "abcd"

      [bili_live_room_event]

      [osu_user_activity_event]
    '';
  };

  # Specify a dir for user to easily mount volume
  workdir = "/app";
  bot_cfg_path = "${workdir}/config.toml";
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

  runAsRoot = ''
    #!${pkgs.runtimeShell}
    mkdir -p ${workdir}
    cp ${bot_config} ${bot_cfg_path}
  '';

  config = {
    env = [
      "LD_LIBRARY_PATH=${LD_LIBRARY_PATH}"
      "TG_MAID_CFG_PATH=${bot_cfg_path}"
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
