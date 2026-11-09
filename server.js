// Party game server, zero dependencies. TV opens /tv (picks a game, gets a room code), phones open / on the same Wi-Fi.
// /test shows a TV plus a few phones in one window; "Add bots" in the lobby fills the room for solo testing.
// Each game mode lives in games/*.js; see games/quickwit.js for the smallest example.
const http = require('http'), fs = require('fs'), os = require('os'), path = require('path');
let PORT = Number(process.env.PORT) || 3000;
const MAX_PLAYERS = 20;

const pick = a => a[Math.floor(Math.random() * a.length)];
const shuffle = a => a.map(v => [Math.random(), v]).sort((x, y) => x[0] - y[0]).map(x => x[1]);
const clean = (v, n) => String(v || '').trim().slice(0, n);

// Settings every game gets; games add their own in `settings`. The first option is the default.
const CORE_SETTINGS = [
  { key: 'speed', label: 'Timers', options: [['normal', 'Normal'], ['relaxed', 'Relaxed'], ['fast', 'Fast'], ['off', 'No timer']] },
  { key: 'rounds', label: 'Rounds', options: [[3, '3'], [1, '1'], [2, '2'], [5, '5']] },
  { key: 'voice', label: 'Voice-over', options: [['on', 'On'], ['off', 'Off']] },
  { key: 'max', label: 'Max players', options: [[20, '20'], [8, '8'], [12, '12'], [16, '16']] },
  { key: 'home', label: 'Home button', options: [['vote', 'Players vote next'], ['menu', 'Back to menu']] },
];
const SPEED = { normal: 1, relaxed: 1.5, fast: 0.6, off: 0 };
// TIMER_SCALE=0.2 makes every countdown 5x shorter, handy when testing alone
const SCALE = Number(process.env.TIMER_SCALE) || 1;
function setTimer(g, secs, next, always) {
  clearTimeout(g.timer);
  // "No timer" still lets short reveal screens move on by themselves
  const k = SPEED[g.settings?.speed] ?? 1;
  secs = secs && (k || always) && Math.max(1, Math.round(secs * (k || 1) * SCALE));
  g.endsAt = secs ? Date.now() + secs * 1000 : 0;
  g.timer = secs ? setTimeout(() => { next(g); broadcast(g); }, secs * 1000) : null;
}
const finish = g => {
  if (g.round < (g.settings.rounds || 3)) { g.phase = 'scores'; setTimer(g, 15, nextRound, true); }
  else { g.phase = 'results'; setTimer(g, 0); }
};
function nextRound(g) {
  addBots(g, g.pendingBots); g.pendingBots = 0;
  g.round++; game(g).start(g);
}

const GAMES = Object.fromEntries(fs.readdirSync(path.join(__dirname, 'games')).filter(f => f.endsWith('.js')).map(f => {
  const m = require(path.join(__dirname, 'games', f));
  m.init({ pick, shuffle, clean, setTimer, finish });
  m.allSettings = [...CORE_SETTINGS, ...(m.settings || [])];
  return [m.id, m];
}));

const rooms = {}; // code -> room
const LETTERS = 'ABCDEFGHJKLMNPQRSTUVWXYZ'; // no I/O, easy to read on a TV
function createRoom() {
  let code;
  do code = Array.from({ length: 4 }, () => pick(LETTERS)).join(''); while (rooms[code]);
  rooms[code] = { code, phase: 'menu', game: null, settings: {}, players: {}, pendingBots: 0, round: 0, clients: new Set(), timer: null, endsAt: 0, lastSeen: Date.now() };
  return code;
}
// drop rooms nobody has touched for 2 hours
setInterval(() => { for (const [c, g] of Object.entries(rooms))
  if (!g.clients.size && Date.now() - g.lastSeen > 2 * 3600e3) { clearTimeout(g.timer); delete rooms[c]; } }, 600e3).unref();

