#!/bin/bash

set -e
set -o pipefail;

REPO=""
IMAGE=""

build() {
  local layer_script=$(nix build --print-out-paths '.#docker')
  local image=$($layer_script \
          | docker load --quiet \
          | sed -n '$s/^Loaded image: //p')
  local repo=$(echo $image | cut -d':' -f1)
  docker image tag "$image" "$repo:latest"

  REPO=$repo
  IMAGE=$image
}

push() {
  docker push $IMAGE
  docker push "$REPO:latest"
}

build && push
