//! C4.0: immutable seeded regional terrain. The authored v1 settlement
//! anchors remain canonical; the corridor, river, crossings and ecology vary.
//! All arithmetic is integer and all random samples have isolated domains.
use std::collections::VecDeque;

use super::{Position, Region, Rules, Scenario, content, qinghe};
use crate::encoding::sha256;

pub const GENERATOR_VERSION: u16 = 1;
pub const GENERATED_REVISION: u16 = 27;
pub const BASE_REVISION: u16 = 26;
pub const WIDTH: u32 = 30;
pub const HEIGHT: u32 = 9;
const CELLS: usize = (WIDTH * HEIGHT) as usize;
const WEST: u32 = 11;
const EAST: u32 = 21;
const WEST_GATE: Position = Position { x: WEST, y: 4 };
const EAST_GATE: Position = Position { x: EAST, y: 4 };

const GEOGRAPHY: u64 = 0x2dd3_90e9_a539_11b1;
const HYDROLOGY: u64 = 0x85d1_6137_2566_bc2f;
const ECOLOGY: u64 = 0xa064_77cf_521e_6a8b;
const ROADS: u64 = 0x6bfa_390e_8844_3e91;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Terrain {
    Ground = 0,
    Wall = 1,
    Meadow = 2,
    Woodland = 3,
    Rock = 4,
    River = 5,
    Road = 6,
    Bridge = 7,
}
impl Terrain {
    pub const fn walkable(self) -> bool {
        !matches!(self, Self::Wall | Self::Rock | Self::River)
    }
    pub const fn glyph(self) -> char {
        match self {
            Self::Ground => '.',
            Self::Wall | Self::Rock => '#',
            Self::Meadow => ',',
            Self::Woodland => ';',
            Self::River => '~',
            Self::Road => ':',
            Self::Bridge => '%',
        }
    }
    pub const fn id(self) -> &'static str {
        match self {
            Self::Ground => "ground",
            Self::Wall => "wall",
            Self::Meadow => "meadow",
            Self::Woodland => "woodland",
            Self::Rock => "rock",
            Self::River => "river",
            Self::Road => "road",
            Self::Bridge => "bridge",
        }
    }
}

/// Quantized relative environmental fields, 0..=100. Not physical units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tile {
    pub terrain: Terrain,
    pub elevation: u8,
    pub moisture: u8,
    pub qi: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageDigests {
    pub geography: [u8; 32],
    pub hydrology: [u8; 32],
    pub ecology: [u8; 32],
    pub roads: [u8; 32],
    pub world: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedRegion {
    seed: u32,
    tiles: [Tile; CELLS],
    river: [u8; HEIGHT as usize],
    bridges: [Position; 2],
    west_regrow: u64,
    garden_regrow: u64,
    road_distance: u16,
    digests: StageDigests,
}

fn pos(x: u32, y: u32) -> Position {
    Position { x, y }
}

fn index(p: Position) -> Option<usize> {
    if p.x < WIDTH && p.y < HEIGHT {
        Some((p.y * WIDTH + p.x) as usize)
    } else {
        None
    }
}

fn position(i: usize) -> Position {
    pos(i as u32 % WIDTH, i as u32 / WIDTH)
}

/// SplitMix64 finalizer; a coordinate hash rather than a shared RNG cursor.
/// Explicit wrapping is part of the frozen generator-v1 contract.
fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

pub(super) fn sample(seed: u32, domain: u64, x: u32, y: u32) -> u64 {
    mix(domain
        ^ u64::from(seed).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (u64::from(x) << 32)
        ^ u64::from(y))
}

pub(super) fn field(seed: u32, domain: u64, x: u32, y: u32) -> u8 {
    // Bilinear interpolation of coarse integer lattice samples.
    const SX: u32 = 4;
    const SY: u32 = 3;
    let (gx, gy, dx, dy) = (x / SX, y / SY, x % SX, y % SY);
    let lattice = |a, b| (sample(seed, domain, a, b) % 101) as u32;
    let low = lattice(gx, gy) * (SX - dx) + lattice(gx + 1, gy) * dx;
    let high = lattice(gx, gy + 1) * (SX - dx) + lattice(gx + 1, gy + 1) * dx;
    ((low * (SY - dy) + high * dy) / (SX * SY)) as u8
}

fn stage_digest(domain: &[u8], seed: u32, payload: &[u8]) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(domain.len() + 6 + payload.len());
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(&GENERATOR_VERSION.to_le_bytes());
    bytes.extend_from_slice(&seed.to_le_bytes());
    bytes.extend_from_slice(payload);
    sha256(&bytes)
}

