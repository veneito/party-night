// Elemix: a shedding card game. Match the top card's element or value; first to empty their hand wins the round
// and scores the cards left in everyone else's hand. Elements: 0 fire, 1 water, 2 leaf, 3 moon; 4 = wild.
// Values: 0-9, 'freeze' (skip next), 'twist' (reverse), 'gift' (next draws 2), 'prism' (wild), 'storm' (wild, next draws 4).
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

const HAND: usize = 7;
const ELEMENTS: [&str; 4] = ["fire", "water", "leaf", "moon"];
#[derive(Clone, serde::Serialize)]
pub struct Card { e: u8, v: Value }
#[derive(Default)]
pub struct D { order: Vec<String>, hands: IndexMap<String, Vec<Card>>, dir: i64, turn: usize, turn_no: i64, winner: Option<String>, won: i64,
    stock: Vec<Card>, pile: Vec<Card>, elem: u8, log: String }
pub struct Elemix;

fn deck(n: usize) -> Vec<Card> { // one deck per 6 players so big rooms don't run dry
    let mut d = vec![];
    for _ in 0..n {
        for e in 0..4u8 {
            d.push(Card { e, v: json!(0) });
            for v in [json!(1), json!(2), json!(3), json!(4), json!(5), json!(6), json!(7), json!(8), json!(9), json!("freeze"), json!("twist"), json!("gift")] {
                d.push(Card { e, v: v.clone() }); d.push(Card { e, v });
            }
        }
        for _ in 0..4 { d.push(Card { e: 4, v: json!("prism") }); d.push(Card { e: 4, v: json!("storm") }); }
    }
    shuffle(d)
}
fn fits(d: &D, c: &Card) -> bool { c.e == 4 || c.e == d.elem || d.pile.last().is_some_and(|t| t.v == c.v) }
fn draw(d: &mut D, id: &str, n: usize) {
    for _ in 0..n {
        if d.stock.is_empty() { if let Some(t) = d.pile.pop() { d.stock = shuffle(std::mem::take(&mut d.pile)); d.pile = vec![t] } }
        let Some(c) = d.stock.pop() else { return }; // every card is in someone's hand
        if let Some(h) = d.hands.get_mut(id) { h.push(c) }
    }
}
fn cur(d: &D) -> &str { &d.order[d.turn] }
fn step(d: &mut D, n: i64) { let k = d.order.len() as i64; d.turn = (((d.turn as i64 + d.dir * n) % k + k) % k) as usize }
fn skip_gone(d: &mut D, g: &IndexMap<String, Player>) { // skip players who left
    for _ in 0..d.order.len() { if g.contains_key(cur(d)) { break } step(d, 1) }
}
fn next_turn(g: &mut Room, n: i64) {
    let d = dm!(g, Elem);
    step(d, n);
    skip_gone(d, &g.players);
    d.turn_no += 1;
    set_timer(g, 25.0, timeout, false);
}
fn timeout(g: &mut Room) {
    let d = dm!(g, Elem);
    let c = cur(d).to_string();
    draw(d, &c, 1);
    d.log = format!("{} ran out of time and drew", g.players.get(&c).map_or("undefined", |p| &p.name));
    next_turn(g, 1);
}
fn vs(v: &Value) -> String { v.as_str().map_or_else(|| v.to_string(), String::from) }
fn play(g: &mut Room, id: &str, i: usize, elem: u8) {
    let d = dm!(g, Elem);
    let c = d.hands[id].remove(i);
    d.pile.push(c.clone());
    d.elem = if c.e == 4 { elem } else { c.e };
    d.log = format!("{} played {}", g.players[id].name, if c.e == 4 { vs(&c.v) } else { format!("{} {}", ELEMENTS[c.e as usize], vs(&c.v)) });
    if d.hands[id].is_empty() { return win(g, id) }
    let live = d.order.iter().filter(|x| g.players.contains_key(*x)).count();
    match c.v.as_str() {
        Some("twist") => { d.dir *= -1; if live == 2 { return next_turn(g, 2) } } // with two players, twist acts like freeze
        Some("freeze") => return next_turn(g, 2),
        Some(k @ ("gift" | "storm")) => {
            let n = if k == "gift" { 2 } else { 4 };
            step(d, 1); skip_gone(d, &g.players);
            let who = cur(d).to_string();
            draw(d, &who, n);
            d.log += &format!(" · {} draws {n} and is skipped", g.players[&who].name);
            return next_turn(g, 1);
        }
        _ => {}
    }
    next_turn(g, 1);
}
fn win(g: &mut Room, id: &str) {
    let d = dm!(g, Elem);
    let pts: i64 = d.hands.iter().filter(|(pid, _)| *pid != id).flat_map(|(_, h)| h)
        .map(|c| c.v.as_i64().unwrap_or(if matches!(c.v.as_str(), Some("prism" | "storm")) { 50 } else { 20 })).sum();
    d.winner = Some(id.into());
    d.won = pts * 10;
    let won = d.won;
    g.add(id, won);
    finish(g);
}

