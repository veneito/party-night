// Rooms, timers, bots and the shared actions every game gets. Each game mode lives in games/*.rs.
#![allow(non_snake_case)] // `G` is the current game, `g` the room
use crate::games::{self, Data, Game, GAMES};
use indexmap::IndexMap;
use rand::{seq::SliceRandom, Rng};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

pub const NOT_NOW: &str = "Not now";
pub type Next = fn(&mut Room);
pub type Res = Result<(), String>;
pub fn no<T>(m: &str) -> Result<T, String> { Err(m.into()) }

pub struct Player { pub name: String, pub score: i64, pub av: i64, pub bot: bool, pub photo: Option<String>, pub photo_v: String }

pub struct Room {
    pub code: String,
    pub phase: &'static str,
    pub game: Option<usize>,
    pub settings: Map<String, Value>,
    pub players: IndexMap<String, Player>,
    pub pending_bots: i64,
    pub round: i64,
    pub clients: Vec<UnboundedSender<String>>,
    pub timer: Option<(Instant, Next)>,
    pub last_seen: Instant,
    pub host: Option<String>,
    pub gvotes: IndexMap<String, String>,
    pub used: Vec<String>, // prompts already shown in this room, so they don't repeat
    pub d: Data,
}

impl Room {
    pub fn name(&self, id: &str) -> String { self.players.get(id).map_or("(left)".into(), |p| p.name.clone()) }
    pub fn set_str(&self, k: &str) -> &str { self.settings.get(k).and_then(|v| v.as_str()).unwrap_or("") }
    // like `g.settings.x || def` in JS
    pub fn set_or(&self, k: &str, def: f64) -> f64 { self.settings.get(k).and_then(|v| v.as_f64()).filter(|x| *x != 0.0).unwrap_or(def) }
    pub fn add(&mut self, id: &str, pts: i64) { if let Some(p) = self.players.get_mut(id) { p.score += pts } }
    pub fn real_ids(&self) -> Vec<String> { self.players.iter().filter(|(_, p)| !p.bot).map(|(id, _)| id.clone()).collect() }
    pub fn ids(&self) -> Vec<String> { self.players.keys().cloned().collect() }
}

