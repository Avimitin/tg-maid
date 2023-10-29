{ version }:
let
  imageName = "ghcr.io/avimitin/tg-maid";
in
{
  builder =
    { dockerTools
    , tg-maid
    , cacert
    , yt-dlp
    , ffmpeg
    , netcat-openbsd
    }: dockerTools.streamLayeredImage {
      name = imageName;
      tag = version;

      contents = [ cacert yt-dlp ffmpeg ];

      fakeRootCommands = ''
        mkdir -p /app /tmp
      '';
      enableFakechroot = true;

      config = {
        env = [ "TG_MAID_CFG_PATH=/app/config.toml" ];
        cmd = [ "${tg-maid}/bin/tgbot" ];
        healthcheck = {
          test = [
            "CMD-SHELL"
            "${netcat-openbsd}/bin/nc -z 127.0.0.1 11451 || exit 1"
          ];
        };
      };
    };

  finalizer =
    { writeShellScriptBin
    , lib
    , isLatest ? false
    , doPush ? false
    }: with lib.strings; writeShellScriptBin "finalize-docker-image" (''
      set -e; set -o pipefail
      # Try verify docker exists and daemon is reachable
      docker info

      $(nix build --print-out-paths --no-link --print-build-logs '.#docker-builder') \
      | docker load --quiet
    '' +
    # If this is the latest build, add new tag for it
    optionalString isLatest ''
      docker image tag ${imageName}:${version} ${imageName}:latest
    '' +
    # If we need to run the push
    optionalString doPush ''
      docker push ${imageName}:${version}
    '' +
    # If we need to run the push and it is the latest build
    optionalString (doPush && isLatest) ''
      docker push ${imageName}:latest
    '');
}
