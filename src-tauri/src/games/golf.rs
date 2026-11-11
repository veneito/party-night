// Mini Golf: everyone putts at once on the same hole, aiming on their phone (drag back from the ball, let go).
// The server plays out each shot in one go and sends the path; the TV animates it. Fewer strokes, more points.
use super::*;
use crate::room::*;
use crate::{dm, dr};
use indexmap::IndexMap;
use serde_json::json;

// course is 1000 x 600; walls are drawn 14 wide, so the ball bounces off at its radius + 7
const R: f64 = 10.0;
const HIT: f64 = R + 7.0;
const CUP: f64 = 16.0;
const MAX_SPEED: f64 = 2000.0;
const MAX_STROKES: i64 = 6;
const BORDER: [[f64; 4]; 4] = [[-7.0, -7.0, 1007.0, -7.0], [1007.0, -7.0, 1007.0, 607.0], [1007.0, 607.0, -7.0, 607.0], [-7.0, 607.0, -7.0, -7.0]];

#[derive(Default)]
pub struct Ball { x: f64, y: f64, n: i64, sunk: bool, shot: i64, path: Vec<i64> }
#[derive(Default)]
pub struct D { hole: Value, walls: Vec<[f64; 4]>, cup: (f64, f64), balls: IndexMap<String, Ball> }
pub struct Golf;

fn pt(v: &Value) -> (f64, f64) { (v[0].as_f64().unwrap_or(0.0), v[1].as_f64().unwrap_or(0.0)) }

// one shot, simulated at 120 steps a second; the path is kept at 30 fps for the TV
fn roll(b: &mut Ball, walls: &[[f64; 4]], cup: (f64, f64), angle: f64, power: f64) {
    let (mut vx, mut vy) = (angle.cos() * power * MAX_SPEED, angle.sin() * power * MAX_SPEED);
    let dt = 1.0 / 120.0;
    b.path.clear();
    for step in 0..2400 {
        b.x += vx * dt; b.y += vy * dt;
        for w in walls.iter().chain(BORDER.iter()) {
            let (ax, ay, dx, dy) = (w[0], w[1], w[2] - w[0], w[3] - w[1]);
            let t = (((b.x - ax) * dx + (b.y - ay) * dy) / (dx * dx + dy * dy).max(1e-9)).clamp(0.0, 1.0);
            let (nx, ny) = (b.x - (ax + t * dx), b.y - (ay + t * dy));
            let d = nx.hypot(ny);
            if d < HIT && d > 1e-9 {
                let (nx, ny) = (nx / d, ny / d);
                b.x += nx * (HIT - d); b.y += ny * (HIT - d);
                let dot = vx * nx + vy * ny;
                if dot < 0.0 { vx = (vx - 2.0 * dot * nx) * 0.8; vy = (vy - 2.0 * dot * ny) * 0.8; }
            }
        }
        let speed = vx.hypot(vy);
        // too fast and it skips over the cup
        if (b.x - cup.0).hypot(b.y - cup.1) < CUP && speed < 900.0 { b.sunk = true; b.x = cup.0; b.y = cup.1; }
        if step % 4 == 0 { b.path.extend([b.x.round() as i64, b.y.round() as i64]) }
        if b.sunk || speed < 8.0 { break }
        vx *= 0.985; vy *= 0.985;
    }
    b.path.extend([b.x.round() as i64, b.y.round() as i64]);
}

fn tally(g: &mut Room) {
    let d = dm!(g, Golf);
    for (id, b) in &d.balls { if b.sunk { if let Some(p) = g.players.get_mut(id) { p.score += (MAX_STROKES + 1 - b.n) * 100 } } }
    finish(g);
}

