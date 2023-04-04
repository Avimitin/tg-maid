FROM archlinux:latest

# Prepare
WORKDIR /build/src/rusty-maid
COPY src /build/src/rusty-maid/src/
COPY Cargo.toml Cargo.lock /build/src/rusty-maid/

RUN pacman -Syu --noconfirm --needed \
      base-devel \
      rust \
      noto-fonts-cjk \
      openbsd-netcat
RUN cargo fetch --locked

# Build
RUN CARGO_BUILD_JOBS=$(nproc) \
      cargo build --release --frozen

# Package
RUN cp /build/src/rusty-maid/target/release/tgbot /usr/bin/rusty-maid
RUN rm -rf /build
RUN pacman -Rs --noconfirm base-devel rust noto-fonts-cjk
RUN pacman -Scc --noconfirm

# Run
HEALTHCHECK CMD nc -z 127.0.0.1 11451 || exit 1
ENTRYPOINT ["/usr/bin/rusty-maid"]
