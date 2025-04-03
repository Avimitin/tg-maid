# TG Maid

My utility bot.

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
