// Shared by TV and phone: avatars, escaping, countdown, confetti.
const AV_EMOJI = ['🦊', '🐸', '🐙', '🦄', '🐼', '🐯', '🦉', '🐵', '🐧', '🦖', '🐨', '🦁', '🐰', '🐳', '🦜', '🐢', '🦔', '🐝', '🦩', '🐲',
  '🐶', '🐱', '🐮', '🐷', '🐔', '🦋', '🐞', '🦀', '🐠', '🦓', '🦒', '🐘', '🦥', '🦦', '🐺', '🦝', '🐗', '👽', '🤖', '👻'];
const AV_COLOR = ['#ff6b6b', '#4ecdc4', '#ffd93d', '#a78bfa', '#6bcb77', '#ff9f43', '#4d96ff', '#f472b6', '#2dd4bf', '#facc15',
  '#fb7185', '#60a5fa', '#c084fc', '#34d399', '#f97316', '#38bdf8', '#e879f9', '#a3e635', '#fda4af', '#fbbf24'];
const avColor = p => AV_COLOR[p.av % AV_COLOR.length];
// GIF It prompts are GIPHY ids; offline shows a note (a deleted GIF gets GIPHY's own "not available" card)
const gif = (s, p, w = 'giphy') => s.game === 'gif' ? `<img class="gif" src="https://media.giphy.com/media/${encodeURIComponent(p)}/${w}.gif" alt="GIF" onerror="this.replaceWith('No GIF this time. Caption it anyway!')">` : esc(p);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const plural = (n, w) => `${n} ${w}${n === 1 ? '' : 's'}`;
// p.ph is the URL of an uploaded photo, if the player added one
const avatar = (p, size = '') => `<span class="av ${size}" style="--c:${avColor(p)}">${p.ph ? `<img src="${esc(p.ph)}" alt="">` : AV_EMOJI[p.av % AV_EMOJI.length]}</span>`;

// countdown ring: server sends seconds left; we tick locally between updates
const timer = { left: 0, total: 0, el: null };
function setTimerFrom(s, el) {
  timer.el = el;
  if (s.left > timer.left + 1 || !s.left) timer.total = s.left; // a new countdown started
  timer.left = s.left; drawTimer();
}
function drawTimer() {
  const { el, left, total } = timer; if (!el) return;
  el.hidden = !left;
  el.classList.toggle('low', left > 0 && left <= 10);
  el.style.setProperty('--p', total ? left / total : 0);
  el.querySelector('b').textContent = left;
}
setInterval(() => { if (timer.left > 0) { timer.left--; if (timer.left && timer.left <= 5) sfx('tick'); drawTimer(); } }, 1000);

// small dependency-free confetti burst
function confetti(n = 140) {
  const c = document.createElement('canvas'), x = c.getContext('2d');
  c.style.cssText = 'position:fixed;inset:0;pointer-events:none;z-index:99';
  c.width = innerWidth; c.height = innerHeight; document.body.appendChild(c);
  const bits = Array.from({ length: n }, () => ({ x: c.width / 2, y: c.height / 3, vx: (Math.random() - .5) * 18, vy: Math.random() * -16 - 4,
    r: Math.random() * 6 + 4, c: AV_COLOR[Math.floor(Math.random() * AV_COLOR.length)], a: Math.random() * 6 }));
  let t = 0;
  (function f() {
    x.clearRect(0, 0, c.width, c.height);
    for (const b of bits) { b.vy += .45; b.x += b.vx; b.y += b.vy; b.a += .1; x.fillStyle = b.c;
      x.save(); x.translate(b.x, b.y); x.rotate(b.a); x.fillRect(-b.r, -b.r / 2, b.r * 2, b.r); x.restore(); }
    if (++t < 160) requestAnimationFrame(f); else c.remove();
  })();
}

