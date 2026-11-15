// Dungeon Vote: the party walks through rooms, each with a monster and some treasure. Everyone secretly picks:
// fight (beat it only if enough of you fight), loot (grab the treasure, but only if the looters aren't more than
// the fighters), sneak (safe, nothing) or escape (leave now and keep your bag). Three hearts each; at zero you're out
// and your bag is lost. Whoever makes it out banks their bag as points.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

const ROOMS: usize = 6;
const CHOICES: [&str; 4] = ["fight", "loot", "sneak", "escape"];
#[derive(Clone, Default)]
struct Hero { hp: i64, bag: i64, out: bool, gone: bool }
#[derive(Default)]
pub struct D { room: usize, monster: (String, String), need: usize, gold: i64, heroes: IndexMap<String, Hero>, picks: IndexMap<String, usize>, last: Vec<(String, usize)>, log: String }
pub struct Dungeon;

fn active(d: &D) -> Vec<String> { d.heroes.iter().filter(|(_, h)| !h.out && !h.gone).map(|(id, _)| id.clone()).collect() }
fn new_room(g: &mut Room) {
    let d = dm!(g, Dungeon);
    let m = pick(data("dungeon", "MONSTERS").as_array().map(|v| v.as_slice()).unwrap_or(&[]));
    let n = active(d).len();
    d.monster = (m[0].as_str().unwrap_or("👹").into(), m[1].as_str().unwrap_or("Monster").into());
    // tougher monsters deeper in; need is a share of whoever is still inside
    let frac = m[2].as_f64().unwrap_or(0.4) + d.room as f64 * 0.03;
    d.need = ((n as f64 * frac).ceil() as usize).clamp(1, n.max(1));
    d.gold = 80 + 60 * d.room as i64 + (rnd() * 60.0) as i64;
    d.picks.clear();
    g.phase = "room";
    set_timer(g, 25.0, resolve, false);
}
fn resolve(g: &mut Room) {
    let names: IndexMap<String, String> = g.players.iter().map(|(id, p)| (id.clone(), p.name.clone())).collect();
    let d = dm!(g, Dungeon);
    let last: Vec<(String, usize)> = active(d).into_iter().map(|id| { let c = d.picks.get(&id).copied().unwrap_or(2); (id, c) }).collect(); // no answer = sneak
    let who = |c: usize| last.iter().filter(|x| x.1 == c).map(|x| x.0.clone()).collect::<Vec<_>>();
    let (fight, loot, esc) = (who(0), who(1), who(3));
    d.last = last;
    let win = fight.len() >= d.need;
    let greedy = loot.len() > fight.len();
    for id in &esc { d.heroes[id].gone = true }
    if win {
        for id in &fight { d.heroes[id].bag += 40 }
        for id in &loot { if greedy { d.heroes[id].hp -= 1 } else { d.heroes[id].bag += d.gold / loot.len() as i64 } }
    } else {
        for id in fight.iter().chain(&loot) { d.heroes[id].hp -= 1 }
    }
    let mut fell = vec![];
    for (id, h) in d.heroes.iter_mut() { if h.hp <= 0 && !h.out { h.out = true; h.bag = 0; fell.push(id.clone()) } }
    d.log = format!("{}{}{}",
        if win { format!("The {} is beaten! ", d.monster.1) }
        else if fight.is_empty() && loot.is_empty() { format!("Everyone tiptoes past the {}. ", d.monster.1) }
        else { format!("The {} wins! Fighters and looters get hurt. ", d.monster.1) },
        if win && greedy { "Too many looters, the alarm went off! " } else if win && !loot.is_empty() { "The looters split the gold. " } else { "" },
        if fell.is_empty() { String::new() } else { format!("{} knocked out!", fell.iter().map(|id| names.get(id).cloned().unwrap_or_default()).collect::<Vec<_>>().join(", ")) });
    d.room += 1;
    g.phase = "outcome";
    set_timer(g, 7.0, next, true);
}
fn next(g: &mut Room) {
    let d = dm!(g, Dungeon);
    if d.room < ROOMS && !active(d).is_empty() { return new_room(g) }
    // the survivors walk out with their bags
    let won: Vec<(String, i64)> = d.heroes.iter().filter(|(_, h)| !h.out).map(|(id, h)| (id.clone(), h.bag)).collect();
    for (id, p) in won { g.add(&id, p) }
    finish(g);
}

impl Game for Dungeon {
    fn id(&self) -> &'static str { "dungeon" }
    fn start(&self, g: &mut Room) {
        let heroes = g.ids().into_iter().map(|id| (id, Hero { hp: 3, ..Default::default() })).collect();
        g.d = Data::Dungeon(D { heroes, ..Default::default() });
        new_room(g);
    }
    fn advance(&self, phase: &str) -> Option<Next> {
        match phase { "room" => Some(resolve), "outcome" => Some(next), _ => None }
    }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        let d = dr!(g, Dungeon);
        g.phase == "room" && (d.picks.contains_key(id) || !active(d).iter().any(|x| x == id))
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Dungeon(d) = &g.d else { return };
        if d.heroes.is_empty() { return }
        s.insert("turnNo".into(), json!(d.room));
        s.insert("dungeon".into(), json!({
            "rooms": ROOMS, "icon": d.monster.0, "monster": d.monster.1, "need": d.need, "gold": d.gold, "log": d.log,
            "heroes": d.heroes.iter().map(|(id, h)| (id.clone(), json!({ "hp": h.hp, "bag": h.bag, "out": h.out, "gone": h.gone }))).collect::<IndexMap<_, _>>(),
            // what everyone picked shows only after the room is decided
            "last": if g.phase == "room" { json!([]) } else { json!(d.last.iter().map(|(id, c)| json!({ "id": id, "pick": CHOICES[*c] })).collect::<Vec<_>>()) },
        }));
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        if kind(a) != "choose" || g.phase != "room" { return no(NOT_NOW) }
        let d = dm!(g, Dungeon);
        if !active(d).iter().any(|x| x == id) { return no("You're out of the dungeon") }
        let Some(c) = CHOICES.iter().position(|c| Some(*c) == a.get("pick").and_then(|v| v.as_str())) else { return no("Pick one") };
        d.picks.insert(id.into(), c);
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Dungeon);
        if !active(d).iter().any(|x| x == id) { return None }
        let h = &d.heroes[id];
        let c = if h.hp == 1 && h.bag > 0 { "escape" } else { pick(&["fight", "fight", "loot", "sneak"]) };
        Some(json!({ "type": "choose", "pick": c }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fight_loot_and_escape() {
        let mut g = crate::room::test_room(&["a", "b", "c"]);
        Dungeon.start(&mut g);
        dm!(g, Dungeon).need = 1;
        dm!(g, Dungeon).gold = 100;
        for (id, c) in [("a", "fight"), ("b", "loot"), ("c", "escape")] { Dungeon.act(&mut g, &json!({ "type": "choose", "pick": c }), id).unwrap() }
        resolve(&mut g);
        let d = dr!(g, Dungeon);
        assert_eq!((d.heroes["a"].bag, d.heroes["b"].bag, d.heroes["c"].gone), (40, 100, true));
        // monster too strong: fighters and looters lose a heart
        next(&mut g);
        dm!(g, Dungeon).need = 2;
        for (id, c) in [("a", "fight"), ("b", "sneak")] { Dungeon.act(&mut g, &json!({ "type": "choose", "pick": c }), id).unwrap() }
        resolve(&mut g);
        let d = dr!(g, Dungeon);
        assert_eq!((d.heroes["a"].hp, d.heroes["b"].hp), (2, 3));
    }
}
