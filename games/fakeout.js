// Fake Out: a real fact with a blank. Everyone writes a believable fake, then tries to pick the real one.
// Picking the truth scores, and so does every player your fake fooled.
const SECS = { lie: 60, pick: 30 };
const FACTS = [
  ['In Japan, there is an island overrun by friendly ___.', 'rabbits'],
  ['The national animal of Scotland is the ___.', 'unicorn'],
  ['Before toilet paper, many Americans used ___.', 'corn cobs'],
  ['A group of flamingos is called a ___.', 'flamboyance'],
  ['Wombats are the only animals that poop in the shape of ___.', 'cubes'],
  ['In 1932, Australia went to war against ___ and lost.', 'emus'],
  ['The inventor of the Pringles can was buried in ___.', 'a Pringles can'],
  ['Octopuses have ___ hearts.', 'three'],
  ['In Switzerland it is illegal to own just one ___.', 'guinea pig'],
  ['The first thing ever sold on eBay was a broken ___.', 'laser pointer'],
  ['Ketchup was sold in the 1830s as a ___.', 'medicine'],
  ['A bolt of lightning is about five times hotter than ___.', "the sun's surface"],
  ['Sea otters hold hands while they ___.', 'sleep'],
  ['The dot over a lowercase "i" is called a ___.', 'tittle'],
  ['Bananas are slightly ___.', 'radioactive'],
  ['Cows have best friends and get ___ when separated.', 'stressed'],
  ['Honey found in ancient Egyptian tombs was still ___.', 'edible'],
  ['A snail can sleep for up to ___.', 'three years'],
  ['The longest English word you can type with only the top keyboard row is ___.', 'typewriter'],
  ['Scientists once gave ___ LSD to see how it affected their webs.', 'spiders'],
  ["The world's oldest known piece of chewing gum is ___ years old.", '9,000'],
  ["In Iceland, there's an app to check you're not dating your ___.", 'cousin'],
  ['A day on Venus is longer than a year on ___.', 'Venus'],
  ['The tiny pocket in jeans was first made to hold ___.', 'pocket watches'],
  ['Butterflies taste with their ___.', 'feet'],
  ["A shrimp's heart is in its ___.", 'head'],
  ['The inventor of the frisbee was turned into a frisbee after he ___.', 'died'],
  ['Peanuts are not nuts, they are ___.', 'legumes'],
  ["The first oranges weren't orange, they were ___.", 'green'],
  ['A group of crows is called a ___.', 'murder'],
  ['Hot water can freeze faster than cold water. This is called the ___ effect.', 'Mpemba'],
  ["Koalas have fingerprints so similar to humans' they could confuse ___.", 'crime scene investigators'],
  ['The Eiffel Tower can be about 15 cm taller in the ___.', 'summer'],
  ['The first video ever uploaded to YouTube was filmed at a ___.', 'zoo'],
];
const norm = s => s.toLowerCase().replace(/[^a-z0-9]/g, '');

let h;
function start(g) {
  g.phase = 'lie'; g.lies = {}; g.picks = {}; g.list = [];
  g.used ??= [];
  const fresh = FACTS.filter(f => !g.used.includes(f[0]));
  g.fact = h.pick(fresh.length ? fresh : FACTS); g.used.push(g.fact[0]);
  h.setTimer(g, SECS.lie, startPick);
}
function startPick(g) {
  g.list = h.shuffle([{ text: g.fact[1], truth: true }, ...Object.entries(g.lies).map(([owner, text]) => ({ owner, text }))]);
  g.phase = 'pick';
  h.setTimer(g, SECS.pick, tally);
}
function tally(g) {
  for (const [id, i] of Object.entries(g.picks)) {
    const a = g.list[i];
    if (a.truth) { if (g.players[id]) g.players[id].score += 500; }
    else if (g.players[a.owner]) g.players[a.owner].score += 250;
  }
  h.finish(g);
}

module.exports = {
  id: 'fakeout', name: 'Fake Out', min: 3, art: '🕵️',
  desc: 'A weird real fact with a blank. Write a fake answer, then spot the truth and fool your friends.',
  tag: 'Bluffing · 6 min',
  bot(g, id) {
    if (g.phase === 'lie') return { type: 'lie', text: h.pick(['penguins', 'a spoon', 'seven', 'socks', 'cheese', 'pigeons', 'a lawnmower', 'twelve years']) };
    if (g.phase === 'pick') { const ok = g.list.map((a, i) => a.owner === id ? -1 : i).filter(i => i >= 0); return { type: 'guess', index: h.pick(ok) }; }
  },
  init(helpers) { h = helpers; },
  start,
  advance: { lie: startPick, pick: tally },
  isDone(g, id) {
    if (g.phase === 'lie') return id in g.lies;
    if (g.phase === 'pick') return id in g.picks;
    return false;
  },
  state(g, s) {
    if (g.fact) s.fact = g.fact[0];
    if (g.phase === 'pick') s.list = g.list.map(a => ({ text: a.text }));
    if ((g.phase === 'scores' || g.phase === 'results') && g.list.length) s.last = { fact: g.fact[0], list: g.list.map((a, i) => ({
      text: a.text, truth: !!a.truth, name: a.truth ? null : g.players[a.owner]?.name ?? '(left)',
      fooled: Object.entries(g.picks).filter(([, v]) => v === i).map(([pid]) => g.players[pid]?.name ?? '(left)') })) };
  },
  act(g, a, id) {
    if (a.type === 'lie') {
      const text = h.clean(a.text, 40);
      if (g.phase !== 'lie' || !text) return 'Not now';
      if (norm(text) === norm(g.fact[1])) return "That's actually the real answer! Try a fake one";
      if (Object.entries(g.lies).some(([o, t]) => o !== id && norm(t) === norm(text))) return 'Someone already wrote that, try another';
      g.lies[id] = text;
    } else if (a.type === 'guess') {
      const i = Number(a.index);
      if (g.phase !== 'pick' || !g.list[i]) return 'Not now';
      if (g.list[i].owner === id) return "That's your own fake";
      g.picks[id] = i;
    } else return 'Unknown action';
  },
};