const game = g => GAMES[g.game];
const inGame = g => !['menu', 'lobby', 'scores', 'results', 'gamevote'].includes(g.phase);
// "What's next?": Home opens a vote on every phone: replay this game or pick another. Most votes wins, ties random.
function startGameVote(g) { setTimer(g, 0); g.phase = 'gamevote'; g.gvotes = {}; setTimer(g, 20, endGameVote, true); }
function endGameVote(g) {
  const n = {}; for (const v of Object.values(g.gvotes)) n[v] = (n[v] || 0) + 1;
  const top = Math.max(0, ...Object.values(n)), win = top ? pick(Object.keys(n).filter(k => n[k] === top)) : g.game ? 'replay' : pick(Object.keys(GAMES));
  setTimer(g, 0);
  if (win === 'replay') { g.phase = 'lobby'; for (const p of Object.values(g.players)) p.score = 0; }
  else { g.phase = 'menu'; gameAct(g, { type: 'pick', game: win }); }
}
// each player gets the lowest free avatar slot (colour + animal), so 20 players are all distinct
const freeAvatar = g => { const used = new Set(Object.values(g.players).map(p => p.av)); let i = 0; while (used.has(i)) i++; return i; };

function publicState(g) {
  const G = game(g);
  fixHost(g);
  const s = { code: g.code, host: g.host, phase: g.phase, rounds: g.settings.rounds || 3, pendingBots: g.pendingBots, game: g.game, round: g.round, settings: g.settings,
    games: Object.values(GAMES).map(({ id, name, desc, min, tag, art, allSettings }) => ({ id, name, desc, min, tag, art, settings: allSettings })),
    // seconds left, not a timestamp, so phone clock skew doesn't matter
    left: g.endsAt ? Math.max(0, Math.ceil((g.endsAt - Date.now()) / 1000)) : 0,
    players: Object.entries(g.players).map(([id, p]) => ({ id, name: p.name, av: p.av, ph: p.photo ? `/photo/${g.code}/${encodeURIComponent(id)}?v=${p.photoV}` : undefined, bot: p.bot, score: p.score, done: g.phase === 'gamevote' ? id in g.gvotes : inGame(g) && G.isDone(g, id) })) };
  if (G) G.state(g, s);
  if (g.phase === 'gamevote') { s.gvotes = {}; for (const v of Object.values(g.gvotes)) s.gvotes[v] = (s.gvotes[v] || 0) + 1; }
  return s;
}
// The host is a player whose phone gets the TV's controls. First real player by default; passes on if they go.
function fixHost(g) { if (!g.players[g.host] || g.players[g.host].bot) g.host = Object.keys(g.players).find(id => !g.players[id].bot) || null; }
const broadcast = g => { const d = `data: ${JSON.stringify(publicState(g))}\n\n`; g.clients.forEach(c => c.write(d)); };

function act(a) {
  if (a.type === 'create') return { code: createRoom() };
  const g = rooms[clean(a.room, 4).toUpperCase()];
  if (!g) return { error: 'Room not found' };
  g.lastSeen = Date.now();
  if (a.type === 'ping') return { ok: true }; // connection check, changes nothing
  const err = gameAct(g, a);
  if (err) return { error: err };
  broadcast(g);
  return { ok: true };
}