impl Game for Golf {
    fn id(&self) -> &'static str { "golf" }
    fn start(&self, g: &mut Room) {
        g.phase = "putt";
        let holes = data("golf", "HOLES").as_array().unwrap();
        // "mixed" (the default) plays holes of every difficulty
        let level = g.set_str("level");
        let all: Vec<usize> = (0..holes.len()).filter(|&i| matches!(level, "" | "mixed") || holes[i]["level"] == level).collect();
        let fresh: Vec<usize> = all.iter().copied().filter(|i| !g.used.contains(&format!("hole{i}"))).collect();
        let i = pick(if fresh.is_empty() { &all } else { &fresh });
        g.used.push(format!("hole{i}"));
        let h = holes[i].clone();
        let (sx, sy) = pt(&h["start"]);
        let walls = h["walls"].as_array().map(|a| a.iter().map(|w| [0, 1, 2, 3].map(|k| w[k].as_f64().unwrap_or(0.0))).collect()).unwrap_or_default();
        let balls = g.ids().into_iter().map(|id| (id, Ball { x: sx, y: sy, ..Default::default() })).collect();
        g.d = Data::Golf(D { cup: pt(&h["cup"]), hole: h, walls, balls });
        set_timer(g, 120.0, tally, false);
    }
    fn advance(&self, phase: &str) -> Option<Next> { (phase == "putt").then_some(tally as Next) }
    fn is_done(&self, g: &Room, id: &str) -> bool {
        dr!(g, Golf).balls.get(id).map_or(true, |b| b.sunk || b.n >= MAX_STROKES)
    }
    fn state(&self, g: &Room, s: &mut Map<String, Value>) {
        let Data::Golf(d) = &g.d else { return };
        s.insert("hole".into(), d.hole.clone());
        s.insert("maxStrokes".into(), json!(MAX_STROKES));
        s.insert("balls".into(), json!(d.balls.iter().map(|(id, b)| json!({ "id": id, "x": b.x.round(), "y": b.y.round(),
            "n": b.n, "in": b.sunk, "shot": b.shot, "path": b.path })).collect::<Vec<_>>()));
    }
    fn act(&self, g: &mut Room, a: &Value, id: &str) -> Res {
        if kind(a) != "putt" { return no("Unknown action") }
        let (angle, power) = (num(a.get("a")), num(a.get("p")));
        if g.phase != "putt" || !angle.is_finite() || !power.is_finite() { return no(NOT_NOW) }
        let d = dm!(g, Golf);
        let Some(b) = d.balls.get_mut(id).filter(|b| !b.sunk && b.n < MAX_STROKES) else { return no(NOT_NOW) };
        b.n += 1; b.shot += 1;
        roll(b, &d.walls, d.cup, angle, power.clamp(0.05, 1.0));
        Ok(())
    }
    fn bot(&self, g: &Room, id: &str) -> Option<Value> {
        let d = dr!(g, Golf);
        let b = d.balls.get(id)?;
        let (dx, dy) = (d.cup.0 - b.x, d.cup.1 - b.y);
        // straight at the cup, a bit off; a shot rolls about 0.556 x its start speed before stopping
        Some(json!({ "type": "putt", "a": dy.atan2(dx) + (rnd() - 0.5) * 0.3, "p": (dx.hypot(dy) / (MAX_SPEED * 0.556) * (0.85 + rnd() * 0.3)).min(1.0) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn putts_sink_and_walls_hold() {
        let mut b = Ball { x: 150.0, y: 300.0, ..Default::default() };
        roll(&mut b, &[], (850.0, 300.0), 0.0, 700.0 / (MAX_SPEED * 0.556));
        assert!(b.sunk);
        // full power into a wall: bounces around but never gets through or off the course
        let mut b = Ball { x: 500.0, y: 300.0, ..Default::default() };
        roll(&mut b, &[[700.0, 0.0, 700.0, 600.0]], (900.0, 300.0), 0.0, 1.0);
        assert!(!b.sunk && b.x > 0.0 && b.x < 700.0);
    }
    #[test]
    fn every_hole_is_playable() {
        // tee and cup sit on the course, clear of every wall, and each difficulty has holes
        let holes = data("golf", "HOLES").as_array().unwrap();
        for lv in ["easy", "medium", "hard"] { assert!(holes.iter().any(|h| h["level"] == lv), "{lv}") }
        for h in holes {
            for p in [pt(&h["start"]), pt(&h["cup"])] {
                assert!(p.0 >= HIT && p.0 <= 1000.0 - HIT && p.1 >= HIT && p.1 <= 600.0 - HIT, "{h}");
                for w in h["walls"].as_array().unwrap() {
                    let mut b = Ball { x: p.0, y: p.1, ..Default::default() };
                    roll(&mut b, &[[0, 1, 2, 3].map(|k| w[k].as_f64().unwrap())], (-99.0, -99.0), 0.0, 0.0);
                    assert!((b.x, b.y) == p, "{h} {w}");
                }
            }
        }
    }
}
