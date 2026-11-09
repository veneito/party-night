// Caption This: the TV shows a little emoji scene, everyone captions it, everyone votes. Same flow as Quick Wit.
const SCENES = ['🦖☕🎩', '🐸👑🚽', '👵🏍️🔥', '🐧🏖️🍹', '🤖💔🍕', '🐄🛸😱', '🧛‍♂️🦷🪥', '🐌🏁🏆', '👨‍🍳🐙🍝', '🦆🔪🍞',
  '🐻🛁🦆', '👻📸😐', '🐢💨🚓', '🧙‍♂️📱❌', '🐔🎤🌟', '🦒🛗😬', '🐱💻📉', '🧟‍♂️🛒🥦', '🐝💍🌸', '🐶🎓📜', '🦀💼📈', '🐼🎂🕯️',
  '🐷💅✨', '🦉📚😴', '🐊🦷🧑‍⚕️', '👽🍔🤤', '🐑✂️😳', '🦄🚬😎', '🐙🎸🔥', '🐭🧀🔒', '🎅🏝️🍹', '🐐🎻🌙', '🦕🚗💨', '🐒🍌💍', '🐟🚲❓', '🦇🏦💰', '🐧🧥❄️🥵', '🤠🐂🪩', '🐿️🥜🏋️', '🦩🥂💃', '🐞🚀🌕', '🧜‍♀️🛁🧽'];
module.exports = require('./quickwit').make({
  prompts: () => SCENES,
  id: 'caption', name: 'Caption This', min: 3, art: '🖼️',
  desc: 'The TV shows a silly emoji scene. Write the best caption, then vote.',
  tag: 'Quick · 4 min',
  settings: [{ key: 'points', label: 'Points per vote', options: [[100, '100'], [250, '250'], [500, '500']] }],
});
