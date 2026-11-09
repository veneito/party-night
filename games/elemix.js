// Elemix: a shedding card game. Match the top card's element or value; first to empty their hand wins the round
// and scores the cards left in everyone else's hand. Elements: 0 fire, 1 water, 2 leaf, 3 moon; 4 = wild.
// Values: 0-9, 'freeze' (skip next), 'twist' (reverse), 'gift' (next draws 2), 'prism' (wild), 'storm' (wild, next draws 4).
const SECS = { turn: 25 };
const HAND = 7;
const POINTS = { freeze: 20, twist: 20, gift: 20, prism: 50, storm: 50 };

let h;
function deck(n) { // one deck per 6 players so big rooms don't run dry
  const d = [];
  for (let k = 0; k < n; k++) {
    for (let e = 0; e < 4; e++) {
      d.push({ e, v: 0 });
      for (const v of [1, 2, 3, 4, 5, 6, 7, 8, 9, 'freeze', 'twist', 'gift']) d.push({ e, v }, { e, v });
    }
    for (let i = 0; i < 4; i++) d.push({ e: 4, v: 'prism' }, { e: 4, v: 'storm' });
  }
  return h.shuffle(d);
}
const top = g => g.pile[g.pile.length - 1];
const fits = (g, c) => c.e === 4 || c.e === g.elem || c.v === top(g).v;
function draw(g, id, n) {
  for (let i = 0; i < n; i++) {
    if (!g.stock.length) { const t = g.pile.pop(); g.stock = h.shuffle(g.pile); g.pile = [t]; }
    if (!g.stock.length) return; // every card is in someone's hand
    g.hands[id].push(g.stock.pop());
  }
}
const live = g => g.order.filter(id => g.players[id]);
const cur = g => g.order[g.turn];
function step(g, n = 1) { const k = g.order.length; g.turn = ((g.turn + g.dir * n) % k + k) % k; }
function nextTurn(g, n = 1) {
  step(g, n);
  while (!g.players[cur(g)]) step(g); // skip players who left
  g.turnNo++;
  h.setTimer(g, SECS.turn, timeout);
}
function timeout(g) { draw(g, cur(g), 1); g.log = `${g.players[cur(g)]?.name} ran out of time and drew`; nextTurn(g); }

function start(g) {
  g.order = h.shuffle(Object.keys(g.players)); g.hands = {}; g.dir = 1; g.turn = 0; g.turnNo = 0; g.winner = null;
  g.stock = deck(Math.ceil(g.order.length / 6));
  for (const id of g.order) { g.hands[id] = []; draw(g, id, HAND); }
  let first; while ((first = g.stock.pop()).e === 4 || typeof first.v !== 'number') g.stock.unshift(first); // start on a plain number
  g.pile = [first]; g.elem = first.e; g.log = '';
  g.phase = 'turn'; h.setTimer(g, SECS.turn, timeout);
}
function play(g, id, i, elem) {
  const c = g.hands[id][i], name = g.players[id].name;
  g.hands[id].splice(i, 1);
  g.pile.push(c); g.elem = c.e === 4 ? elem : c.e;
  g.log = `${name} played ${c.e === 4 ? c.v : `${['fire', 'water', 'leaf', 'moon'][c.e]} ${c.v}`}`;
  if (!g.hands[id].length) return win(g, id);
  if (c.v === 'twist') { g.dir *= -1; if (live(g).length === 2) return nextTurn(g, 2); } // with two players, twist acts like freeze
  if (c.v === 'freeze') return nextTurn(g, 2);
  if (c.v === 'gift' || c.v === 'storm') {
    step(g); while (!g.players[cur(g)]) step(g);
    draw(g, cur(g), c.v === 'gift' ? 2 : 4); g.log += ` · ${g.players[cur(g)].name} draws ${c.v === 'gift' ? 2 : 4} and is skipped`;
    return nextTurn(g);
  }
  nextTurn(g);
}
function win(g, id) {
  let pts = 0;
  for (const [pid, hand] of Object.entries(g.hands)) if (pid !== id) for (const c of hand) pts += typeof c.v === 'number' ? c.v : POINTS[c.v];
  g.winner = id; g.won = pts * 10; g.players[id].score += g.won;
  h.finish(g);
}

