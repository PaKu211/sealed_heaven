use std::collections::VecDeque;

use super::{DIRECTIONS, Direction, Position, Scenario};

pub(super) fn position(x: u32, y: u32) -> Position {
    Position { x, y }
}

pub(super) fn moved(p: Position, direction: Direction) -> Option<Position> {
    let (x, y) = match direction {
        Direction::North => (p.x, p.y.checked_sub(1)?),
        Direction::East => (p.x.checked_add(1)?, p.y),
        Direction::South => (p.x, p.y.checked_add(1)?),
        Direction::West => (p.x.checked_sub(1)?, p.y),
    };
    Some(position(x, y))
}

pub(super) fn distance(from: Position, to: Position) -> u32 {
    from.x.abs_diff(to.x).saturating_add(from.y.abs_diff(to.y))
}

pub(super) fn in_reach(from: Position, to: Position) -> bool {
    distance(from, to) <= 1
}

/// Integer line of sight. A diagonal step cannot peek around an opaque corner.
pub(super) fn can_see(scenario: &Scenario, from: Position, to: Position) -> bool {
    can_see_with(from, to, |p| scenario.walkable(p))
}

pub(super) fn can_see_with(
    from: Position,
    to: Position,
    walkable: impl Fn(Position) -> bool,
) -> bool {
    if !walkable(from) || !walkable(to) {
        return false;
    }
    let (mut x, mut y) = (i64::from(from.x), i64::from(from.y));
    let (end_x, end_y) = (i64::from(to.x), i64::from(to.y));
    let (dx, dy) = ((end_x - x).abs(), (end_y - y).abs());
    let (sx, sy) = ((end_x - x).signum(), (end_y - y).signum());
    let (mut crossed_x, mut crossed_y) = (0i64, 0i64);
    let open = |x, y| walkable(position(x as u32, y as u32));
    while (x, y) != (end_x, end_y) {
        let horizontal = i128::from(2 * crossed_x + 1) * i128::from(dy);
        let vertical = i128::from(2 * crossed_y + 1) * i128::from(dx);
        match horizontal.cmp(&vertical) {
            std::cmp::Ordering::Less => {
                x += sx;
                crossed_x += 1;
            }
            std::cmp::Ordering::Greater => {
                y += sy;
                crossed_y += 1;
            }
            std::cmp::Ordering::Equal => {
                if !open(x + sx, y) || !open(x, y + sy) {
                    return false;
                }
                x += sx;
                y += sy;
                crossed_x += 1;
                crossed_y += 1;
            }
        }
        if !open(x, y) {
            return false;
        }
    }
    true
}

/// Shortest path with a fixed NESW tie break. The first cell is returned so each
/// actor can attempt at most one movement per canonical tick.
pub(super) fn next_step(
    scenario: &Scenario,
    from: Position,
    destination: Position,
    reach: u32,
    can_enter: impl Fn(Position) -> bool,
) -> Option<Position> {
    if distance(from, destination) <= reach {
        return None;
    }
    let index = |cell: Position| (cell.y * scenario.width + cell.x) as usize;
    let mut frontier = VecDeque::from([from]);
    let mut first_steps = vec![None; (scenario.width * scenario.height) as usize];
    first_steps[index(from)] = Some(from);
    while let Some(current) = frontier.pop_front() {
        let first_step = first_steps[index(current)].expect("enqueued cell has a first step");
        for direction in DIRECTIONS {
            let Some(next) = moved(current, direction) else {
                continue;
            };
            if next.x >= scenario.width || next.y >= scenario.height {
                continue;
            }
            let next_index = index(next);
            if first_steps[next_index].is_some() || !can_enter(next) {
                continue;
            }
            let step = if current == from { next } else { first_step };
            if distance(next, destination) <= reach {
                return Some(step);
            }
            first_steps[next_index] = Some(step);
            frontier.push_back(next);
        }
    }
    None
}

use super::World;

/// Ephemeral occupancy snapshot for one path query. Rebuilt after every
/// commit, so earlier ID-ordered moves, swaps and deaths cannot stale it.
/// No canonical state, cache invalidation or new ordering semantics.
struct Occupancy {
    width: u32,
    height: u32,
    cells: Vec<bool>,
}
impl Occupancy {
    fn new(width: u32, height: u32, positions: impl IntoIterator<Item = Position>) -> Self {
        let mut cells = vec![false; (width * height) as usize];
        for p in positions {
            assert!(p.x < width && p.y < height, "invalid occupied cell");
            cells[(p.y * width + p.x) as usize] = true;
        }
        Self {
            width,
            height,
            cells,
        }
    }
    fn occupied(&self, p: Position) -> bool {
        p.x < self.width && p.y < self.height && self.cells[(p.y * self.width + p.x) as usize]
    }
}
impl World {
    pub(super) fn route_step(
        &self,
        from: Position,
        destination: Position,
        reach: u32,
    ) -> Option<Position> {
        if self.scheduler().is_none() {
            return next_step(self.scenario, from, destination, reach, |p| {
                self.walkable(p) && !self.occupied(p)
            });
        }
        if distance(from, destination) <= reach {
            return None;
        }
        let occupants = self
            .npcs
            .values()
            .filter(|n| n.alive())
            .map(|n| n.position)
            .chain(self.player.alive().then_some(self.player.position));
        let occupied = Occupancy::new(self.scenario.width, self.scenario.height, occupants);
        next_step(self.scenario, from, destination, reach, |p| {
            self.walkable(p) && !occupied.occupied(p)
        })
    }
}

#[cfg(test)]
mod occupancy_tests {
    use super::*;
    use std::{hint::black_box, time::Instant};
    #[test]
    fn indexed_paths_match_scan_reference_on_dense_synthetic_populations() {
        let w = World::exact_lod_campaign(0);
        let scenario = Scenario {
            width: 64,
            height: 64,
            ..*w.scenario
        };
        let mut slots: Vec<_> = (1..63)
            .flat_map(|x| (1..63).map(move |y| position(x, y)))
            .collect();
        // A reproducible permutation, with no external RNG or clock input.
        slots.sort_by_key(|p| u64::from(p.x * 64 + p.y).wrapping_mul(0x9e3779b97f4a7c15));
        for n in [20, 100, 500, 1000, 2000] {
            let occupants = &slots[..n];
            let queries = [
                (position(0, 0), position(63, 63), 0),
                (position(0, 63), position(63, 0), 1),
                (position(0, 0), position(63, 32), 0),
                (position(63, 0), position(0, 63), 1),
            ];
            let mut reference = Vec::new();
            let start = Instant::now();
            for &(from, to, reach) in &queries {
                reference.push(black_box(next_step(&scenario, from, to, reach, |p| {
                    !occupants.contains(&p)
                })));
            }
            let scan = start.elapsed();
            let start = Instant::now();
            let mut result = Vec::new();
            for &(from, to, reach) in &queries {
                let index = Occupancy::new(64, 64, occupants.iter().copied());
                result.push(black_box(next_step(&scenario, from, to, reach, |p| {
                    !index.occupied(p)
                })));
            }
            let indexed = start.elapsed();
            assert_eq!(reference, result);
            assert!(result.iter().all(Option::is_some));
            println!(
                "synthetic actors={n} queries={} scan_us={} indexed_us={}",
                queries.len(),
                scan.as_micros(),
                indexed.as_micros()
            );
        }
    }
}