fn neighbors(p: Position) -> impl Iterator<Item = Position> {
    [
        p.y.checked_sub(1).map(|y| pos(p.x, y)),
        p.x.checked_add(1).map(|x| pos(x, p.y)),
        p.y.checked_add(1).map(|y| pos(p.x, y)),
        p.x.checked_sub(1).map(|x| pos(x, p.y)),
    ]
    .into_iter()
    .flatten()
    .filter(|p| index(*p).is_some())
}

fn distances(tiles: &[Tile; CELLS], start: Position) -> [u16; CELLS] {
    let mut distance = [u16::MAX; CELLS];
    let Some(i) = index(start).filter(|&i| tiles[i].terrain.walkable()) else {
        return distance;
    };
    distance[i] = 0;
    let mut queue = VecDeque::from([start]);
    while let Some(p) = queue.pop_front() {
        let next_distance = distance[index(p).unwrap()] + 1;
        for n in neighbors(p) {
            let i = index(n).unwrap();
            if distance[i] == u16::MAX && tiles[i].terrain.walkable() {
                distance[i] = next_distance;
                queue.push_back(n);
            }
        }
    }
    distance
}

fn cadence(qi: u8, fastest: u64) -> u64 {
    fastest + 4 * u64::from((100 - qi).min(99) / 25)
}

impl GeneratedRegion {
    /// Pure generation from the released content pack and a u32 seed.
    pub fn generate(seed: u32) -> Self {
        Self::from_scenario(seed, qinghe::region(seed, Rules::DutyCoherence))
    }

    pub(super) fn from_scenario(seed: u32, scenario: &Scenario) -> Self {
        Self::generate_with_ecology(seed, scenario, ECOLOGY)
    }