function gameAct(g, a) {
  const id = clean(a.id, 40), me = g.players[id], G = game(g);
  const count = Object.keys(g.players).length;
  switch (a.type) {
    case 'join': {
      const name = clean(a.text, 16);
      if (!name) return 'Name required';
      if (me) { me.name = name; break; }
      if (inGame(g)) return 'Game in progress, join after this round';
      if (count + g.pendingBots >= maxOf(g)) return 'Room is full';
      g.players[id] = { name, score: 0, av: avatarFree(g, a.av, id) ? Number(a.av) : freeAvatar(g) };
      break;
    }
    case 'photo': // player uploads a picture (phone shrinks it to a small JPEG first), or clears it with ''
      if (!me) return 'Not now';
      if (a.data && (typeof a.data !== 'string' || a.data.length > 30000 || !PHOTO.test(a.data))) return "That picture didn't work, try another";
      me.photo = a.data || null; me.photoV = Date.now().toString(36); // unique per upload, so browsers never show a cached older photo
      break;
    case 'avatar': // player picks their emoji; one per player so everyone is easy to spot
      if (!me || inGame(g)) return 'Not now';
      if (!avatarFree(g, a.av, id)) return 'Someone already has that one';
      me.av = Number(a.av);
      break;
    case 'pick': // TV chooses a game from the menu
      if (g.phase !== 'menu' || !GAMES[a.game]) return 'Not now';
      g.game = a.game; g.phase = 'lobby'; g.round = 0;
      // fresh defaults per game, but the voice-over choice carries over
      g.settings = { ...Object.fromEntries(GAMES[a.game].allSettings.map(s => [s.key, s.options[0][0]])), ...(g.settings.voice && { voice: g.settings.voice }), ...(g.settings.home && { home: g.settings.home }) };
      for (const p of Object.values(g.players)) p.score = 0;
      break;
    case 'set': { // TV changes a setting in the lobby
      const def = (G?.allSettings || CORE_SETTINGS).find(s => s.key === a.key), opt = def?.options.find(o => o[0] === a.value);
      if ((g.phase !== 'lobby' && a.key !== 'voice' && a.key !== 'home') || !opt) return 'Not now';
      g.settings[a.key] = opt[0];
      break;
    }
    case 'start': // lobby "start", scoreboard "next round", final "play again"
      if (!['lobby', 'scores', 'results'].includes(g.phase)) return 'Not now';
      if (Object.keys(g.players).length + g.pendingBots < G.min) return `Need ${G.min}+ players`;
      if (g.phase !== 'scores') { g.round = 0; for (const p of Object.values(g.players)) p.score = 0; } // a fresh game
      nextRound(g); break;
    case 'menu': // Home: a player vote (if that setting is on and there's a game to replay), else straight to the menu
      if (g.game && g.phase !== 'gamevote' && g.phase !== 'lobby' && g.settings.home !== 'menu' && Object.keys(g.players).length) { startGameVote(g); break; }
      setTimer(g, 0); g.phase = 'menu'; g.game = null; break; // players stay joined
    case 'recode': { // TV "New code" on the menu: fresh code, everyone has to join again
      if (g.phase !== 'menu') return 'Not now';
      let code; do code = Array.from({ length: 4 }, () => pick(LETTERS)).join(''); while (rooms[code]);
      delete rooms[g.code]; g.code = code; rooms[code] = g; g.players = {}; g.pendingBots = 0;
      break;
    }
    case 'gamevote': // "Let players vote" from the menu (or anywhere): every phone picks the next game
      if (!Object.keys(g.players).length) return 'Nobody has joined yet';
      if (g.phase !== 'gamevote') startGameVote(g);
      break;
    case 'gvote': // a player's pick in the "what's next?" vote
      if (g.phase !== 'gamevote' || !me || (a.game === 'replay' ? !g.game : !GAMES[a.game])) return 'Not now';
      g.gvotes[id] = a.game;
      if (Object.keys(g.players).every(p => p in g.gvotes)) endGameVote(g);
      break;
    case 'skip': if (g.phase === 'gamevote') endGameVote(g); else if (g.phase === 'scores') nextRound(g); else if (inGame(g) && G.advance[g.phase]) G.advance[g.phase](g); break; // TV "time's up" / "next"
    case 'host': // host hands the crown to another player
      if (!g.players[clean(a.who, 40)] || g.players[clean(a.who, 40)].bot) return 'Pick a real player';
      g.host = clean(a.who, 40); break;
    case 'kick': // TV removes someone who stopped playing, any time
      delete g.players[clean(a.who, 40)];
      if (inGame(g) && Object.keys(g.players).every(p => G.isDone(g, p)) && G.advance[g.phase]) G.advance[g.phase](g);
      break;
    case 'bots': { // TV adds bots; mid-round they wait and join when the next round starts
      const n = Math.max(0, Math.min(Number(a.n) || 1, maxOf(g) - count - g.pendingBots));
      if (inGame(g)) g.pendingBots += n; else addBots(g, n);
      break;
    }
    case 'unbot': { // TV removes bots (n of them, or all), any time
      if (g.pendingBots) { const k = Math.min(g.pendingBots, Number(a.n) || Infinity); g.pendingBots -= k; if (a.n) break; }
      let n = Number(a.n) || Infinity;
      for (const [pid, p] of Object.entries(g.players).reverse()) if (p.bot && n-- > 0) delete g.players[pid];
      // the removed bots may have been the ones everyone was waiting on
      if (inGame(g) && Object.keys(g.players).every(p => G.isDone(g, p)) && G.advance[g.phase]) G.advance[g.phase](g);
      break;
    }
    default: {
      if (!inGame(g) || !me) return 'Not now';
      const err = G.act(g, a, id);
      if (err) return err;
      // everyone finished early: move on without waiting for the timer
      if (Object.keys(g.players).every(p => G.isDone(g, p)) && G.advance[g.phase]) G.advance[g.phase](g);
    }
  }
}

