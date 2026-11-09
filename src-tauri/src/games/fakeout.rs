// Fake Out: a real fact with a blank. Everyone writes a believable fake, then tries to pick the real one.
// Picking the truth scores, and so does every player your fake fooled.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

struct Item { owner: Option<String>, text: String } // owner None = the real answer
#[derive(Default)]
pub struct D { fact: (String, String), lies: IndexMap<String, String>, picks: IndexMap<String, usize>, list: Vec<Item> }
pub struct FakeOut;

fn norm(s: &str) -> String { s.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit()).collect() }
fn start_pick(g: &mut Room) {
    let d = dm!(g, Fake);
    let mut list = vec![Item { owner: None, text: d.fact.1.clone() }];
    list.extend(d.lies.iter().map(|(o, t)| Item { owner: Some(o.clone()), text: t.clone() }));
    d.list = shuffle(list);
    g.phase = "pick";
    set_timer(g, 30.0, tally, false);
}
fn tally(g: &mut Room) {
    let d = dm!(g, Fake);
    for (id, &i) in &d.picks {
        match &d.list[i].owner { None => { if let Some(p) = g.players.get_mut(id) { p.score += 500 } }
            Some(o) => if let Some(p) = g.players.get_mut(o) { p.score += 250 } }
    }
    finish(g);
}

impl Game for FakeOut {
    fn id(&self) -> &'static str { "fakeout" }
    fn start(&self, g: &mut Room) {
        g.phase = "lie";
        let all: Vec<(String, String)> = data("fakeout", "FACTS").as_array().unwrap().iter()
            .map(|f| (f[0].as_str().unwrap().into(), f[1].as_str().unwrap().into())).collect();
        let fresh: Vec<(String, String)> = all.iter().filter(|f| !g.used.contains(&f.0)).cloned().collect();
        let fact = pick(if fresh.is_empty() { &all } else { &fresh });
        g.used.push(fact.0.clone());
        g.d = Data::Fake(D { fact, ..Default::default() });
        set_timer(g, 60.0, start_pick, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "lie" => Some(start_pick), "pick" => Some(tally), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Fake);
        match g.phase { "lie" => d.lies.contains_key(id), "pick" => d.picks.contains_key(id), _ => false }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Fake(d) = &g.d else { return };
        s.insert("fact".into(), json!(d.fact.0));
        if g.phase == "pick" { s.insert("list".into(), json!(d.list.iter().map(|a| json!({ "text": a.text })).collect::<Vec<_>>())); }
        if matches!(g.phase, "scores" | "results") && !d.list.is_empty() {
            s.insert("last".into(), json!({ "fact": d.fact.0, "list": d.list.iter().enumerate().map(|(i, a)| json!({
                "text": a.text, "truth": a.owner.is_none(), "name": a.owner.as_ref().map(|o| g.name(o)),
                "fooled": d.picks.iter().filter(|(_, v)| **v == i).map(|(pid, _)| g.name(pid)).collect::<Vec<_>>() })).collect::<Vec<_>>() }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let d = dm!(g, Fake);
        match kind(a) {
            "lie" => {
                let text = clean(a.get("text"), 40);
                if phase != "lie" || text.is_empty() { return no(NOT_NOW) }
                if norm(&text) == norm(&d.fact.1) { return no("That's actually the real answer! Try a fake one") }
                if d.lies.iter().any(|(o, t)| o != id && norm(t) == norm(&text)) { return no("Someone already wrote that, try another") }
                d.lies.insert(id.into(), text);
            }
            "guess" => {
                let Some(i) = index(a.get("index"), d.list.len()).filter(|_| phase == "pick") else { return no(NOT_NOW) };
                if d.list[i].owner.as_deref() == Some(id) { return no("That's your own fake") }
                d.picks.insert(id.into(), i);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Fake);
        match g.phase {
            "lie" => Some(json!({ "type": "lie", "text": pick(&["penguins", "a spoon", "seven", "socks", "cheese", "pigeons", "a lawnmower", "twelve years"]) })),
            "pick" => {
                let ok: Vec<usize> = (0..d.list.len()).filter(|&i| d.list[i].owner.as_deref() != Some(id)).collect();
                (!ok.is_empty()).then(|| json!({ "type": "guess", "index": pick(&ok) }))
            }
            _ => None,
        }
    }
}
