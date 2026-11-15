// Rank It: one player (the judge, rotating each round) secretly ranks four things from best to worst.
// Everyone else guesses the judge's order. 100 per item in the right spot, 200 more for all four.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

#[derive(Default)]
pub struct D { judge: String, topic: String, items: Vec<String>, ranks: IndexMap<String, Vec<usize>> } // ranks: id -> item indexes, best first
pub struct RankIt;

fn hits(a: &[usize], b: &[usize]) -> usize { a.iter().zip(b).filter(|(x, y)| x == y).count() }
fn score(h: usize) -> i64 { 100 * h as i64 + if h == 4 { 200 } else { 0 } }
fn tally(g: &mut Room) {
    let d = dr!(g, Rank);
    let mut won: Vec<(String, i64)> = vec![];
    if let Some(truth) = d.ranks.get(&d.judge) {
        for (id, r) in d.ranks.iter().filter(|(id, _)| **id != d.judge) {
            let h = hits(r, truth);
            won.push((id.clone(), score(h)));
            won.push((d.judge.clone(), 50 * h as i64)); // the judge scores for being easy to read
        }
    }
    for (id, p) in won { g.add(&id, p) }
    finish(g);
}

impl Game for RankIt {
    fn id(&self) -> &'static str { "rankit" }
    fn start(&self, g: &mut Room) {
        g.phase = "rank";
        let ids = g.ids();
        let judge = ids[(g.round.max(1) as usize - 1) % ids.len()].clone();
        let all: Vec<Value> = data("rankit", "TOPICS").as_array().cloned().unwrap_or_default();
        let fresh: Vec<Value> = all.iter().filter(|t| !g.used.iter().any(|u| Some(u.as_str()) == t[0].as_str())).cloned().collect();
        let t = pick(if fresh.is_empty() { &all } else { &fresh });
        let topic = t[0].as_str().unwrap_or("").to_string();
        g.used.push(topic.clone());
        g.d = Data::Rank(D { judge, topic, items: shuffle(strs(&t[1])), ..Default::default() });
        set_timer(g, 45.0, tally, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> { (phase == "rank").then_some(tally as Next) }
    fn is_done(&self, g: &Room, id: &str) -> bool { g.phase == "rank" && dr!(g, Rank).ranks.contains_key(id) }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Rank(d) = &g.d else { return };
        if d.topic.is_empty() { return }
        s.insert("judge".into(), json!(d.judge));
        s.insert("topic".into(), json!(d.topic));
        s.insert("items".into(), json!(d.items));
        if matches!(g.phase, "scores" | "results") {
            let truth = d.ranks.get(&d.judge);
            s.insert("last".into(), json!({ "truth": truth, "list": d.ranks.iter().filter(|(id, _)| **id != d.judge).map(|(id, r)| json!({
                "id": id, "hits": truth.map_or(0, |t| hits(r, t)) })).collect::<Vec<_>>() }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        if kind(a) != "rank" { return no("Unknown action") }
        if g.phase != "rank" { return no(NOT_NOW) }
        let d = dm!(g, Rank);
        let order: Vec<usize> = a.get("order").and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_u64().map(|n| n as usize)).collect()).unwrap_or_default();
        let mut sorted = order.clone(); sorted.sort();
        if sorted != (0..d.items.len()).collect::<Vec<_>>() { return no("Rank all four") }
        d.ranks.insert(id.into(), order);
        Ok(())
    }
    fn bot(&self, g: &Room, _: &str) -> Option<Value> {
        Some(json!({ "type": "rank", "order": shuffle((0..dr!(g, Rank).items.len()).collect::<Vec<_>>()) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guess_scores_against_judge() {
        let mut g = crate::room::test_room(&["a", "b", "c"]);
        RankIt.start(&mut g);
        assert_eq!(dr!(g, Rank).judge, "a");
        RankIt.act(&mut g, &json!({ "type": "rank", "order": [0, 1, 2, 3] }), "a").unwrap();
        RankIt.act(&mut g, &json!({ "type": "rank", "order": [0, 1, 2, 3] }), "b").unwrap();
        RankIt.act(&mut g, &json!({ "type": "rank", "order": [1, 0, 2, 3] }), "c").unwrap();
        assert!(RankIt.act(&mut g, &json!({ "type": "rank", "order": [0, 0, 2, 3] }), "c").is_err());
        RankIt.advance(g.phase).unwrap()(&mut g);
        assert_eq!([g.players["a"].score, g.players["b"].score, g.players["c"].score], [300, 600, 200]);
    }
}