    fn generate_with_ecology(seed: u32, scenario: &Scenario, ecology_domain: u64) -> Self {
        assert_eq!((scenario.width, scenario.height), (WIDTH, HEIGHT));
        let mut tiles = std::array::from_fn(|i| {
            let p = position(i);
            Tile {
                terrain: Terrain::Ground,
                elevation: field(seed, GEOGRAPHY, p.x, p.y),
                moisture: 0,
                qi: 0,
            }
        });
        let elevation: Vec<_> = tiles.iter().map(|t| t.elevation).collect();
        let geography = stage_digest(b"SH:C40:GEO\0", seed, &elevation);

        // A continuous north-south barrier (8-connected water blocks
        // 4-connected land). Crossings are deliberately carved later.
        let mut river = [0u8; HEIGHT as usize];
        let mut x = 15 + (sample(seed, HYDROLOGY, 0, 0) % 3) as i32;
        for y in 0..HEIGHT {
            if y != 0 {
                x = (x + (sample(seed, HYDROLOGY, y, 1) % 3) as i32 - 1).clamp(15, 17);
            }
            river[y as usize] = x as u8;
        }
        let hydrology = stage_digest(b"SH:C40:HYDRO\0", seed, &river);

        for (i, tile) in tiles.iter_mut().enumerate() {
            let p = position(i);
            let river_x = u32::from(river[p.y as usize]);
            let river_distance = p.x.abs_diff(river_x).min(10);
            let humidity = field(seed, ecology_domain ^ 0x71f9, p.x, p.y);
            tile.moisture = ((u32::from(humidity) * 3 + (100 - 10 * river_distance)) / 4) as u8;
            let raw_qi = field(seed, ecology_domain, p.x, p.y);
            tile.qi = ((u32::from(raw_qi) * 3 + u32::from(tile.moisture)) / 4) as u8;
            tile.terrain = if p.x <= WEST || p.x >= EAST {
                // Keep settlement layout, paths, NPC homes and sites immutable.
                if scenario.walkable(p) {
                    Terrain::Ground
                } else {
                    Terrain::Wall
                }
            } else if p.x == river_x {
                Terrain::River
            } else if tile.elevation >= 63 {
                Terrain::Rock
            } else if tile.moisture >= 56 {
                Terrain::Woodland
            } else {
                Terrain::Meadow
            };
        }
        let ecology_bytes: Vec<_> = tiles.iter().flat_map(|t| [t.moisture, t.qi]).collect();
        let ecology = stage_digest(b"SH:C40:ECO\0", seed, &ecology_bytes);

        // Two crossings separated by at least three rows prevent a single
        // traffic choke from being the only static route. Stable road stream.
        let first = 1 + (sample(seed, ROADS, 0, 0) % 7) as u32;
        let alternatives: Vec<_> = (1..=7u32).filter(|&y| y.abs_diff(first) >= 3).collect();
        let second = alternatives[(sample(seed, ROADS, 1, 0) % alternatives.len() as u64) as usize];
        let mut rows = [first, second];
        rows.sort_unstable();
        let bridges = rows.map(|y| pos(u32::from(river[y as usize]), y));
        for y in rows {
            // Don't recolor authored settlement cells; they already supply the
            // path between y=4 and each corridor entry.
            for x in WEST + 1..EAST {
                let t = &mut tiles[index(pos(x, y)).unwrap()];
                t.terrain = if x == u32::from(river[y as usize]) {
                    Terrain::Bridge
                } else {
                    Terrain::Road
                };
            }
        }
        let mut road_bytes = Vec::new();
        for bridge in bridges {
            road_bytes.extend_from_slice(&[bridge.x as u8, bridge.y as u8]);
        }
        for (i, t) in tiles.iter().enumerate() {
            if matches!(t.terrain, Terrain::Road | Terrain::Bridge) {
                road_bytes.extend_from_slice(&(i as u16).to_le_bytes());
                road_bytes.push(t.terrain as u8);
            }
        }
        let roads = stage_digest(b"SH:C40:ROADS\0", seed, &road_bytes);

        // Close isolated non-authored land pockets; never change protected
        // cells or create a third crossing while repairing connectivity.
        let connected = distances(&tiles, scenario.entrance);
        for (i, tile) in tiles.iter_mut().enumerate() {
            let p = position(i);
            if WEST < p.x && p.x < EAST && tile.terrain.walkable() && connected[i] == u16::MAX {
                tile.terrain = Terrain::Rock;
            }
        }
        let west_regrow = cadence(tiles[index(scenario.patch).unwrap()].qi, 8);
        let garden_regrow = cadence(tiles[index(Region::sites().garden).unwrap()].qi, 12);
        let road_distance = distances(&tiles, WEST_GATE)[index(EAST_GATE).unwrap()];

        // Fixed byte ordering, explicit enum tags, integer little-endian
        // parameters and exact authored-content fingerprint. No Rust layout
        // bytes, platform-sized integers, locale, clocks or unordered maps.
        let mut canonical = Vec::with_capacity(4 * CELLS + 256);
        canonical.extend_from_slice(content::CONTENT_SHA256.as_bytes());
        canonical.extend_from_slice(&WIDTH.to_le_bytes());
        canonical.extend_from_slice(&HEIGHT.to_le_bytes());
        for tile in &tiles {
            canonical.extend_from_slice(&[
                tile.terrain as u8,
                tile.elevation,
                tile.moisture,
                tile.qi,
            ]);
        }
        canonical.extend_from_slice(&river);
        canonical.extend_from_slice(&road_bytes);
        canonical.extend_from_slice(&west_regrow.to_le_bytes());
        canonical.extend_from_slice(&garden_regrow.to_le_bytes());
        canonical.extend_from_slice(&road_distance.to_le_bytes());
        for stage in [geography, hydrology, ecology, roads] {
            canonical.extend_from_slice(&stage);
        }
        let world = stage_digest(b"SH:C40:WORLD\0", seed, &canonical);
        let result = Self {
            seed,
            tiles,
            river,
            bridges,
            west_regrow,
            garden_regrow,
            road_distance,
            digests: StageDigests {
                geography,
                hydrology,
                ecology,
                roads,
                world,
            },
        };
        assert!(
            result.valid(scenario),
            "generator violated authored topology"
        );
        result
    }

