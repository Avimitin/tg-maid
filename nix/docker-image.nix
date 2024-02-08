{ lib
, version
, dockerTools
, bot
, cacert
, yt-dlp
, ffmpeg
, netcat-openbsd

, bash
, writeShellScriptBin
}:

let
  self = dockerTools.streamLayeredImage rec {
    name = "ghcr.io/avimitin/tg-maid";
    tag = version;

    contents = [ cacert yt-dlp ffmpeg ];

    fakeRootCommands = ''
      mkdir -p /app /tmp
    '';
    enableFakechroot = true;

    config = {
      env = [ "TG_MAID_CFG_PATH=/app/config.toml" ];
      cmd = [ "${bot}/bin/tgbot" ];
      healthcheck = {
        test = [
          "CMD-SHELL"
          "${netcat-openbsd}/bin/nc -z 127.0.0.1 11451 || exit 1"
        ];
      };
    };

    passthru = {
      finalizer = writeShellScriptBin "finalize-docker-image" ''
        set -e; set -o pipefail
        # Try verify docker exists and daemon is reachable
        # This is impure, but it is flaky to let docker CLI in nixpkgs to interact with docker daemon from platform that I don't know.
        # So let's enforce user install docker on their platform.
        docker info

        ${bash}/bin/bash ${self} | docker load --quiet

        [ -n "$LATEST_IMAGE"  ] && docker image tag ${name}:${tag} ${name}:latest
        [ -n "$PUSH_IMAGE" ] && docker push ${name}:${tag}

        [ -n "$LATEST_IMAGE"  ] && [ -n "$PUSH_IMAGE" ] && docker push ${name}:latest
      '';
    };
  };
in
self
