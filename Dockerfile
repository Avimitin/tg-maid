FROM archlinux:latest

# Prepare
WORKDIR /build/src/rusty-maid
COPY src Cargo.toml Cargo.lock /build/src/rusty-maid

RUN pacman -Syu --noconfirm --needed \
      noto-fonts-cjk \
      openbsd-netcat
RUN pacman -Scc --noconfirm
RUN cargo fetch --locked

# Build
RUN cargo build --release --frozen

# Package
RUN cp /build/src/rusty-maid/target/release/tgbot /usr/bin/rusty-maid
RUN rm -rf /build

# Run
HEALTHCHECK CMD nc -z 127.0.0.1 11451 || exit 1
ENTRYPOINT ["/usr/bin/rusty-maid"]