    fn valid(&self, scenario: &Scenario) -> bool {
        let connected = distances(&self.tiles, scenario.entrance);
        let reachable = |p| index(p).is_some_and(|i| connected[i] != u16::MAX);
        if ![
            scenario.entrance,
            scenario.patch,
            scenario.shrine,
            scenario.clinic,
            WEST_GATE,
            EAST_GATE,
        ]
        .into_iter()
        .chain(Region::sites().positions())
        .all(reachable)
            || !scenario.npcs.iter().all(|npc| {
                reachable(npc.spawn)
                    && reachable(npc.home)
                    && npc.route.iter().copied().all(reachable)
            })
            || !content::LOTS.iter().all(|&(_, _, p, _)| reachable(p))
            || self.road_distance == u16::MAX
            || self.bridges[0].y.abs_diff(self.bridges[1].y) < 3
        {
            return false;
        }
        for (i, tile) in self.tiles.iter().enumerate() {
            let p = position(i);
            let protected = p.x <= WEST || p.x >= EAST;
            let on_river = p.x == u32::from(self.river[p.y as usize]);
            if (protected && tile.terrain.walkable() != scenario.walkable(p))
                || (tile.terrain.walkable() && connected[i] == u16::MAX)
                || (on_river != matches!(tile.terrain, Terrain::River | Terrain::Bridge))
                || (self.bridges.contains(&p) != (tile.terrain == Terrain::Bridge))
                || [tile.elevation, tile.moisture, tile.qi]
                    .into_iter()
                    .any(|v| v > 100)
            {
                return false;
            }
        }
        true
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }
    pub fn tile(&self, p: Position) -> Option<Tile> {
        index(p).map(|i| self.tiles[i])
    }
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
    pub fn walkable(&self, p: Position) -> bool {
        self.tile(p).is_some_and(|t| t.terrain.walkable())
    }
    pub fn river(&self) -> &[u8; HEIGHT as usize] {
        &self.river
    }
    pub fn bridges(&self) -> [Position; 2] {
        self.bridges
    }
    pub fn west_regrow_every(&self) -> u64 {
        self.west_regrow
    }
    pub fn garden_regrow_every(&self) -> u64 {
        self.garden_regrow
    }
    /// Shortest static path between the two authored settlement gates.
    pub fn gate_distance(&self) -> u16 {
        self.road_distance
    }
    pub fn digests(&self) -> StageDigests {
        self.digests
    }
    pub fn shortest_path(&self, from: Position, to: Position) -> Option<u16> {
        index(to).and_then(|i| {
            let distance = distances(&self.tiles, from)[i];
            (distance != u16::MAX).then_some(distance)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecology_stream_does_not_reroll_geography_river_roads_or_collision() {
        for seed in [0, 1, 7, 255, u32::MAX] {
            let scenario = qinghe::region(seed, Rules::DutyCoherence);
            let a = GeneratedRegion::generate(seed);
            let b = GeneratedRegion::generate_with_ecology(seed, scenario, ECOLOGY ^ 123);
            assert_eq!(a.digests.geography, b.digests.geography);
            assert_eq!(a.digests.hydrology, b.digests.hydrology);
            assert_eq!(a.digests.roads, b.digests.roads);
            assert_ne!(a.digests.ecology, b.digests.ecology);
            assert_ne!(a.digests.world, b.digests.world);
            assert!(
                a.tiles
                    .iter()
                    .zip(b.tiles)
                    .all(|(a, b)| a.terrain.walkable() == b.terrain.walkable())
            );
        }
    }
}
