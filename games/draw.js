// Draw & Guess: one player draws a secret word on their phone, it appears live on the TV, everyone else races to guess it.
const SECS = { draw: 80 };
const WORDS = ['pizza', 'rocket', 'snowman', 'spider', 'castle', 'banana', 'robot', 'volcano', 'guitar', 'dragon', 'umbrella', 'pirate',
  'rainbow', 'octopus', 'cactus', 'ghost', 'hamburger', 'bicycle', 'mermaid', 'tornado', 'giraffe', 'lighthouse', 'skeleton', 'cupcake',
  'unicorn', 'airplane', 'sandwich', 'penguin', 'treasure', 'dinosaur', 'tent', 'snail', 'crown', 'island', 'alien', 'popcorn',
  'butterfly', 'toilet', 'campfire', 'whale', 'wizard', 'ladder', 'sunglasses', 'kite', 'zombie', 'hot dog', 'mushroom', 'shark'];
const MAX_STROKES = 400, MAX_POINTS = 600;
const norm = t => String(t).toLowerCase().replace(/[^a-z]/g, '');

let h;
const realIds = g => Object.keys(g.players).filter(id => !g.players[id].bot);
function start(g) {
  g.phase = 'draw'; g.strokes = []; g.got = []; g.feed = [];
  g.word = h.pick(WORDS);
  // take turns drawing; bots can't draw, so only real players get a turn
  const ids = realIds(g);
  g.drawer = ids[(g.round - 1) % ids.length];
  h.setTimer(g, SECS.draw, end);
}
function end(g) { h.finish(g); }
function correct(g, id) {
  if (g.got.includes(id)) return;
  g.got.push(id);
  if (g.players[id]) g.players[id].score += Math.max(300, 1000 - 150 * (g.got.length - 1));
  if (g.players[g.drawer]) g.players[g.drawer].score += 250;
}

module.exports = {
  id: 'draw', name: 'Draw & Guess', min: 2,
  desc: 'One player draws a secret word on their phone. Everyone else races to guess it.',
  tag: 'Drawing · 5 min',
  bot(g, id) {
    if (g.phase === 'draw' && !g.got.includes(id)) return { type: 'guess', text: Math.random() < .25 ? g.word : h.pick(WORDS) };
  },
  init(helpers) { h = helpers; },
  start,
  advance: { draw: end },
  isDone(g, id) { return g.phase === 'draw' && (id === g.drawer || g.got.includes(id)); },
  state(g, s) {
    if (g.phase === 'draw') {
      s.drawer = g.drawer; s.strokes = g.strokes; s.feed = g.feed.slice(-6);
      s.hint = g.word.replace(/[a-z]/g, '_'); s.got = g.got.length;
      // Note: the word rides in the shared broadcast (a peek in devtools shows it), fine among friends, same as Odd One Out.
      s.word = g.word;
    }
    if ((g.phase === 'scores' || g.phase === 'results') && g.word) s.last = { word: g.word, strokes: g.strokes,
      drawer: g.players[g.drawer]?.name ?? '(left)', got: g.got.map(id => g.players[id]?.name ?? '(left)') };
  },
  act(g, a, id) {
    if (g.phase !== 'draw') return 'Not now';
    if (a.type === 'stroke') {
      if (id !== g.drawer) return 'Not your turn to draw';
      const p = Array.isArray(a.p) ? a.p.slice(0, MAX_POINTS * 2).map(n => Math.max(0, Math.min(1000, Math.round(Number(n) || 0)))) : [];
      if (p.length < 2 || g.strokes.length >= MAX_STROKES) return 'Not now';
      g.strokes.push({ c: /^#[0-9a-f]{6}$/i.test(a.c) ? a.c : '#231a08', w: [4, 10, 24].includes(a.w) ? a.w : 10, p });
    } else if (a.type === 'undo' || a.type === 'wipe') {
      if (id !== g.drawer) return 'Not your turn to draw';
      if (a.type === 'undo') g.strokes.pop(); else g.strokes = [];
    } else if (a.type === 'guess') {
      const text = h.clean(a.text, 30);
      if (id === g.drawer || !text) return 'Not now';
      if (g.got.includes(id)) return 'You already got it!';
      if (norm(text) === norm(g.word)) { correct(g, id); g.feed.push({ id, ok: true }); }
      else { g.feed.push({ id, text }); return 'Nope, keep guessing!'; }
    } else return 'Unknown action';
  },
};

// self-check: run `node games/draw.js`
if (require.main === module) {
  const assert = require('assert'), m = module.exports;
  let fin = 0;
  m.init({ pick: a => a[0], clean: (v, n) => String(v || '').trim().slice(0, n), setTimer() {}, finish: () => fin++ });
  const g = { round: 1, players: { a: { score: 0 }, b: { score: 0 }, c: { score: 0, bot: true } } };
  m.start(g);
  assert.equal(g.drawer, 'a');
  assert.equal(m.act(g, { type: 'stroke', p: [0, 0, 5000, 10] }, 'a'), undefined);
  assert.deepEqual(g.strokes[0].p, [0, 0, 1000, 10]);
  assert.ok(m.act(g, { type: 'stroke', p: [1, 1] }, 'b'));
  assert.ok(m.act(g, { type: 'guess', text: 'nope' }, 'b'));
  assert.equal(m.act(g, { type: 'guess', text: ' PIZZA ' }, 'b'), undefined);
  assert.equal(g.players.b.score, 1000); assert.equal(g.players.a.score, 250);
  assert.ok(m.isDone(g, 'a') && m.isDone(g, 'b') && !m.isDone(g, 'c'));
  console.log('draw.js ok');
}
