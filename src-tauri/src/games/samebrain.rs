// Same Brain: one prompt ("Name a pizza topping"), everyone writes one word.
// You score for every other player who wrote the same thing, so think like the room.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

#[derive(Default)]
pub struct D { prompt: String, words: IndexMap<String, String> } // id -> word as typed
pub struct SameBrain;

// matching words grouped, biggest group first: (word as first typed, ids)
fn groups(d: &D) -> Vec<(String, Vec<String>)> {
    let mut m: IndexMap<String, (String, Vec<String>)> = IndexMap::new();
    for (id, w) in &d.words { m.entry(loneclue::norm(w)).or_insert_with(|| (w.clone(), vec![])).1.push(id.clone()) }
    let mut v: Vec<_> = m.into_values().collect();
    v.sort_by_key(|g| std::cmp::Reverse(g.1.len()));
    v
}
fn tally(g: &mut Room) {
    for (_, ids) in groups(dr!(g, Same)) { for id in &ids { g.add(id, 100 * (ids.len() as i64 - 1)) } }
    finish(g);
}

impl Game for SameBrain {
    fn id(&self) -> &'static str { "samebrain" }
    fn start(&self, g: &mut Room) {
        g.phase = "think";
        let all = strs(data("samebrain", "PROMPTS"));
        let fresh: Vec<String> = all.iter().filter(|t| !g.used.contains(t)).cloned().collect();
        let prompt = pick(if fresh.is_empty() { &all } else { &fresh });
        g.used.push(prompt.clone());
        g.d = Data::Same(D { prompt, ..Default::default() });
        set_timer(g, 30.0, tally, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> { (phase == "think").then_some(tally as Next) }
    fn is_done(&self, g: &Room, id: &str) -> bool { g.phase == "think" && dr!(g, Same).words.contains_key(id) }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Same(d) = &g.d else { return };
        if g.phase == "think" { s.insert("prompt".into(), json!(d.prompt)); }
        if matches!(g.phase, "scores" | "results") && !d.prompt.is_empty() {
            s.insert("last".into(), json!({ "prompt": d.prompt, "groups": groups(d).iter().map(|(w, ids)| json!({
                "word": w, "ids": ids })).collect::<Vec<_>>() }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        if kind(a) != "word" { return no("Unknown action") }
        let text = clean(a.get("text"), 24);
        if g.phase != "think" || text.is_empty() { return no(NOT_NOW) }
        if loneclue::norm(&text).is_empty() { return no("Use letters") }
        dm!(g, Same).words.insert(id.into(), text);
        Ok(())
    }
    fn bot(&self, _: &Room, _: &str) -> Option<Value> {
        Some(json!({ "type": "word", "text": pick(&["pizza", "dog", "red", "banana", "Paris", "socks", "coffee"]) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_score_per_other_player() {
        let mut g = crate::room::test_room(&["a", "b", "c"]);
        SameBrain.start(&mut g);
        for (id, w) in [("a", "Cats"), ("b", "cat!"), ("c", "dog")] { SameBrain.act(&mut g, &json!({ "type": "word", "text": w }), id).unwrap() }
        SameBrain.advance(g.phase).unwrap()(&mut g);
        assert_eq!([g.players["a"].score, g.players["b"].score, g.players["c"].score], [100, 100, 0]);
    }
}
