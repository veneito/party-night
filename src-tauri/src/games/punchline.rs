// Punchline Club: submit words -> build joke setups from others' words + write punchlines -> jokes duel head-to-head.
// Prompt packs (data/games.json): each category is what a phone asks for, the bots' answers, and setups where {w}
// always stands alone as a noun phrase, since players type their own "a"/"my"/"the".
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;
use std::collections::HashMap;

const WORDS_EACH: usize = 3;
const OPTIONS: usize = 3;
#[derive(Clone)]
struct Joke { owner: String, setup: String, punch: String }
struct Word { cat: String, w: String, owner: String }
#[derive(Default)]
pub struct D {
    cats: IndexMap<String, Vec<String>>,
    words_in: Vec<String>,
    words: Vec<Word>,
    opts: IndexMap<String, Vec<Vec<String>>>,
    jokes: Vec<Joke>,
    votes: IndexMap<String, usize>,
    duels: Vec<Vec<Joke>>,
    di: usize,
}
pub struct Punchline;

fn pack(g: &Room) -> &'static Map<String, Value> {
    let p = data("punchline", "PACKS");
    p.get(g.set_str("pack")).unwrap_or(&p["spicy"]).as_object().unwrap()
}
fn jokes_each(g: &Room) -> usize { g.set_or("jokes", 2.0) as usize }
fn jokes_by(d: &D, id: &str) -> usize { d.jokes.iter().filter(|j| j.owner == id).count() }
fn cap(s: &str) -> String { let mut c = s.chars(); c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect()) }

fn start_write(g: &mut Room) {
    let (pk, each, ids) = (pack(g), jokes_each(g), g.ids());
    let d = dm!(g, Punch);
    let fallback = Word { cat: pk.keys().next().unwrap().clone(), w: "a rubber duck".into(), owner: String::new() };
    for id in ids {
        // prefer other players' words; fall back to own, then to a generic word so nobody is stuck
        let pool: Vec<&Word> = d.words.iter().filter(|w| w.owner != id).collect();
        let src: Vec<&Word> = if !pool.is_empty() { pool } else if !d.words.is_empty() { d.words.iter().collect() } else { vec![&fallback] };
        let opts = (0..each).map(|_| {
            let mut set: Vec<String> = vec![];
            for _ in 0..50 {
                if set.len() >= OPTIONS { break }
                let w = pick(&src);
                let cat = pk.get(&w.cat).unwrap_or_else(|| pk.values().next().unwrap());
                let s = cap(&pick(&strs(&cat["setups"])).replace("{w}", &w.w));
                if !set.contains(&s) { set.push(s) }
            }
            set
        }).collect();
        d.opts.insert(id, opts);
    }
    g.phase = "write";
    set_timer(g, 120.0, start_duels, false);
}
fn start_duels(g: &mut Room) {
    let d = dm!(g, Punch);
    // pair jokes by different authors; a leftover joke sits out
    let mut left = shuffle(d.jokes.clone());
    let mut duels = vec![];
    while left.len() >= 2 {
        let mut count: HashMap<String, usize> = HashMap::new();
        for j in &left { *count.entry(j.owner.clone()).or_default() += 1 }
        left.sort_by(|x, y| count[&y.owner].cmp(&count[&x.owner])); // re-rank each pass: pair from the biggest pile
        let a = left.remove(0);
        if let Some(i) = left.iter().position(|j| j.owner != a.owner) { let b = left.remove(i); duels.push(shuffle(vec![a, b])) }
    }
    d.duels = duels;
    d.di = usize::MAX; // next_duel moves to 0
    next_duel(g);
}
fn next_duel(g: &mut Room) {
    let d = dm!(g, Punch);
    d.di = d.di.wrapping_add(1);
    d.votes.clear();
    if d.di >= d.duels.len() { return finish(g) }
    g.phase = "duel";
    set_timer(g, 25.0, reveal, false);
}
fn reveal(g: &mut Room) {
    let d = dm!(g, Punch);
    let duel = &d.duels[d.di];
    let mut counts = [0i64; 2];
    for &v in d.votes.values() { counts[v] += 1 }
    for (i, j) in duel.iter().enumerate() { if let Some(p) = g.players.get_mut(&j.owner) { p.score += counts[i] * 100 } }
    // bonus for a clean sweep with at least 2 votes
    let w = if counts[0] > counts[1] { Some(0) } else if counts[1] > counts[0] { Some(1) } else { None };
    if let Some(w) = w { if counts[1 - w] == 0 && counts[w] >= 2 { if let Some(p) = g.players.get_mut(&duel[w].owner) { p.score += 200 } } }
    g.phase = "reveal";
    set_timer(g, 7.0, next_duel, true);
}
const BOT_PUNCH: &[&str] = &["my mom still brings it up at Christmas", "now we're legally married", "the fire department got involved", "I still have the receipt",
    "nobody clapped", "it was the best night of my life", "the HOA sent a strongly worded letter", "it's on the news", "I'm not allowed back in Ohio",
    "we don't talk about it", "that's how I met your mother", "I had to change my name"];