// Sound effects, synthesized with Web Audio so there are no files to load.
// Browsers only allow sound after a tap/click/key press on the page, so the context starts on the first one.
// The desktop app allows sound straight away, and a controller never clicks, so it also tries on load.
// sfxVol is 0..1, kept per device; older installs saved a plain mute flag
let actx, sfxVol = 1;
try { sfxVol = localStorage.sfxVol != null ? Number(localStorage.sfxVol) : localStorage.muted === '1' ? 0 : 1; } catch {}
function wakeAudio() { try { actx ??= new AudioContext(); if (actx.state !== 'running') actx.resume(); } catch {} }
// (not on Linux: there the web view's sound setup can stall the page at load; a key or controller press wakes it instead)
if (window.partyApp && !/Linux/.test(navigator.userAgent)) wakeAudio();
for (const ev of ['pointerdown', 'keydown']) addEventListener(ev, wakeAudio, { capture: true });
function tone(freq, t0, dur, type = 'sine', vol = .2, slide = 0) {
  const o = actx.createOscillator(), g = actx.createGain(), t = actx.currentTime + t0;
  o.type = type; o.frequency.setValueAtTime(freq, t);
  if (slide) o.frequency.exponentialRampToValueAtTime(freq * slide, t + dur);
  g.gain.setValueAtTime(0, t); g.gain.linearRampToValueAtTime(vol * sfxVol, t + .01); g.gain.exponentialRampToValueAtTime(.001, t + dur);
  o.connect(g).connect(actx.destination); o.start(t); o.stop(t + dur + .05);
}
const SFX = {
  tap: () => tone(660, 0, .08, 'triangle', .15),
  join: () => { tone(520, 0, .12, 'triangle'); tone(780, .08, .18, 'triangle'); },
  leave: () => tone(400, 0, .25, 'sine', .15, .5),
  go: () => [392, 523, 659, 784].forEach((f, i) => tone(f, i * .09, .25, 'square', .08)),
  tick: () => tone(1200, 0, .05, 'square', .06),
  done: () => tone(880, 0, .1, 'sine', .12),
  whoosh: () => tone(200, 0, .35, 'sawtooth', .07, 4),
  reveal: () => { for (let i = 0; i < 10; i++) tone(140 + i * 8, i * .05, .06, 'square', .05); [523, 659, 784, 1047].forEach((f, i) => tone(f, .55 + i * .07, .4, 'triangle', .15)); },
  fanfare: () => [[523, 0], [523, .15], [523, .3], [659, .45], [784, .75], [659, 1], [784, 1.15], [1047, 1.3]].forEach(([f, t]) => tone(f, t, .35, 'square', .08)),
  sad: () => [392, 370, 349, 330].forEach((f, i) => tone(f, i * .3, .35, 'triangle', .12)),
};
// Sounds players can drop into answers as <name>. Same synth as above.
const SOUNDS = {
  boing: () => tone(180, 0, .5, 'sine', .25, 3),
  honk: () => { tone(330, 0, .18, 'sawtooth', .12); tone(330, .25, .3, 'sawtooth', .12); },
  rimshot: () => { tone(200, 0, .08, 'triangle', .2); tone(180, .2, .08, 'triangle', .2); tone(2400, .4, .3, 'square', .05, .5); },
  airhorn: () => [0, .3, .45].forEach(t => { tone(466, t, .25, 'sawtooth', .1); tone(470, t, .25, 'sawtooth', .1); }),
  whistle: () => tone(400, 0, .7, 'sine', .15, 4),
  fail: () => [392, 370, 349, 330].forEach((f, i) => tone(f, i * .3, .35, 'triangle', .12)),
  ding: () => { tone(1320, 0, .6, 'sine', .15); tone(1980, 0, .4, 'sine', .06); },
  crickets: () => { for (let i = 0; i < 6; i++) tone(4200, i * .12 + (i > 2 ? .4 : 0), .05, 'sine', .05); },
  drumroll: () => { for (let i = 0; i < 16; i++) tone(120, i * .05, .05, 'square', .06); tone(80, .85, .4, 'triangle', .25); },
  wow: () => tone(300, 0, .6, 'square', .07, 2.5),
};
Object.assign(SFX, SOUNDS);
const TAG = new RegExp(`<(${Object.keys(SOUNDS).join('|')})>`, 'g');
// escaped text with sound tags turned into chips; data-snd lets the TV play them in order
const withSounds = t => esc(t).replace(new RegExp(`&lt;(${Object.keys(SOUNDS).join('|')})&gt;`, 'g'), (_, n) => `<span class="snd" data-snd="${n}">🔊 ${n}</span>`);
const noSounds = t => String(t ?? '').replace(TAG, ' ');
// plays every chip inside el one after another, lighting each up
function playChips(el, gap = 900) {
  [...el.querySelectorAll('.snd')].forEach((c, i) => setTimeout(() => { sfx(c.dataset.snd); c.classList.remove('hit'); void c.offsetWidth; c.classList.add('hit'); }, 400 + i * gap));
}

