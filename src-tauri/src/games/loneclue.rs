// Lone Clue: one player guesses a secret word. Everyone else writes a one-word clue, but matching clues cancel out,
// so the guesser only sees the clues nobody else thought of.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

#[derive(Default)]
pub struct D { word: String, guesser: Option<String>, clues: IndexMap<String, String>, answer: Option<String>, cancelled: Vec<String>, correct: bool }
pub struct LoneClue;

// "Cats" and "cat!" count as the same clue
fn norm(t: &str) -> String {
    let s: String = t.to_lowercase().chars().filter(|c| c.is_ascii_lowercase()).collect();
    s.strip_suffix("es").or_else(|| s.strip_suffix('s')).unwrap_or(&s).to_string()
}
fn start_guess(g: &mut Room) {
    let d = dm!(g, Lone);
    let mut n: IndexMap<String, usize> = IndexMap::new();
    for t in d.clues.values() { *n.entry(norm(t)).or_default() += 1 }
    d.cancelled = d.clues.iter().filter(|(_, t)| n[&norm(t)] > 1).map(|(id, _)| id.clone()).collect();
    g.phase = "guess";
    set_timer(g, 40.0, end, false);
}
fn end(g: &mut Room) {
    let d = dm!(g, Lone);
    d.correct = d.answer.as_ref().is_some_and(|a| norm(a) == norm(&d.word));
    if d.correct {
        if let Some(p) = d.guesser.as_ref().and_then(|x| g.players.get_mut(x)) { p.score += 1000 }
        for id in d.clues.keys() { if !d.cancelled.contains(id) { if let Some(p) = g.players.get_mut(id) { p.score += 500 } } }
    }
    finish(g);
}

impl Game for LoneClue {
    fn id(&self) -> &'static str { "loneclue" }
    fn start(&self, g: &mut Room) {
        g.phase = "clue";
        // take turns guessing; bots can't guess well, so only real players get a turn
        let ids = g.real_ids();
        let guesser = (!ids.is_empty()).then(|| ids[(g.round - 1).max(0) as usize % ids.len()].clone());
        g.d = Data::Lone(D { word: pick(&strs(data("loneclue", "WORDS"))), guesser, ..Default::default() });
        set_timer(g, 45.0, start_guess, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "clue" => Some(start_guess), "guess" => Some(end), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Lone);
        let me = d.guesser.as_deref() == Some(id);
        match g.phase { "clue" => me || d.clues.contains_key(id), "guess" => !me || d.answer.is_some(), _ => false }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Lone(d) = &g.d else { return };
        if let Some(x) = &d.guesser { s.insert("guesser".into(), json!(x)); }
        // Note: the word rides in the shared broadcast (a peek in devtools shows it), fine among friends, same as Odd One Out.
        if matches!(g.phase, "clue" | "guess") { s.insert("word".into(), json!(d.word)); }
        if g.phase == "guess" {
            s.insert("kept".into(), json!(d.clues.iter().filter(|(id, _)| !d.cancelled.contains(id)).map(|(id, t)| json!({ "id": id, "text": t })).collect::<Vec<_>>()));
            s.insert("cancelled".into(), json!(d.cancelled.len()));
        }
        if matches!(g.phase, "scores" | "results") && !d.word.is_empty() {
            s.insert("last".into(), json!({ "word": d.word, "answer": d.answer, "correct": d.correct, "guesser": g.name(d.guesser.as_deref().unwrap_or("")),
                "clues": d.clues.iter().map(|(id, t)| json!({ "name": g.name(id), "text": t, "out": d.cancelled.contains(id), "k": norm(t) })).collect::<Vec<_>>() }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let d = dm!(g, Lone);
        let me = d.guesser.as_deref() == Some(id);
        match kind(a) {
            "clue" => {
                let text = clean(a.get("text"), 20);
                if phase != "clue" || me || text.is_empty() { return no(NOT_NOW) }
                if text.contains(char::is_whitespace) { return no("One word only") }
                let (t, w) = (norm(&text), norm(&d.word));
                if t.contains(&w) || w.contains(&t) { return no("Don't just say the word!") }
                d.clues.insert(id.into(), text);
            }
            "guess" => {
                let text = clean(a.get("text"), 30);
                if phase != "guess" || !me || text.is_empty() || d.answer.is_some() { return no(NOT_NOW) }
                d.answer = Some(text);
                end(g);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Lone);
        (g.phase == "clue" && d.guesser.as_deref() != Some(id))
            .then(|| json!({ "type": "clue", "text": pick(&["big", "fun", "loud", "summer", "yummy", "scary", "round", "fast", "old", "wet"]) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matching_clues_cancel() {
        assert_eq!(norm("Cats!"), "cat");
        assert_eq!(norm("cheeses"), "chees");
        assert_eq!(norm("Cheese"), norm("cheese!"));
    }
}
