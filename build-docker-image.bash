#!/bin/bash

set -e
set -o pipefail;

PUSH_IMG=${PUSH_IMG:-0}

build() {
  local layer_script=$(nix build --print-out-paths '.#docker')
  local image=$($layer_script \
          | docker load --quiet \
          | sed -n '$s/^Loaded image: //p')
  local repo=$(echo $image | cut -d':' -f1)
  docker image tag "$image" "$repo:latest"

  if (( $PUSH_IMG )); then
    push $image
    push "$repo:latest"
  fi
}

push() {
  local image=$1; shift
  docker push $image
}

build
