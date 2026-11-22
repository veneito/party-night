// Cook-off: for a real cooking contest. Each phone picks "I'm cooking" or "I'm tasting",
// then the tasters vote for the best dish. Each cook scores 100 per vote.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

#[derive(Default)]
pub struct D { cooks: IndexMap<String, bool>, votes: IndexMap<String, String> } // id -> is a cook; voter -> cook
pub struct CookOff;

fn cook_ids(d: &D) -> Vec<String> { d.cooks.iter().filter(|(_, c)| **c).map(|(id, _)| id.clone()).collect() }
fn count(d: &D, cook: &str) -> usize { d.votes.values().filter(|v| *v == cook).count() }
fn start_vote(g: &mut Room) {
    if cook_ids(dr!(g, Cook)).is_empty() { return finish(g) }
    g.phase = "vote";
    set_timer(g, 120.0, tally, false); // tasting takes a while
}
fn tally(g: &mut Room) {
    let d = dr!(g, Cook);
    let won: Vec<(String, i64)> = cook_ids(d).iter().map(|c| (c.clone(), 100 * count(d, c) as i64)).collect();
    for (id, p) in won { g.add(&id, p) }
    finish(g);
}

impl Game for CookOff {
    fn id(&self) -> &'static str { "cookoff" }
    fn start(&self, g: &mut Room) {
        g.phase = "role";
        g.d = Data::Cook(D::default());
        set_timer(g, 45.0, start_vote, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "role" => Some(start_vote), "vote" => Some(tally), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Cook);
        match g.phase { "role" => d.cooks.contains_key(id), "vote" => d.cooks.get(id) == Some(&true) || d.votes.contains_key(id), _ => false }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Cook(d) = &g.d else { return };
        s.insert("cooks".into(), json!(cook_ids(d)));
        if matches!(g.phase, "scores" | "results") {
            let mut v: Vec<Value> = cook_ids(d).iter().map(|c| json!({ "id": c, "votes": count(d, c) })).collect();
            v.sort_by_key(|x| -x["votes"].as_i64().unwrap_or(0));
            s.insert("last".into(), json!({ "list": v }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let d = dm!(g, Cook);
        match kind(a) {
            "role" => {
                if phase != "role" { return no(NOT_NOW) }
                d.cooks.insert(id.into(), truthy(a.get("cook")));
            }
            "vote" => {
                let pick = clean(a.get("cook"), 64);
                if phase != "vote" { return no(NOT_NOW) }
                if d.cooks.get(id) == Some(&true) { return no("Cooks don't vote") }
                if d.cooks.get(&pick) != Some(&true) { return no("Pick a cook") }
                d.votes.insert(id.into(), pick);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, _: &str) -> Option<Value> {
        match g.phase {
            "role" => Some(json!({ "type": "role", "cook": false })),
            "vote" => { let c = cook_ids(dr!(g, Cook)); (!c.is_empty()).then(|| json!({ "type": "vote", "cook": pick(&c) })) }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tasters_vote_for_cooks() {
        let mut g = crate::room::test_room(&["a", "b", "c", "d"]);
        CookOff.start(&mut g);
        for (id, c) in [("a", true), ("b", true), ("c", false), ("d", false)] { CookOff.act(&mut g, &json!({ "type": "role", "cook": c }), id).unwrap() }
        CookOff.advance(g.phase).unwrap()(&mut g);
        assert_eq!(g.phase, "vote");
        assert!(CookOff.act(&mut g, &json!({ "type": "vote", "cook": "b" }), "a").is_err());
        assert!(CookOff.act(&mut g, &json!({ "type": "vote", "cook": "d" }), "c").is_err());
        for v in ["c", "d"] { CookOff.act(&mut g, &json!({ "type": "vote", "cook": "a" }), v).unwrap() }
        CookOff.advance(g.phase).unwrap()(&mut g);
        assert_eq!([g.players["a"].score, g.players["b"].score], [200, 0]);
    }
}
