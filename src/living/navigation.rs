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
    if !scenario.walkable(from) || !scenario.walkable(to) {
        return false;
    }
    let (mut x, mut y) = (i64::from(from.x), i64::from(from.y));
    let (end_x, end_y) = (i64::from(to.x), i64::from(to.y));
    let (dx, dy) = ((end_x - x).abs(), (end_y - y).abs());
    let (sx, sy) = ((end_x - x).signum(), (end_y - y).signum());
    let (mut crossed_x, mut crossed_y) = (0i64, 0i64);
    let open = |x, y| scenario.walkable(position(x as u32, y as u32));
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
