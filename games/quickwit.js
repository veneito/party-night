// Quick Wit: everyone answers the same prompt, then votes for the best answer.
const SECS = { answer: 60, vote: 30 };
// Prompts are specific little situations rather than generic "the worst X" lines: specifics get funnier answers.
const PROMPTS = {
  family: [
    'Your pilot comes on the speaker and says, very calmly:',
    'The real reason the dinosaurs went extinct',
    'You open a fortune cookie. It just says:',
    'Your dog finally learns to talk. Its first sentence:',
    'The name of the worst theme park in the world',
    'The thing your teacher actually does all weekend',
    'A superhero whose only power is extremely mild',
    'The new flavor of ice cream that got the company sued',
    'You find a note in your lunchbox from your mom. It says:',
    'What the cat is writing in its diary about you',
    'The worst thing to hear from the person cutting your hair',
    'A terrible name for a boat',
    'The fourth Little Pig built his house out of',
    'Grandma\'s secret recipe has one weird ingredient:',
    'The rule the lifeguard made up just for you',
    'A theme for a birthday party that made every kid cry',
    'What aliens write in their review of Earth',
    'The worst mascot for a sports team',
    'The real reason Santa only works one night a year',
    'The museum\'s newest exhibit, which nobody understands',
    'You\'re the new GPS voice. What do you say when someone misses the exit?',
    'Your houseplant left you a note before it died. It says:',
    'The school just banned this from lunch, and kids are furious:',
    'The worst thing to find in your cereal box',
    "Your robot vacuum has started leaving notes. Today's says:",
    'The real reason the Easter Bunny hides the eggs',
    'A terrible name for a hamster',
    'The worst prize at the fair',
    'What the snowman is thinking right before he melts',
    "Your teacher's secret talent, revealed at the school assembly",
    'A board game that ended a family friendship',
    'The weirdest thing a squirrel could steal from you',
    'The first rule of the treehouse club',
    'What your sneakers would say if they could talk',
    'A new holiday nobody asked for',
    "The tooth fairy's side hustle",
    'Things you should never say to a pirate',
  ],
  spicy: [
    'Your date leans in close and whispers... "I have to tell you something."',
    'The text you accidentally sent to your boss instead of your crush',
    'What Grandpa\'s "special" brownies are actually made of',
    'The one line in your dating profile that gets zero matches',
    'Your mom finds your browser history. The first thing she sees:',
    'The real reason you got dumped, according to your ex\'s group chat',
    'A terrible safe word',
    'What the priest is sweating about this Sunday',
    'Your therapist\'s private notes about you just say:',
    'The worst thing to yell at the exact wrong moment',
    'A condom flavor that should never have left the lab',
    'The 11th commandment, cut for being "too specific"',
    'What actually happened at the office Christmas party',
    'Your roommate heard this through the wall. Now they won\'t look at you.',
    'The most cursed thing to say when meeting your partner\'s parents',
    'The strip club with the worst name in town',
    'The tattoo you got in Vegas and can\'t explain',
    'Your Uber driver turns around and says:',
    'What your search history would say about you at your funeral',
    'A sex position named after an IKEA product',
    'The walk of shame, but make it worse. You\'re carrying:',
    'The divorce lawyer\'s slogan',
    'What\'s REALLY in the hot tub',
    'The worst bachelor party activity, planned by your weird cousin',
    'The noise your bed makes that ended the relationship',
    'Your Tinder date shows up with:',
    'The reason the neighbours called the cops (again)',
    'The worst thing to say during a job interview, and you said it',
    'Your new tattoo artist was clearly drunk. Your tattoo now says:',
    'The real reason the wedding was called off',
    'What your Uber rating says about you',
    'The last thing you want to hear from the next bathroom stall',
    'The note your one-night stand left on the pillow',
    "Your bachelor party ended at the hospital. The doctor's report says:",
    'What your Spotify Wrapped revealed to the whole office',
    'A terrible thing to whisper to a horse',
    'The worst name for a sex toy company',
    'Your houseguest found the drawer. Which drawer?',
    "What's really in Grandma's 'medicine' cabinet",
    'The new HR policy, written because of you specifically',
    'The worst thing to accidentally post on the family group chat',
    "Your landlord's most unhinged text",
  ],
};

let h;
// Caption This reuses this whole flow with its own prompts, see make() below
const make = ({ prompts, ...meta }) => {
function start(g) {
  g.phase = 'answer'; g.answers = {}; g.votes = {}; g.list = [];
  g.prompt = h.pick(prompts(g));
  h.setTimer(g, SECS.answer, startVote);
}
function startVote(g) {
  g.list = h.shuffle(Object.entries(g.answers).map(([owner, text]) => ({ owner, text })));
  if (g.list.length < 2) return tally(g);
  g.phase = 'vote';
  h.setTimer(g, SECS.vote, tally);
}
function tally(g) {
  for (const i of Object.values(g.votes)) { const p = g.players[g.list[i]?.owner]; if (p) p.score += g.settings.points || 100; }
  h.finish(g);
}

return {
  id: 'quickwit', name: 'Quick Wit', min: 2, art: '⚡',
  desc: 'One prompt, everyone answers, everyone votes. Fast and simple.',
  tag: 'Quick · 3 min',
  settings: [{ key: 'pack', label: 'Prompts', options: [['spicy', 'Spicy 18+'], ['family', 'Family']] }, { key: 'points', label: 'Points per vote', options: [[100, '100'], [250, '250'], [500, '500']] }],
  ...meta,
  bot(g, id) {
    if (g.phase === 'answer') return { type: 'answer', text: h.pick(['a confused llama', 'soup, again', 'my uncle Steve', 'tiny hats', 'nobody knows', 'free WiFi']) };
    if (g.phase === 'vote') { const ok = g.list.map((a, i) => a.owner === id ? -1 : i).filter(i => i >= 0); return { type: 'vote', index: h.pick(ok) }; }
  },
  init(helpers) { h = helpers; },
  start,
  advance: { answer: startVote, vote: tally },
  isDone(g, id) {
    if (g.phase === 'answer') return id in g.answers;
    // a lone answer's author has nothing to vote for, so they count as done
    if (g.phase === 'vote') return id in g.votes || g.list.every(a => a.owner === id);
    return false;
  },
  state(g, s) {
    if (g.phase === 'answer' || g.phase === 'vote') s.prompt = g.prompt;
    // owners stay hidden until results so votes are anonymous
    if (g.phase === 'vote') s.list = g.list.map(a => ({ text: a.text }));
    if ((g.phase === 'scores' || g.phase === 'results') && g.list) s.last = { prompt: g.prompt, list: g.list.map((a, i) => ({
      text: a.text, name: g.players[a.owner]?.name ?? '(left)', votes: Object.values(g.votes).filter(v => v === i).length })) };
  },
  act(g, a, id) {
    if (a.type === 'answer') {
      const text = h.clean(a.text, 80);
      if (g.phase !== 'answer' || !text) return 'Not now';
      g.answers[id] = text;
    } else if (a.type === 'vote') {
      const i = Number(a.index);
      if (g.phase !== 'vote' || !g.list[i]) return 'Not now';
      if (g.list[i].owner === id) return "Can't vote for yourself";
      g.votes[id] = i;
    } else return 'Unknown action';
  },
};
};
module.exports = make({ prompts: g => PROMPTS[g.settings.pack] || PROMPTS.spicy });
module.exports.make = make;
