# 🎉 Party Night

Party games for game night. Put it on the TV, everyone joins from their phone. No app to install on the phones, no accounts. Just a room code (or scan the QR code) and you're in.

## Games

- **Punchline Club**: everyone hands in a few random words, then writes jokes using other people's words. The jokes go head to head and the room votes.
- **Quick Wit**: everyone answers the same silly prompt, then votes for the best answer.

Both come with a family pack and a spicy pack. Up to 20 players, with bots to fill the room if you're short on people.

## Playing

1. Download the latest installer from [Releases](https://github.com/veneito/party-night/releases) and run it.
2. Open Party Night on the computer hooked up to your TV.
3. Phones on the same Wi-Fi scan the QR code or go to the address on screen and type the room code.
4. Pick a game, change the settings if you want, and hit start.

F11 toggles fullscreen. The ⚙️ button has sound, voice-over and update settings. The app checks for updates when it opens and asks before installing one.

If phones can't connect, make sure they're on the same Wi-Fi as the computer and not a guest network. The installer adds a Windows Firewall rule for the game, so allow it if Windows asks.

## Running from source

You need [Node.js](https://nodejs.org) 18 or newer.

```
npm install
npm start          # desktop app
npm run server     # just the server, open http://localhost:3000/tv in a browser
```

`http://localhost:3000/test` opens a test page with the TV and a few phones side by side, which is the easiest way to try things out alone.

To build the Windows installer: `npm run dist` (it ends up in `dist/`).

## How it fits together

- `server.js` runs the rooms, timers and bots. Phones and the TV get live updates over server-sent events and send actions with plain POST requests.
- `games/` has one file per game. Each game is a small module with its phases, settings and bot behavior, so adding a new one doesn't touch the rest.
- `public/` is the TV screen, the phone screen and the shared styles. No build step, no framework.
- `app.js` wraps it all in an Electron window and handles updates.

## Releasing

Bump `version` in `package.json`, then run with a GitHub token that can write to the repo:

```
GH_TOKEN=... npm run release
```

That builds the installer and uploads it to a new GitHub release. Installed copies pick it up the next time they open.

## License

MIT
