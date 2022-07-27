FROM rust:latest AS build-env
WORKDIR /src/butler
COPY . /src/butler
RUN cargo build --release

FROM debian:latest
COPY --from=build-env /src/butler/target/release/butler /bin/butler
ENTRYPOINT ["/bin/butler"]
