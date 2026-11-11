// Knockout: everyone's puck sits on a round ice floe. All players aim at once on their phone (drag back, let go),
// then every shot slides at the same time and pucks bump each other off the edge. The ice shrinks each turn.
// Last puck standing takes the round; everyone scores for each turn they stay on.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

// arena is 1000 x 600, ice centred at (500, 300)
const C: (f64, f64) = (500.0, 300.0);
const R0: f64 = 280.0;
const SHRINK: f64 = 20.0;
const PUCK: f64 = 22.0;
const MAX_SPEED: f64 = 900.0; // a full-power shot slides about 500, so it can still carry you off
const MAX_TURNS: i64 = 10;
const OFF: i64 = -999; // a fallen puck's spot in a frame

#[derive(Default)]
pub struct Puck { x: f64, y: f64, vx: f64, vy: f64, out: bool, aimed: bool, turns: i64 }
#[derive(Default)]
pub struct D { r: f64, last_r: f64, turn: i64, pucks: IndexMap<String, Puck>, frames: Vec<Vec<i64>> }
pub struct Knockout;

// plays every shot out together at 120 steps a second; frames kept at 30 fps for the screens
fn slide(d: &mut D) {
    let dt = 1.0 / 120.0;
    d.frames.clear();
    for step in 0..2400 {
        let ids: Vec<usize> = (0..d.pucks.len()).filter(|&i| !d.pucks[i].out).collect();
        for &i in &ids { let p = &mut d.pucks[i]; p.x += p.vx * dt; p.y += p.vy * dt; }
        // equal-mass bounces: swap the velocity along the line between centres
        for (k, &i) in ids.iter().enumerate() { for &j in &ids[k + 1..] {
            let (a, b) = (&d.pucks[i], &d.pucks[j]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let dist = dx.hypot(dy);
            if dist >= PUCK * 2.0 || dist < 1e-9 { continue }
            let (nx, ny) = (dx / dist, dy / dist);
            let rel = (a.vx - b.vx) * nx + (a.vy - b.vy) * ny;
            let push = (PUCK * 2.0 - dist) / 2.0;
            let imp = if rel > 0.0 { rel * 0.95 } else { 0.0 };
            let a = &mut d.pucks[i]; a.x -= nx * push; a.y -= ny * push; a.vx -= imp * nx; a.vy -= imp * ny;
            let b = &mut d.pucks[j]; b.x += nx * push; b.y += ny * push; b.vx += imp * nx; b.vy += imp * ny;
        } }
        let mut moving = false;
        for &i in &ids {
            let p = &mut d.pucks[i];
            if (p.x - C.0).hypot(p.y - C.1) > d.r { p.out = true }
            p.vx *= 0.985; p.vy *= 0.985;
            moving |= p.vx.hypot(p.vy) > 8.0;
        }
        if step % 4 == 0 { d.frames.push(d.pucks.values().flat_map(|p| if p.out { [OFF, OFF] } else { [p.x.round() as i64, p.y.round() as i64] }).collect()) }
        if !moving { break }
    }
    for p in d.pucks.values_mut() { p.vx = 0.0; p.vy = 0.0; }
    d.frames.push(d.pucks.values().flat_map(|p| if p.out { [OFF, OFF] } else { [p.x.round() as i64, p.y.round() as i64] }).collect());
}

fn resolve(g: &mut Room) {
    let d = dm!(g, Knockout);
    slide(d);
    d.turn += 1;
    for p in d.pucks.values_mut() { p.aimed = false; if !p.out { p.turns += 1 } }
    let left = d.pucks.values().filter(|p| !p.out).count();
    if left <= 1 || d.turn >= MAX_TURNS { return end(g) }
    d.last_r = d.r;
    d.r = (d.r - SHRINK).max(150.0);
    // a few extra seconds so the slide plays out before the aim clock matters
    set_timer(g, 18.0, resolve, true);
}

fn end(g: &mut Room) {
    let d = dm!(g, Knockout);
    let left: Vec<String> = d.pucks.iter().filter(|(_, p)| !p.out).map(|(id, _)| id.clone()).collect();
    let pts: Vec<(String, i64)> = d.pucks.iter().map(|(id, p)| (id.clone(), p.turns * 50 + if !p.out && left.len() == 1 { 300 } else { 0 })).collect();
    for (id, n) in pts { g.add(&id, n) }
    // let the last slide play out on the TV before the scores
    g.phase = "splash";
    set_timer(g, 7.0, finish, true);
}

impl Game for Knockout {
    fn id(&self) -> &'static str { "knockout" }
    fn start(&self, g: &mut Room) {
        g.phase = "aim";
        let ids = shuffle(g.ids());
        let n = ids.len().max(1) as f64;
        let pucks = ids.into_iter().enumerate().map(|(i, id)| {
            let a = i as f64 / n * std::f64::consts::TAU;
            (id, Puck { x: C.0 + a.cos() * R0 * 0.6, y: C.1 + a.sin() * R0 * 0.6, ..Default::default() })
        }).collect();
        g.d = Data::Knockout(D { r: R0, last_r: R0, pucks, ..Default::default() });
        set_timer(g, 15.0, resolve, true);
    }
    fn advance(&self, phase: &str) -> Option<Next> { (phase == "aim").then_some(resolve as Next) }
    fn is_done(&self, g: &Room, id: &str) -> bool { dr!(g, Knockout).pucks.get(id).map_or(true, |p| p.out || p.aimed) }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Knockout(d) = &g.d else { return };
        s.insert("ko".into(), json!({ "r": d.r, "lastR": d.last_r, "turn": d.turn, "frames": d.frames,
            "pucks": d.pucks.iter().map(|(id, p)| json!({ "id": id, "x": p.x.round(), "y": p.y.round(), "out": p.out, "turns": p.turns })).collect::<Vec<_>>() }));
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        if kind(a) != "shot" { return no("Unknown action") }
        let (angle, power) = (num(a.get("a")), num(a.get("p")));
        if g.phase != "aim" || !angle.is_finite() || !power.is_finite() { return no(NOT_NOW) }
        let d = dm!(g, Knockout);
        let Some(p) = d.pucks.get_mut(id).filter(|p| !p.out && !p.aimed) else { return no(NOT_NOW) };
        let v = power.clamp(0.0, 1.0) * MAX_SPEED;
        p.vx = angle.cos() * v; p.vy = angle.sin() * v; p.aimed = true;
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Knockout);
        let me = d.pucks.get(id)?;
        // charge the nearest rival, a bit off
        let t = d.pucks.iter().filter(|(k, p)| *k != id && !p.out)
            .min_by(|a, b| (a.1.x - me.x).hypot(a.1.y - me.y).total_cmp(&(b.1.x - me.x).hypot(b.1.y - me.y)))?.1;
        Some(json!({ "type": "shot", "a": (t.y - me.y).atan2(t.x - me.x) + (rnd() - 0.5) * 0.4, "p": 0.35 + rnd() * 0.4 }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn two(ax: f64, bx: f64, v: f64) -> D {
        let mut d = D { r: R0, ..Default::default() };
        d.pucks.insert("a".into(), Puck { x: ax, y: 300.0, vx: v, ..Default::default() });
        d.pucks.insert("b".into(), Puck { x: bx, y: 300.0, ..Default::default() });
        d
    }
    #[test]
    fn hits_knock_pucks_off() {
        // a hard hit sends the other puck off the edge while the hitter stays on
        let mut d = two(400.0, 600.0, MAX_SPEED);
        slide(&mut d);
        assert!(!d.pucks["a"].out && d.pucks["b"].out);
        // a still puck stays put and on the ice
        let mut d = two(400.0, 600.0, 0.0);
        slide(&mut d);
        assert!(!d.pucks["a"].out && !d.pucks["b"].out && d.pucks["b"].x == 600.0);
    }
}
