// Land Grab: a hex map. Each turn everyone secretly claims one free tile at the same time; if two or more pick
// the same tile it turns to rock and nobody gets it. After the last turn you score 100 per tile in your biggest
// connected patch of land.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

const TURNS: usize = 8;
const ROCK: &str = "#";
#[derive(Default)]
pub struct D { w: usize, h: usize, board: Vec<String>, turn: usize, picks: IndexMap<String, usize>, clash: Vec<usize> } // board: "" free, "#" rock, else owner id
pub struct LandGrab;

// hex neighbours, rows offset like bricks (odd rows shifted right)
fn near(d: &D, i: usize) -> Vec<usize> {
    let (c, r) = ((i % d.w) as i64, (i / d.w) as i64);
    let o = r % 2; // odd rows lean right
    [(-1, 0), (1, 0), (o - 1, -1), (o, -1), (o - 1, 1), (o, 1)].iter()
        .map(|(dc, dr)| (c + dc, r + dr)).filter(|(x, y)| *x >= 0 && *y >= 0 && *x < d.w as i64 && *y < d.h as i64)
        .map(|(x, y)| y as usize * d.w + x as usize).collect()
}
// size of this player's biggest connected patch
fn biggest(d: &D, id: &str) -> usize {
    let mut seen = vec![false; d.board.len()];
    let mut best = 0;
    for s in 0..d.board.len() {
        if seen[s] || d.board[s] != id { continue }
        let (mut st, mut n) = (vec![s], 0);
        seen[s] = true;
        while let Some(i) = st.pop() { n += 1; for j in near(d, i) { if !seen[j] && d.board[j] == id { seen[j] = true; st.push(j) } } }
        best = best.max(n);
    }
    best
}
fn free(d: &D) -> Vec<usize> { (0..d.board.len()).filter(|i| d.board[*i].is_empty()).collect() }
fn resolve(g: &mut Room) {
    let d = dm!(g, Land);
    let mut count: IndexMap<usize, Vec<String>> = IndexMap::new();
    for (id, t) in &d.picks { count.entry(*t).or_default().push(id.clone()) }
    d.clash.clear();
    for (t, ids) in count {
        if ids.len() == 1 { d.board[t] = ids[0].clone() } else { d.board[t] = ROCK.into(); d.clash.push(t) }
    }
    d.picks.clear();
    d.turn += 1;
    if d.turn < TURNS && !free(d).is_empty() { return set_timer(g, 20.0, resolve, false) }
    let won: Vec<(String, i64)> = g.ids().iter().map(|id| (id.clone(), 100 * biggest(dr!(g, Land), id) as i64)).collect();
    for (id, p) in won { g.add(&id, p) }
    finish(g);
}

impl Game for LandGrab {
    fn id(&self) -> &'static str { "landgrab" }
    fn start(&self, g: &mut Room) {
        let (w, h) = if g.players.len() <= 8 { (9, 6) } else { (12, 8) };
        let mut board = vec![String::new(); w * h];
        for _ in 0..w * h / 10 { let i = (rnd() * (w * h) as f64) as usize % (w * h); board[i] = ROCK.into() } // a few rocks for shape
        g.d = Data::Land(D { w, h, board, ..Default::default() });
        g.phase = "claim";
        set_timer(g, 20.0, resolve, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> { (phase == "claim").then_some(resolve as Next) }
    fn is_done(&self, g: &Room, id: &str) -> bool { g.phase == "claim" && dr!(g, Land).picks.contains_key(id) }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Land(d) = &g.d else { return };
        if d.w == 0 { return }
        s.insert("land".into(), json!({ "w": d.w, "h": d.h, "board": d.board, "clash": d.clash, "turns": TURNS,
            "big": g.ids().iter().map(|id| (id.clone(), biggest(d, id))).collect::<IndexMap<_, _>>() }));
        s.insert("turnNo".into(), json!(d.turn));
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        if kind(a) != "claim" || g.phase != "claim" { return no(NOT_NOW) }
        let d = dm!(g, Land);
        let Some(t) = index(a.get("tile"), d.board.len()).filter(|t| d.board[*t].is_empty()) else { return no("That tile's taken") };
        d.picks.insert(id.into(), t);
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Land);
        let f = free(d);
        if f.is_empty() { return None }
        // grow next to its own land when it can
        let next: Vec<usize> = f.iter().copied().filter(|i| near(d, *i).iter().any(|j| d.board[*j] == id)).collect();
        Some(json!({ "type": "claim", "tile": pick(if next.is_empty() || rnd() < 0.2 { &f } else { &next }) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clashes_turn_to_rock_and_patches_count() {
        let mut g = crate::room::test_room(&["a", "b", "c"]);
        LandGrab.start(&mut g);
        dm!(g, Land).board.iter_mut().for_each(|t| t.clear());
        for (id, t) in [("a", 0), ("b", 0), ("c", 10)] { LandGrab.act(&mut g, &json!({ "type": "claim", "tile": t }), id).unwrap() }
        resolve(&mut g);
        let d = dr!(g, Land);
        assert_eq!((d.board[0].as_str(), d.board[10].as_str()), (ROCK, "c"));
        assert!(LandGrab.act(&mut g, &json!({ "type": "claim", "tile": 0 }), "a").is_err());
        // row 0 col 1 and col 2 touch; row 1 col 1 (index 10) touches row 0 col 1 and col 2 (odd row leans right)
        let d = dm!(g, Land);
        d.board[1] = "c".into();
        assert_eq!(biggest(d, "c"), 2);
        assert_eq!(near(d, 10).len(), 6);
    }
}