impl Game for Elemix {
    fn id(&self) -> &'static str { "elemix" }
    fn start(&self, g: &mut Room) {
        let order = shuffle(g.ids());
        let mut d = D { stock: deck(order.len().div_ceil(6)), dir: 1, ..Default::default() };
        for id in &order { d.hands.insert(id.clone(), vec![]); draw(&mut d, id, HAND) }
        d.order = order;
        // start on a plain number
        while let Some(c) = d.stock.pop() { if c.e == 4 || !c.v.is_number() { d.stock.insert(0, c) } else { d.elem = c.e; d.pile = vec![c]; break } }
        g.d = Data::Elem(d);
        g.phase = "turn";
        set_timer(g, 25.0, timeout, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> { (phase == "turn").then_some(timeout as Next) }
    fn is_done(&self, g: &Room, id: &str) -> bool { g.phase == "turn" && cur(dr!(g, Elem)) != id }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Elem(d) = &g.d else { return };
        if g.phase == "turn" {
            // Note: hands ride in the shared broadcast (a peek in devtools shows them), fine among friends, same as the other games' secrets.
            s.insert("hands".into(), json!(d.hands));
            s.insert("top".into(), json!(d.pile.last()));
            s.insert("recent".into(), json!(d.pile[d.pile.len().saturating_sub(4)..]));
            s.insert("elem".into(), json!(d.elem));
            s.insert("cur".into(), json!(cur(d)));
            s.insert("dir".into(), json!(d.dir));
            s.insert("turnNo".into(), json!(d.turn_no));
            s.insert("log".into(), json!(d.log));
            s.insert("stock".into(), json!(d.stock.len()));
            s.insert("order".into(), json!(d.order.iter().filter(|id| g.players.contains_key(*id)).collect::<Vec<_>>()));
            s.insert("playable".into(), json!(d.hands.iter().map(|(id, h)| (id, h.iter().map(|c| fits(d, c)).collect::<Vec<_>>())).collect::<IndexMap<_, _>>()));
        }
        if matches!(g.phase, "scores" | "results") { if let Some(w) = &d.winner {
            s.insert("last".into(), json!({ "winner": g.name(w), "won": d.won, "left": d.order.iter().filter(|id| *id != w && g.players.contains_key(*id))
                .map(|id| json!({ "name": g.name(id), "n": d.hands[id].len() })).collect::<Vec<_>>() }));
        } }
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        let d = dm!(g, Elem);
        if g.phase != "turn" || cur(d) != id { return no("It's not your turn") }
        match kind(a) {
            "draw" => {
                draw(d, id, 1);
                d.log = format!("{} drew a card", g.players[id].name);
                next_turn(g, 1);
            }
            "play" => {
                let Some(i) = index(a.get("i"), d.hands[id].len()) else { return no("Pick a card") };
                let c = &d.hands[id][i];
                if !fits(d, c) { return no("That card doesn't match") }
                let elem = num(a.get("elem"));
                if c.e == 4 && ![0.0, 1.0, 2.0, 3.0].contains(&elem) { return no("Pick an element") }
                play(g, id, i, if elem.is_nan() { 0 } else { elem as u8 });
            }
            _ => return no("Unknown action"),
        }
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Elem);
        if g.phase != "turn" || cur(d) != id { return None }
        let hand = &d.hands[id];
        let Some(i) = hand.iter().position(|c| fits(d, c)) else { return Some(json!({ "type": "draw" })) };
        let mut count = [0; 4];
        for c in hand { if c.e < 4 { count[c.e as usize] += 1 } }
        let top = *count.iter().max().unwrap();
        Some(json!({ "type": "play", "i": i, "elem": count.iter().position(|x| *x == top).unwrap() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::test_room;
    #[test]
    fn rules_and_bots() {
        let mut g = test_room(&["a", "b", "c"]);
        let m = Elemix;
        m.start(&mut g);
        let d = dr!(g, Elem);
        assert_eq!(d.stock.len() + d.pile.len() + 21, 108);
        assert_eq!(d.hands["a"].len(), 7);
        assert!(d.pile[0].v.is_number());
        // rig a known table
        let d = dm!(g, Elem);
        d.order = vec!["a".into(), "b".into(), "c".into()];
        d.pile = vec![Card { e: 0, v: json!(5) }]; d.elem = 0; d.turn = 0; d.dir = 1;
        let c = |e, v: Value| Card { e, v };
        d.hands = IndexMap::from([("a".into(), vec![c(1, json!("freeze")), c(0, json!("freeze")), c(0, json!(2))]), ("b".into(), vec![c(3, json!(1))]),
            ("c".into(), vec![c(4, json!("storm")), c(2, json!(9))])]);
        let play = |g: &mut Room, id: &str, a: Value| m.act(g, &a, id);
        assert!(play(&mut g, "a", json!({ "type": "play", "i": 0 })).is_err()); // water freeze doesn't match fire 5
        assert!(play(&mut g, "b", json!({ "type": "play", "i": 2 })).is_err()); // not b's turn
        assert!(play(&mut g, "a", json!({ "type": "play", "i": 1 })).is_ok()); // fire freeze: skips b
        assert_eq!(cur(dr!(g, Elem)), "c");
        assert!(play(&mut g, "c", json!({ "type": "play", "i": 0, "elem": 9 })).is_err()); // wild needs an element
        assert!(play(&mut g, "c", json!({ "type": "play", "i": 0, "elem": 1 })).is_ok()); // storm, water: a draws 4, skipped
        let d = dr!(g, Elem);
        assert_eq!((d.hands["a"].len(), d.elem, cur(d)), (6, 1, "b"));
        assert!(play(&mut g, "b", json!({ "type": "play", "i": 0 })).is_err()); // moon 1 doesn't match water storm
        assert!(play(&mut g, "b", json!({ "type": "draw" })).is_ok());
        assert_eq!(cur(dr!(g, Elem)), "c");
        dm!(g, Elem).hands["c"] = vec![c(1, json!(3))];
        assert!(play(&mut g, "c", json!({ "type": "play", "i": 0 })).is_ok()); // c empties hand and wins
        assert_eq!(g.phase, "results");
        assert!(g.players["c"].score > 0);
        // a bot always makes a legal move
        m.start(&mut g);
        for _ in 0..500 {
            if g.phase != "turn" { break }
            let id = cur(dr!(g, Elem)).to_string();
            let mv = m.bot(&g, &id).unwrap();
            assert!(m.act(&mut g, &mv, &id).is_ok());
        }
    }
}
