// Lone Clue: one player guesses a secret word. Everyone else writes a one-word clue, but matching clues cancel out,
// so the guesser only sees the clues nobody else thought of.
const SECS = { clue: 45, guess: 40 };
const WORDS = ['pizza', 'rocket', 'snowman', 'castle', 'banana', 'robot', 'volcano', 'guitar', 'dragon', 'umbrella', 'pirate', 'rainbow',
  'octopus', 'cactus', 'ghost', 'bicycle', 'mermaid', 'tornado', 'giraffe', 'lighthouse', 'cupcake', 'unicorn', 'penguin', 'treasure',
  'dinosaur', 'island', 'popcorn', 'butterfly', 'campfire', 'wizard', 'zombie', 'shark', 'chocolate', 'library', 'wedding', 'circus',
  'jungle', 'football', 'hospital', 'airport', 'museum', 'kitchen', 'vampire', 'moon', 'piano', 'coffee', 'birthday', 'beach', 'ninja',
  'cowboy', 'honey', 'pumpkin', 'diamond', 'tiger', 'spider', 'magnet', 'parachute', 'igloo', 'compass', 'lemon'];
// "Cats" and "cat!" count as the same clue
const norm = t => String(t).toLowerCase().replace(/[^a-z]/g, '').replace(/(es|s)$/, '');

let h;
const realIds = g => Object.keys(g.players).filter(id => !g.players[id].bot);
function start(g) {
  g.phase = 'clue'; g.clues = {}; g.answer = null;
  g.word = h.pick(WORDS);
  // take turns guessing; bots can't guess well, so only real players get a turn
  const ids = realIds(g);
  g.guesser = ids[(g.round - 1) % ids.length];
  h.setTimer(g, SECS.clue, startGuess);
}
function startGuess(g) {
  const n = {};
  for (const t of Object.values(g.clues)) n[norm(t)] = (n[norm(t)] || 0) + 1;
  g.cancelled = Object.keys(g.clues).filter(id => n[norm(g.clues[id])] > 1);
  g.phase = 'guess'; h.setTimer(g, SECS.guess, end);
}
function end(g) {
  g.correct = g.answer != null && norm(g.answer) === norm(g.word);
  if (g.correct) {
    if (g.players[g.guesser]) g.players[g.guesser].score += 1000;
    for (const id of Object.keys(g.clues)) if (!g.cancelled.includes(id) && g.players[id]) g.players[id].score += 500;
  }
  h.finish(g);
}
const kept = g => Object.keys(g.clues).filter(id => !g.cancelled.includes(id)).map(id => ({ id, text: g.clues[id] }));

module.exports = {
  id: 'loneclue', name: 'Lone Clue', min: 3,
  desc: 'Give a one-word clue to help one player guess the secret word. Matching clues cancel out!',
  tag: 'Co-op words · 5 min',
  bot(g, id) {
    if (g.phase === 'clue' && id !== g.guesser) return { type: 'clue', text: h.pick(['big', 'fun', 'loud', 'summer', 'yummy', 'scary', 'round', 'fast', 'old', 'wet']) };
  },
  init(helpers) { h = helpers; },
  start,
  advance: { clue: startGuess, guess: end },
  isDone(g, id) {
    if (g.phase === 'clue') return id === g.guesser || id in g.clues;
    if (g.phase === 'guess') return id !== g.guesser || g.answer != null;
    return false;
  },
  state(g, s) {
    s.guesser = g.guesser;
    // Note: the word rides in the shared broadcast (a peek in devtools shows it), fine among friends, same as Odd One Out.
    if (g.phase === 'clue' || g.phase === 'guess') s.word = g.word;
    if (g.phase === 'guess') { s.kept = kept(g); s.cancelled = g.cancelled.length; }
    if ((g.phase === 'scores' || g.phase === 'results') && g.word) s.last = { word: g.word, answer: g.answer, correct: g.correct,
      guesser: g.players[g.guesser]?.name ?? '(left)',
      clues: Object.entries(g.clues).map(([id, text]) => ({ name: g.players[id]?.name ?? '(left)', text, out: g.cancelled.includes(id) })) };
  },
  act(g, a, id) {
    if (a.type === 'clue') {
      const text = h.clean(a.text, 20);
      if (g.phase !== 'clue' || id === g.guesser || !text) return 'Not now';
      if (/\s/.test(text)) return 'One word only';
      if (norm(text).includes(norm(g.word)) || norm(g.word).includes(norm(text))) return "Don't just say the word!";
      g.clues[id] = text;
    } else if (a.type === 'guess') {
      const text = h.clean(a.text, 30);
      if (g.phase !== 'guess' || id !== g.guesser || !text || g.answer != null) return 'Not now';
      g.answer = text; end(g);
    } else return 'Unknown action';
  },
};

// self-check: run `node games/loneclue.js`
if (require.main === module) {
  const assert = require('assert'), m = module.exports;
  let fin = 0;
  m.init({ pick: a => a[0], clean: (v, n) => String(v || '').trim().slice(0, n), setTimer() {}, finish: () => fin++ });
  const g = { round: 1, players: { a: { score: 0 }, b: { score: 0 }, c: { score: 0 }, d: { score: 0, bot: true } } };
  m.start(g);
  assert.equal(g.guesser, 'a');
  assert.ok(m.act(g, { type: 'clue', text: 'pizzas' }, 'b'));
  assert.ok(m.act(g, { type: 'clue', text: 'hot' }, 'a'));
  m.act(g, { type: 'clue', text: 'Cheese' }, 'b'); m.act(g, { type: 'clue', text: 'cheese!' }, 'c'); m.act(g, { type: 'clue', text: 'italy' }, 'd');
  m.advance.clue(g);
  assert.deepEqual(g.cancelled, ['b', 'c']);
  assert.equal(m.act(g, { type: 'guess', text: ' Pizza ' }, 'a'), undefined);
  assert.equal(fin, 1); assert.equal(g.players.a.score, 1000); assert.equal(g.players.d.score, 500); assert.equal(g.players.b.score, 0);
  console.log('loneclue.js ok');
}