// ---- small helpers shared by the games (JS-like coercions keep the phone/TV protocol unchanged)
pub fn rnd() -> f64 { rand::thread_rng().gen() }
pub fn pick<T: Clone>(a: &[T]) -> T { a[rand::thread_rng().gen_range(0..a.len())].clone() }
pub fn shuffle<T>(mut v: Vec<T>) -> Vec<T> { v.shuffle(&mut rand::thread_rng()); v }
pub fn round(x: f64) -> f64 { (x + 0.5).floor() } // Math.round
pub fn num(v: Option<&Value>) -> f64 {
    match v {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => *b as u8 as f64,
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(s)) => { let t = s.trim(); if t.is_empty() { 0.0 } else { t.parse().unwrap_or(f64::NAN) } }
        _ => f64::NAN,
    }
}
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        _ => true,
    }
}
pub fn clean(v: Option<&Value>, n: usize) -> String {
    let s = match v {
        Some(Value::String(s)) => s.clone(),
        Some(x @ Value::Number(_)) if truthy(Some(x)) => x.to_string(),
        Some(Value::Bool(true)) => "true".into(),
        _ => String::new(),
    };
    s.trim().chars().take(n).collect()
}
pub fn index(v: Option<&Value>, len: usize) -> Option<usize> {
    let x = num(v);
    (x >= 0.0 && x.fract() == 0.0 && (x as usize) < len).then_some(x as usize)
}
pub fn kind(a: &Value) -> &str { a.get("type").and_then(|v| v.as_str()).unwrap_or("") }
// one pen stroke from a phone: points clamped to the 0-1000 canvas, colour and width from a fixed set
pub fn stroke(a: &Value) -> Option<Value> {
    let p: Vec<i64> = a.get("p").and_then(|v| v.as_array()).map(|arr| arr.iter().take(1200).map(|n| {
        let x = num(Some(n));
        round(if x.is_nan() { 0.0 } else { x }).clamp(0.0, 1000.0) as i64
    }).collect()).unwrap_or_default();
    if p.len() < 2 { return None }
    let c = a.get("c").and_then(|v| v.as_str())
        .filter(|s| s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())).unwrap_or("#231a08");
    let w = a.get("w").and_then(|v| v.as_f64()).filter(|w| [4.0, 10.0, 24.0].contains(w)).unwrap_or(10.0) as i64;
    Some(json!({ "c": c, "w": w, "p": p }))
}
fn base36(mut n: u64, min: usize) -> String {
    let mut s = vec![];
    while n > 0 || s.len() < min { s.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[(n % 36) as usize]); n /= 36; }
    s.reverse(); String::from_utf8(s).unwrap()
}
fn enc(s: &str) -> String { // encodeURIComponent
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

// ---- settings every game gets; games add their own. The first option is the default.
pub static CORE_SETTINGS: LazyLock<Vec<Value>> = LazyLock::new(|| serde_json::from_value(json!([
    { "key": "speed", "label": "Timers", "options": [["normal", "Normal"], ["relaxed", "Relaxed"], ["fast", "Fast"], ["off", "No timer"]] },
    { "key": "rounds", "label": "Rounds", "options": [[3, "3"], [1, "1"], [2, "2"], [5, "5"]] },
    { "key": "voice", "label": "Voice-over", "options": [["on", "On"], ["off", "Off"]] },
    { "key": "max", "label": "Max players", "options": [[20, "20"], [8, "8"], [12, "12"], [16, "16"]] },
    { "key": "home", "label": "Home button", "options": [["vote", "Players vote next"], ["menu", "Back to menu"]] },
])).unwrap());
static ALL_SETTINGS: LazyLock<Vec<Vec<Value>>> = LazyLock::new(|| GAMES.iter().map(|g| {
    let mut v = CORE_SETTINGS.clone();
    v.extend(games::meta(g.id())["settings"].as_array().cloned().unwrap_or_default());
    v
}).collect());
static GAMES_JSON: LazyLock<Value> = LazyLock::new(|| Value::Array(GAMES.iter().enumerate().map(|(i, g)| {
    let m = games::meta(g.id());
    json!({ "id": m["id"], "name": m["name"], "desc": m["desc"], "min": m["min"], "tag": m["tag"], "art": m["art"], "settings": ALL_SETTINGS[i] })
}).collect()));
const MAX_PLAYERS: f64 = 20.0;
// TIMER_SCALE=0.2 makes every countdown 5x shorter, handy when testing alone
static SCALE: LazyLock<f64> = LazyLock::new(|| std::env::var("TIMER_SCALE").ok().and_then(|s| s.parse().ok()).filter(|x: &f64| *x > 0.0).unwrap_or(1.0));

pub fn set_timer(g: &mut Room, secs: f64, next: Next, always: bool) {
    // "No timer" still lets short reveal screens move on by themselves
    let k = match g.set_str("speed") { "relaxed" => 1.5, "fast" => 0.6, "off" => 0.0, _ => 1.0 };
    let s = if secs > 0.0 && (k > 0.0 || always) { round(secs * if k > 0.0 { k } else { 1.0 } * *SCALE).max(1.0) } else { 0.0 };
    g.timer = (s > 0.0).then(|| (Instant::now() + Duration::from_secs_f64(s), next));
}
pub fn clear_timer(g: &mut Room) { g.timer = None }
pub fn finish(g: &mut Room) {
    if (g.round as f64) < g.set_or("rounds", 3.0) { g.phase = "scores"; set_timer(g, 15.0, next_round, true) }
    else { g.phase = "results"; clear_timer(g) }
}
fn next_round(g: &mut Room) {
    let n = g.pending_bots; add_bots(g, n); g.pending_bots = 0;
    g.round += 1;
    if let Some(G) = game(g) { G.start(g) }
}
#[allow(non_snake_case)]
pub fn game(g: &Room) -> Option<&'static dyn Game> { g.game.map(|i| &*GAMES[i]) }
pub fn in_game(g: &Room) -> bool { !matches!(g.phase, "menu" | "lobby" | "scores" | "results" | "gamevote") }

// ---- rooms
pub type Rooms = HashMap<String, Room>;
static ROOMS: LazyLock<Mutex<Rooms>> = LazyLock::new(Default::default);
// a panic in one game must not lock every room forever
pub fn rooms() -> MutexGuard<'static, Rooms> { ROOMS.lock().unwrap_or_else(|e| e.into_inner()) }
const LETTERS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ"; // no I/O, easy to read on a TV
fn fresh_code(rooms: &Rooms) -> String {
    loop { let c: String = (0..4).map(|_| pick(LETTERS) as char).collect(); if !rooms.contains_key(&c) { return c } }
}
fn create_room(rooms: &mut Rooms) -> String {
    let code = fresh_code(rooms);
    rooms.insert(code.clone(), Room { code: code.clone(), phase: "menu", game: None, settings: Map::new(), players: IndexMap::new(), pending_bots: 0,
        round: 0, clients: vec![], timer: None, last_seen: Instant::now(), host: None, gvotes: IndexMap::new(), used: vec![], d: Data::None });
    code
}

// "What's next?": Home opens a vote on every phone: replay this game or pick another. Most votes wins, ties random.
fn start_game_vote(g: &mut Room) { g.phase = "gamevote"; g.gvotes.clear(); set_timer(g, 20.0, end_game_vote, true) }
fn end_game_vote(g: &mut Room) {
    let mut n: IndexMap<&str, usize> = IndexMap::new();
    for v in g.gvotes.values() { *n.entry(v.as_str()).or_default() += 1 }
    let top = n.values().copied().max().unwrap_or(0);
    let win = if top > 0 { pick(&n.iter().filter(|(_, c)| **c == top).map(|(k, _)| k.to_string()).collect::<Vec<_>>()) }
        else if g.game.is_some() { "replay".into() } else { pick(&GAMES.iter().map(|x| x.id().to_string()).collect::<Vec<_>>()) };
    clear_timer(g);
    if win == "replay" { g.phase = "lobby"; for p in g.players.values_mut() { p.score = 0 } }
    else { g.phase = "menu"; game_act(g, &json!({ "type": "pick", "game": win })); }
}
// each player gets the lowest free avatar slot (colour + animal), so 20 players are all distinct
fn free_avatar(g: &Room) -> i64 { (0..).find(|i| !g.players.values().any(|p| p.av == *i)).unwrap() }
fn avatar_free(g: &Room, av: Option<&Value>, id: &str) -> bool {
    let x = av.and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
    x.fract() == 0.0 && (0.0..40.0).contains(&x) && !g.players.iter().any(|(pid, p)| pid != id && p.av == x as i64)
}
fn max_of(g: &Room) -> i64 { g.set_or("max", MAX_PLAYERS) as i64 }
const BOT_NAMES: [&str; 20] = ["Ada", "Bix", "Cog", "Dot", "Echo", "Fizz", "Gizmo", "Hex", "Ion", "Jinx", "Kilo", "Lux", "Mo", "Nano", "Ozzy", "Pip", "Quark", "Rex", "Sprocket", "Tink"];
fn add_bots(g: &mut Room, n: i64) {
    for _ in 0..n {
        if g.players.len() as i64 >= max_of(g) { break }
        let av = free_avatar(g);
        let id = format!("bot{}", base36(rand::thread_rng().gen_range(0..36u64.pow(6)), 6));
        g.players.insert(id, Player { name: format!("Bot {}", BOT_NAMES[av as usize % BOT_NAMES.len()]), score: 0, av, bot: true, photo: None, photo_v: String::new() });
    }
}

// The host is a player whose phone gets the TV's controls. First real player by default; passes on if they go.
fn fix_host(g: &mut Room) {
    if !g.host.as_ref().and_then(|h| g.players.get(h)).is_some_and(|p| !p.bot) { g.host = g.real_ids().into_iter().next() }
}
pub fn public_state(g: &mut Room) -> Value {
    fix_host(g);
    let G = game(g);
    let left = g.timer.map_or(0, |(t, _)| t.saturating_duration_since(Instant::now()).as_secs_f64().ceil() as u64);
    let players: Vec<Value> = g.players.iter().map(|(id, p)| {
        let mut o = json!({ "id": id, "name": p.name, "av": p.av, "score": p.score,
            "done": if g.phase == "gamevote" { g.gvotes.contains_key(id) } else { in_game(g) && G.is_some_and(|G| G.is_done(g, id)) } });
        if p.photo.is_some() { o["ph"] = json!(format!("/photo/{}/{}?v={}", g.code, enc(id), p.photo_v)) }
        if p.bot { o["bot"] = json!(true) }
        o
    }).collect();
    let mut s = Map::new();
    s.insert("code".into(), json!(g.code));
    s.insert("host".into(), json!(g.host));
    s.insert("phase".into(), json!(g.phase));
    s.insert("rounds".into(), json!(g.set_or("rounds", 3.0)));
    s.insert("pendingBots".into(), json!(g.pending_bots));
    s.insert("game".into(), json!(G.map(|G| G.id())));
    s.insert("round".into(), json!(g.round));
    s.insert("settings".into(), Value::Object(g.settings.clone()));
    s.insert("games".into(), GAMES_JSON.clone());
    s.insert("left".into(), json!(left)); // seconds left, not a timestamp, so phone clock skew doesn't matter
    s.insert("players".into(), Value::Array(players));
    if let Some(G) = G { G.state(g, &mut s) }
    if g.phase == "gamevote" {
        let mut n = Map::new();
        for v in g.gvotes.values() { let c = n.get(v).and_then(|x| x.as_i64()).unwrap_or(0); n.insert(v.clone(), json!(c + 1)); }
        s.insert("gvotes".into(), Value::Object(n));
    }
    Value::Object(s)
}
pub fn broadcast(g: &mut Room) {
    let d = public_state(g).to_string();
    g.clients.retain(|c| c.send(d.clone()).is_ok());
}

// one request from a phone or the TV
pub fn act(a: &Value) -> Value {
    let mut rooms = rooms();
    if kind(a) == "create" { return json!({ "code": create_room(&mut rooms) }) }
    let code = clean(a.get("room"), 4).to_uppercase();
    let Some(g) = rooms.get_mut(&code) else { return json!({ "error": "Room not found" }) };
    g.last_seen = Instant::now();
    match kind(a) {
        "ping" => return json!({ "ok": true }), // connection check, changes nothing
        "recode" => { // TV "New code" on the menu: fresh code, everyone has to join again
            if g.phase != "menu" { return json!({ "error": NOT_NOW }) }
            let mut g = rooms.remove(&code).unwrap();
            g.code = fresh_code(&rooms); g.players.clear(); g.pending_bots = 0;
            broadcast(&mut g);
            rooms.insert(g.code.clone(), g);
            return json!({ "ok": true });
        }
        _ => {}
    }
    if let Some(err) = game_act(g, a) { return json!({ "error": err }) }
    broadcast(g);
    json!({ "ok": true })
}

// everyone finished early: move on without waiting for the timer
fn advance_if_done(g: &mut Room) {
    let Some(G) = game(g) else { return };
    if in_game(g) && g.players.keys().all(|p| G.is_done(g, p)) { if let Some(f) = G.advance(g.phase) { f(g) } }
}

pub fn game_act(g: &mut Room, a: &Value) -> Option<String> {
    let id = clean(a.get("id"), 40);
    let me = g.players.contains_key(&id);
    let count = g.players.len() as i64;
    let G = game(g);
    match kind(a) {
        "join" => {
            let name = clean(a.get("text"), 16);
            if name.is_empty() { return Some("Name required".into()) }
            if let Some(p) = g.players.get_mut(&id) { p.name = name; return None }
            if in_game(g) { return Some("Game in progress, join after this round".into()) }
            if count + g.pending_bots >= max_of(g) { return Some("Room is full".into()) }
            let av = if avatar_free(g, a.get("av"), &id) { num(a.get("av")) as i64 } else { free_avatar(g) };
            g.players.insert(id, Player { name, score: 0, av, bot: false, photo: None, photo_v: String::new() });
        }
        "photo" => { // player uploads a picture (phone shrinks it to a small JPEG first), or clears it with ''
            if !me { return Some(NOT_NOW.into()) }
            let data = a.get("data");
            if truthy(data) && !data.and_then(|v| v.as_str()).is_some_and(|s| s.len() <= 30000 && is_photo(s)) {
                return Some("That picture didn't work, try another".into());
            }
            let p = g.players.get_mut(&id).unwrap();
            p.photo = data.and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from);
            // unique per upload, so browsers never show a cached older photo
            p.photo_v = base36(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64, 1);
        }
        "avatar" => { // player picks their emoji; one per player so everyone is easy to spot
            if !me || in_game(g) { return Some(NOT_NOW.into()) }
            if !avatar_free(g, a.get("av"), &id) { return Some("Someone already has that one".into()) }
            g.players[&id].av = num(a.get("av")) as i64;
        }
        "pick" => { // TV chooses a game from the menu
            let want = a.get("game").and_then(|v| v.as_str()).unwrap_or("");
            let Some(i) = GAMES.iter().position(|x| x.id() == want).filter(|_| g.phase == "menu") else { return Some(NOT_NOW.into()) };
            g.game = Some(i); g.phase = "lobby"; g.round = 0;
            // fresh defaults per game, but the voice-over and home choices carry over
            let mut s: Map<String, Value> = ALL_SETTINGS[i].iter().map(|d| (d["key"].as_str().unwrap().to_string(), d["options"][0][0].clone())).collect();
            for k in ["voice", "home"] { if let Some(v) = g.settings.get(k).filter(|v| truthy(Some(v))) { s.insert(k.into(), v.clone()); } }
            g.settings = s;
            for p in g.players.values_mut() { p.score = 0 }
        }
        "set" => { // TV changes a setting in the lobby
            let key = a.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let defs = g.game.map_or(&*CORE_SETTINGS, |i| &ALL_SETTINGS[i]);
            let opt = defs.iter().find(|d| d["key"] == key).and_then(|d| d["options"].as_array()?.iter().find(|o| Some(&o[0]) == a.get("value")));
            let Some(opt) = opt.filter(|_| g.phase == "lobby" || key == "voice" || key == "home") else { return Some(NOT_NOW.into()) };
            g.settings.insert(key.into(), opt[0].clone());
        }
        "start" => { // lobby "start", scoreboard "next round", final "play again"
            let Some(G) = G.filter(|_| matches!(g.phase, "lobby" | "scores" | "results")) else { return Some(NOT_NOW.into()) };
            let min = games::meta(G.id())["min"].as_i64().unwrap_or(2);
            if count + g.pending_bots < min { return Some(format!("Need {min}+ players")) }
            if g.phase != "scores" { g.round = 0; for p in g.players.values_mut() { p.score = 0 } } // a fresh game
            next_round(g);
        }
        "menu" => { // Home: a player vote (if that setting is on and there's a game to replay), else straight to the menu
            if g.game.is_some() && g.phase != "gamevote" && g.phase != "lobby" && g.set_str("home") != "menu" && count > 0 { start_game_vote(g); return None }
            clear_timer(g); g.phase = "menu"; g.game = None; // players stay joined
        }
        "gamevote" => { // "Let players vote" from the menu (or anywhere): every phone picks the next game
            if count == 0 { return Some("Nobody has joined yet".into()) }
            if g.phase != "gamevote" { start_game_vote(g) }
        }
        "gvote" => { // a player's pick in the "what's next?" vote
            let want = a.get("game").and_then(|v| v.as_str()).unwrap_or("");
            let ok = if want == "replay" { g.game.is_some() } else { GAMES.iter().any(|x| x.id() == want) };
            if g.phase != "gamevote" || !me || !ok { return Some(NOT_NOW.into()) }
            g.gvotes.insert(id, want.into());
            if g.players.keys().all(|p| g.gvotes.contains_key(p)) { end_game_vote(g) }
        }
        "skip" => { // TV "time's up" / "next"
            if g.phase == "gamevote" { end_game_vote(g) }
            else if g.phase == "scores" { next_round(g) }
            else if let Some(f) = G.filter(|_| in_game(g)).and_then(|G| G.advance(g.phase)) { f(g) }
        }
        "host" => { // host hands the crown to another player
            let who = clean(a.get("who"), 40);
            if !g.players.get(&who).is_some_and(|p| !p.bot) { return Some("Pick a real player".into()) }
            g.host = Some(who);
        }
        "kick" => { // TV removes someone who stopped playing, any time
            g.players.shift_remove(&clean(a.get("who"), 40));
            advance_if_done(g);
        }
        "bots" => { // TV adds bots; mid-round they wait and join when the next round starts
            let want = num(a.get("n"));
            let want = if want.is_nan() || want == 0.0 { 1 } else { want as i64 };
            let n = want.min(max_of(g) - count - g.pending_bots).max(0);
            if in_game(g) { g.pending_bots += n } else { add_bots(g, n) }
        }
        "unbot" => { // TV removes bots (n of them, or all), any time
            let n = num(a.get("n"));
            let mut n = if n.is_nan() || n == 0.0 { i64::MAX } else { n as i64 };
            if g.pending_bots > 0 {
                let k = g.pending_bots.min(n); g.pending_bots -= k;
                if truthy(a.get("n")) { return None }
            }
            let bots: Vec<String> = g.players.iter().rev().filter(|(_, p)| p.bot).map(|(id, _)| id.clone()).collect();
            for b in bots { if n <= 0 { break } g.players.shift_remove(&b); n -= 1; }
            // the removed bots may have been the ones everyone was waiting on
            advance_if_done(g);
        }
        _ => {
            let Some(G) = G.filter(|_| in_game(g) && me) else { return Some(NOT_NOW.into()) };
            if let Err(e) = G.act(g, a, &id) { return Some(e) }
            advance_if_done(g);
        }
    }
    None
}
#[cfg(test)]
pub fn test_room(ids: &[&str]) -> Room {
    let mut r = Rooms::new();
    let code = create_room(&mut r);
    let mut g = r.remove(&code).unwrap();
    g.round = 1;
    g.settings.insert("rounds".into(), json!(1));
    for id in ids { g.players.insert(id.to_string(), Player { name: id.to_uppercase(), score: 0, av: 0, bot: false, photo: None, photo_v: String::new() }); }
    g
}
// photos live only in memory on the player, so they disappear when the player is removed or the room closes
fn is_photo(s: &str) -> bool {
    s.strip_prefix("data:image/jpeg;base64,").is_some_and(|b| !b.is_empty() && b.bytes().all(|c| c.is_ascii_alphanumeric() || b"+/=".contains(&c)))
}

