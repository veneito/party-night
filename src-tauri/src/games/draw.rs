// Draw & Guess: one player draws a secret word on their phone, it appears live on the TV, everyone else races to guess it.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use serde_json::json;

#[derive(Default)]
pub struct D {
    word: String, strokes: Vec<Value>, got: Vec<String>, feed: Vec<Value>, drawer: Option<String>,
    // letters revealed in the hint as time runs out: positions in reveal order, and how many are shown
    reveal: Vec<usize>, shown: usize, started: Option<std::time::Instant>, secs: f64,
}
pub struct Draw;

fn words() -> Vec<String> { strs(data("draw", "WORDS")) }
// about one letter per three, so longer words give away more; none for words of three letters or less
fn max_reveal(word: &str) -> usize { (word.chars().filter(|c| c.is_ascii_lowercase()).count().saturating_sub(1)) / 3 }
// reveals are spread evenly over the round: with 2 to give, one at 1/3 of the time and one at 2/3
fn due(d: &D) -> usize {
    let (Some(t), true) = (d.started, d.secs > 0.0) else { return 0 };
    let frac = t.elapsed().as_secs_f64() / d.secs;
    ((frac * (d.reveal.len() + 1) as f64) as usize).min(d.reveal.len())
}
fn hint(d: &D) -> String {
    d.word.chars().enumerate().map(|(i, c)| if !c.is_ascii_lowercase() || d.reveal[..d.shown].contains(&i) { c } else { '_' }).collect()
}
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
        let word = pick(&words());
        let letters: Vec<usize> = word.chars().enumerate().filter(|(_, c)| c.is_ascii_lowercase()).map(|(i, _)| i).collect();
        let reveal = shuffle(letters).into_iter().take(max_reveal(&word)).collect();
        set_timer(g, 80.0, finish, false);
        // the round's real length, after the speed setting ("No timer" means no reveals)
        let secs = g.timer.map_or(0.0, |(t, _)| t.saturating_duration_since(std::time::Instant::now()).as_secs_f64());
        g.d = Data::Draw(D { word, drawer, reveal, secs, started: Some(std::time::Instant::now()), ..Default::default() });
    }
    fn tick(&self, g: &mut Room) -> bool {
        let Data::Draw(d) = &mut g.d else { return false };
        if g.phase != "draw" || due(d) == d.shown { return false }
        d.shown = due(d);
        true
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
            s.insert("hint".into(), json!(hint(d)));
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hint_reveals_more_for_longer_words() {
        assert_eq!((max_reveal("cat"), max_reveal("pizza"), max_reveal("snowman"), max_reveal("lighthouse"), max_reveal("hot dog")), (0, 1, 2, 3, 1));
        let mut d = D { word: "hot dog".into(), reveal: vec![4, 1], secs: 90.0, started: Some(std::time::Instant::now()), ..Default::default() };
        assert_eq!(hint(&d), "___ ___");
        assert_eq!(due(&d), 0);
        d.shown = 2;
        assert_eq!(hint(&d), "_o_ d__");
        // a third of the way through, the first letter is due
        d.started = Some(std::time::Instant::now() - std::time::Duration::from_secs(31));
        assert_eq!(due(&d), 1);
    }
}
