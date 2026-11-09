// Hot Take: agree or disagree, then guess how much of the room agreed. Closest guesses score.
const SECS = 40;
const TAKES = ['Cereal is a soup.', 'A hot dog is a sandwich.', 'Pineapple belongs on pizza.', 'Socks with sandals is fine.',
  'Breakfast for dinner beats dinner.', 'Cats are better than dogs.', 'Water is wet.', "It's fine to recline your seat on a plane.",
  'Movies were better before phones.', 'Ghosting is OK after one date.', 'The toilet paper goes over, not under.', "Mondays aren't that bad.",
  'Texting "k" is rude.', 'Pluto is a planet.', 'You should shower at night, not in the morning.', "Birthdays after 30 don't matter.",
  'Sharing a dessert is a crime.', 'Gifts should come with receipts.', 'Ketchup is a smoothie.', 'Singing in the car alone counts as a concert.',
  'Board games ruin friendships.', 'Leftover pizza is better cold.', 'Every movie is too long now.', 'Sleeping in socks is fine.', 'Kids should be allowed to swear at home.', 'Snow days should exist for adults too.', 'The middle seat gets both armrests.', 'Voice notes are rude.', 'Bananas are better green.', "It's OK to read the last page of a book first.", 'Camping is just being homeless on purpose.', 'You should tip at self-checkout.', 'Halloween is the best holiday.', 'Talking to plants actually works.', 'Chocolate ice cream beats vanilla.'];

let h;
function start(g) {
  g.phase = 'take'; g.takes = {};
  g.used ??= [];
  const fresh = TAKES.filter(t => !g.used.includes(t));
  g.take = h.pick(fresh.length ? fresh : TAKES); g.used.push(g.take);
  h.setTimer(g, SECS, tally);
}
const pct = g => { const t = Object.values(g.takes); return t.length ? Math.round(t.filter(x => x.agree).length / t.length * 100) : 0; };
function tally(g) {
  const real = pct(g);
  // 500 for a perfect guess, 25 fewer per point off, never below zero
  for (const [id, t] of Object.entries(g.takes)) if (g.players[id]) g.players[id].score += Math.max(0, 500 - 25 * Math.abs(t.guess - real));
  h.finish(g);
}

module.exports = {
  id: 'hottake', name: 'Hot Take', min: 2, art: '🌶️',
  desc: 'Agree or disagree, then guess how much of the room agreed with you. No typing.',
  tag: 'Quick · 3 min',
  bot() { return { type: 'take', agree: Math.random() < .5, guess: Math.round(Math.random() * 10) * 10 }; },
  init(helpers) { h = helpers; },
  start,
  advance: { take: tally },
  isDone(g, id) { return g.phase === 'take' && id in g.takes; },
  state(g, s) {
    if (g.phase === 'take') s.take = g.take;
    if ((g.phase === 'scores' || g.phase === 'results') && g.take) s.last = { take: g.take, pct: pct(g),
      list: Object.entries(g.takes).map(([id, t]) => ({ name: g.players[id]?.name ?? '(left)', av: g.players[id]?.av ?? 0, ...t })) };
  },
  act(g, a, id) {
    if (a.type !== 'take') return 'Unknown action';
    const guess = Number(a.guess);
    if (g.phase !== 'take') return 'Not now';
    if (typeof a.agree !== 'boolean') return 'Agree or disagree first';
    if (!(guess >= 0 && guess <= 100)) return 'Guess a number from 0 to 100';
    g.takes[id] = { agree: a.agree, guess: Math.round(guess) };
  },
};
