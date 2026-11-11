// Objection!: one player is accused of a tiny crime, another prosecutes. Each argues out loud for 45 s using
// two evidence cards, then everyone else votes guilty or not guilty. Roles rotate every round.
// Jurors can shout Objection! once per speech for laughs (it just flashes on the TV).
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

const WIN: i64 = 200; // the side the jury backs
const JUROR: i64 = 50; // juror who voted with the majority
const TALK: f64 = 45.0;

#[derive(Default)]
pub struct D {
    order: Vec<String>, // seating for role rotation, kept across rounds
    crime: String,
    pros: String,
    accused: String,
    cards: IndexMap<String, Vec<String>>,
    turn: usize, // 0 prosecutor talks, 1 accused talks
    done: bool,
    objection: Option<String>, // who just objected
    objected: Vec<String>,      // jurors who objected this speech
    votes: IndexMap<String, bool>, // juror -> guilty?
    verdict: Option<bool>,
}
pub struct Objection;

fn speaker(d: &D) -> &str { if d.turn == 0 { &d.pros } else { &d.accused } }
fn jurors(d: &D, g: &Room) -> Vec<String> { g.players.keys().filter(|p| **p != d.pros && **p != d.accused).cloned().collect() }
fn talk(g: &mut Room) {
    let d = dm!(g, Obj);
    d.done = false;
    d.objection = None;
    d.objected.clear();
    g.phase = "speak";
    set_timer(g, TALK, next_turn, false);
}
fn next_turn(g: &mut Room) {
    let d = dm!(g, Obj);
    if d.turn == 0 { d.turn = 1; return talk(g) }
    g.phase = "vote";
    set_timer(g, 20.0, verdict, false);
}
fn verdict(g: &mut Room) {
    let d = dm!(g, Obj);
    let guilty = d.votes.values().filter(|v| **v).count();
    let is_guilty = guilty * 2 > d.votes.len(); // a tie is not guilty
    d.verdict = Some(is_guilty);
    let winner = if is_guilty { d.pros.clone() } else { d.accused.clone() };
    let right: Vec<String> = d.votes.iter().filter(|(_, v)| **v == is_guilty).map(|(j, _)| j.clone()).collect();
    g.add(&winner, WIN);
    for j in right { g.add(&j, JUROR) }
    g.phase = "sentence";
    set_timer(g, 9.0, finish, true);
}