module.exports = {
  id: 'elemix', name: 'Elemix', min: 2, art: '🃏',
  desc: 'Match the element or the number, play tricks on your friends, and be the first to empty your hand.',
  tag: 'Cards · 10 min',
  bot(g, id) {
    if (g.phase !== 'turn' || cur(g) !== id) return;
    const i = g.hands[id].findIndex(c => fits(g, c));
    if (i < 0) return { type: 'draw' };
    const count = [0, 0, 0, 0]; for (const c of g.hands[id]) if (c.e < 4) count[c.e]++;
    return { type: 'play', i, elem: count.indexOf(Math.max(...count)) };
  },
  init(helpers) { h = helpers; },
  start,
  advance: { turn: timeout },
  isDone(g, id) { return g.phase === 'turn' && cur(g) !== id; },
  state(g, s) {
    if (g.phase === 'turn') {
      // Note: hands ride in the shared broadcast (a peek in devtools shows them), fine among friends, same as the other games' secrets.
      s.hands = g.hands; s.top = top(g); s.recent = g.pile.slice(-4); s.elem = g.elem; s.cur = cur(g); s.dir = g.dir; s.turnNo = g.turnNo; s.log = g.log;
      s.stock = g.stock.length; s.order = g.order.filter(id => g.players[id]);
      s.playable = Object.fromEntries(Object.entries(g.hands).map(([id, hand]) => [id, hand.map(c => fits(g, c))]));
    }
    if ((g.phase === 'scores' || g.phase === 'results') && g.winner) s.last = { winner: g.players[g.winner]?.name ?? '(left)', won: g.won,
      left: g.order.filter(id => id !== g.winner && g.players[id]).map(id => ({ name: g.players[id].name, n: g.hands[id].length })) };
  },
  act(g, a, id) {
    if (g.phase !== 'turn' || cur(g) !== id) return "It's not your turn";
    if (a.type === 'draw') { draw(g, id, 1); g.log = `${g.players[id].name} drew a card`; return void nextTurn(g); }
    if (a.type !== 'play') return 'Unknown action';
    const i = Number(a.i), c = g.hands[id][i];
    if (!c) return 'Pick a card';
    if (!fits(g, c)) return "That card doesn't match";
    const elem = Number(a.elem);
    if (c.e === 4 && ![0, 1, 2, 3].includes(elem)) return 'Pick an element';
    play(g, id, i, elem);
  },
};

// self-check: run `node games/elemix.js`
if (require.main === module) {
  const assert = require('assert'), m = module.exports;
  let fin = 0;
  m.init({ pick: a => a[0], shuffle: a => [...a], setTimer() {}, finish: () => fin++ });
  const g = { round: 1, players: { a: { name: 'A', score: 0 }, b: { name: 'B', score: 0 }, c: { name: 'C', score: 0 } } };
  m.start(g);
  assert.equal(g.stock.length + g.pile.length + 21, 108);
  assert.equal(g.hands.a.length, 7); assert.equal(typeof top(g).v, 'number');
  // rig a known table
  g.pile = [{ e: 0, v: 5 }]; g.elem = 0; g.turn = 0; g.dir = 1;
  g.hands = { a: [{ e: 1, v: 'freeze' }, { e: 0, v: 'freeze' }, { e: 0, v: 2 }], b: [{ e: 3, v: 1 }], c: [{ e: 4, v: 'storm' }, { e: 2, v: 9 }] };
  assert.ok(m.act(g, { type: 'play', i: 0 }, 'a')); // water freeze doesn't match fire 5
  assert.ok(m.act(g, { type: 'play', i: 2 }, 'b')); // not b's turn
  assert.equal(m.act(g, { type: 'play', i: 1 }, 'a'), undefined); // fire freeze: skips b
  assert.equal(cur(g), 'c');
  assert.ok(m.act(g, { type: 'play', i: 0, elem: 9 }, 'c')); // wild needs an element
  assert.equal(m.act(g, { type: 'play', i: 0, elem: 1 }, 'c'), undefined); // storm, choose water: a draws 4, skipped
  assert.equal(g.hands.a.length, 6); assert.equal(g.elem, 1); assert.equal(cur(g), 'b');
  assert.ok(m.act(g, { type: 'play', i: 0 }, 'b')); // moon 1 doesn't match water storm
  assert.equal(m.act(g, { type: 'draw' }, 'b'), undefined); assert.equal(cur(g), 'c');
  g.hands.c = [{ e: 1, v: 3 }];
  assert.equal(m.act(g, { type: 'play', i: 0 }, 'c'), undefined); // c empties hand and wins
  assert.equal(fin, 1); assert.ok(g.players.c.score > 0);
  // a bot always makes a legal move
  m.start(g); for (let t = 0; t < 500 && fin === 1; t++) { const id = cur(g); assert.equal(m.act(g, m.bot(g, id), id), undefined); }
  console.log('elemix.js ok');
}
