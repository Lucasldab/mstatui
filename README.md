# mstatui

Terminal UI for [ListenBrainz](https://listenbrainz.org) listen stats.
Recent plays, top artists/tracks/releases, all from your scrobble history.

Built for the kitty mini-window pattern: launch, glance, close.

## Install

```sh
cargo install --path .
```

Or build and copy:

```sh
cargo build --release
install -m 755 target/release/mstatui ~/.local/bin/
```

## Run

```sh
mstatui
```

`username` and `token` are read in this order:

1. `~/.config/mstatui/config.toml`
2. `MSTATUI_USERNAME` / `MSTATUI_TOKEN` env vars
3. `~/.local/share/mpris-scrobbler/credentials` (the `[listenbrainz]` section)

## Config

Copy `config.example.toml` to `~/.config/mstatui/config.toml` and edit.

Every field is optional. See the example file for defaults.

## Keybindings (default, vim-style)

| key | action |
|---|---|
| `q` / `Esc` | quit |
| `h` / `l` | prev / next tab |
| `j` / `k` | move down / up |
| `g` / `G` | top / bottom |
| `t` | cycle range (week → month → year → all) |
| `r` | refresh |
| `Enter` | open selected on MusicBrainz |

## License

MIT
