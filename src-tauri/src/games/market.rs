// Market Day: five goods, everyone starts with 100 coins. Each turn there's a rumour (true about two times in three);
// everyone makes one trade at once: buy some of a good, or sell all of one. Then prices move: what the room bought
// goes up, what it sold goes down, and the rumour comes true or doesn't. After the last turn everything is sold.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

const TURNS: usize = 6;
const GOODS: [(&str, &str, i64); 5] = [("🐟", "Fish", 8), ("🧀", "Cheese", 15), ("🍯", "Honey", 25), ("🧶", "Wool", 12), ("💎", "Gems", 40)];
#[derive(Clone)]
struct Wallet { coins: i64, hold: [i64; 5] }
#[derive(Default)]
pub struct D { price: [i64; 5], prev: [i64; 5], turn: usize, rumour: (usize, bool), news: String,
    wallets: IndexMap<String, Wallet>, orders: IndexMap<String, (usize, i64)> } // order: (good, +buy qty / -1 sell all / 0 hold)
pub struct Market;

fn wallet<'a>(d: &'a mut D, id: &str) -> &'a mut Wallet { d.wallets.entry(id.into()).or_insert(Wallet { coins: 100, hold: [0; 5] }) }
fn worth(d: &D, w: &Wallet) -> i64 { w.coins + (0..5).map(|i| w.hold[i] * d.price[i]).sum::<i64>() }
fn new_rumour(d: &mut D) { d.rumour = ((rnd() * 5.0) as usize % 5, rnd() < 0.5) }
fn resolve(g: &mut Room) {
    let n = g.players.len().max(1) as f64;
    let d = dm!(g, Market);
    let mut flow = [0i64; 5];
    let orders: Vec<(String, (usize, i64))> = d.orders.drain(..).collect();
    for (id, (i, q)) in orders {
        let p = d.price[i];
        let w = wallet(d, &id);
        if q > 0 && q * p <= w.coins { w.coins -= q * p; w.hold[i] += q; flow[i] += q }
        if q < 0 && w.hold[i] > 0 { flow[i] -= w.hold[i]; w.coins += w.hold[i] * p; w.hold[i] = 0 }
    }
    d.prev = d.price;
    let (r, up) = d.rumour;
    let real = rnd() < 0.67;
    for i in 0..5 {
        // demand: each unit bought per player nudges the price 4%, capped at +-40%
        let mut k = 1.0 + (0.04 * flow[i] as f64 / n).clamp(-0.4, 0.4) + (rnd() - 0.5) * 0.1;
        if i == r && real { k *= if up { 1.6 } else { 0.5 } }
        d.price[i] = round(d.price[i] as f64 * k).max(1.0) as i64;
    }
    d.news = format!("The {} rumour was {}!", GOODS[r].1.to_lowercase(), if real { "true" } else { "false" });
    d.turn += 1;
    if d.turn < TURNS { new_rumour(d); return set_timer(g, 30.0, resolve, false) }
    let won: Vec<(String, i64)> = d.wallets.iter().map(|(id, w)| (id.clone(), worth(d, w))).collect();
    for (id, p) in won { g.add(&id, p) }
    finish(g);
}

impl Game for Market {
    fn id(&self) -> &'static str { "market" }
    fn start(&self, g: &mut Room) {
        let price = GOODS.map(|x| x.2);
        let mut d = D { price, prev: price, ..Default::default() };
        for id in g.ids() { wallet(&mut d, &id); }
        new_rumour(&mut d);
        g.d = Data::Market(d);
        g.phase = "trade";
        set_timer(g, 30.0, resolve, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> { (phase == "trade").then_some(resolve as Next) }
    fn is_done(&self, g: &Room, id: &str) -> bool { g.phase == "trade" && dr!(g, Market).orders.contains_key(id) }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Market(d) = &g.d else { return };
        if d.wallets.is_empty() { return }
        s.insert("turnNo".into(), json!(d.turn));
        s.insert("market".into(), json!({
            "turns": TURNS, "news": d.news, "rumour": { "good": d.rumour.0, "up": d.rumour.1 },
            "goods": (0..5).map(|i| json!({ "icon": GOODS[i].0, "name": GOODS[i].1, "price": d.price[i], "prev": d.prev[i] })).collect::<Vec<_>>(),
            "wallets": d.wallets.iter().map(|(id, w)| (id.clone(), json!({ "coins": w.coins, "hold": w.hold, "worth": worth(d, w) }))).collect::<IndexMap<_, _>>(),
        }));
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        if kind(a) != "trade" || g.phase != "trade" { return no(NOT_NOW) }
        let d = dm!(g, Market);
        let q = num(a.get("qty"));
        let i = index(a.get("good"), 5).unwrap_or(0);
        let q = if q.is_nan() { 0 } else { q.clamp(-1.0, 999.0) as i64 };
        let w = wallet(d, id).clone();
        if q > 0 && q * d.price[i] > w.coins { return no("Not enough coins") }
        if q < 0 && w.hold[i] == 0 { return no("You don't have any") }
        d.orders.insert(id.into(), (i, q));
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Market);
        let w = d.wallets.get(id)?;
        let i = (rnd() * 5.0) as usize % 5;
        if w.hold[i] > 0 && rnd() < 0.4 { return Some(json!({ "type": "trade", "good": i, "qty": -1 })) }
        let max = w.coins / d.price[i];
        Some(json!({ "type": "trade", "good": i, "qty": if max > 0 { 1 + (rnd() * max as f64) as i64 % max } else { 0 } }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trades_and_final_worth() {
        let mut g = crate::room::test_room(&["a", "b"]);
        Market.start(&mut g);
        assert!(Market.act(&mut g, &json!({ "type": "trade", "good": 4, "qty": 3 }), "a").is_err()); // 120 > 100 coins
        assert!(Market.act(&mut g, &json!({ "type": "trade", "good": 0, "qty": -1 }), "a").is_err()); // holds no fish
        Market.act(&mut g, &json!({ "type": "trade", "good": 0, "qty": 5 }), "a").unwrap();
        Market.act(&mut g, &json!({ "type": "trade", "good": 0, "qty": 0 }), "b").unwrap();
        resolve(&mut g);
        let d = dr!(g, Market);
        assert_eq!((d.wallets["a"].coins, d.wallets["a"].hold[0]), (60, 5));
        for _ in 1..TURNS { resolve(&mut g) }
        assert_eq!(g.players["b"].score, 100);
        assert!(g.players["a"].score > 0);
    }
}
