// One file per game mode. A game is a small set of phases: start sets up a round, `advance` moves on when time
// runs out (or everyone is done), `act` takes a player's move, `state` adds what the TV and phones need to see.
// The word lists and menu text live in data/games.json; see quickwit.rs for the smallest example.
use crate::room::{Next, Res, Room};
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub mod draw;
pub mod elemix;
pub mod fakeout;
pub mod golf;
pub mod hottake;
pub mod loneclue;
pub mod objection;
pub mod oddone;
pub mod punchline;
pub mod quickwit;
pub mod receipts;
pub mod telephone;

pub trait Game: Send + Sync {
    fn id(&self) -> &'static str;
    fn start(&self, g: &mut Room);
    fn advance(&self, phase: &str) -> Option<Next>;
    fn is_done(&self, g: &Room, id: &str) -> bool;
    fn state(&self, g: &Room, s: &mut Map<String, Value>);
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res;
    fn bot(&self, g: &Room, id: &str) -> Option<Value>;
    // called every second during a round; true = something changed on its own (e.g. a hint letter), so send it out
    fn tick(&self, _g: &mut Room) -> bool { false }
}

// menu order: alphabetical by id, same as before
pub static GAMES: LazyLock<Vec<Box<dyn Game>>> = LazyLock::new(|| vec![
    Box::new(quickwit::QuickWit("caption")),
    Box::new(draw::Draw),
    Box::new(elemix::Elemix),
    Box::new(fakeout::FakeOut),
    Box::new(quickwit::QuickWit("gif")),
    Box::new(golf::Golf),
    Box::new(hottake::HotTake),
    Box::new(loneclue::LoneClue),
    Box::new(objection::Objection),
    Box::new(oddone::OddOne),
    Box::new(punchline::Punchline),
    Box::new(quickwit::QuickWit("quickwit")),
    Box::new(receipts::Receipts),
    Box::new(telephone::Telephone),
]);

// each round's own state; it stays after the round so the scoreboard can show what happened
#[derive(Default)]
pub enum Data {
    #[default]
    None,
    Quick(quickwit::D),
    Draw(draw::D),
    Elem(elemix::D),
    Fake(fakeout::D),
    Golf(golf::D),
    Hot(hottake::D),
    Lone(loneclue::D),
    Obj(objection::D),
    Odd(oddone::D),
    Punch(punchline::D),
    Receipts(receipts::D),
    Tel(telephone::D),
}
// this game's round state; only used once the game's start() has run
#[macro_export]
macro_rules! dm { ($g:expr, $v:ident) => { match &mut $g.d { $crate::games::Data::$v(d) => d, _ => unreachable!() } } }
#[macro_export]
macro_rules! dr { ($g:expr, $v:ident) => { match &$g.d { $crate::games::Data::$v(d) => d, _ => unreachable!() } } }

static DATA: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(include_str!("../../data/games.json")).unwrap());
pub fn meta(id: &str) -> &'static Value { &DATA[id]["meta"] }
pub fn data(id: &str, key: &str) -> &'static Value { &DATA[id]["data"][key] }
pub fn strs(v: &Value) -> Vec<String> { v.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default() }
