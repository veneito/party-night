// Telephone Sketch: everyone writes a prompt, then the chains pass phone to phone, alternating draw what you read / describe what you see.
// The TV reveals each chain start to finish, then everyone votes for the funniest chain.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

const MAX_STEPS: usize = 8;
#[derive(Clone)]
struct Entry { by: String, text: Option<String>, strokes: Option<Vec<Value>> }
#[derive(Default)]
pub struct D { order: Vec<String>, chains: Vec<Vec<Entry>>, step: usize, steps: usize, pads: IndexMap<String, Vec<Value>>, tvotes: IndexMap<String, usize>, show: usize }
pub struct Telephone;

fn starters() -> Vec<String> { strs(data("telephone", "STARTERS")) }
fn entry_json(e: &Entry, name: Option<String>) -> Value {
    let mut o = json!({});
    if let Some(n) = name { o["name"] = json!(n) }
    if let Some(t) = &e.text { o["text"] = json!(t) }
    if let Some(s) = &e.strokes { o["strokes"] = json!(s) }
    o
}
// which chain this player is handling at the current step
fn my_chain(d: &D, id: &str) -> Option<usize> {
    let n = d.order.len();
    d.order.iter().position(|x| x == id).map(|i| (i + n * MAX_STEPS - d.step) % n)
}
fn in_step(d: &D, id: &str) -> bool { my_chain(d, id).is_some_and(|c| d.chains[c].len() > d.step) }
fn next_step(g: &mut Room) {
    let drawing = g.phase == "tdraw";
    let d = dm!(g, Tel);
    let n = d.order.len();
    // fill in anyone who ran out of time, so every chain stays the same length
    for c in 0..d.chains.len() {
        if d.chains[c].len() > d.step { continue }
        let by = d.order[(c + d.step) % n].clone();
        let e = if drawing { Entry { strokes: Some(d.pads.get(&by).cloned().unwrap_or_default()), text: None, by } }
            else { Entry { text: Some(if d.step > 0 { "(no idea)".into() } else { pick(&starters()) }), strokes: None, by } };
        d.chains[c].push(e);
    }
    d.pads.clear();
    d.step += 1;
    if d.step >= d.steps { d.show = 0; g.phase = "tshow"; return set_timer(g, 20.0, next_show, false) }
    g.phase = if d.step % 2 == 1 { "tdraw" } else { "tguess" };
    set_timer(g, if g.phase == "tdraw" { 70.0 } else { 40.0 }, next_step, false);
}
fn next_show(g: &mut Room) {
    let d = dm!(g, Tel);
    d.show += 1;
    if d.show < d.chains.len() { return set_timer(g, 20.0, next_show, false) }
    g.phase = "tvote";
    set_timer(g, 25.0, tally, false);
}
fn tally(g: &mut Room) {
    let d = dm!(g, Tel);
    for (voter, &c) in &d.tvotes {
        for e in &d.chains[c] { if e.by != *voter { if let Some(p) = g.players.get_mut(&e.by) { p.score += 200 } } }
    }
    finish(g);
}
fn chain_out(g: &Room, ch: &[Entry]) -> Value { json!(ch.iter().map(|e| entry_json(e, Some(g.name(&e.by)))).collect::<Vec<_>>()) }

