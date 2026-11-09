# 🎉 Party Night

Party games for game night. Put it on the TV, everyone joins from their phone. No app to install on the phones, no accounts. Just a room code (or scan the QR code) and you're in.

## Games

- **Punchline Club**: everyone hands in a few random words, then writes jokes using other people's words. The jokes go head to head and the room votes.
- **Quick Wit**: everyone answers the same silly prompt, then votes for the best answer.

Both come with a family pack and a spicy pack. Up to 20 players, with bots to fill the room if you're short on people.

## Playing

1. Download the latest installer from [Releases](https://github.com/veneito/party-night/releases) and run it. On Linux or SteamOS (desktop mode), download the `.AppImage`, mark it executable and open it.
2. Open Party Night on the computer hooked up to your TV.
3. Phones on the same Wi-Fi scan the QR code or go to the address on screen and type the room code.
4. Pick a game, change the settings if you want, and hit start.

F11 toggles fullscreen. The ⚙️ button has sound, voice-over and update settings. The app checks for updates when it opens and asks before installing one.

If phones can't connect, make sure they're on the same Wi-Fi as the computer and not a guest network. The installer adds a Windows Firewall rule for the game, so allow it if Windows asks.

## Running from source

You need [Rust](https://rustup.rs) and the Tauri CLI (`cargo install tauri-cli`).

```
cd src-tauri
cargo tauri dev            # desktop app
cargo run -- --server      # just the server, open http://localhost:3000/tv in a browser
cargo test                 # game rule checks
```

`http://localhost:3000/test` opens a test page with the TV and a few phones side by side, which is the easiest way to try things out alone (`start.bat` does this for you). Set `TIMER_SCALE=0.2` to make every countdown 5x shorter.

To build the Windows installer: `cargo tauri build` in `src-tauri` (it ends up in `src-tauri/target/release/bundle/nsis/`).

## How it fits together

- `src-tauri/src/server.rs` is the web server. Phones and the TV get live updates over server-sent events and send actions with plain POST requests.
- `src-tauri/src/room.rs` runs the rooms, timers and bots.
- `src-tauri/src/games/` has one file per game: its phases, settings and bot behavior, so adding a new one doesn't touch the rest. Prompts and word lists live in `src-tauri/data/games.json`.
- `public/` is the TV screen, the phone screen and the shared styles. No build step, no framework. They're built into the app, so rebuild after changing them.
- `src-tauri/src/main.rs` wraps it all in a Tauri window and handles updates.

## Releasing

Bump `version` in `src-tauri/tauri.conf.json` (and `src-tauri/Cargo.toml`), commit, then push a matching tag:

```
git tag v1.4.0
git push origin v1.4.0
```

GitHub Actions builds the Windows installer and the Linux AppImage (works on SteamOS in desktop mode) and publishes them with `latest.json` as a GitHub release. Installed copies pick it up the next time they open.

The build signs updates with the key in the repo secret `TAURI_SIGNING_PRIVATE_KEY` (the contents of `~/.tauri/party-night.key`). Keep a backup of that key: without it, installed copies can't be updated.

## License

MIT