const maxOf = g => g.settings.max || MAX_PLAYERS;
const avatarFree = (g, av, id) => Number.isInteger(av) && av >= 0 && av < 40 && !Object.entries(g.players).some(([pid, p]) => pid !== id && p.av === av);
// photos live only in memory on the player, so they disappear when the player is removed or the room closes
const PHOTO = /^data:image\/jpeg;base64,[A-Za-z0-9+/=]+$/;
function addBots(g, n) {
  for (let i = 0; i < n && Object.keys(g.players).length < maxOf(g); i++) {
    const av = freeAvatar(g);
    g.players['bot' + Math.random().toString(36).slice(2, 8)] = { name: `Bot ${BOT_NAMES[av % BOT_NAMES.length]}`, score: 0, av, bot: true };
  }
}
const BOT_NAMES = ['Ada', 'Bix', 'Cog', 'Dot', 'Echo', 'Fizz', 'Gizmo', 'Hex', 'Ion', 'Jinx', 'Kilo', 'Lux', 'Mo', 'Nano', 'Ozzy', 'Pip', 'Quark', 'Rex', 'Sprocket', 'Tink'];
// bots take a turn now and then, staggered so it looks like real people finishing
setInterval(() => {
  for (const g of Object.values(rooms)) {
    if (g.phase === 'gamevote') { // bots vote too, at random
      for (const [id, p] of Object.entries(g.players)) if (p.bot && !(id in g.gvotes) && Math.random() < .4) { gameAct(g, { type: 'gvote', id, game: pick([...(g.game ? ['replay'] : []), ...Object.keys(GAMES)]) }); broadcast(g); if (g.phase !== 'gamevote') break; }
      continue;
    }
    if (!inGame(g)) continue;
    const G = game(g);
    for (const [id, p] of Object.entries(g.players)) {
      if (!p.bot || G.isDone(g, id) || Math.random() < 0.6 || !inGame(g)) continue;
      // a bug in one game's bot must not take the whole server down mid-party
      try { const move = G.bot?.(g, id); if (move && !gameAct(g, { ...move, id })) broadcast(g); } catch (e) { console.error(e); }
    }
  }
}, 700).unref();

