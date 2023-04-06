FROM debian:bookworm-slim

COPY target/release/tgbot /usr/bin/tgmaid

RUN apt-get update -yy && apt-get install -yy \
      --no-install-recommends \
      ca-certificates \
      netcat-openbsd \
      && apt-get clean \
      && rm -rf /var/lib/apt/lists/*

HEALTHCHECK CMD nc -z 127.0.0.1 11451 || exit 1
ENTRYPOINT ["/usr/bin/tgmaid"]
