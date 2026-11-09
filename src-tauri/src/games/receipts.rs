// Receipts: everyone confesses something small and true to the same prompt, then the room works out who wrote what.
// Confessions come up one at a time; everyone picks who they think wrote it, then the author is revealed.
// Once per game each player can Frame someone (write in their style; if the room pins it on them, double points)
// and Plead the Fifth (their confession's author stays secret and nobody scores on it).
// The last round is "Most Wanted": after the guessing, the room votes for the most shocking confession.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

const RIGHT: i64 = 100; // per correct guess
const FOOLED: i64 = 50; // to the author, per player who guessed wrong
const WANTED: i64 = 300; // Most Wanted winner, if nobody guessed them

struct Entry { by: String, text: String, framed: Option<String>, guesses: IndexMap<String, String>, pleaded: bool, frame_worked: bool }
#[derive(Default)]
pub struct D {
    prompt: String,
    last_round: bool,
    written: IndexMap<String, (String, Option<String>)>, // id -> (text, framed player)
    entries: Vec<Entry>,
    cur: usize,
    wanted: IndexMap<String, usize>,
    wanted_win: Option<(usize, bool)>, // (entry, got the bonus)
    // once per game, so these carry over from round to round
    frames_used: Vec<String>,
    pleas_used: Vec<String>,
}
pub struct Receipts;

fn present(g: &Room, ids: &[String]) -> Value { json!(ids.iter().filter(|i| g.players.contains_key(*i)).collect::<Vec<_>>()) }

fn start_guess(g: &mut Room) {
    let d = dm!(g, Receipts);
    let written = std::mem::take(&mut d.written);
    d.entries = shuffle(written.into_iter().map(|(by, (text, framed))| Entry { by, text, framed, guesses: IndexMap::new(), pleaded: false, frame_worked: false }).collect());
    d.cur = 0;
    if d.entries.is_empty() { return finish(g) }
    g.phase = "guess";
    set_timer(g, 30.0, reveal, false);
}
fn reveal(g: &mut Room) {
    let d = dm!(g, Receipts);
    let e = &mut d.entries[d.cur];
    if !e.pleaded {
        let mut fooled = 0;
        for (voter, pick) in &e.guesses {
            if *pick == e.by { if let Some(p) = g.players.get_mut(voter) { p.score += RIGHT } } else { fooled += 1 }
        }
        // a frame works when the person framed got the most votes (ties count)
        if let Some(t) = &e.framed {
            let mut n: IndexMap<&str, usize> = IndexMap::new();
            for p in e.guesses.values() { *n.entry(p.as_str()).or_default() += 1 }
            let top = n.values().copied().max().unwrap_or(0);
            e.frame_worked = top > 0 && n.get(t.as_str()) == Some(&top);
        }
        let pts = FOOLED * fooled * if e.frame_worked { 2 } else { 1 };
        if let Some(p) = g.players.get_mut(&e.by) { p.score += pts }
    }
    g.phase = "reveal";
    set_timer(g, 8.0, next_entry, true);
}
fn next_entry(g: &mut Room) {
    let d = dm!(g, Receipts);
    d.cur += 1;
    if d.cur < d.entries.len() { g.phase = "guess"; return set_timer(g, 30.0, reveal, false) }
    if d.last_round && d.entries.len() > 1 { g.phase = "wanted"; return set_timer(g, 30.0, tally_wanted, false) }
    finish(g);
}
fn tally_wanted(g: &mut Room) {
    let d = dm!(g, Receipts);
    let mut n = vec![0; d.entries.len()];
    for &i in d.wanted.values() { n[i] += 1 }
    let top = n.iter().copied().max().unwrap_or(0);
    if top > 0 {
        let win = pick(&(0..n.len()).filter(|i| n[*i] == top).collect::<Vec<_>>());
        let e = &d.entries[win];
        let hidden = !e.guesses.values().any(|p| *p == e.by);
        if hidden { if let Some(p) = g.players.get_mut(&e.by) { p.score += WANTED } }
        d.wanted_win = Some((win, hidden));
    }
    finish(g);
}

