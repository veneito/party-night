// Draw & Guess: one player draws a secret word on their phone, it appears live on the TV, everyone else races to guess it.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use serde_json::json;

#[derive(Default)]
pub struct D { word: String, strokes: Vec<Value>, got: Vec<String>, feed: Vec<Value>, drawer: Option<String> }
pub struct Draw;

fn words() -> Vec<String> { strs(data("draw", "WORDS")) }
fn norm(t: &str) -> String { t.to_lowercase().chars().filter(|c| c.is_ascii_lowercase()).collect() }
fn correct(g: &mut Room, id: &str) {
    let d = dm!(g, Draw);
    if d.got.iter().any(|x| x == id) { return }
    d.got.push(id.into());
    let n = d.got.len() as i64;
    if let Some(p) = g.players.get_mut(id) { p.score += (1000 - 150 * (n - 1)).max(300) }
    if let Some(p) = d.drawer.as_ref().and_then(|dr| g.players.get_mut(dr)) { p.score += 250 }
}

impl Game for Draw {
    fn id(&self) -> &'static str { "draw" }
    fn start(&self, g: &mut Room) {
        g.phase = "draw";
        // take turns drawing; bots can't draw, so only real players get a turn
        let ids = g.real_ids();
        let drawer = (!ids.is_empty()).then(|| ids[(g.round - 1).max(0) as usize % ids.len()].clone());
        g.d = Data::Draw(D { word: pick(&words()), drawer, ..Default::default() });
        set_timer(g, 80.0, finish, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> { (phase == "draw").then_some(finish as Next) }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Draw);
        g.phase == "draw" && (d.drawer.as_deref() == Some(id) || d.got.iter().any(|x| x == id))
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Draw(d) = &g.d else { return };
        if g.phase == "draw" {
            s.insert("drawer".into(), json!(d.drawer));
            s.insert("strokes".into(), json!(d.strokes));
            s.insert("feed".into(), json!(d.feed[d.feed.len().saturating_sub(6)..]));
            s.insert("hint".into(), json!(d.word.chars().map(|c| if c.is_ascii_lowercase() { '_' } else { c }).collect::<String>()));
            s.insert("got".into(), json!(d.got.len()));
            // Note: the word rides in the shared broadcast (a peek in devtools shows it), fine among friends, same as Odd One Out.
            s.insert("word".into(), json!(d.word));
        }
        if matches!(g.phase, "scores" | "results") && !d.word.is_empty() {
            s.insert("last".into(), json!({ "word": d.word, "strokes": d.strokes, "drawer": g.name(d.drawer.as_deref().unwrap_or("")),
                "got": d.got.iter().map(|id| g.name(id)).collect::<Vec<_>>() }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        if g.phase != "draw" { return no(NOT_NOW) }
        let d = dm!(g, Draw);
        let drawing = d.drawer.as_deref() == Some(id);
        match kind(a) {
            "stroke" => {
                if !drawing { return no("Not your turn to draw") }
                let Some(s) = stroke(a).filter(|_| d.strokes.len() < 400) else { return no(NOT_NOW) };
                d.strokes.push(s);
            }
            k @ ("undo" | "wipe") => {
                if !drawing { return no("Not your turn to draw") }
                if k == "undo" { d.strokes.pop(); } else { d.strokes.clear() }
            }
            "guess" => {
                let text = clean(a.get("text"), 30);
                if drawing || text.is_empty() { return no(NOT_NOW) }
                if d.got.iter().any(|x| x == id) { return no("You already got it!") }
                if norm(&text) != norm(&d.word) { d.feed.push(json!({ "id": id, "text": text })); return no("Nope, keep guessing!") }
                d.feed.push(json!({ "id": id, "ok": true }));
                correct(g, id);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Draw);
        (g.phase == "draw" && !d.got.iter().any(|x| x == id))
            .then(|| json!({ "type": "guess", "text": if rnd() < 0.25 { d.word.clone() } else { pick(&words()) } }))
    }
}