impl Game for Telephone {
    fn id(&self) -> &'static str { "telephone" }
    fn start(&self, g: &mut Room) {
        let order = shuffle(g.ids());
        let n = order.len();
        g.d = Data::Tel(D { chains: vec![vec![]; n], steps: n.min(MAX_STEPS), order, ..Default::default() });
        g.phase = "tstart";
        set_timer(g, 40.0, next_step, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "tstart" | "tdraw" | "tguess" => Some(next_step), "tshow" => Some(next_show), "tvote" => Some(tally), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Tel);
        if !d.order.iter().any(|x| x == id) { return true }
        match g.phase { "tstart" | "tdraw" | "tguess" => in_step(d, id), "tvote" => d.tvotes.contains_key(id), _ => false }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Tel(d) = &g.d else { return };
        s.insert("step".into(), json!(d.step));
        s.insert("steps".into(), json!(d.steps));
        if matches!(g.phase, "tdraw" | "tguess") { // what each player got handed: the previous entry in their chain
            s.insert("task".into(), Value::Object(d.order.iter().map(|id| (id.clone(), entry_json(&d.chains[my_chain(d, id).unwrap()][d.step - 1], None))).collect()));
        }
        if g.phase == "tdraw" { s.insert("pads".into(), json!(d.pads)); }
        if g.phase == "tshow" {
            s.insert("show".into(), json!(d.show));
            s.insert("chain".into(), chain_out(g, &d.chains[d.show]));
            s.insert("chainCount".into(), json!(d.chains.len()));
        }
        if g.phase == "tvote" { s.insert("firsts".into(), json!(d.chains.iter().map(|ch| &ch[0].text).collect::<Vec<_>>())); }
        if matches!(g.phase, "scores" | "results") && !d.chains.is_empty() {
            let n: Vec<usize> = (0..d.chains.len()).map(|c| d.tvotes.values().filter(|v| **v == c).count()).collect();
            let top = *n.iter().max().unwrap();
            let best = n.iter().position(|x| *x == top).unwrap();
            s.insert("last".into(), json!({ "votes": n[best], "chain": chain_out(g, &d.chains[best]) }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let bot = g.players.get(id).is_some_and(|p| p.bot);
        let d = dm!(g, Tel);
        let Some(mine) = my_chain(d, id) else { return no(NOT_NOW) };
        let drawing = phase == "tdraw" && !in_step(d, id);
        match kind(a) {
            "stroke" => {
                if !drawing { return no(NOT_NOW) }
                let pad = d.pads.entry(id.into()).or_default();
                let Some(s) = stroke(a).filter(|_| pad.len() < 400) else { return no(NOT_NOW) };
                pad.push(s);
            }
            k @ ("undo" | "wipe") => {
                if !drawing { return no(NOT_NOW) }
                if k == "undo" { if let Some(p) = d.pads.get_mut(id) { p.pop(); } } else { d.pads.insert(id.into(), vec![]); }
            }
            "entry" => {
                if !matches!(phase, "tstart" | "tdraw" | "tguess") || in_step(d, id) { return no(NOT_NOW) }
                if phase == "tdraw" {
                    // bots hand in strokes directly; real players have been sending them one by one
                    let strokes = match a.get("strokes").and_then(|v| v.as_array()) {
                        Some(s) if bot => s.clone(),
                        _ => d.pads.get(id).cloned().unwrap_or_default(),
                    };
                    if strokes.is_empty() { return no("Draw something first!") }
                    d.chains[mine].push(Entry { by: id.into(), text: None, strokes: Some(strokes) });
                } else {
                    let text = clean(a.get("text"), 60);
                    if text.is_empty() { return no(if phase == "tstart" { "Write something to draw" } else { "Describe the drawing" }) }
                    d.chains[mine].push(Entry { by: id.into(), text: Some(text), strokes: None });
                }
            }
            "tvote" => {
                let Some(c) = index(a.get("chain"), d.chains.len()).filter(|_| phase == "tvote") else { return no(NOT_NOW) };
                d.tvotes.insert(id.into(), c);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, _: &str) -> Option<Value> {
        let d = dr!(g, Tel);
        match g.phase {
            "tstart" | "tguess" => Some(json!({ "type": "entry", "text": pick(&starters()) })),
            "tdraw" => Some(json!({ "type": "entry", "strokes": [{ "c": "#231a08", "w": 10, "p": (0..12).map(|_| (rnd() * 1000.0) as i64).collect::<Vec<_>>() }] })),
            "tvote" => Some(json!({ "type": "tvote", "chain": (rnd() * d.chains.len() as f64) as usize })),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::test_room;
    // same walk-through as the old games/telephone.js self-check, with shuffle left to chance
    #[test]
    fn chains_pass_along_and_score() {
        let mut g = test_room(&["a", "b", "c"]);
        let t = Telephone;
        let act = |g: &mut Room, id: &str, a: Value| {
            let r = t.act(g, &a, id);
            if r.is_ok() && g.players.keys().all(|p| t.is_done(g, p)) { t.advance(g.phase).unwrap()(g) }
            r
        };
        t.start(&mut g);
        for id in ["a", "b", "c"] { assert!(act(&mut g, id, json!({ "type": "entry", "text": format!("prompt {id}") })).is_ok()) }
        assert_eq!(g.phase, "tdraw");
        assert!(act(&mut g, "a", json!({ "type": "entry" })).is_err()); // nothing drawn yet
        for id in ["a", "b", "c"] { act(&mut g, id, json!({ "type": "stroke", "p": [0, 0, 10, 10] })).unwrap(); act(&mut g, id, json!({ "type": "entry" })).unwrap(); }
        assert_eq!(g.phase, "tguess");
        act(&mut g, "a", json!({ "type": "entry", "text": "guess a" })).unwrap();
        next_step(&mut g); // b and c time out
        assert_eq!(g.phase, "tshow");
        let d = dr!(g, Tel);
        assert!(d.chains.iter().all(|ch| ch.len() == 3));
        assert_eq!(d.chains.iter().filter(|ch| ch[2].text.as_deref() == Some("(no idea)")).count(), 2);
        for _ in 0..3 { next_show(&mut g) }
        assert_eq!(g.phase, "tvote");
        for id in ["a", "b", "c"] { act(&mut g, id, json!({ "type": "tvote", "chain": 1 })).unwrap(); }
        assert_eq!(g.phase, "results");
        // each author in chain 1 gets 200 per vote from someone else: 2 votes each
        assert!(g.players.values().all(|p| p.score == 400));
    }
}
