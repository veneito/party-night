// Telephone Sketch: everyone writes a prompt, then the chains pass phone to phone, alternating draw what you read / describe what you see.
// The TV reveals each chain start to finish, then everyone votes for the funniest chain.
const SECS = { tstart: 40, tdraw: 70, tguess: 40, tshow: 20, tvote: 25 };
const MAX_STEPS = 8, MAX_STROKES = 400, MAX_POINTS = 600;
const STARTERS = ['a cat running for president', 'grandma on a skateboard', 'a sad pizza', 'the moon eating a sandwich', 'a dog doing taxes',
  'a haunted toaster', 'a shark at the dentist', 'a cowboy riding a snail', 'a robot falling in love', 'a very angry cloud'];

let h;
const who = (g, c, k) => g.order[(c + k) % g.order.length]; // player handling chain c at step k
const myChain = (g, id) => (g.order.indexOf(id) - g.step + g.order.length * MAX_STEPS) % g.order.length;
function start(g) {
  g.order = h.shuffle(Object.keys(g.players));
  g.chains = g.order.map(() => []); g.step = 0; g.steps = Math.min(g.order.length, MAX_STEPS);
  g.pads = {}; g.tvotes = {};
  g.phase = 'tstart'; h.setTimer(g, SECS.tstart, nextStep);
}
const inStep = (g, id) => g.chains[myChain(g, id)].length > g.step;
function nextStep(g) {
  // fill in anyone who ran out of time, so every chain stays the same length
  g.chains.forEach((ch, c) => { if (ch.length <= g.step) ch.push(g.phase === 'tdraw'
    ? { by: who(g, c, g.step), strokes: g.pads[who(g, c, g.step)] || [] } : { by: who(g, c, g.step), text: g.step ? '(no idea)' : h.pick(STARTERS) }); });
  g.pads = {};
  if (++g.step >= g.steps) { g.show = 0; g.phase = 'tshow'; return h.setTimer(g, SECS.tshow, nextShow); }
  g.phase = g.step % 2 ? 'tdraw' : 'tguess';
  h.setTimer(g, SECS[g.phase], nextStep);
}
function nextShow(g) {
  if (++g.show < g.chains.length) return h.setTimer(g, SECS.tshow, nextShow);
  g.phase = 'tvote'; h.setTimer(g, SECS.tvote, tally);
}
function tally(g) {
  for (const [voter, c] of Object.entries(g.tvotes))
    for (const e of g.chains[c]) if (e.by !== voter && g.players[e.by]) g.players[e.by].score += 200;
  h.finish(g);
}
const chainOut = (g, ch) => ch.map(e => ({ name: g.players[e.by]?.name ?? '(left)', text: e.text, strokes: e.strokes }));