const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://x');
  if (url.pathname === '/events') {
    const g = rooms[(url.searchParams.get('room') || '').toUpperCase()];
    if (!g) { res.writeHead(404); res.end(); return; }
    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', Connection: 'keep-alive' });
    res.write(`data: ${JSON.stringify(publicState(g))}\n\n`);
    g.clients.add(res); req.on('close', () => { g.clients.delete(res); g.lastSeen = Date.now(); });
    return;
  }
  if (url.pathname === '/action' && req.method === 'POST') {
    let body = '';
    req.on('data', c => { body += c; if (body.length > 40000) req.destroy(); });
    req.on('end', () => {
      let out; try { out = act(JSON.parse(body)); } catch (e) { console.error(e); out = { error: 'Bad request' }; }
      res.writeHead(out.error ? 400 : 200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(out));
    });
    return;
  }
  if (url.pathname.startsWith('/photo/')) {
    const [, , code, pid] = url.pathname.split('/'), photo = rooms[code]?.players[decodeURIComponent(pid || '')]?.photo;
    if (!photo) { res.writeHead(404); res.end(); return; }
    res.writeHead(200, { 'Content-Type': 'image/jpeg', 'Cache-Control': 'max-age=86400' }); // ?v= changes when the photo does
    res.end(Buffer.from(photo.split(',')[1], 'base64')); return;
  }
  if (url.pathname === '/ip') { res.end(lanUrl()); return; }
  if (url.pathname === '/qr.svg') { // scan-to-join: opens the phone page with the room code filled in
    const room = (url.searchParams.get('room') || '').replace(/[^A-Z]/gi, '').slice(0, 4).toUpperCase();
    require('qrcode').toString(`${lanUrl()}/?room=${room}`, { type: 'svg', margin: 1, color: { dark: '#1a1230', light: '#ffffff' } })
      .then(svg => { res.writeHead(200, { 'Content-Type': 'image/svg+xml', 'Cache-Control': 'no-store' }); res.end(svg); })
      .catch(() => { res.writeHead(500); res.end(); });
    return;
  }
  const asset = { '/style.css': 'text/css', '/common.js': 'text/javascript' }[url.pathname];
  if (asset) { res.writeHead(200, { 'Content-Type': asset }); fs.createReadStream(path.join(__dirname, 'public', url.pathname)).pipe(res); return; }
  const file = url.pathname.startsWith('/tv') ? 'tv.html' : url.pathname.startsWith('/test') ? 'test.html' : 'phone.html';
  res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
  fs.createReadStream(path.join(__dirname, 'public', file)).pipe(res);
});

// starts on the given port, or the next free one if it's taken; resolves with the port used
function start(port = PORT) {
  return new Promise((resolve, reject) => {
    server.once('error', e => e.code === 'EADDRINUSE' && port < PORT + 20 ? resolve(start(port + 1)) : reject(e));
    server.listen(port, () => { PORT = port; resolve(port); });
  });
}
if (require.main === module) start().then(p => console.log(`TV:     http://localhost:${p}/tv
Phones: ${lanUrl()}
Test:   http://localhost:${p}/test`));
// shut everything down: round timers, every phone/TV live connection, then the server itself
function stop() {
  for (const g of Object.values(rooms)) { clearTimeout(g.timer); g.clients.forEach(c => c.end()); }
  server.closeAllConnections?.();
  return new Promise(r => server.close(() => r()));
}
module.exports = { start, stop, lanUrl };

// The address phones should type. Skips virtual adapters (VirtualBox, Hyper-V, WSL, VPNs) that phones can't reach.
function lanUrl() {
  const VIRTUAL = /virtual|vbox|vmware|vethernet|wsl|hyper-v|docker|tailscale|zerotier|hamachi|loopback/i;
  const nets = Object.entries(os.networkInterfaces()).flatMap(([name, list]) => list.map(i => ({ name, ...i })))
    .filter(i => i.family === 'IPv4' && !i.internal && !i.address.startsWith('169.254.') && !VIRTUAL.test(i.name));
  const best = nets.find(i => /wi-?fi|wlan|wireless/i.test(i.name)) || nets.find(i => /ethernet|^en|^eth/i.test(i.name)) || nets[0];
  return `http://${best?.address || 'localhost'}:${PORT}`;
}
