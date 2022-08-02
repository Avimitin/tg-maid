FROM rust:latest AS build-env
WORKDIR /src/butler
COPY . /src/butler
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=build-env /src/butler/target/release/butler /bin/butler
RUN apt-get update && apt-get install -y \
      --no-install-recommends \
      ca-certificates \
      && apt-get clean \
      && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["/bin/butler"]