function sfx(name) { if (!sfxVol || !actx || actx.state !== 'running') return; try { SFX[name]?.(); } catch {} }
function setSfxVol(v) { sfxVol = v; try { localStorage.sfxVol = v; } catch {} }

// When the live connection drops, find out why before acting: true = room still there (just reconnect),
// false = room is gone (server restarted), null = server unreachable right now (try again shortly).
// Without this, a plain page reload looked like "room gone" and phones/TV forgot their room.
async function roomAlive(room) {
  try {
    const r = await fetch('/action', { method: 'POST', body: JSON.stringify({ type: 'ping', room }) });
    return r.ok ? true : (await r.json()).error !== 'Room not found';
  } catch { return null; }
}

// Swap a screen's HTML with a quick crossfade where the browser supports View Transitions.
// `after` runs once the new HTML is in place (focus, confetti, sizing...).
const calm = matchMedia('(prefers-reduced-motion: reduce)');
// drawing strokes as SVG; data-live so new strokes swap in on the TV without replaying animations
const sketch = (strokes, k, size) => `<svg data-live="sk-${k}" viewBox="0 0 1000 1000" style="width:${size};aspect-ratio:1;max-width:100%;background:#fff;border-radius:1rem;flex:none">${strokes.map(t =>
  `<polyline points="${t.p.join(' ')}" fill="none" stroke="${esc(t.c)}" stroke-width="${t.w}" stroke-linecap="round" stroke-linejoin="round"/>`).join('')}</svg>`;
// Elemix cards: element colour, a big tilted element mark, value in the middle and both corners
const ELEM = [['🔥', 'Fire'], ['💧', 'Water'], ['🌿', 'Leaf'], ['🌙', 'Moon'], ['✦', 'Wild']];
const CARD_FACE = { freeze: ['❄', 'Freeze'], twist: ['⇄', 'Twist'], gift: ['+2', 'Gift'], prism: ['✦', 'Prism'], storm: ['+4', 'Storm'] };
const ecard = (c, attrs = '', cls = '') => { const [face, label] = CARD_FACE[c.v] || [c.v, ELEM[c.e][1]];
  return `<button class="ecard e${c.e} ${cls}" ${attrs} aria-label="${label} ${ELEM[c.e][1]}"><i class="mark">${ELEM[c.e][0]}</i>
    <span class="cn tl">${face}</span><b>${face}</b><small>${label}</small><span class="cn br">${face}</span></button>`; };
function swap(el, html, after) {
  const go = () => { el.innerHTML = html; countUp(el); after?.(); };
  if (document.startViewTransition && !calm.matches && !document.hidden) document.startViewTransition(go); else go();
}
// numbers marked data-count="1234" tick up from 0
function countUp(root) {
  for (const el of root.querySelectorAll('[data-count]')) {
    const end = Number(el.dataset.count), t0 = performance.now(), suffix = el.dataset.suffix || '';
    if (calm.matches || !end) { el.textContent = end + suffix; continue; }
    (function tick(t) { const k = Math.min(1, (t - t0) / 1100); el.textContent = Math.round(end * (1 - (1 - k) ** 3)) + suffix; if (k < 1) requestAnimationFrame(tick); })(t0);
  }
}

