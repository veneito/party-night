// Hot Take: agree or disagree, then guess how much of the room agreed. Closest guesses score.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

#[derive(Default)]
pub struct D { take: String, takes: IndexMap<String, (bool, i64)> } // takes: id -> (agree, guess)
pub struct HotTake;

fn pct(d: &D) -> i64 {
    if d.takes.is_empty() { return 0 }
    round(d.takes.values().filter(|t| t.0).count() as f64 / d.takes.len() as f64 * 100.0) as i64
}
fn tally(g: &mut Room) {
    let d = dm!(g, Hot);
    let real = pct(d);
    // 500 for a perfect guess, 25 fewer per point off, never below zero
    for (id, t) in &d.takes { if let Some(p) = g.players.get_mut(id) { p.score += (500 - 25 * (t.1 - real).abs()).max(0) } }
    finish(g);
}

impl Game for HotTake {
    fn id(&self) -> &'static str { "hottake" }
    fn start(&self, g: &mut Room) {
        g.phase = "take";
        let all = strs(data("hottake", "TAKES"));
        let fresh: Vec<String> = all.iter().filter(|t| !g.used.contains(t)).cloned().collect();
        let take = pick(if fresh.is_empty() { &all } else { &fresh });
        g.used.push(take.clone());
        g.d = Data::Hot(D { take, ..Default::default() });
        set_timer(g, 40.0, tally, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> { (phase == "take").then_some(tally as Next) }
    fn is_done(&self, g: &Room, id: &str) -> bool { g.phase == "take" && dr!(g, Hot).takes.contains_key(id) }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Hot(d) = &g.d else { return };
        if g.phase == "take" { s.insert("take".into(), json!(d.take)); }
        if matches!(g.phase, "scores" | "results") && !d.take.is_empty() {
            s.insert("last".into(), json!({ "take": d.take, "pct": pct(d), "list": d.takes.iter().map(|(id, t)| json!({
                "name": g.name(id), "av": g.players.get(id).map_or(0, |p| p.av), "agree": t.0, "guess": t.1 })).collect::<Vec<_>>() }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        if kind(a) != "take" { return no("Unknown action") }
        let guess = num(a.get("guess"));
        if g.phase != "take" { return no(NOT_NOW) }
        let Some(agree) = a.get("agree").and_then(|v| v.as_bool()) else { return no("Agree or disagree first") };
        if !(0.0..=100.0).contains(&guess) { return no("Guess a number from 0 to 100") }
        dm!(g, Hot).takes.insert(id.into(), (agree, round(guess) as i64));
        Ok(())
    }
    fn bot(&self, _: &Room, _: &str) -> Option<Value> {
        Some(json!({ "type": "take", "agree": rnd() < 0.5, "guess": round(rnd() * 10.0) * 10.0 }))
    }
}