module.exports = {
  id: 'telephone', name: 'Telephone Sketch', min: 3,
  desc: 'Write something, draw what you read, describe what you see. Watch it all fall apart on the TV.',
  tag: 'Drawing · 10 min',
  bot(g, id) {
    if (g.phase === 'tstart' || g.phase === 'tguess') return { type: 'entry', text: h.pick(STARTERS) };
    if (g.phase === 'tdraw') return { type: 'entry', strokes: [{ c: '#231a08', w: 10, p: Array.from({ length: 12 }, () => Math.floor(Math.random() * 1000)) }] };
    if (g.phase === 'tvote') return { type: 'tvote', chain: Math.floor(Math.random() * g.chains.length) };
  },
  init(helpers) { h = helpers; },
  start,
  advance: { tstart: nextStep, tdraw: nextStep, tguess: nextStep, tshow: nextShow, tvote: tally },
  isDone(g, id) {
    if (!g.order.includes(id)) return true;
    if (['tstart', 'tdraw', 'tguess'].includes(g.phase)) return inStep(g, id);
    if (g.phase === 'tvote') return id in g.tvotes;
    return false;
  },
  state(g, s) {
    s.step = g.step; s.steps = g.steps;
    if (g.phase === 'tdraw' || g.phase === 'tguess') // what each player got handed: the previous entry in their chain
      s.task = Object.fromEntries(g.order.map(id => { const e = g.chains[myChain(g, id)][g.step - 1]; return [id, { text: e.text, strokes: e.strokes }]; }));
    if (g.phase === 'tdraw') s.pads = g.pads;
    if (g.phase === 'tshow') { s.show = g.show; s.chain = chainOut(g, g.chains[g.show]); s.chainCount = g.chains.length; }
    if (g.phase === 'tvote') s.firsts = g.chains.map(ch => ch[0].text);
    if ((g.phase === 'scores' || g.phase === 'results') && g.chains) {
      const n = g.chains.map((_, c) => Object.values(g.tvotes).filter(v => v === c).length), best = n.indexOf(Math.max(...n));
      s.last = { votes: n[best], chain: chainOut(g, g.chains[best]) };
    }
  },
  act(g, a, id) {
    const drawing = g.phase === 'tdraw' && !inStep(g, id);
    if (a.type === 'stroke') {
      if (!drawing) return 'Not now';
      const pad = g.pads[id] ||= [], p = Array.isArray(a.p) ? a.p.slice(0, MAX_POINTS * 2).map(n => Math.max(0, Math.min(1000, Math.round(Number(n) || 0)))) : [];
      if (p.length < 2 || pad.length >= MAX_STROKES) return 'Not now';
      pad.push({ c: /^#[0-9a-f]{6}$/i.test(a.c) ? a.c : '#231a08', w: [4, 10, 24].includes(a.w) ? a.w : 10, p });
    } else if (a.type === 'undo' || a.type === 'wipe') {
      if (!drawing) return 'Not now';
      if (a.type === 'undo') g.pads[id]?.pop(); else g.pads[id] = [];
    } else if (a.type === 'entry') {
      if (!['tstart', 'tdraw', 'tguess'].includes(g.phase) || inStep(g, id)) return 'Not now';
      const ch = g.chains[myChain(g, id)];
      if (g.phase === 'tdraw') {
        // bots hand in strokes directly; real players have been sending them one by one
        const strokes = g.players[id].bot && Array.isArray(a.strokes) ? a.strokes : g.pads[id] || [];
        if (!strokes.length) return 'Draw something first!';
        ch.push({ by: id, strokes });
      } else {
        const text = h.clean(a.text, 60);
        if (!text) return g.phase === 'tstart' ? 'Write something to draw' : 'Describe the drawing';
        ch.push({ by: id, text });
      }
    } else if (a.type === 'tvote') {
      const c = Number(a.chain);
      if (g.phase !== 'tvote' || !Number.isInteger(c) || !g.chains[c]) return 'Not now';
      g.tvotes[id] = c;
    } else return 'Unknown action';
  },
};

// self-check: run `node games/telephone.js`
if (require.main === module) {
  const assert = require('assert'), m = module.exports;
  let fin = 0;
  m.init({ pick: a => a[0], shuffle: a => a, clean: (v, n) => String(v || '').trim().slice(0, n), setTimer() {}, finish: () => fin++ });
  const g = { round: 1, players: { a: { score: 0 }, b: { score: 0 }, c: { score: 0 } } }, line = { p: [0, 0, 10, 10] };
  const act = (id, x) => { const e = m.act(g, x, id); if (!e && Object.keys(g.players).every(p => m.isDone(g, p))) m.advance[g.phase](g); return e; };
  m.start(g);
  for (const id of 'abc') assert.equal(act(id, { type: 'entry', text: 'prompt ' + id }), undefined);
  assert.equal(g.phase, 'tdraw');
  assert.ok(act('a', { type: 'entry' })); // nothing drawn yet
  for (const id of 'abc') { act(id, { type: 'stroke', ...line }); act(id, { type: 'entry' }); }
  assert.equal(g.phase, 'tguess');
  act('a', { type: 'entry', text: 'guess a' }); m.advance.tguess(g); // b and c time out
  assert.equal(g.phase, 'tshow');
  // chain 0 was started by a, drawn by b, described by c (who timed out)
  assert.deepEqual(g.chains[0].map(e => e.by), ['a', 'b', 'c']);
  assert.equal(g.chains[0][2].text, '(no idea)');
  m.advance.tshow(g); m.advance.tshow(g); m.advance.tshow(g);
  assert.equal(g.phase, 'tvote');
  act('a', { type: 'tvote', chain: 1 }); act('b', { type: 'tvote', chain: 1 }); act('c', { type: 'tvote', chain: 1 });
  assert.equal(fin, 1);
  assert.equal(g.players.a.score, 400); // chain 1 = b, c, a: each scores 200 per vote from someone else
  console.log('telephone.js ok');
}
