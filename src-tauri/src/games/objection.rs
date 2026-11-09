// Objection!: a mock trial over a tiny crime. Arguments happen out loud in the room; phones hold the evidence,
// the objections, the jury vote and the judge's rulings.
// A round: crime -> prosecution (60 s) -> defense (60 s) -> a witness and a Surprise Witness (15 s each)
// -> jury votes -> the judge rules (may overrule the jury once per game, picks a favourite argument) -> sentence.
// The last round (when there's more than one) is the Supreme Court: everyone but the judge is on trial for one
// shared crime, each defends themselves, and the judge ranks them from innocent to guilty.
// Roles rotate every round. Anyone can shout Objection! once per game; the judge sustains or overrules it.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

const WIN: i64 = 200; // winning side
const JUROR: i64 = 50; // juror who voted with the majority
const FAVOURITE: i64 = 100; // judge's favourite argument
const SUSTAINED: i64 = 100; // objection the judge sustains
const RANKED: [i64; 3] = [300, 200, 100]; // Supreme Court, from most innocent

#[derive(Default)]
pub struct D {
    order: Vec<String>, // seating for role rotation, kept across rounds
    supreme: bool,
    crime: String,
    judge: String,
    pros: String,
    def: String,
    cards: IndexMap<String, Vec<String>>, // evidence each speaker has to work in
    marked: IndexMap<String, Vec<bool>>,  // evidence the speaker has ticked off as used
    turns: Vec<(String, &'static str)>, // who speaks, in order: (player, role)
    turn: usize,
    ready: bool,
    rested: bool,
    objection: Option<(String, String, Option<bool>)>, // (by, against, sustained?)
    votes: IndexMap<String, bool>,                     // juror -> guilty?
    ruled: Option<(bool, Option<String>)>,             // (overruled the jury, favourite)
    ranking: Vec<String>,
    verdict: Option<bool>, // final: guilty?
    majority: Option<bool>,
    // once per game, so these carry over from round to round
    objected: Vec<String>,
    overruled: Vec<String>,
}
pub struct Objection;

fn speaker(d: &D) -> Option<&str> { d.turns.get(d.turn).map(|t| t.0.as_str()) }
fn jurors(d: &D, g: &Room) -> Vec<String> {
    g.players.keys().filter(|p| ![&d.judge, &d.pros, &d.def].contains(p)).cloned().collect()
}
fn begin(g: &mut Room) {
    let d = dm!(g, Obj);
    d.turn = 0;
    next_speaker(g, false);
}
// moves to the next speaker (or the vote / ranking when everyone has spoken)
fn next_speaker(g: &mut Room, advance: bool) {
    let d = dm!(g, Obj);
    if advance { d.turn += 1 }
    d.rested = false;
    d.objection = None;
    let Some((_, role)) = d.turns.get(d.turn) else {
        if d.supreme { g.phase = "rank"; return set_timer(g, 45.0, sentence, false) }
        g.phase = "verdict";
        return set_timer(g, 25.0, start_ruling, false);
    };
    let secs = match *role { "prosecution" | "defense" => 60.0, "plea" => 20.0, _ => 15.0 };
    g.phase = "speak";
    set_timer(g, secs, next_turn, false);
}
fn next_turn(g: &mut Room) { next_speaker(g, true) }
fn start_ruling(g: &mut Room) {
    let d = dm!(g, Obj);
    let guilty = d.votes.values().filter(|v| **v).count();
    let not = d.votes.len() - guilty;
    d.majority = if guilty > not { Some(true) } else if not > guilty { Some(false) } else { None };
    g.phase = "ruling";
    set_timer(g, 25.0, sentence, false);
}
fn sentence(g: &mut Room) {
    let d = dm!(g, Obj);
    if d.supreme {
        // anyone the judge didn't get to is ranked last, in speaking order
        for (p, _) in &d.turns { if !d.ranking.contains(p) { d.ranking.push(p.clone()) } }
        for (i, p) in d.ranking.iter().enumerate() { if let (Some(pts), Some(pl)) = (RANKED.get(i), g.players.get_mut(p)) { pl.score += pts } }
    } else {
        let (overrule, fav) = d.ruled.clone().unwrap_or((false, None));
        // a tied jury means not guilty; an overrule flips whatever the jury decided
        let guilty = d.majority.unwrap_or(false) != overrule;
        d.verdict = Some(guilty);
        let winner = if guilty { &d.pros } else { &d.def };
        if let Some(p) = g.players.get_mut(winner) { p.score += WIN }
        if let Some(m) = d.majority { for (j, v) in &d.votes { if *v == m { if let Some(p) = g.players.get_mut(j) { p.score += JUROR } } } }
        if let Some(p) = fav.and_then(|f| g.players.get_mut(&f)) { p.score += FAVOURITE }
    }
    g.phase = "sentence";
    set_timer(g, 9.0, finish, true);
}
fn draw_cards(deck: &mut Vec<String>, n: usize) -> Vec<String> {
    if deck.len() < n { *deck = shuffle(strs(data("objection", "EVIDENCE"))) }
    deck.split_off(deck.len() - n)
}

impl Game for Objection {
    fn id(&self) -> &'static str { "objection" }
    fn start(&self, g: &mut Room) {
        let (order, objected, overruled) = match &mut g.d {
            Data::Obj(d) if g.round > 1 => (std::mem::take(&mut d.order), std::mem::take(&mut d.objected), std::mem::take(&mut d.overruled)),
            _ => Default::default(),
        };
        // real players get the speaking roles first; bots fill in when there aren't enough people
        let mut order: Vec<String> = order.into_iter().filter(|p| g.players.contains_key(p)).collect();
        let (real, bots): (Vec<String>, Vec<String>) = g.ids().into_iter().filter(|p| !order.contains(p)).partition(|p| !g.players[p].bot);
        order.extend(shuffle(real)); order.extend(shuffle(bots));
        let n = order.len().max(1);
        let r = (g.round - 1).max(0) as usize;
        let rounds = g.set_or("rounds", 3.0);
        let supreme = rounds > 1.0 && g.round as f64 >= rounds;
        let seat = |k: usize| order.get((r + k) % n).cloned().unwrap_or_default();
        let (judge, pros, def) = (seat(0), seat(1), seat(2));
        let mut deck = shuffle(strs(data("objection", "EVIDENCE")));
        let mut cards = IndexMap::new();
        let mut turns = vec![];
        let crime;
        if supreme {
            crime = pick(&strs(data("objection", "SUPREME")));
            for p in order.iter().filter(|p| **p != judge) { cards.insert(p.clone(), draw_cards(&mut deck, 1)); turns.push((p.clone(), "plea")); }
        } else {
            let packs = data("objection", "CRIMES");
            let all = strs(packs.get(g.set_str("pack")).unwrap_or(&packs["classic"]));
            let fresh: Vec<String> = all.iter().filter(|c| !g.used.contains(c)).cloned().collect();
            crime = pick(if fresh.is_empty() { &all } else { &fresh });
            g.used.push(crime.clone());
            cards.insert(pros.clone(), draw_cards(&mut deck, 3));
            cards.insert(def.clone(), draw_cards(&mut deck, 3));
            turns.push((pros.clone(), "prosecution"));
            turns.push((def.clone(), "defense"));
            // a witness, then a Surprise Witness who must defend the accused with a random card
            let mut js = shuffle(order.iter().filter(|p| ![&judge, &pros, &def].contains(p)).cloned().collect::<Vec<_>>());
            if let Some(w) = js.pop() { turns.push((w, "witness")) }
            if let Some(w) = js.pop() { cards.insert(w.clone(), draw_cards(&mut deck, 1)); turns.push((w, "surprise")) }
        }
        let marked = cards.iter().map(|(p, c)| (p.clone(), vec![false; c.len()])).collect();
        g.d = Data::Obj(D { order, supreme, crime, judge, pros, def, cards, marked, turns, objected, overruled, ..Default::default() });
        g.phase = "crime";
        set_timer(g, 15.0, begin, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase {
            "crime" => Some(begin), "speak" => Some(next_turn), "verdict" => Some(start_ruling),
            "ruling" | "rank" => Some(sentence), "sentence" => Some(finish), _ => None,
        }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Obj);
        match g.phase {
            "crime" => id != d.judge || d.ready, // the judge opens court
            "speak" => speaker(d) != Some(id) || d.rested,
            "verdict" => !jurors(d, g).iter().any(|j| j == id) || d.votes.contains_key(id),
            "ruling" => id != d.judge || d.ruled.is_some(),
            "rank" => id != d.judge || !d.ranking.is_empty(),
            _ => false,
        }
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Obj(d) = &g.d else { return };
        let turn = d.turns.get(d.turn).map(|(p, role)| json!({ "id": p, "role": role }));
        s.insert("view".into(), json!(format!("{}:{}:{}", d.turn, d.objection.as_ref().map_or("-", |o| match o.2 { None => "?", Some(true) => "y", Some(false) => "n" }), d.marked.values().flatten().filter(|m| **m).count())));
        s.insert("trial".into(), json!({
            "supreme": d.supreme, "crime": d.crime, "judge": d.judge, "pros": d.pros, "def": d.def,
            "cards": d.cards, "marked": d.marked, "turn": turn, "turnNo": d.turn + 1, "turnCount": d.turns.len(),
            "turns": d.turns.iter().map(|(p, r)| json!({ "id": p, "role": r })).collect::<Vec<_>>(),
            "objection": d.objection.as_ref().map(|(by, against, ruled)| json!({ "by": by, "against": against, "sustained": ruled })),
            "objected": d.objected, "canOverrule": !d.overruled.contains(&d.judge),
            "jurors": jurors(d, g),
        }));
        if g.phase == "ruling" {
            s.insert("tally".into(), json!({ "guilty": d.votes.values().filter(|v| **v).count(), "not": d.votes.values().filter(|v| !**v).count(), "majority": d.majority }));
        }
        if matches!(g.phase, "sentence" | "scores" | "results") {
            s.insert("last".into(), json!({
                "supreme": d.supreme, "crime": d.crime, "guilty": d.verdict, "majority": d.majority,
                "overruled": d.ruled.as_ref().is_some_and(|r| r.0), "favourite": d.ruled.as_ref().and_then(|r| r.1.as_ref()).map(|f| g.name(f)),
                "pros": g.name(&d.pros), "def": g.name(&d.def), "judge": g.name(&d.judge),
                "ranking": d.ranking.iter().map(|p| json!({ "id": p, "name": g.name(p) })).collect::<Vec<_>>(),
            }));
        }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let phase = g.phase;
        let juror = { let d = dr!(g, Obj); jurors(d, g).iter().any(|j| j == id) };
        let d = dm!(g, Obj);
        let judge = d.judge == id;
        match kind(a) {
            "open" => { if phase != "crime" || !judge { return no(NOT_NOW) } d.ready = true; }
            "rest" => { if phase != "speak" || speaker(d) != Some(id) { return no(NOT_NOW) } d.rested = true; }
            "mark" => { // the speaker ticks off a piece of evidence they've worked in
                let Some(m) = d.marked.get_mut(id) else { return no(NOT_NOW) };
                let Some(i) = index(a.get("index"), m.len()) else { return no(NOT_NOW) };
                m[i] = !m[i];
            }
            "object" => {
                let Some(sp) = speaker(d).map(String::from).filter(|_| phase == "speak") else { return no(NOT_NOW) };
                if sp == id || judge { return no("You can't object right now") }
                if d.objected.iter().any(|x| x == id) { return no("You already used your objection") }
                if d.objection.as_ref().is_some_and(|o| o.2.is_none()) { return no("The judge is still ruling on one") }
                d.objected.push(id.into());
                d.objection = Some((id.into(), sp, None));
            }
            "rule" => { // judge on an objection
                let Some(o) = d.objection.as_mut().filter(|o| judge && o.2.is_none()) else { return no(NOT_NOW) };
                let sustain = a.get("sustain").and_then(|v| v.as_bool()).unwrap_or(false);
                o.2 = Some(sustain);
                let by = o.0.clone();
                if sustain { g.add(&by, SUSTAINED) }
            }
            "vote" => {
                if phase != "verdict" || !juror { return no(NOT_NOW) }
                let Some(guilty) = a.get("guilty").and_then(|v| v.as_bool()) else { return no("Guilty or not guilty?") };
                d.votes.insert(id.into(), guilty);
            }
            "ruling" => {
                if phase != "ruling" || !judge || d.ruled.is_some() { return no(NOT_NOW) }
                let overrule = a.get("overrule").and_then(|v| v.as_bool()).unwrap_or(false);
                if overrule && d.overruled.iter().any(|x| x == id) { return no("You already overruled a jury this game") }
                let fav = Some(clean(a.get("fav"), 40)).filter(|f| d.turns.iter().any(|(p, _)| p == f));
                if fav.is_none() { return no("Pick your favourite argument") }
                if overrule { d.overruled.push(id.into()) }
                d.ruled = Some((overrule, fav));
            }
            "rank" => {
                if phase != "rank" || !judge { return no(NOT_NOW) }
                let order: Vec<String> = a.get("order").and_then(|v| v.as_array()).map(|o| o.iter().map(|x| clean(Some(x), 40)).collect()).unwrap_or_default();
                let mut want: Vec<&String> = d.turns.iter().map(|t| &t.0).collect();
                let mut got: Vec<&String> = order.iter().collect();
                want.sort(); got.sort();
                if want != got { return no("Rank everyone on trial") }
                d.ranking = order;
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Obj);
        match g.phase {
            "crime" if d.judge == id => Some(json!({ "type": "open" })),
            "speak" if speaker(d) == Some(id) => Some(json!({ "type": "rest" })),
            "speak" if d.judge == id && d.objection.as_ref().is_some_and(|o| o.2.is_none()) => Some(json!({ "type": "rule", "sustain": rnd() < 0.5 })),
            "verdict" => Some(json!({ "type": "vote", "guilty": rnd() < 0.5 })),
            "ruling" if d.judge == id => Some(json!({ "type": "ruling", "overrule": rnd() < 0.15 && !d.overruled.iter().any(|x| x == id),
                "fav": pick(&d.turns.iter().map(|t| t.0.clone()).collect::<Vec<_>>()) })),
            "rank" if d.judge == id => Some(json!({ "type": "rank", "order": shuffle(d.turns.iter().map(|t| t.0.clone()).collect::<Vec<_>>()) })),
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
    fn trial_then_supreme_court() {
        let mut g = test_room(&["a", "b", "c", "d", "e"]);
        g.settings.insert("rounds".into(), json!(2));
        Objection.start(&mut g);
        let (judge, pros, def) = { let d = dr!(g, Obj); (d.judge.clone(), d.pros.clone(), d.def.clone()) };
        let jury: Vec<String> = g.ids().into_iter().filter(|p| ![&judge, &pros, &def].contains(&p)).collect();
        assert_eq!(dr!(g, Obj).cards[&pros].len(), 3);
        act(&mut g, &judge, json!({ "type": "open" })).unwrap();
        assert_eq!(g.phase, "speak");
        // a juror objects to the prosecutor once; the judge sustains it
        assert!(act(&mut g, &judge, json!({ "type": "object" })).is_err());
        act(&mut g, &jury[0], json!({ "type": "object" })).unwrap();
        act(&mut g, &judge, json!({ "type": "rule", "sustain": true })).unwrap();
        assert!(act(&mut g, &jury[0], json!({ "type": "object" })).is_err()); // only once per game
        act(&mut g, &pros, json!({ "type": "mark", "index": 1 })).unwrap();
        assert!(dr!(g, Obj).marked[&pros][1]);
        // everyone rests in turn: prosecution, defense, witness, surprise witness
        while g.phase == "speak" { let sp = speaker(dr!(g, Obj)).unwrap().to_string(); act(&mut g, &sp, json!({ "type": "rest" })).unwrap(); }
        assert_eq!(dr!(g, Obj).turns.len(), 4);
        assert_eq!(g.phase, "verdict");
        for j in &jury { act(&mut g, j, json!({ "type": "vote", "guilty": true })).unwrap(); }
        assert_eq!(g.phase, "ruling");
        // the judge overrules the guilty jury and likes the defense best
        act(&mut g, &judge, json!({ "type": "ruling", "overrule": true, "fav": def })).unwrap();
        assert_eq!(g.phase, "sentence");
        assert_eq!(dr!(g, Obj).verdict, Some(false));
        assert_eq!(g.players[&def].score, WIN + FAVOURITE);
        assert_eq!(g.players[&jury[0]].score, SUSTAINED + JUROR);
        // round 2 is the Supreme Court: everyone but the new judge pleads, then gets ranked
        g.round = 2;
        Objection.start(&mut g);
        let d = dr!(g, Obj);
        assert!(d.supreme && d.turns.len() == 4 && d.judge == pros);
        assert!(d.objected.contains(&jury[0])); // objections stay used
        let judge = d.judge.clone();
        let order: Vec<String> = d.turns.iter().map(|t| t.0.clone()).collect();
        act(&mut g, &judge, json!({ "type": "open" })).unwrap();
        while g.phase == "speak" { let sp = speaker(dr!(g, Obj)).unwrap().to_string(); act(&mut g, &sp, json!({ "type": "rest" })).unwrap(); }
        assert_eq!(g.phase, "rank");
        assert!(act(&mut g, &judge, json!({ "type": "rank", "order": [order[0]] })).is_err());
        let before = g.players[&order[3]].score;
        act(&mut g, &judge, json!({ "type": "rank", "order": [order[3], order[2], order[1], order[0]] })).unwrap();
        assert_eq!(g.phase, "sentence");
        assert_eq!(g.players[&order[3]].score, before + 300);
    }
}
