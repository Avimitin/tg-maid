# TG Maid

My utility bot.

## Apple Music concert previews

Posting an Apple Music concert URL triggers a PNG card rendered in Rust using the
bundled CJK fonts. The card shows the event title, listed artist, listed time with
timezone or UTC offset, venue, and available address. Its concise caption links to
the artist, ticket booking (when provided), and the Apple Music event.
Apple's share artwork (circular artist photo with its calendar badge) appears on
the left, with event details on the right against a matching light background.
If artwork is missing or unavailable, a simple music placeholder is used.

Preview detection runs even when URL cleanup makes no changes or fails. Duplicate
event IDs across storefronts are handled once per message. Successful extraction
is cached in memory for 30 minutes (up to 256 events); later messages still receive
a preview. Extraction or rendering failures fall back to normal URL cleanup:
changed links are sent, unchanged links produce no reply.

The extractor reads public MusicEvent JSON and concert sections embedded in Apple's
page, with a US storefront retry. It verifies the event ID and rejects redirects to
unrelated pages. Apple's HTML format can change, and regional redirects can prevent
retrieval. Set `apple_music = true` under `[proxy]` to use the configured default
proxy, or provide a proxy URL directly. No browser or Apple developer key is required.
The listed time is not labeled as doors or performance start, and missing artists or
ticket URLs are not inferred from the event title.

To generate a card locally without sending Telegram messages:

```bash
nix develop
cargo run --example concert_preview -- \
  'https://music.apple.com/jp/concerts/ce.1b03d66d-296c-49f0-b2ec-7984ecc4d085' \
  /tmp/concert.png
```

The example accepts `APPLE_MUSIC_PROXY` for its optional proxy. Run the offline
preview tests with `cargo test apple_music --lib` and
`cargo test url_preview_tests --bin tgbot`. Parser fixtures are representative
samples of Apple's published page format, not captured live responses.

## Run with docker-compose (Recommended)

```bash
curl -SL https://raw.githubusercontent.com/Avimitin/tg-butler/master/example.docker-compose.yml \
     -o docker-compose.yml
mkdir app
# Detail configuration see below
vim ./app/config.toml
docker-compose up -d
```

## Configuration

The bot will try to read configuration from a `config.toml` file under directory
`$XDG_CONFIG_HOME/tg_maid` or `$HOME/.config/tg_maid`.
You can also manually specify the config file path by environment variable `TG_MAID_CFG_PATH`.
The content of the file should followed the [TOML spec](https://toml.io/en/).

The below table describe all the configuration:

- Top Level

| Key               | Value Type         | Docs                                                                  |
|-------------------|--------------------|-----------------------------------------------------------------------|
| bot_token         | String             | Token for the Telegram Bot                                            |
| redis_addr        | String             | An URL prefixed with `redis://` that can be connect to a redis daemon |
| log_level         | String (Optional)  | Unused now                                                            |
| health_check_port | int_u16 (Optional) | Port number for docker to check the bot alive or not                  |

> Notice: if you are using docker-compose, set the `redis_addr` to `redis://${service}:${port}` where `${service}`
> is your redis service name in docker-compose.yml. In my example.docker-compose.yml it is `cache`.

- DeepL Translate: `[deepl]`

| Key     | Value Type | Docs                           |
|---------|------------|--------------------------------|
| api_key | String     | API Key for DeepL authenticate |

- Bilibili Live Room Event: `[bili_live_room_event]`

| Key                       | Value Type                                              | Docs                                                             |
|---------------------------|---------------------------------------------------------|------------------------------------------------------------------|
| String (Telegram Chat ID) | `List[Number]` (List of Streamer **UID** Not Room ID!!) | Per chat configuration for notifying bilibili live stream status |

- Proxy (Optional) : `proxy`

| Key      | Value Type                | Docs                                                                                                                                                                                       |
|----------|---------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| default  | String (Optional)         | Proxy URL                                                                                                                                                                                  |
| telegram | String or bool (Optional) | When filled in as a string, this string is used as the proxy URL, when filled in as `true`, the `default` URL is used as the proxy, which will be invalid if the `default` does not exist. |
| youtube  | String or bool (Optional) | When filled in as a string, this string is used as the proxy URL, when filled in as `true`, the `default` URL is used as the proxy, which will be invalid if the `default` does not exist. |
| deepl    | String or bool (Optional) | When filled in as a string, this string is used as the proxy URL, when filled in as `true`, the `default` URL is used as the proxy, which will be invalid if the `default` does not exist. |
| bilibili | String or bool (Optional) | When filled in as a string, this string is used as the proxy URL, when filled in as `true`, the `default` URL is used as the proxy, which will be invalid if the `default` does not exist. |
| apple_music | String or bool (Optional) | Proxy for fetching Apple Music concert pages; `true` uses `default`. |

> Fill in this option if you need to use a web proxy because your network cannot access certain services directly.

Below is an example configuration:

```toml
bot_token = "abcde"
redis_addr = "redis://localhost"
log_level = "INFO"
health_check_port = 11451

[deepl]
api_key = "abcde"

[bili_live_room_event]
"-10012345" = [ 1000, 2000, 3000 ]
"-10054321" = [ 1000, 2000, 3000 ]

# optional
[proxy]
default = "http://127.0.0.1:7890"
# use default URL for some services
telegram = true
# use another proxy for some services
deepl = "http://127.0.0.1:7891"
```

## How to build

### Docker

```bash
cd tg-maid
docker buildx build -t avimitin/tg-maid:latest .
```

### Normal Build

Require dependency:

  - rust
  - pkg-config
  - openssl
  - noto-sans-cjk
  - openbsd-netcat

```bash
cargo build --release
```

### Nix flake

If you don't know how to setup the build environment,
you can follow the [`Nix` installation guide](https://nixos.org/manual/nix/stable/installation/installing-binary.html),
or install nix through your system package manager:

```bash
pacman -S nix
```

Then enable the [`flakes`](https://nixos.wiki/wiki/Flakes#Enable_flakes) feature
and run the following command to get the development shell:

```bash
nix develop
```

You can now use your favourite editor to start contributing to this project:

```bash
$EDITOR .
```

To build the bot executable:

```bash
nix build .#tg-maid.bot
./result/bin/tgbot
```

To build the docker image:

```bash
nix run .#tg-maid.docker-image | docker load
```

## TODO

- [x] Read config from file
- [x] Implement the make quote functionality
- [ ] Get random restaurant suggestion from DianPing
- [x] New command `/roll [range]`
- [x] Add Nix flake
