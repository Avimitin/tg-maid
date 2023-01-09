FROM rust:latest AS build-env
WORKDIR /src/butler
COPY . /src/butler
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=build-env /src/butler/target/release/rusty-maid /bin/maid
RUN apt-get update && apt-get install -y \
      --no-install-recommends \
      netcat-openbsd \
      ca-certificates \
      && apt-get clean \
      && rm -rf /var/lib/apt/lists/*
HEALTHCHECK CMD nc -z 127.0.0.1 11451 || exit 1
ENTRYPOINT ["/bin/maid"]
