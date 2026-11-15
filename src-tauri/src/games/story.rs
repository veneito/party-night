// Story Chain: every story starts with an opening line. Each step, every player adds the next sentence to a
// different story, seeing only the line right before theirs. The TV reads each story out, then everyone votes
// for their favourite line. 100 points per vote.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

const MAX_STEPS: usize = 4;
#[derive(Default)]
pub struct D { order: Vec<String>, stories: Vec<Vec<(String, String)>>, step: usize, steps: usize, show: usize, votes: IndexMap<String, usize> } // story: (writer, line), writer "" = opening line
pub struct Story;

// story this player writes for at the current step
fn mine(d: &D, id: &str) -> Option<usize> { let n = d.order.len(); d.order.iter().position(|x| x == id).map(|i| (i + n * MAX_STEPS - d.step) % n) }
fn wrote(d: &D, id: &str) -> bool { mine(d, id).is_some_and(|s| d.stories[s].len() > d.step + 1) }
// every line players wrote, in story order: (story, writer, text)
fn lines(d: &D) -> Vec<(usize, &String, &String)> { d.stories.iter().enumerate().flat_map(|(s, st)| st.iter().skip(1).filter(|(w, _)| !w.is_empty()).map(move |(w, t)| (s, w, t))).collect() }
fn next_step(g: &mut Room) {
    let d = dm!(g, Story);
    for st in d.stories.iter_mut() { if st.len() < d.step + 2 { st.push((String::new(), "…and then nothing happened.".into())) } }
    d.step += 1;
    if d.step < d.steps { return set_timer(g, 50.0, next_step, false) }
    d.show = 0; g.phase = "sshow";
    set_timer(g, 25.0, next_show, false);
}
fn next_show(g: &mut Room) {
    let d = dm!(g, Story);
    d.show += 1;
    if d.show < d.stories.len() { return set_timer(g, 25.0, next_show, false) }
    g.phase = "svote";
    set_timer(g, 30.0, tally, false);
}
fn tally(g: &mut Room) {
    let d = dr!(g, Story);
    let l = lines(d);
    let won: Vec<String> = d.votes.values().filter_map(|i| l.get(*i).map(|x| x.1.clone())).collect();
    for id in won { g.add(&id, 100) }
    finish(g);
}

impl Game for Story {
    fn id(&self) -> &'static str { "story" }
    fn start(&self, g: &mut Room) {
        let order = shuffle(g.ids());
        let starts = shuffle(strs(data("story", "STARTERS")));
        let stories = (0..order.len()).map(|i| vec![(String::new(), starts[i % starts.len()].clone())]).collect();
        g.d = Data::Story(D { steps: order.len().min(MAX_STEPS), stories, order, ..Default::default() });
        g.phase = "swrite";
        set_timer(g, 50.0, next_step, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "swrite" => Some(next_step), "sshow" => Some(next_show), "svote" => Some(tally), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Story);
        if !d.order.iter().any(|x| x == id) { return true }
        match g.phase { "swrite" => wrote(d, id), "svote" => d.votes.contains_key(id), _ => false }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Story(d) = &g.d else { return };
        s.insert("turnNo".into(), json!(d.step));
        s.insert("steps".into(), json!(d.steps));
        if g.phase == "swrite" { // each player sees only the line before theirs
            s.insert("prev".into(), Value::Object(d.order.iter().map(|id| (id.clone(), json!(d.stories[mine(d, id).unwrap()][d.step].1))).collect()));
        }
        let story = |st: &Vec<(String, String)>| json!(st.iter().map(|(w, t)| json!({ "text": t, "name": if w.is_empty() { String::new() } else { g.name(w) } })).collect::<Vec<_>>());
        if g.phase == "sshow" { s.insert("show".into(), json!(d.show)); s.insert("count".into(), json!(d.stories.len())); s.insert("story".into(), story(&d.stories[d.show])); }
        if g.phase == "svote" { s.insert("lines".into(), json!(lines(d).iter().map(|(st, w, t)| json!({ "s": st, "by": w, "text": t })).collect::<Vec<_>>())); }
        if matches!(g.phase, "scores" | "results") && !d.stories.is_empty() {
            let mut v: Vec<Value> = lines(d).iter().enumerate().map(|(i, (_, w, t))| json!({ "text": t, "name": g.name(w),
                "votes": d.votes.values().filter(|x| **x == i).count() })).collect();
            v.sort_by_key(|x| -(x["votes"].as_i64().unwrap_or(0)));
            v.truncate(3);
            s.insert("last".into(), json!({ "list": v }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let d = dm!(g, Story);
        match kind(a) {
            "line" => {
                let text = clean(a.get("text"), 120);
                let Some(st) = mine(d, id).filter(|_| phase == "swrite" && !text.is_empty() && !wrote(d, id)) else { return no(NOT_NOW) };
                d.stories[st].push((id.into(), text));
            }
            "vote" => {
                let l = lines(d);
                let Some(i) = index(a.get("index"), l.len()).filter(|_| phase == "svote") else { return no(NOT_NOW) };
                if l[i].1 == id { return no("Pick someone else's line") }
                d.votes.insert(id.into(), i);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Story);
        match g.phase {
            "swrite" => Some(json!({ "type": "line", "text": pick(&["Suddenly, a goose appeared.", "Nobody expected the soup to talk.", "So they ran.",
                "It was, of course, a trap.", "Then the lights went out.", "Grandma had seen this before."]) })),
            "svote" => { let ok: Vec<usize> = lines(d).iter().enumerate().filter(|(_, l)| l.1 != id).map(|(i, _)| i).collect();
                (!ok.is_empty()).then(|| json!({ "type": "vote", "index": pick(&ok) })) }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lines_pass_along() {
        let mut g = crate::room::test_room(&["a", "b", "c"]);
        Story.start(&mut g);
        for step in 0..3 {
            for id in ["a", "b", "c"] { Story.act(&mut g, &json!({ "type": "line", "text": format!("{id}{step}") }), id).unwrap() }
            Story.advance(g.phase).unwrap()(&mut g);
        }
        assert_eq!(g.phase, "sshow");
        // every story got one line from each player
        for st in &dr!(g, Story).stories { let mut w: Vec<&str> = st[1..].iter().map(|x| x.0.as_str()).collect(); w.sort(); assert_eq!(w, ["a", "b", "c"]) }
    }
}
