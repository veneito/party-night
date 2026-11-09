// Odd One Out: everyone gets the same secret word except one player. Everyone gives a one-word clue,
// then the room votes on who had the different word. The odd one wins by blending in.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

#[derive(Default)]
pub struct D { word: String, odd_word: String, odd: String, clues: IndexMap<String, String>, votes: IndexMap<String, String>, caught: bool }
pub struct OddOne;

fn start_accuse(g: &mut Room) { g.phase = "accuse"; set_timer(g, 30.0, tally, false) }
fn tally(g: &mut Room) {
    let d = dm!(g, Odd);
    let mut count: IndexMap<&str, i64> = IndexMap::new();
    for v in d.votes.values() { *count.entry(v.as_str()).or_default() += 1 }
    let top = count.values().copied().max().unwrap_or(0);
    // caught = the odd one got the most votes (a tie at the top counts as caught)
    d.caught = top > 0 && count.get(d.odd.as_str()) == Some(&top);
    if d.caught { for (id, v) in &d.votes { if *v == d.odd { if let Some(p) = g.players.get_mut(id) { p.score += 500 } } } }
    else if let Some(p) = g.players.get_mut(&d.odd) { p.score += 1000 }
    finish(g);
}

impl Game for OddOne {
    fn id(&self) -> &'static str { "oddone" }
    fn start(&self, g: &mut Room) {
        g.phase = "clue";
        let pairs = data("oddone", "PAIRS").as_array().unwrap();
        let pair = shuffle(strs(&pick(pairs)));
        g.d = Data::Odd(D { word: pair[0].clone(), odd_word: pair[1].clone(), odd: pick(&g.ids()), ..Default::default() });
        set_timer(g, 45.0, start_accuse, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "clue" => Some(start_accuse), "accuse" => Some(tally), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Odd);
        match g.phase { "clue" => d.clues.contains_key(id), "accuse" => d.votes.contains_key(id), _ => false }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Odd(d) = &g.d else { return };
        // Note: secret words ride in the shared broadcast (a peek in devtools shows them), fine among friends.
        if g.phase == "clue" {
            s.insert("secret".into(), json!(g.players.keys().map(|id| (id.clone(), if *id == d.odd { &d.odd_word } else { &d.word })).collect::<IndexMap<_, _>>()));
        }
        if g.phase == "accuse" { s.insert("clues".into(), json!(d.clues.iter().map(|(id, t)| json!({ "id": id, "text": t })).collect::<Vec<_>>())); }
        if matches!(g.phase, "scores" | "results") && !d.word.is_empty() {
            s.insert("last".into(), json!({ "word": d.word, "oddWord": d.odd_word, "caught": d.caught, "odd": g.name(&d.odd),
                "clues": d.clues.iter().map(|(id, t)| json!({ "name": g.name(id), "text": t, "odd": *id == d.odd,
                    "votes": d.votes.values().filter(|v| *v == id).count() })).collect::<Vec<_>>() }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let d = dm!(g, Odd);
        match kind(a) {
            "clue" => {
                let text = clean(a.get("text"), 20);
                if phase != "clue" || text.is_empty() { return no(NOT_NOW) }
                if text.contains(char::is_whitespace) { return no("One word only") }
                if (if id == d.odd { &d.odd_word } else { &d.word }).to_lowercase().contains(&text.to_lowercase()) { return no("Don't just say the word!") }
                d.clues.insert(id.into(), text);
            }
            "accuse" => {
                let who = clean(a.get("who"), 40);
                if phase != "accuse" || !d.clues.contains_key(&who) { return no(NOT_NOW) }
                if who == id { return no("You can't pick yourself") }
                d.votes.insert(id.into(), who);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Odd);
        match g.phase {
            "clue" => Some(json!({ "type": "clue", "text": pick(&["yummy", "fun", "big", "loud", "weird", "summer", "expensive", "hmm", "classic", "messy"]) })),
            "accuse" => {
                let ok: Vec<&String> = d.clues.keys().filter(|p| *p != id).collect();
                (!ok.is_empty()).then(|| json!({ "type": "accuse", "who": pick(&ok) }))
            }
            _ => None,
        }
    }
}
