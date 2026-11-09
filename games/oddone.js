// Odd One Out: everyone gets the same secret word except one player. Everyone gives a one-word clue,
// then the room votes on who had the different word. The odd one wins by blending in.
const SECS = { clue: 45, accuse: 30 };
const PAIRS = [['pizza', 'burger'], ['beach', 'pool'], ['cat', 'dog'], ['coffee', 'tea'], ['wedding', 'funeral'], ['guitar', 'violin'],
  ['airport', 'train station'], ['vampire', 'zombie'], ['snow', 'rain'], ['doctor', 'dentist'], ['pirate', 'ninja'], ['moon', 'sun'],
  ['bank', 'casino'], ['soccer', 'basketball'], ['lion', 'tiger'], ['hotel', 'hospital'], ['ice cream', 'yogurt'], ['library', 'museum'],
  ['camping', 'picnic'], ['chess', 'poker'], ['birthday', 'christmas'], ['spider', 'snake'], ['jeans', 'pajamas'], ['zoo', 'farm'],
  ['shark', 'dolphin'], ['taco', 'burrito'], ['knight', 'samurai'], ['volcano', 'earthquake'], ['candle', 'flashlight'], ['cowboy', 'pirate'], ['umbrella', 'raincoat'], ['bakery', 'pizzeria'], ['wizard', 'witch'], ['tennis', 'badminton'], ['ghost', 'skeleton'], ['subway', 'bus'], ['honey', 'syrup'], ['owl', 'bat'], ['desert', 'jungle']];

let h;
function start(g) {
  g.phase = 'clue'; g.clues = {}; g.votes = {};
  const [a, b] = h.shuffle(h.pick(PAIRS));
  g.word = a; g.oddWord = b; g.odd = h.pick(Object.keys(g.players));
  h.setTimer(g, SECS.clue, startAccuse);
}
function startAccuse(g) { g.phase = 'accuse'; h.setTimer(g, SECS.accuse, tally); }
function tally(g) {
  const count = {};
  for (const v of Object.values(g.votes)) count[v] = (count[v] || 0) + 1;
  const top = Math.max(0, ...Object.values(count));
  // caught = the odd one got the most votes (a tie at the top counts as caught)
  g.caught = top > 0 && count[g.odd] === top;
  if (g.caught) { for (const [id, v] of Object.entries(g.votes)) if (v === g.odd && g.players[id]) g.players[id].score += 500; }
  else if (g.players[g.odd]) g.players[g.odd].score += 1000;
  h.finish(g);
}

module.exports = {
  id: 'oddone', name: 'Odd One Out', min: 3,
  desc: "Everyone gets the same secret word but one. Give a clue, then find who's faking it.",
  tag: 'Deduction · 5 min',
  bot(g, id) {
    if (g.phase === 'clue') return { type: 'clue', text: h.pick(['yummy', 'fun', 'big', 'loud', 'weird', 'summer', 'expensive', 'hmm', 'classic', 'messy']) };
    if (g.phase === 'accuse') return { type: 'accuse', who: h.pick(Object.keys(g.clues).filter(p => p !== id)) };
  },
  init(helpers) { h = helpers; },
  start,
  advance: { clue: startAccuse, accuse: tally },
  isDone(g, id) {
    if (g.phase === 'clue') return id in g.clues;
    if (g.phase === 'accuse') return id in g.votes;
    return false;
  },
  state(g, s) {
    // Note: secret words ride in the shared broadcast (a peek in devtools shows them), fine among friends.
    if (g.phase === 'clue') s.secret = Object.fromEntries(Object.keys(g.players).map(id => [id, id === g.odd ? g.oddWord : g.word]));
    if (g.phase === 'accuse') s.clues = Object.entries(g.clues).map(([id, text]) => ({ id, text }));
    if ((g.phase === 'scores' || g.phase === 'results') && g.word) s.last = { word: g.word, oddWord: g.oddWord, caught: g.caught,
      odd: g.players[g.odd]?.name ?? '(left)',
      clues: Object.entries(g.clues).map(([id, text]) => ({ name: g.players[id]?.name ?? '(left)', text, odd: id === g.odd,
        votes: Object.values(g.votes).filter(v => v === id).length })) };
  },
  act(g, a, id) {
    if (a.type === 'clue') {
      const text = h.clean(a.text, 20);
      if (g.phase !== 'clue' || !text) return 'Not now';
      if (/\s/.test(text)) return 'One word only';
      if ((id === g.odd ? g.oddWord : g.word).toLowerCase().includes(text.toLowerCase())) return "Don't just say the word!";
      g.clues[id] = text;
    } else if (a.type === 'accuse') {
      const who = h.clean(a.who, 40);
      if (g.phase !== 'accuse' || !(who in g.clues)) return 'Not now';
      if (who === id) return "You can't pick yourself";
      g.votes[id] = who;
    } else return 'Unknown action';
  },
};
