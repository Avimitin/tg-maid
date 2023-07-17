{ pkgs, do_push ? false, is_latest ? false, name, tag }:
with pkgs.lib.strings;

# Load tarball into docker images
pkgs.writeScript "finalize-docker-image" (''
  #!${pkgs.bash}/bin/bash
  set -e; set -o pipefail
  # Try verify docker exists and daemon is reachable
  docker info

  nix build --print-out-paths '.#docker' \
  | docker load --quiet
'' +
  # If this is the latest build, add new tag for it
  optionalString is_latest ''
    docker image tag ${name}:${tag} ${name}:latest
  '' +
  # If we need to run the push
  optionalString do_push ''
    docker push ${name}:${tag}
  '' +
  # If we need to run the push and it is the latest build
  optionalString (do_push && is_latest) ''
    docker push ${name}:latest
  '')