impl Game for Punchline {
    fn id(&self) -> &'static str { "punchline" }
    fn start(&self, g: &mut Room) {
        g.phase = "words";
        let keys: Vec<String> = pack(g).keys().cloned().collect();
        let cats = g.ids().into_iter().map(|id| (id, shuffle(keys.clone()).into_iter().take(WORDS_EACH).collect())).collect();
        g.d = Data::Punch(D { cats, ..Default::default() });
        set_timer(g, 45.0, start_write, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "words" => Some(start_write), "write" => Some(start_duels), "duel" => Some(reveal), "reveal" => Some(next_duel), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Punch);
        match g.phase {
            "words" => d.words_in.iter().any(|x| x == id),
            "write" => jokes_by(d, id) >= jokes_each(g),
            "duel" => d.votes.contains_key(id) || d.duels[d.di].iter().any(|j| j.owner == id),
            _ => false,
        }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Punch(d) = &g.d else { return };
        // Note: per-player prompts ride in the shared broadcast (a peek in devtools shows others'), fine among friends.
        if g.phase == "words" { s.insert("cats".into(), json!(d.cats)); }
        if g.phase == "write" {
            s.insert("opts".into(), Value::Object(g.players.keys().map(|id| (id.clone(), json!(d.opts.get(id).and_then(|o| o.get(jokes_by(d, id)))))).collect()));
        }
        if matches!(g.phase, "duel" | "reveal") {
            let rev = g.phase == "reveal";
            s.insert("duelNo".into(), json!(d.di + 1));
            s.insert("duelCount".into(), json!(d.duels.len()));
            s.insert("duel".into(), json!(d.duels[d.di].iter().enumerate().map(|(i, j)| {
                let mut o = json!({ "setup": j.setup, "punch": j.punch });
                if rev { o["name"] = json!(g.name(&j.owner)); o["votes"] = json!(d.votes.values().filter(|v| **v == i).count()) }
                o
            }).collect::<Vec<_>>()));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let (phase, each) = (g.phase, jokes_each(g));
        let d = dm!(g, Punch);
        match kind(a) {
            "words" => {
                let Some(cats) = d.cats.get(id).filter(|_| phase == "words" && !d.words_in.iter().any(|x| x == id)) else { return no(NOT_NOW) };
                let ws: Vec<String> = a.get("words").and_then(|v| v.as_array()).map(|w| w.iter().map(|w| clean(Some(w), 30)).collect()).unwrap_or_default();
                if ws.len() != WORDS_EACH || ws.iter().any(|w| w.is_empty()) { return no("Fill in every word") }
                let new: Vec<Word> = ws.into_iter().zip(cats).map(|(w, c)| Word { cat: c.clone(), w, owner: id.into() }).collect();
                d.words_in.push(id.into());
                d.words.extend(new);
            }
            "joke" => {
                let n = jokes_by(d, id);
                let Some(opts) = d.opts.get(id).filter(|_| phase == "write" && n < each) else { return no(NOT_NOW) };
                let Some(setup) = opts.get(n).and_then(|o| index(a.get("pick"), o.len()).map(|i| o[i].clone())) else { return no("Pick a setup") };
                let punch = clean(a.get("text"), 100);
                if punch.is_empty() { return no("Write a punchline") }
                d.jokes.push(Joke { owner: id.into(), setup, punch });
            }
            "vote" => {
                let i = num(a.get("index"));
                if phase != "duel" || (i != 0.0 && i != 1.0) { return no(NOT_NOW) }
                if d.duels[d.di].iter().any(|j| j.owner == id) { return no("It's your joke, sit tight") }
                d.votes.insert(id.into(), i as usize);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Punch);
        match g.phase {
            "words" => {
                let pk = pack(g);
                Some(json!({ "type": "words", "words": d.cats.get(id)?.iter().map(|c| {
                    let b = pk.get(c).map(|x| strs(&x["bot"])).unwrap_or_default();
                    if b.is_empty() { "a rubber duck".into() } else { pick(&b) }
                }).collect::<Vec<_>>() }))
            }
            "write" => Some(json!({ "type": "joke", "pick": 0, "text": pick(BOT_PUNCH) })),
            "duel" => Some(json!({ "type": "vote", "index": pick(&[0, 1]) })),
            _ => None,
        }
    }
}