impl Game for Receipts {
    fn id(&self) -> &'static str { "receipts" }
    fn start(&self, g: &mut Room) {
        let (frames_used, pleas_used) = match &mut g.d {
            Data::Receipts(d) if g.round > 1 => (std::mem::take(&mut d.frames_used), std::mem::take(&mut d.pleas_used)),
            _ => Default::default(),
        };
        let last_round = g.round as f64 >= g.set_or("rounds", 3.0);
        let all = strs(data("receipts", if last_round { "FINAL" } else { "PROMPTS" }));
        let fresh: Vec<String> = all.iter().filter(|t| !g.used.contains(t)).cloned().collect();
        let prompt = pick(if fresh.is_empty() { &all } else { &fresh });
        g.used.push(prompt.clone());
        g.d = Data::Receipts(D { prompt, last_round, frames_used, pleas_used, ..Default::default() });
        g.phase = "confess";
        set_timer(g, 75.0, start_guess, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "confess" => Some(start_guess), "guess" => Some(reveal), "reveal" => Some(next_entry), "wanted" => Some(tally_wanted), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Receipts);
        match g.phase {
            "confess" => d.written.contains_key(id),
            "guess" => d.entries[d.cur].by == id || d.entries[d.cur].guesses.contains_key(id),
            "wanted" => d.wanted.contains_key(id) || d.entries.iter().all(|e| e.by == id),
            _ => false,
        }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Receipts(d) = &g.d else { return };
        s.insert("prompt".into(), json!(d.prompt));
        s.insert("lastRound".into(), json!(d.last_round));
        s.insert("framesUsed".into(), present(g, &d.frames_used));
        s.insert("pleasUsed".into(), present(g, &d.pleas_used));
        if matches!(g.phase, "guess" | "reveal") {
            let e = &d.entries[d.cur];
            s.insert("view".into(), json!(d.cur));
            s.insert("entryNo".into(), json!(d.cur + 1));
            s.insert("entryCount".into(), json!(d.entries.len()));
            // Note: the author rides in the shared broadcast so a phone knows its own confession (a peek in devtools shows it), fine among friends.
            s.insert("entry".into(), json!({ "text": e.text, "by": e.by }));
        }
        if g.phase == "reveal" {
            let e = &d.entries[d.cur];
            s.insert("reveal".into(), json!({
                "pleaded": e.pleaded, "framed": e.framed, "frameWorked": e.frame_worked,
                "guesses": e.guesses.iter().map(|(v, p)| json!({ "id": v, "pick": p, "ok": *p == e.by })).collect::<Vec<_>>(),
            }));
        }
        if g.phase == "wanted" { s.insert("list".into(), json!(d.entries.iter().map(|e| json!({ "text": e.text, "by": e.by })).collect::<Vec<_>>())); }
        if matches!(g.phase, "scores" | "results") && !d.entries.is_empty() {
            s.insert("last".into(), json!({ "prompt": d.prompt, "entries": d.entries.iter().map(|e| json!({
                "text": e.text, "name": if e.pleaded { json!(null) } else { json!(g.name(&e.by)) }, "pleaded": e.pleaded,
                "framed": e.framed.as_ref().map(|f| g.name(f)), "frameWorked": e.frame_worked,
                "right": e.guesses.values().filter(|p| **p == e.by).count(), "guesses": e.guesses.len() })).collect::<Vec<_>>(),
                "wanted": d.wanted_win.map(|(i, bonus)| json!({ "text": d.entries[i].text, "name": g.name(&d.entries[i].by), "bonus": bonus })) }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let others: Vec<String> = g.players.keys().filter(|p| *p != id).cloned().collect();
        let d = dm!(g, Receipts);
        match kind(a) {
            "confess" => {
                if phase != "confess" || d.written.contains_key(id) { return no(NOT_NOW) }
                let text = clean(a.get("text"), 120);
                if text.is_empty() { return no("Write your confession") }
                let framed = Some(clean(a.get("frame"), 40)).filter(|f| !f.is_empty());
                if let Some(f) = &framed {
                    if d.frames_used.iter().any(|x| x == id) { return no("You already used your Frame Job") }
                    if !others.contains(f) { return no("Pick someone to frame") }
                    d.frames_used.push(id.into());
                }
                d.written.insert(id.into(), (text, framed));
            }
            "guess" => {
                if phase != "guess" { return no(NOT_NOW) }
                let who = clean(a.get("who"), 40);
                let e = &mut d.entries[d.cur];
                if e.by == id { return no("That one's yours, keep a straight face") }
                if !others.contains(&who) { return no("Pick someone else") }
                e.guesses.insert(id.into(), who);
            }
            "plead" => {
                if phase != "guess" || d.entries[d.cur].by != id { return no(NOT_NOW) }
                if d.pleas_used.iter().any(|x| x == id) { return no("You already pleaded the Fifth") }
                d.pleas_used.push(id.into());
                d.entries[d.cur].pleaded = true;
                reveal(g);
            }
            "wanted" => {
                let Some(i) = index(a.get("index"), d.entries.len()).filter(|_| phase == "wanted") else { return no(NOT_NOW) };
                if d.entries[i].by == id { return no("Can't vote for your own") }
                d.wanted.insert(id.into(), i);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Receipts);
        match g.phase {
            "confess" => Some(json!({ "type": "confess", "text": pick(&strs(data("receipts", "BOT"))) })),
            "guess" => {
                let ok: Vec<&String> = g.players.keys().filter(|p| *p != id).collect();
                (!ok.is_empty()).then(|| json!({ "type": "guess", "who": pick(&ok) }))
            }
            "wanted" => {
                let ok: Vec<usize> = (0..d.entries.len()).filter(|&i| d.entries[i].by != id).collect();
                (!ok.is_empty()).then(|| json!({ "type": "wanted", "index": pick(&ok) }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::test_room;
    fn act(g: &mut Room, id: &str, a: Value) -> Res {
        let r = Receipts.act(g, &a, id);
        if r.is_ok() && g.players.keys().all(|p| Receipts.is_done(g, p)) { if let Some(f) = Receipts.advance(g.phase) { f(g) } }
        r
    }
    #[test]
    fn guessing_frames_and_pleas() {
        let mut g = test_room(&["a", "b", "c", "d"]);
        g.settings.insert("rounds".into(), json!(2)); // round 1 of 2: no Most Wanted yet
        Receipts.start(&mut g);
        // c frames d; a uses the frame, then can't use it again next round
        act(&mut g, "a", json!({ "type": "confess", "text": "I eat soup with a fork" })).unwrap();
        assert!(act(&mut g, "c", json!({ "type": "confess", "text": "x", "frame": "zzz" })).is_err());
        act(&mut g, "c", json!({ "type": "confess", "text": "I love line dancing", "frame": "d" })).unwrap();
        act(&mut g, "b", json!({ "type": "confess", "text": "I fake laugh" })).unwrap();
        act(&mut g, "d", json!({ "type": "confess", "text": "I hum in lifts" })).unwrap();
        assert_eq!(g.phase, "guess");
        // play out every confession: everyone (except the author) blames d
        while g.phase == "guess" {
            let by = dr!(g, Receipts).entries[dr!(g, Receipts).cur].by.clone();
            if by == "b" { act(&mut g, "b", json!({ "type": "plead" })).unwrap(); } // b hides theirs
            else { for v in ["a", "b", "c", "d"] { if v != by { act(&mut g, v, json!({ "type": "guess", "who": if v == "d" { "a" } else { "d" } })).unwrap(); } } }
            assert_eq!(g.phase, "reveal");
            next_entry(&mut g);
        }
        assert_eq!(g.phase, "scores");
        // a's: d right (+100), b and c fooled -> a +100. c framed d and the room blamed d: 3 fooled, doubled -> c +300.
        // d's: a, b, c right (+100 each). b pleaded: nobody scores on it.
        let s = |id: &str| g.players[id].score;
        assert_eq!((s("a"), s("b"), s("c"), s("d")), (200, 100, 400, 100));
        assert!(act(&mut g, "a", json!({ "type": "confess", "text": "again" })).is_err());
        // round 2 is the last: Most Wanted, and used frames/pleas carry over
        g.round = 2;
        Receipts.start(&mut g);
        assert!(dr!(g, Receipts).last_round);
        assert!(act(&mut g, "c", json!({ "type": "confess", "text": "y", "frame": "a" })).is_err());
    }
}
