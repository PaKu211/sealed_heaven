use sealed_sim::living::{Background, Direction, Intent, Position, World};
use std::collections::{BTreeSet, VecDeque};
fn dist(a: Position, b: Position) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}
fn direction(w: &World, target: Position) -> Option<Direction> {
    let start = w.player().position;
    let mut q = VecDeque::from([(start, None)]);
    let mut seen = BTreeSet::from([start]);
    while let Some((p, first)) = q.pop_front() {
        if dist(p, target) <= 1 {
            return first;
        }
        for (d, x, y) in [
            (Direction::North, Some(p.x), p.y.checked_sub(1)),
            (Direction::East, p.x.checked_add(1), Some(p.y)),
            (Direction::South, Some(p.x), p.y.checked_add(1)),
            (Direction::West, p.x.checked_sub(1), Some(p.y)),
        ] {
            let (Some(x), Some(y)) = (x, y) else {
                continue;
            };
            let next = Position { x, y };
            if !w.walkable(next)
                || w.npcs().any(|n| n.alive() && n.position == next)
                || !seen.insert(next)
            {
                continue;
            }
            q.push_back((next, first.or(Some(d))));
        }
    }
    None
}

pub fn trace(entrust: bool) -> (World, Vec<&'static str>) {
    let seed = if entrust { 3 } else { 0 };
    let mut w = World::road_history_campaign(seed);
    let mut commands = vec!["background herbalist"];
    w.step(0, Intent::ChooseBackground(Background::Herbalist))
        .unwrap();
    let target = w.road_history().unwrap().cache(1).unwrap().deposit.position;
    for _ in 0..250 {
        if dist(w.player().position, target) <= 1 {
            break;
        }
        let (intent, cmd) = match direction(&w, target) {
            Some(d) => (
                Intent::Move(d),
                match d {
                    Direction::North => "north",
                    Direction::East => "east",
                    Direction::South => "south",
                    Direction::West => "west",
                },
            ),
            None => (Intent::Wait, "wait"),
        };
        w.step(w.tick(), intent).unwrap();
        commands.push(cmd);
    }
    for _ in 0..300 {
        if w.local_caches()
            .iter()
            .any(|c| c.deposit.id == 1 && if entrust { c.can_entrust } else { c.can_take })
        {
            return (w, commands);
        }
        w.step(w.tick(), Intent::Wait).unwrap();
        commands.push("wait");
    }
    panic!("actual encounter did not become available");
}
