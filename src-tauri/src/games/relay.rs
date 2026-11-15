// Charades and Forbidden Words: two teams take turns. One player gets words on their phone and acts them out
// (Relay("charades")) or describes them without the banned words (Relay("taboo")); their team guesses out loud.
// The actor taps Got it / Pass. In Forbidden Words the other team sees the banned words and can buzz a slip.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use serde_json::json;

#[derive(Default)]
pub struct D { actor: String, team: usize, deck: Vec<Value>, n: usize, got: Vec<String>, passed: Vec<String>, buzzed: Vec<String> }
pub struct Relay(pub &'static str);

// teams by join order: 0, 1, 0, 1...
fn teams(g: &Room) -> [Vec<String>; 2] {
    let mut t = [vec![], vec![]];
    for (i, id) in g.ids().into_iter().enumerate() { t[i % 2].push(id) }
    t
}
fn team_of(g: &Room, id: &str) -> Option<usize> { g.ids().iter().position(|x| x == id).map(|i| i % 2) }
fn card(d: &D) -> Option<&Value> { d.deck.get(d.n) }
fn word(c: &Value) -> String { c.as_str().or_else(|| c[0].as_str()).unwrap_or("").to_string() }
fn go(g: &mut Room) { g.phase = "act"; set_timer(g, 60.0, finish, true) }
fn next_card(g: &mut Room, kind: &str, team: usize, pts: i64) {
    let members = teams(g)[team].clone();
    let d = dm!(g, Relay);
    let Some(c) = card(d) else { return };
    let w = word(c);
    match kind { "got" => d.got.push(w), "pass" => d.passed.push(w), _ => d.buzzed.push(w) }
    d.n += 1;
    for id in members { g.add(&id, pts) }
    if card(dr!(g, Relay)).is_none() { finish(g) } // deck ran out
}

impl Game for Relay {
    fn id(&self) -> &'static str { self.0 }
    fn start(&self, g: &mut Room) {
        let t = teams(g);
        let team = (g.round.max(1) as usize - 1) % 2;
        let pool = &t[if t[1].is_empty() { 0 } else { team }];
        let actor = pool[(g.round.max(1) as usize - 1) / 2 % pool.len()].clone();
        let all: Vec<Value> = data(self.0, "CARDS").as_array().cloned().unwrap_or_default();
        let fresh: Vec<Value> = all.iter().filter(|c| !g.used.contains(&word(c))).cloned().collect();
        let deck = shuffle(if fresh.len() < 20 { all } else { fresh });
        for c in &deck { g.used.push(word(c)) }
        g.d = Data::Relay(D { actor, team, deck, ..Default::default() });
        g.phase = "ready";
        set_timer(g, 20.0, go, true); // time to stand up; the actor can start sooner
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "ready" => Some(go), "act" => Some(finish), _ => None }
    }
    fn is_done(&self, _: &Room, _: &str) -> bool { false } // turns end on the clock, not when everyone's in
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Relay(d) = &g.d else { return };
        if d.actor.is_empty() { return }
        let t = teams(g);
        s.insert("actor".into(), json!(d.actor));
        s.insert("team".into(), json!(d.team));
        s.insert("teams".into(), json!(t));
        s.insert("turnNo".into(), json!(d.n));
        s.insert("got".into(), json!(d.got));
        s.insert("passed".into(), json!(d.passed));
        s.insert("buzzed".into(), json!(d.buzzed));
        // the secret word rides in the shared update (like Odd One Out); only the right phones show it
        if g.phase == "act" { if let Some(c) = card(d) { s.insert("card".into(), json!({ "word": word(c), "banned": c.get(1) })); } }
        let tot = |k: usize| t[k].first().and_then(|id| g.players.get(id)).map_or(0, |p| p.score);
        s.insert("totals".into(), json!([tot(0), tot(1)]));
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let d = dr!(g, Relay);
        let (team, actor) = (d.team, d.actor == id);
        // n says which card the tap was about, so a double tap never skips two words
        if num(a.get("n")) as usize != d.n && kind(a) != "go" { return no(NOT_NOW) }
        match kind(a) {
            "go" if actor && g.phase == "ready" => go(g),
            "got" if actor && g.phase == "act" => next_card(g, "got", team, 100),
            "pass" if actor && g.phase == "act" => next_card(g, "pass", team, 0),
            "buzz" if self.0 == "taboo" && g.phase == "act" && team_of(g, id) == Some(1 - team) => next_card(g, "buzz", 1 - team, 50),
            _ => return no(NOT_NOW),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Relay);
        if g.phase == "ready" { return (d.actor == id).then(|| json!({ "type": "go" })) }
        if d.actor == id { return Some(json!({ "type": if rnd() < 0.6 { "got" } else { "pass" }, "n": d.n })) }
        (self.0 == "taboo" && team_of(g, id) == Some(1 - d.team) && rnd() < 0.05).then(|| json!({ "type": "buzz", "n": d.n }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn team_scores_and_buzz() {
        let mut g = crate::room::test_room(&["a", "b", "c", "d"]);
        let r = Relay("taboo");
        r.start(&mut g);
        assert_eq!((dr!(g, Relay).actor.as_str(), g.phase), ("a", "ready"));
        r.act(&mut g, &json!({ "type": "go" }), "a").unwrap();
        r.act(&mut g, &json!({ "type": "got", "n": 0 }), "a").unwrap();
        assert!(r.act(&mut g, &json!({ "type": "got", "n": 0 }), "a").is_err()); // stale double tap
        assert!(r.act(&mut g, &json!({ "type": "buzz", "n": 1 }), "c").is_err()); // own team can't buzz
        r.act(&mut g, &json!({ "type": "buzz", "n": 1 }), "b").unwrap();
        assert_eq!([g.players["a"].score, g.players["c"].score, g.players["b"].score, g.players["d"].score], [100, 100, 50, 50]);
    }
}
