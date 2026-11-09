// Quick Wit: everyone answers the same prompt, then votes for the best answer.
// Caption This is the same flow with emoji scenes instead of prompts (QuickWit("caption")).
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

#[derive(Default)]
pub struct D { prompt: String, answers: IndexMap<String, String>, votes: IndexMap<String, usize>, list: Vec<(String, String)> } // list: (owner, text)
pub struct QuickWit(pub &'static str);

fn prompts(g: &Room, id: &str) -> Vec<String> {
    if id == "caption" { return strs(data("caption", "SCENES")) }
    let p = data("quickwit", "PROMPTS");
    strs(p.get(g.set_str("pack")).unwrap_or(&p["spicy"]))
}
fn start_vote(g: &mut Room) {
    let d = dm!(g, Quick);
    d.list = shuffle(d.answers.iter().map(|(o, t)| (o.clone(), t.clone())).collect());
    if d.list.len() < 2 { return tally(g) }
    g.phase = "vote";
    set_timer(g, 30.0, tally, false);
}
fn tally(g: &mut Room) {
    let pts = g.set_or("points", 100.0) as i64;
    let d = dm!(g, Quick);
    for &i in d.votes.values() { if let Some((o, _)) = d.list.get(i) { if let Some(p) = g.players.get_mut(o) { p.score += pts } } }
    finish(g);
}

impl Game for QuickWit {
    fn id(&self) -> &'static str { self.0 }
    fn start(&self, g: &mut Room) {
        g.phase = "answer";
        g.d = Data::Quick(D { prompt: pick(&prompts(g, self.0)), ..Default::default() });
        set_timer(g, 60.0, start_vote, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "answer" => Some(start_vote), "vote" => Some(tally), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Quick);
        match g.phase {
            "answer" => d.answers.contains_key(id),
            // a lone answer's author has nothing to vote for, so they count as done
            "vote" => d.votes.contains_key(id) || d.list.iter().all(|(o, _)| o == id),
            _ => false,
        }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Quick(d) = &g.d else { return };
        if matches!(g.phase, "answer" | "vote") { s.insert("prompt".into(), json!(d.prompt)); }
        // owners stay hidden until results so votes are anonymous
        if g.phase == "vote" { s.insert("list".into(), json!(d.list.iter().map(|(_, t)| json!({ "text": t })).collect::<Vec<_>>())); }
        if matches!(g.phase, "scores" | "results") {
            s.insert("last".into(), json!({ "prompt": d.prompt, "list": d.list.iter().enumerate().map(|(i, (o, t))| json!({
                "text": t, "name": g.name(o), "votes": d.votes.values().filter(|v| **v == i).count() })).collect::<Vec<_>>() }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        match kind(a) {
            "answer" => {
                let text = clean(a.get("text"), 80);
                if g.phase != "answer" || text.is_empty() { return no(NOT_NOW) }
                dm!(g, Quick).answers.insert(id.into(), text);
            }
            "vote" => {
                let d = dm!(g, Quick);
                let Some(i) = index(a.get("index"), d.list.len()).filter(|_| g.phase == "vote") else { return no(NOT_NOW) };
                if d.list[i].0 == id { return no("Can't vote for yourself") }
                d.votes.insert(id.into(), i);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Quick);
        match g.phase {
            "answer" => Some(json!({ "type": "answer", "text": pick(&["a confused llama", "soup, again", "my uncle Steve", "tiny hats", "nobody knows", "free WiFi"]) })),
            "vote" => {
                let ok: Vec<usize> = (0..d.list.len()).filter(|&i| d.list[i].0 != id).collect();
                (!ok.is_empty()).then(|| json!({ "type": "vote", "index": pick(&ok) }))
            }
            _ => None,
        }
    }
}
