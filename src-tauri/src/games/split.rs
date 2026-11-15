// Split the Room: everyone writes a yes/no question meant to split the room 50/50, then answers everyone else's.
// The closer a question comes to an even split, the more its writer scores.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

#[derive(Default)]
pub struct D { qs: IndexMap<String, String>, list: Vec<(String, String)>, answers: IndexMap<String, Vec<Option<bool>>> } // list: (writer, question)
pub struct Split;

// (yes, total) for question i; the writer's own answer never counts
fn count(d: &D, i: usize) -> (usize, usize) {
    let v: Vec<bool> = d.answers.iter().filter(|(id, _)| **id != d.list[i].0).filter_map(|(_, a)| a.get(i).copied().flatten()).collect();
    (v.iter().filter(|x| **x).count(), v.len())
}
// 500 for a perfect 50/50, 10 fewer per point off, zero for a one-sided room
fn pts(yes: usize, n: usize) -> i64 { if n < 2 { 0 } else { (500 - 10 * (round(yes as f64 / n as f64 * 100.0) as i64 - 50).abs()).max(0) } }
fn start_answer(g: &mut Room) {
    let d = dm!(g, Split);
    d.list = shuffle(d.qs.iter().map(|(o, t)| (o.clone(), t.clone())).collect());
    if d.list.len() < 2 { return tally(g) }
    let n = d.list.len() as f64;
    g.phase = "split";
    set_timer(g, 20.0 + 6.0 * n, tally, false);
}
fn tally(g: &mut Room) {
    let d = dm!(g, Split);
    let won: Vec<(String, i64)> = (0..d.list.len()).map(|i| { let (y, n) = count(d, i); (d.list[i].0.clone(), pts(y, n)) }).collect();
    for (id, p) in won { g.add(&id, p) }
    finish(g);
}

impl Game for Split {
    fn id(&self) -> &'static str { "split" }
    fn start(&self, g: &mut Room) {
        g.phase = "ask";
        g.d = Data::Split(D::default());
        set_timer(g, 60.0, start_answer, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "ask" => Some(start_answer), "split" => Some(tally), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Split);
        match g.phase { "ask" => d.qs.contains_key(id), "split" => d.answers.contains_key(id), _ => false }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Split(d) = &g.d else { return };
        // "by" lets each phone skip its own question; the TV doesn't show it until the results
        if g.phase == "split" { s.insert("qs".into(), json!(d.list.iter().map(|(o, t)| json!({ "text": t, "by": o })).collect::<Vec<_>>())); }
        if matches!(g.phase, "scores" | "results") && !d.list.is_empty() {
            let mut v: Vec<Value> = (0..d.list.len()).map(|i| { let (y, n) = count(d, i);
                json!({ "text": d.list[i].1, "id": d.list[i].0, "yes": y, "n": n, "pts": pts(y, n) }) }).collect();
            v.sort_by_key(|x| -x["pts"].as_i64().unwrap_or(0));
            s.insert("last".into(), json!({ "list": v }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let d = dm!(g, Split);
        match kind(a) {
            "ask" => {
                let text = clean(a.get("text"), 90);
                if phase != "ask" || text.is_empty() { return no(NOT_NOW) }
                d.qs.insert(id.into(), text);
            }
            "split" => {
                if phase != "split" { return no(NOT_NOW) }
                let Some(arr) = a.get("answers").and_then(|v| v.as_array()) else { return no("Answer the questions") };
                let ans: Vec<Option<bool>> = (0..d.list.len()).map(|i| if d.list[i].0 == id { None } else { arr.get(i).and_then(|v| v.as_bool()) }).collect();
                if (0..d.list.len()).any(|i| d.list[i].0 != id && ans[i].is_none()) { return no("Answer every question") }
                d.answers.insert(id.into(), ans);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, _: &str) -> Option<Value> {
        match g.phase {
            "ask" => Some(json!({ "type": "ask", "text": pick(&["Is a hot dog a sandwich?", "Do you sleep with socks on?", "Pineapple on pizza?",
                "Would you go to space?", "Is cereal a soup?", "Do you talk to your plants?"]) })),
            "split" => Some(json!({ "type": "split", "answers": (0..dr!(g, Split).list.len()).map(|_| rnd() < 0.5).collect::<Vec<_>>() })),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn even_split_scores_most() {
        assert_eq!(pts(1, 2), 500);
        assert_eq!(pts(2, 2), 0);
        assert_eq!(pts(1, 1), 0);
        let mut g = crate::room::test_room(&["a", "b", "c"]);
        Split.start(&mut g);
        for id in ["a", "b", "c"] { Split.act(&mut g, &json!({ "type": "ask", "text": format!("q {id}?") }), id).unwrap() }
        Split.advance(g.phase).unwrap()(&mut g);
        assert_eq!(g.phase, "split");
        // b says yes and c says no to a's question; every other question gets yes from everyone
        let list: Vec<String> = dr!(g, Split).list.iter().map(|q| q.0.clone()).collect();
        for id in ["a", "b", "c"] {
            let v: Vec<bool> = list.iter().map(|w| if w == "a" { id == "b" } else { true }).collect();
            Split.act(&mut g, &json!({ "type": "split", "answers": v }), id).unwrap();
        }
        Split.advance(g.phase).unwrap()(&mut g);
        assert_eq!([g.players["a"].score, g.players["b"].score], [500, 0]);
    }
}
