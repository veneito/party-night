// Punchline Club: submit words -> build joke setups from others' words + write punchlines -> jokes duel head-to-head.
const SECS = { words: 45, write: 120, duel: 25, reveal: 7 };
const WORDS_EACH = 3, OPTIONS = 3;
const jokesEach = g => g.settings.jokes || 2;
// Two prompt packs, chosen in the lobby. Each category is what a phone asks for (with an example so people
// know what to type), the bots' answers, and setups written so ANY answer for that category reads naturally.
// Rule for setups: {w} always stands alone as a noun phrase — never "a {w}" or "my {w}" — since players
// type their own "a"/"my"/"the".
const PACKS = {
  family: {
    'a person you know (like "my dentist")': {
      bot: ['my dentist', 'Grandma', 'my gym teacher', 'the mailman', 'my little brother'],
      setups: ['{w} has a secret YouTube channel, and it\'s all', '{w} is banned from the bowling alley because', 'Nobody sits next to {w} at lunch since',
        '{w} just won a talent show by', 'The scariest thing {w} ever said to me:'],
    },
    'a famous person (like "Shrek")': {
      bot: ['Shrek', 'Elmo', 'Gordon Ramsay', 'Abraham Lincoln', 'Darth Vader'],
      setups: ['{w} showed up to my birthday party and', '{w} just released a cookbook called', 'Breaking news: {w} has been spotted',
        '{w} tried online dating. Their bio says', 'I beat {w} at Mario Kart and they'],
    },
    'an animal (like "a raccoon")': {
      bot: ['a raccoon', 'a goose', 'my neighbour\'s cat', 'a llama', 'a very large pigeon'],
      setups: ['I let {w} babysit my kids and', 'You know {w} is plotting something when', '{w} got into the school play, playing',
        'If {w} could talk, the first thing it would say is', 'I trained {w} to fetch, and now it brings me'],
    },
    'a place (like "the DMV")': {
      bot: ['the DMV', 'Olive Garden', 'a Costco', 'the moon', 'a haunted car wash'],
      setups: ['{w} has a new tourism slogan:', 'I got lost in {w} for three days and survived by', 'The worst part of a school trip to {w} is',
        'Never go to {w} on a Monday, because', '{w} just got its first one-star review:'],
    },
    'an object (like "a garden hose")': {
      bot: ['a garden hose', 'a rubber duck', 'a haunted lamp', 'a leaf blower', 'a broken trampoline'],
      setups: ['Grandma left me {w} in her will, with a note that said', 'The instruction manual for {w} just says', 'I brought {w} to show-and-tell and',
        'Airport security pulled {w} out of my bag and', 'The worst thing to get stuck in an elevator with is {w}, because'],
    },
    'a hobby (like "competitive yodeling")': {
      bot: ['competitive yodeling', 'knitting', 'bird watching', 'speed walking', 'ventriloquism'],
      setups: ['I took up {w} to relax. Now', 'You can always spot someone into {w} because', 'My kid wants to go pro at {w}. I told them',
        'The Olympics just added {w}, and the first gold medal went to', 'Nobody warned me that {w} would'],
    },
  },
  spicy: {
    'someone you know (like "my landlord")': {
      bot: ['my landlord', 'Aunt Karen', 'my ex', 'my boss', 'the guy from accounting'],
      setups: ['{w} has a secret second family, and honestly', '{w} isn\'t invited to Thanksgiving anymore because', 'I walked in on {w} and',
        '{w} just got arrested. The charge?', 'At the reading of the will, {w} finally admitted'],
    },
    'a celebrity (like "Shrek")': {
      bot: ['Shrek', 'Gordon Ramsay', 'the Pope', 'Danny DeVito', 'Santa Claus'],
      setups: ['{w} slid into my DMs with just', '{w}\'s OnlyFans is mostly', '{w} just got cancelled for',
        'I\'m pretty sure {w} is the reason my parents got divorced. They', 'Leaked texts show {w} is into'],
    },
    'an object (like "a spatula")': {
      bot: ['a spatula', 'a garden hose', 'a scented candle', 'a leaf blower', 'a fanny pack'],
      setups: ['Our safe word is {w}, ever since', 'Grandma left me {w} in her will, with a note that said', 'Airport security pulled {w} out of my bag and',
        'I\'m not saying {w} saved my marriage, but', 'The cops found {w} at the crime scene, and'],
    },
    'a place (like "Olive Garden")': {
      bot: ['Olive Garden', 'the DMV', 'a Chuck E. Cheese', 'church camp', 'my parents\' basement'],
      setups: ['Our first date was at {w}. By dessert,', 'I got banned from {w} for', 'What happens at {w} stays at {w}, except',
        'I lost my dignity at {w} when', 'Proposing at {w} was a mistake because'],
    },
    'a bad habit (like "drunk texting")': {
      bot: ['drunk texting', 'eating cheese in bed', 'online gambling', 'replying "k"', 'stalking my ex'],
      setups: ['My therapist says {w} is why I', 'I gave up {w} for Lent and lasted', 'The family intervention was about {w}, but nobody mentioned',
        'My dating profile proudly lists {w} under', '{w} has cost me two jobs, one marriage and'],
    },
    'a food (like "a gas station hot dog")': {
      bot: ['a gas station hot dog', 'expired yogurt', 'a whole rotisserie chicken', 'airplane sushi', 'Grandma\'s meatloaf'],
      setups: ['Nothing says romance like {w} and', 'I ate {w} at 3am and woke up', 'The wedding caterer only served {w}, so naturally',
        'My doctor said I have to stop eating {w}, or else', 'I brought {w} to the orgy and'],
    },
    'an animal (like "a raccoon")': {
      bot: ['a raccoon', 'a goose', 'a horny pigeon', 'my roommate\'s python', 'a possum'],
      setups: ['I let {w} plan my bachelor party and', 'My ex left me for {w}, and honestly', 'The vet says {w} has been',
        'I came home to find {w} in my bed, and', '{w} is my emotional support animal, mostly because'],
    },
  },
};
// extra setups, merged into the packs above by category
const MORE = {
  family: [
    ['{w} entered the science fair with', 'The principal called home because {w}'],
    ['{w} got a star on the Walk of Fame for', 'My teacher looks exactly like {w}, especially when'],
    ['{w} ate my homework and left a note:', 'The zoo had to close because {w}'],
    ['{w} is the only place where you can', 'Our family vacation to {w} ended with'],
    ['Dad tried to fix the car with {w} and', 'The museum put {w} on display, labelled'],
    ['My grandpa still holds the world record in {w} for', 'Our school added {w} to gym class, and now'],
  ],
  spicy: [
    ['{w} was the last person I expected to see at the strip club, but', 'My therapist and {w} both agree that'],
    ['{w} officiated our wedding and kept saying', '{w} has a podcast, and every episode is about'],
    ['The hotel minibar now includes {w}, because', 'I tried to return {w} to IKEA and'],
    ['The office retreat at {w} ended with HR', 'My Tinder bio says "find me at {w}", and'],
    ['I listed {w} as a skill on my résumé, and the interviewer', 'My last relationship ended because of {w} and'],
    ['The first date went great until {w} came out and', 'Nothing ruins a hookup like {w}, especially'],
    ['{w} got into the hot tub and', 'The neighbours think {w} is our child because'],
  ],
};
for (const k of ['family', 'spicy']) Object.values(PACKS[k]).forEach((c, i) => c.setups.push(...(MORE[k][i] || [])));
const pack = g => PACKS[g.settings.pack] || PACKS.spicy;
const cap = s => s[0].toUpperCase() + s.slice(1); // "a raccoon is..." -> "A raccoon is..."
// works with any category's setups, so bot jokes still read like jokes
const BOT_PUNCH = ['my mom still brings it up at Christmas', 'now we\'re legally married', 'the fire department got involved', 'I still have the receipt',
  'nobody clapped', 'it was the best night of my life', 'the HOA sent a strongly worded letter', 'it\'s on the news', 'I\'m not allowed back in Ohio',
  'we don\'t talk about it', 'that\'s how I met your mother', 'I had to change my name'];