// Mini Golf course (1000 x 600): grass, cup, walls, then each ball in its player's colour; `mine` gets a white ring
function drawCourse(cv, h, balls, players, mine) {
  const x = cv.getContext('2d'); x.setTransform(cv.width / 1000, 0, 0, cv.height / 600, 0, 0);
  x.fillStyle = '#3fa34d'; x.fillRect(0, 0, 1000, 600);
  x.fillStyle = '#111'; x.beginPath(); x.arc(h.cup[0], h.cup[1], 16, 0, 7); x.fill();
  x.strokeStyle = '#7a4a21'; x.lineWidth = 14; x.lineCap = 'round';
  for (const w of h.walls) { x.beginPath(); x.moveTo(w[0], w[1]); x.lineTo(w[2], w[3]); x.stroke(); }
  for (const b of balls) { if (b.in && !b.moving) continue; const p = players.find(q => q.id === b.id);
    x.fillStyle = p ? avColor(p) : '#fff'; x.strokeStyle = b.id === mine ? '#fff' : '#0008'; x.lineWidth = b.id === mine ? 5 : 2;
    x.beginPath(); x.arc(b.x, b.y, 10, 0, 7); x.fill(); x.stroke(); }
}
// each ball rides along its latest shot's path (30 fps) until the shot is played out
const golfAnim = {};
function golfBalls(s) {
  for (const b of s.balls) if (golfAnim[b.id]?.shot !== b.shot) golfAnim[b.id] = { shot: b.shot, t0: performance.now(), path: b.path };
  return s.balls.map(b => { const a = golfAnim[b.id], i = Math.floor((performance.now() - a.t0) / 33);
    return i * 2 < a.path.length - 2 ? { id: b.id, x: a.path[i * 2], y: a.path[i * 2 + 1], moving: true } : b; });
}
// Knockout ice (1000 x 600): the latest slide plays from its 30 fps frames on the old ice, then the shrunk ice shows
const koAnim = {};
function koView(s) {
  const k = s.ko; if (koAnim.turn !== k.turn) Object.assign(koAnim, { turn: k.turn, t0: performance.now() });
  const i = Math.floor((performance.now() - koAnim.t0) / 66), f = k.frames[i]; // half speed so people can follow the hits
  if (!f || i >= k.frames.length - 1) return { r: k.r, pucks: k.pucks.filter(p => !p.out), moving: false };
  return { r: k.lastR, moving: true, pucks: k.pucks.map((p, j) => ({ id: p.id, x: f[j * 2], y: f[j * 2 + 1] })).filter(p => p.x > -999) };
}
function drawIce(cv, v, players, mine) {
  const x = cv.getContext('2d'); x.setTransform(cv.width / 1000, 0, 0, cv.height / 600, 0, 0);
  x.fillStyle = '#1d5d8f'; x.fillRect(0, 0, 1000, 600);
  x.fillStyle = '#e6f6ff'; x.strokeStyle = '#9fd3f5'; x.lineWidth = 8; x.beginPath(); x.arc(500, 300, v.r, 0, 7); x.fill(); x.stroke();
  for (const b of v.pucks) { const p = players.find(q => q.id === b.id);
    x.fillStyle = p ? avColor(p) : '#888'; x.strokeStyle = b.id === mine ? '#000' : '#0006'; x.lineWidth = b.id === mine ? 6 : 3;
    x.beginPath(); x.arc(b.x, b.y, 22, 0, 7); x.fill(); x.stroke();
    // the player's photo clipped round, or their emoji
    const img = p?.ph && puckImg(p.ph);
    if (img?.complete && img.naturalWidth) { x.save(); x.beginPath(); x.arc(b.x, b.y, 20, 0, 7); x.clip(); x.drawImage(img, b.x - 20, b.y - 20, 40, 40); x.restore(); }
    else if (p) { x.font = '28px sans-serif'; x.textAlign = 'center'; x.textBaseline = 'middle'; x.fillText(AV_EMOJI[p.av % AV_EMOJI.length], b.x, b.y + 2); } }
}
const puckImgs = {};
const puckImg = src => puckImgs[src] ||= Object.assign(new Image(), { src });
const GOLF_CSS = 'width:100%;aspect-ratio:5/3;border:.5rem solid #7a4a21;border-radius:1rem;touch-action:none;display:block';

// casino-chip accents for game cards (TV menu and phone pickers)
const CHIPS = ['#c8202f', '#1f5fbf', '#178a4a', '#24282d', '#d9a21b'];