// Runs every 100 ms: fires round timers, lets bots play (every 700 ms), drops rooms idle for 2 hours.
pub fn tick(n: u64) {
    let mut rooms = rooms();
    let now = Instant::now();
    for g in rooms.values_mut() {
        if let Some((t, next)) = g.timer {
            if t <= now { g.timer = None; next(g); broadcast(g); }
        }
        if n % 10 == 0 && in_game(g) { if let Some(G) = game(g) { if G.tick(g) { broadcast(g) } } }
        // a bug in one game's bot must not take the whole server down mid-party
        if n % 7 == 0 && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bots(g))).is_err() { eprintln!("bot error in room {}", g.code) }
    }
    if n % 6000 == 0 {
        rooms.retain(|_, g| {
            let before = g.clients.len();
            g.clients.retain(|c| !c.is_closed());
            if g.clients.len() != before { g.last_seen = now }
            !g.clients.is_empty() || now.duration_since(g.last_seen) < Duration::from_secs(2 * 3600)
        });
    }
}
// bots take a turn now and then, staggered so it looks like real people finishing
fn bots(g: &mut Room) {
    if g.phase == "gamevote" { // bots vote too, at random
        for id in g.ids() {
            if !g.players[&id].bot || g.gvotes.contains_key(&id) || rnd() >= 0.4 { continue }
            let mut opts: Vec<String> = GAMES.iter().map(|x| x.id().to_string()).collect();
            if g.game.is_some() { opts.insert(0, "replay".into()) }
            game_act(g, &json!({ "type": "gvote", "id": id, "game": pick(&opts) }));
            broadcast(g);
            if g.phase != "gamevote" { break }
        }
        return;
    }
    let Some(G) = game(g).filter(|_| in_game(g)) else { return };
    for id in g.ids() {
        if !g.players.get(&id).is_some_and(|p| p.bot) || !in_game(g) || G.is_done(g, &id) || rnd() < 0.6 { continue }
        if let Some(mut m) = G.bot(g, &id) {
            m["id"] = json!(id);
            if game_act(g, &m).is_none() { broadcast(g) }
        }
    }
}