let h; // helpers from the server core
const duel = g => g.duels?.[g.di];
const jokesBy = (g, id) => g.jokes.filter(j => j.owner === id).length;

function start(g) {
  g.phase = 'words'; g.wordsIn = {}; g.words = []; g.jokes = []; g.votes = {};
  g.cats = Object.fromEntries(Object.keys(g.players).map(id => [id, h.shuffle(Object.keys(pack(g))).slice(0, WORDS_EACH)]));
  h.setTimer(g, SECS.words, startWrite);
}
function startWrite(g) {
  g.phase = 'write'; g.opts = {};
  for (const id of Object.keys(g.players)) {
    // prefer other players' words; fall back to own, then to a generic word so nobody is stuck
    const pool = g.words.filter(w => w.owner !== id);
    const src = pool.length ? pool : g.words.length ? g.words : [{ cat: Object.keys(pack(g))[0], w: 'a rubber duck' }];
    g.opts[id] = Array.from({ length: jokesEach(g) }, () => {
      const set = new Set();
      for (let tries = 0; set.size < OPTIONS && tries < 50; tries++) { const w = h.pick(src); set.add(cap(h.pick((pack(g)[w.cat] || Object.values(pack(g))[0]).setups).replaceAll('{w}', w.w))); }
      return [...set];
    });
  }
  h.setTimer(g, SECS.write, startDuels);
}
function startDuels(g) {
  // pair jokes by different authors; a leftover joke sits out
  const left = h.shuffle(g.jokes), duels = [];
  const count = o => left.filter(j => j.owner === o).length;
  while (left.length >= 2) {
    left.sort((x, y) => count(y.owner) - count(x.owner)); // re-rank each pass: pair from the biggest pile
    const a = left.shift(), i = left.findIndex(j => j.owner !== a.owner);
    if (i >= 0) duels.push(h.shuffle([a, left.splice(i, 1)[0]]));
  }
  g.duels = duels; g.di = -1;
  nextDuel(g);
}
function nextDuel(g) {
  g.di++; g.votes = {};
  if (g.di >= g.duels.length) return h.finish(g);
  g.phase = 'duel';
  h.setTimer(g, SECS.duel, reveal);
}
function reveal(g) {
  const d = duel(g), counts = [0, 0];
  for (const v of Object.values(g.votes)) counts[v]++;
  d.forEach((j, i) => { const p = g.players[j.owner]; if (p) p.score += counts[i] * 100; });
  // bonus for a clean sweep with at least 2 votes
  const w = counts[0] > counts[1] ? 0 : counts[1] > counts[0] ? 1 : -1;
  if (w >= 0 && counts[1 - w] === 0 && counts[w] >= 2 && g.players[d[w].owner]) g.players[d[w].owner].score += 200;
  g.phase = 'reveal';
  h.setTimer(g, SECS.reveal, nextDuel, true);
}

