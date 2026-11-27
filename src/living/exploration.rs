//! C6.0: stable terrain memory, separate from all dynamic observations.
//! Routes use only previously observed walkable cells. A route is advice,
//! never a reservation, automation, or access to live NPC/stock information.
use super::{
    Direction, Position, Region, World,
    expansion::{DEPOT, GROVE, HEIGHT, WIDTH},
    navigation::{distance, moved},
};
use std::collections::VecDeque;

pub const REVISION: u16 = 47;
pub const VERSION: u16 = 1;
const CELLS: usize = (WIDTH * HEIGHT) as usize;
const WORDS: usize = CELLS.div_ceil(64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Landmark {
    Shrine,
    Clinic,
    Patch,
    Market,
    Garden,
    Hall,
    Depot,
    Grove,
}
impl Landmark {
    pub const ALL: [Self; 8] = [
        Self::Shrine,
        Self::Clinic,
        Self::Patch,
        Self::Market,
        Self::Garden,
        Self::Hall,
        Self::Depot,
        Self::Grove,
    ];
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Shrine => "shrine",
            Self::Clinic => "clinic",
            Self::Patch => "patch",
            Self::Market => "market",
            Self::Garden => "garden",
            Self::Hall => "hall",
            Self::Depot => "depot",
            Self::Grove => "grove",
        }
    }
    pub fn position(self, w: &World) -> Position {
        let sites = Region::sites();
        let s = w.scenario();
        match self {
            Self::Shrine => s.shrine,
            Self::Clinic => s.clinic,
            Self::Patch => s.patch,
            Self::Market => sites.market,
            Self::Garden => sites.garden,
            Self::Hall => sites.hall,
            Self::Depot => DEPOT,
            Self::Grove => GROVE,
        }
    }
    pub const fn glyph(self) -> char {
        match self {
            Self::Shrine => '+',
            Self::Clinic => '=',
            Self::Patch => '*',
            Self::Market | Self::Depot => '$',
            Self::Garden | Self::Grove => '&',
            Self::Hall => '^',
        }
    }
}
fn index(p: Position) -> Option<usize> {
    (p.x < WIDTH && p.y < HEIGHT).then(|| (p.y * WIDTH + p.x) as usize)
}
fn pos(i: usize) -> Position {
    Position {
        x: i as u32 % WIDTH,
        y: i as u32 / WIDTH,
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exploration {
    seen: [u64; WORDS],
    last: Option<Position>,
}
impl Default for Exploration {
    fn default() -> Self {
        Self {
            seen: [0; WORDS],
            last: None,
        }
    }
}
impl Exploration {
    pub fn knows(&self, p: Position) -> bool {
        index(p).is_some_and(|i| self.seen[i / 64] & (1u64 << (i % 64)) != 0)
    }
    pub fn count(&self) -> u32 {
        self.seen.iter().map(|w| w.count_ones()).sum()
    }
    fn mark(&mut self, p: Position) {
        if let Some(i) = index(p) {
            self.seen[i / 64] |= 1u64 << (i % 64);
        }
    }
    pub fn cells(&self) -> impl Iterator<Item = Position> + '_ {
        (0..CELLS).map(pos).filter(|&p| self.knows(p))
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut b = b"SH:C60:EXPLORATION:v1\0".to_vec();
        b.extend(VERSION.to_le_bytes());
        b.extend(WIDTH.to_le_bytes());
        b.extend(HEIGHT.to_le_bytes());
        for w in self.seen {
            b.extend(w.to_le_bytes());
        }
        b.push(u8::from(self.last.is_some()));
        if let Some(p) = self.last {
            b.extend(p.x.to_le_bytes());
            b.extend(p.y.to_le_bytes());
        }
        b
    }
}
impl World {
    pub fn exploration(&self) -> Option<&Exploration> {
        self.exploration.as_ref()
    }
    pub fn known_landmarks(&self) -> impl Iterator<Item = Landmark> + '_ {
        Landmark::ALL.into_iter().filter(|l| {
            self.exploration()
                .is_some_and(|e| e.knows(l.position(self)))
        })
    }
    /// Only static geography, never a remembered NPC, corpse, cache or stock.
    pub fn charted_symbol(&self, p: Position) -> Option<char> {
        if !self.exploration()?.knows(p) {
            return None;
        }
        Some(
            Landmark::ALL
                .into_iter()
                .find(|l| l.position(self) == p)
                .map_or_else(
                    || {
                        self.terrain_tile(p)
                            .map_or(if self.walkable(p) { '.' } else { '#' }, |t| {
                                t.terrain.glyph()
                            })
                    },
                    Landmark::glyph,
                ),
        )
    }
    pub(super) fn observe_exploration(&mut self) {
        let Some(e) = self.exploration() else {
            return;
        };
        let p = self.player.position;
        if !self.player.alive() || e.last == Some(p) {
            return;
        }
        let radius = super::SIGHT_RADIUS + 1;
        // Walkable cells must have actual LOS. Opaque edges use exactly the
        // existing normal client rule: a cardinal neighbor in current sight.
        let mut visible = Vec::new();
        for y in p.y.saturating_sub(radius)..=(p.y + radius).min(HEIGHT - 1) {
            for x in p.x.saturating_sub(radius)..=(p.x + radius).min(WIDTH - 1) {
                let cell = Position { x, y };
                if self.player_sees(cell)
                    || (!self.walkable(cell)
                        && super::DIRECTIONS
                            .into_iter()
                            .filter_map(|d| moved(cell, d))
                            .any(|n| index(n).is_some() && self.player_sees(n)))
                {
                    visible.push(cell);
                }
            }
        }
        let e = self.exploration.as_mut().unwrap();
        for cell in visible {
            e.mark(cell);
        }
        e.last = Some(p);
    }
    /// NESW shortest route to an observed landmark or observed adjacent
    /// walkable interaction cell. The empty route means already in reach.
    /// Actor occupancy, hostility and future changes are not predicted.
    pub fn charted_route(&self, to: Landmark) -> Option<Vec<Direction>> {
        let e = self.exploration()?;
        let dest = to.position(self);
        let start = self.player.position;
        if !self.player.alive() || !e.knows(dest) || !e.knows(start) {
            return None;
        }
        let mut came: [Option<(usize, Direction)>; CELLS] = [None; CELLS];
        let start_i = index(start)?;
        came[start_i] = Some((start_i, Direction::North));
        let mut q = VecDeque::from([start]);
        while let Some(p) = q.pop_front() {
            let i = index(p).unwrap();
            if distance(p, dest) <= 1 {
                let mut path = Vec::new();
                let mut at = i;
                while at != start_i {
                    let (prev, d) = came[at].unwrap();
                    path.push(d);
                    at = prev;
                }
                path.reverse();
                return Some(path);
            }
            for d in super::DIRECTIONS {
                let Some(n) = moved(p, d) else {
                    continue;
                };
                let Some(j) = index(n) else {
                    continue;
                };
                if came[j].is_none() && e.knows(n) && self.walkable(n) {
                    came[j] = Some((i, d));
                    q.push_back(n);
                }
            }
        }
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Intent, Outcome};
    fn check_visible(w: &World) {
        let e = w.exploration().unwrap();
        for i in 0..CELLS {
            let p = pos(i);
            let visible = w.player_sees(p)
                || (!w.walkable(p)
                    && super::super::DIRECTIONS
                        .into_iter()
                        .filter_map(|d| moved(p, d))
                        .any(|n| index(n).is_some() && w.player_sees(n)));
            assert_eq!(e.knows(p), visible, "initial visibility {p:?}");
        }
    }
    #[test]
    fn initial_chart_is_exact_and_does_not_reveal_east() {
        for seed in [0, 3, 7, 42] {
            let w = World::sandbox_campaign(seed);
            check_visible(&w);
            assert!(w.exploration().unwrap().count() > 0);
            assert!(!w.exploration().unwrap().knows(DEPOT));
            assert_eq!(w.charted_route(Landmark::Depot), None);
            assert_eq!(w.charted_symbol(DEPOT), None);
            assert!(w.known_landmarks().any(|l| l == Landmark::Shrine));
            assert!(
                !w.exploration()
                    .unwrap()
                    .knows(Position { x: u32::MAX, y: 0 })
            );
        }
    }
    #[test]
    fn chart_retains_only_static_terrain_and_routes_are_read_only() {
        let mut w = World::sandbox_campaign(3);
        let before = w.checkpoint();
        let route = w.charted_route(Landmark::Shrine).unwrap();
        assert_eq!(before, w.checkpoint());
        for d in route {
            assert_eq!(w.step(w.tick(), Intent::Move(d)).unwrap(), Outcome::Moved);
        }
        assert_eq!(w.charted_route(Landmark::Shrine), Some(vec![]));
        let first = w.exploration().unwrap().clone();
        let p = w.player.position;
        for _ in 0..3 {
            w.step(w.tick(), Intent::Move(Direction::East)).unwrap();
        }
        assert!(first.cells().all(|p| w.exploration().unwrap().knows(p)));
        assert!(w.charted_symbol(p).is_some());
        let route = w.charted_route(Landmark::Shrine).unwrap();
        let mut p = w.player.position;
        for d in route {
            p = moved(p, d).unwrap();
            assert!(w.exploration().unwrap().knows(p) && w.walkable(p));
        }
        assert!(distance(p, w.scenario.shrine) <= 1);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
    #[test]
    fn dead_player_and_undiscovered_gaps_cannot_chart_or_route() {
        let mut w = World::sandbox_campaign(3);
        // Controlled geography fixture: disconnected observations.
        let mut e = Exploration::default();
        e.mark(w.player.position);
        e.mark(DEPOT);
        w.exploration = Some(e.clone());
        assert_eq!(w.charted_route(Landmark::Depot), None);
        w.player.hp = 0;
        w.player.position = DEPOT;
        w.observe_exploration();
        assert_eq!(w.exploration(), Some(&e));
        assert!(w.charted_route(Landmark::Depot).is_none());
    }
    #[test]
    fn stale_terrain_never_contains_dynamic_symbols() {
        let mut w = World::sandbox_campaign(3);
        let shrine = w.scenario.shrine;
        let static_glyph = w.charted_symbol(shrine);
        assert_eq!(static_glyph, Some('+'));
        let actor = *w.npcs.keys().next().unwrap();
        w.npcs.get_mut(&actor).unwrap().position = shrine;
        assert_eq!(w.charted_symbol(shrine), static_glyph);
        w.npcs.get_mut(&actor).unwrap().hp = 0;
        assert_eq!(w.charted_symbol(shrine), static_glyph);
    }
    #[test]
    fn charted_revision_preserves_every_other_simulation_field() {
        for seed in [0u32, 3, 7, 42] {
            let mut w = World::sandbox_campaign(seed);
            let mut old = World::governance_campaign(seed);
            let mut rng = u64::from(seed) + 1;
            for tick in 0..3000 {
                rng = rng
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let actions = [
                    Intent::Move(Direction::North),
                    Intent::Move(Direction::East),
                    Intent::Move(Direction::South),
                    Intent::Move(Direction::West),
                    Intent::Wait,
                    Intent::Gather,
                    Intent::Deliver,
                    Intent::Practice,
                    Intent::BuyHerb,
                    Intent::SellHerb,
                    Intent::Talk(super::super::ActorId::new(2)),
                ];
                let action = actions[((rng >> 32) as usize) % actions.len()];
                assert_eq!(
                    w.step(w.tick(), action),
                    old.step(old.tick(), action),
                    "seed {seed} tick {tick}"
                );
                if tick % 128 == 0 || tick == 2999 {
                    let mut projected = w.clone();
                    projected.rules = super::super::Rules::CommunityGovernance;
                    projected.exploration = None;
                    assert_eq!(
                        projected, old,
                        "only the chart may differ at seed {seed} tick {tick}"
                    );
                }
            }
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            println!(
                "seed {seed}: 3000 equivalent ticks, {} charted cells",
                w.exploration().unwrap().count()
            );
        }
    }
}