impl Game for Objection {
    fn id(&self) -> &'static str { "objection" }
    fn start(&self, g: &mut Room) {
        let order = match &mut g.d { Data::Obj(d) if g.round > 1 => std::mem::take(&mut d.order), _ => vec![] };
        // real players get the speaking roles first; bots fill in when there aren't enough people
        let mut order: Vec<String> = order.into_iter().filter(|p| g.players.contains_key(p)).collect();
        let (real, bots): (Vec<String>, Vec<String>) = g.ids().into_iter().filter(|p| !order.contains(p)).partition(|p| !g.players[p].bot);
        order.extend(shuffle(real)); order.extend(shuffle(bots));
        let n = order.len().max(1);
        let r = (g.round - 1).max(0) as usize * 2;
        let seat = |k: usize| order.get((r + k) % n).cloned().unwrap_or_default();
        let (pros, accused) = (seat(0), seat(1));
        let packs = data("objection", "CRIMES");
        let all = strs(packs.get(g.set_str("pack")).unwrap_or(&packs["classic"]));
        let fresh: Vec<String> = all.iter().filter(|c| !g.used.contains(c)).cloned().collect();
        let crime = pick(if fresh.is_empty() { &all } else { &fresh });
        g.used.push(crime.clone());
        let mut deck = shuffle(strs(data("objection", "EVIDENCE")));
        let mut cards = IndexMap::new();
        cards.insert(pros.clone(), deck.split_off(deck.len().saturating_sub(2)));
        cards.insert(accused.clone(), deck.split_off(deck.len().saturating_sub(2)));
        g.d = Data::Obj(D { order, crime, pros, accused, cards, ..Default::default() });
        g.phase = "crime";
        set_timer(g, 20.0, talk, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "crime" => Some(talk), "speak" => Some(next_turn), "vote" => Some(verdict), "sentence" => Some(finish), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Obj);
        match g.phase {
            "speak" => speaker(d) != id || d.done,
            "vote" => !jurors(d, g).iter().any(|j| j == id) || d.votes.contains_key(id),
            _ => false, // the crime screen waits for its timer or the host so everyone can read it
        }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Obj(d) = &g.d else { return };
        s.insert("view".into(), json!(format!("{}:{}", d.turn, d.objection.as_deref().unwrap_or("-"))));
        s.insert("trial".into(), json!({
            "crime": d.crime, "pros": d.pros, "accused": d.accused, "cards": d.cards,
            "speaker": speaker(d), "turnNo": d.turn + 1, "objection": d.objection, "objected": d.objected,
            "jurors": jurors(d, g),
        }));
        if matches!(g.phase, "sentence" | "scores" | "results") {
            s.insert("last".into(), json!({
                "crime": d.crime, "guilty": d.verdict, "pros": g.name(&d.pros), "accused": g.name(&d.accused),
                "guiltyVotes": d.votes.values().filter(|v| **v).count(), "notVotes": d.votes.values().filter(|v| !**v).count(),
            }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let juror = { let d = dr!(g, Obj); jurors(d, g).iter().any(|j| j == id) };
        let d = dm!(g, Obj);
        match kind(a) {
            "rest" => { if phase != "speak" || speaker(d) != id { return no(NOT_NOW) } d.done = true; }
            "object" => {
                if phase != "speak" || !juror { return no(NOT_NOW) }
                if d.objected.iter().any(|x| x == id) { return no("You already objected to this speech") }
                d.objected.push(id.into());
                d.objection = Some(id.into());
            }
            "vote" => {
                if phase != "vote" || !juror { return no(NOT_NOW) }
                let Some(guilty) = a.get("guilty").and_then(|v| v.as_bool()) else { return no("Guilty or not guilty?") };
                d.votes.insert(id.into(), guilty);
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Obj);
        match g.phase {
            "speak" if speaker(d) == id => Some(json!({ "type": "rest" })),
            "vote" => Some(json!({ "type": "vote", "guilty": rnd() < 0.5 })),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::test_room;
    fn act(g: &mut Room, id: &str, a: Value) -> Res {
        let r = Objection.act(g, &a, id);
        if r.is_ok() && g.players.keys().all(|p| Objection.is_done(g, p)) { if let Some(f) = Objection.advance(g.phase) { f(g) } }
        r
    }
    #[test]
    fn trial() {
        let mut g = test_room(&["a", "b", "c", "d", "e"]);
        Objection.start(&mut g);
        let (pros, acc) = { let d = dr!(g, Obj); (d.pros.clone(), d.accused.clone()) };
        let jury: Vec<String> = g.ids().into_iter().filter(|p| *p != pros && *p != acc).collect();
        assert_eq!(dr!(g, Obj).cards[&acc].len(), 2);
        talk(&mut g);
        assert!(act(&mut g, &acc, json!({ "type": "rest" })).is_err()); // not their turn yet
        act(&mut g, &jury[0], json!({ "type": "object" })).unwrap();
        assert!(act(&mut g, &jury[0], json!({ "type": "object" })).is_err()); // once per speech
        assert!(act(&mut g, &pros, json!({ "type": "object" })).is_err()); // speakers can't object
        act(&mut g, &pros, json!({ "type": "rest" })).unwrap();
        act(&mut g, &jury[0], json!({ "type": "object" })).unwrap(); // fresh speech, fresh objection
        act(&mut g, &acc, json!({ "type": "rest" })).unwrap();
        assert_eq!(g.phase, "vote");
        act(&mut g, &jury[0], json!({ "type": "vote", "guilty": true })).unwrap();
        act(&mut g, &jury[1], json!({ "type": "vote", "guilty": true })).unwrap();
        act(&mut g, &jury[2], json!({ "type": "vote", "guilty": false })).unwrap();
        assert_eq!(g.phase, "sentence");
        assert_eq!(dr!(g, Obj).verdict, Some(true));
        assert_eq!(g.players[&pros].score, WIN);
        assert_eq!(g.players[&acc].score, 0);
        assert_eq!(g.players[&jury[0]].score, JUROR);
        assert_eq!(g.players[&jury[2]].score, 0);
        // next round, new speakers
        g.round = 2;
        Objection.start(&mut g);
        let d = dr!(g, Obj);
        assert!(d.pros != pros && d.accused != acc && d.pros != d.accused);
    }
}