module.exports = {
  id: 'punchline', name: 'Punchline Club', min: 3,
  desc: 'Give words, build jokes from your friends\' words, then battle it out joke vs joke.',
  tag: 'Comedy · 15 min',
  settings: [
    { key: 'pack', label: 'Prompts', options: [['spicy', 'Spicy 18+'], ['family', 'Family']] },
    { key: 'jokes', label: 'Jokes each', options: [[2, '2'], [1, '1 (quicker)'], [3, '3']] },
  ],
  bot(g, id) {
    if (g.phase === 'words') return { type: 'words', words: g.cats[id].map(c => h.pick(pack(g)[c]?.bot || ['a rubber duck'])) };
    if (g.phase === 'write') return { type: 'joke', pick: 0, text: h.pick(BOT_PUNCH) };
    if (g.phase === 'duel') return { type: 'vote', index: h.pick([0, 1]) };
  },
  init(helpers) { h = helpers; },
  start,
  advance: { words: startWrite, write: startDuels, duel: reveal, reveal: nextDuel },
  isDone(g, id) {
    if (g.phase === 'words') return id in g.wordsIn;
    if (g.phase === 'write') return jokesBy(g, id) >= jokesEach(g);
    if (g.phase === 'duel') return id in g.votes || duel(g).some(j => j.owner === id);
    return false;
  },
  state(g, s) {
    // Note: per-player prompts ride in the shared broadcast (a peek in devtools shows others'), fine among friends.
    if (g.phase === 'words') s.cats = g.cats;
    if (g.phase === 'write') s.opts = Object.fromEntries(Object.keys(g.players).map(id => [id, g.opts[id]?.[jokesBy(g, id)] || null]));
    if (g.phase === 'duel' || g.phase === 'reveal') {
      const rev = g.phase === 'reveal';
      s.duelNo = g.di + 1; s.duelCount = g.duels.length;
      s.duel = duel(g).map((j, i) => ({ setup: j.setup, punch: j.punch,
        ...(rev && { name: g.players[j.owner]?.name ?? '(left)', votes: Object.values(g.votes).filter(v => v === i).length }) }));
    }
  },
  act(g, a, id) {
    if (a.type === 'words') {
      if (g.phase !== 'words' || id in g.wordsIn) return 'Not now';
      const ws = Array.isArray(a.words) ? a.words.map(w => h.clean(w, 30)) : [];
      if (ws.length !== WORDS_EACH || ws.some(w => !w)) return 'Fill in every word';
      g.wordsIn[id] = true;
      ws.forEach((w, i) => g.words.push({ cat: g.cats[id][i], w, owner: id }));
    } else if (a.type === 'joke') {
      if (g.phase !== 'write' || !g.opts[id] || jokesBy(g, id) >= jokesEach(g)) return 'Not now';
      const setup = g.opts[id][jokesBy(g, id)]?.[Number(a.pick)], punch = h.clean(a.text, 100);
      if (!setup) return 'Pick a setup';
      if (!punch) return 'Write a punchline';
      g.jokes.push({ owner: id, setup, punch });
    } else if (a.type === 'vote') {
      const i = Number(a.index);
      if (g.phase !== 'duel' || (i !== 0 && i !== 1)) return 'Not now';
      if (duel(g).some(j => j.owner === id)) return "It's your joke, sit tight";
      g.votes[id] = i;
    } else return 'Unknown action';
  },
};
