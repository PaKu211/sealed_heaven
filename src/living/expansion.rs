//! C4.1: one contiguous world, an immutable eastern overlay and physical
//! freight. The historical C4.0 map is copied byte-for-byte into the west.
use super::{
    ActorId, Background, Goal, MarketQuote, NpcDefinition, Outcome, Position, Region, Rejection,
    Role, Scenario, World, content, navigation,
    worldgen::{self, GeneratedRegion, Terrain, Tile},
};
use crate::encoding::sha256;
use std::collections::VecDeque;

pub const REVISION: u16 = 28;
pub const GENERATOR_VERSION: u16 = 1;
pub const CARRIER: ActorId = ActorId::new(37);
pub const GROWER: ActorId = ActorId::new(38);
include!(concat!(env!("OUT_DIR"), "/living_expansion.rs"));

const CELLS: usize = (WIDTH * HEIGHT) as usize;
const TERRAIN_DOMAIN: u64 = 0x3194_d2c8_1ba6_7f05;
const WATER_DOMAIN: u64 = 0x537d_c4f9_e6a2_1b08;
const ECO_DOMAIN: u64 = 0xb514_9de8_03f0_6a73;
const ROAD_DOMAIN: u64 = 0x4ca7_9b52_e1f8_d603;
const ENTRY: Position = Position { x: 29, y: 4 };
const EAST_GATE: Position = Position { x: 43, y: 7 };

const fn actors(base: &Scenario) -> [NpcDefinition; 12] {
    assert!(base.npcs.len() == 10);
    let mut result = [EXTRA_NPCS[0]; 12];
    let mut i = 0;
    while i < 10 {
        result[i] = base.npcs[i];
        i += 1;
    }
    result[10] = EXTRA_NPCS[0];
    result[11] = EXTRA_NPCS[1];
    result
}
static RIDGE_ACTORS: [NpcDefinition; 12] = actors(&content::SCENARIO_RIDGE_POLITICS);
static LOWER_ACTORS: [NpcDefinition; 12] = actors(&content::SCENARIO_LOWER_POLITICS);
static RIDGE: Scenario = Scenario {
    width: WIDTH,
    height: HEIGHT,
    terrain: TERRAIN_RIDGE,
    npcs: &RIDGE_ACTORS,
    ..content::SCENARIO_RIDGE_POLITICS
};
static LOWER: Scenario = Scenario {
    width: WIDTH,
    height: HEIGHT,
    terrain: TERRAIN_LOWER,
    npcs: &LOWER_ACTORS,
    ..content::SCENARIO_LOWER_POLITICS
};
pub(super) fn scenario(seed: u32) -> &'static Scenario {
    if seed & 1 == 0 { &RIDGE } else { &LOWER }
}

pub fn manifest_json() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/living_expansion.json"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedRegion {
    seed: u32,
    tiles: [Tile; CELLS],
    river: [u8; HEIGHT as usize],
    bridges: [Position; 2],
    grove_regrow: u64,
    road_distance: u16,
    digest: [u8; 32],
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
fn distances(tiles: &[Tile; CELLS], start: Position) -> [u16; CELLS] {
    let mut result = [u16::MAX; CELLS];
    let Some(i) = index(start).filter(|&i| tiles[i].terrain.walkable()) else {
        return result;
    };
    let mut q = VecDeque::from([start]);
    result[i] = 0;
    while let Some(p) = q.pop_front() {
        let distance = result[index(p).unwrap()] + 1;
        for d in super::DIRECTIONS {
            let Some(n) = navigation::moved(p, d) else {
                continue;
            };
            let Some(i) = index(n) else {
                continue;
            };
            if result[i] == u16::MAX && tiles[i].terrain.walkable() {
                result[i] = distance;
                q.push_back(n);
            }
        }
    }
    result
}
impl ExpandedRegion {
    pub fn generate(seed: u32) -> Self {
        Self::from_base(seed, &GeneratedRegion::generate(seed))
    }
    pub(super) fn from_base(seed: u32, base: &GeneratedRegion) -> Self {
        assert_eq!(base.seed(), seed);
        let mut river = [0; HEIGHT as usize];
        let mut x = 36 + (worldgen::sample(seed, WATER_DOMAIN, 0, 0) % 3) as i32;
        for y in 0..HEIGHT {
            if y > 0 {
                x = (x + (worldgen::sample(seed, WATER_DOMAIN, y, 1) % 3) as i32 - 1).clamp(36, 38);
            }
            river[y as usize] = x as u8;
        }
        let mut tiles = std::array::from_fn(|i| {
            let p = position(i);
            if p.x < worldgen::WIDTH {
                return base.tile(p).unwrap_or(Tile {
                    terrain: Terrain::Wall,
                    elevation: 0,
                    moisture: 0,
                    qi: 0,
                });
            }
            let elevation = worldgen::field(seed, TERRAIN_DOMAIN, p.x, p.y);
            let distance = p.x.abs_diff(u32::from(river[p.y as usize])).min(10);
            let moisture = ((u32::from(worldgen::field(seed, ECO_DOMAIN ^ 0x7423, p.x, p.y)) * 3
                + 100
                - 10 * distance)
                / 4) as u8;
            let qi = ((u32::from(worldgen::field(seed, ECO_DOMAIN, p.x, p.y)) * 3
                + u32::from(moisture))
                / 4) as u8;
            let terrain = if p.x >= 43 {
                Terrain::Ground
            } else if p.x == u32::from(river[p.y as usize]) {
                Terrain::River
            } else if elevation >= 63 {
                Terrain::Rock
            } else if moisture >= 56 {
                Terrain::Woodland
            } else {
                Terrain::Meadow
            };
            Tile {
                terrain,
                elevation,
                moisture,
                qi,
            }
        });
        let first = 1 + (worldgen::sample(seed, ROAD_DOMAIN, 0, 0) % 13) as u32;
        let other: Vec<_> = (1..=13u32).filter(|&y| y.abs_diff(first) >= 5).collect();
        let second =
            other[(worldgen::sample(seed, ROAD_DOMAIN, 1, 0) % other.len() as u64) as usize];
        let mut rows = [first, second];
        rows.sort_unstable();
        let bridges = rows.map(|y| pos(u32::from(river[y as usize]), y));
        // Two eastern crossings; their feeder roads connect the Willowford
        // boundary to the authored Pineford town. Never edit western cells.
        for y in rows {
            for x in 30..43 {
                tiles[index(pos(x, y)).unwrap()].terrain = if x == u32::from(river[y as usize]) {
                    Terrain::Bridge
                } else {
                    Terrain::Road
                };
            }
        }
        for y in 0..HEIGHT {
            for x in [31, 42] {
                tiles[index(pos(x, y)).unwrap()].terrain = Terrain::Road;
            }
        }
        for x in 30..=31 {
            tiles[index(pos(x, 4)).unwrap()].terrain = Terrain::Road;
        }
        let connected = distances(&tiles, ENTRY);
        for (i, t) in tiles.iter_mut().enumerate() {
            if position(i).x >= 30 && t.terrain.walkable() && connected[i] == u16::MAX {
                t.terrain = Terrain::Rock;
            }
        }
        let grove_regrow = REGROW_FASTEST
            + REGROW_STEP * u64::from((100 - tiles[index(GROVE).unwrap()].qi).min(99) / 25);
        let road_distance = distances(&tiles, ENTRY)[index(EAST_GATE).unwrap()];
        let mut bytes = b"SH:C41:WORLD:v1\0".to_vec();
        bytes.extend_from_slice(&GENERATOR_VERSION.to_le_bytes());
        bytes.extend_from_slice(&seed.to_le_bytes());
        bytes.extend_from_slice(content::CONTENT_SHA256.as_bytes());
        bytes.extend_from_slice(CONTENT_SHA256.as_bytes());
        bytes.extend_from_slice(&base.digests().world);
        bytes.extend_from_slice(&WIDTH.to_le_bytes());
        bytes.extend_from_slice(&HEIGHT.to_le_bytes());
        for t in &tiles {
            bytes.extend_from_slice(&[t.terrain as u8, t.elevation, t.moisture, t.qi]);
        }
        bytes.extend_from_slice(&river);
        for p in bridges {
            bytes.extend_from_slice(&p.x.to_le_bytes());
            bytes.extend_from_slice(&p.y.to_le_bytes());
        }
        bytes.extend_from_slice(&grove_regrow.to_le_bytes());
        bytes.extend_from_slice(&road_distance.to_le_bytes());
        let digest = sha256(&bytes);
        let result = Self {
            seed,
            tiles,
            river,
            bridges,
            grove_regrow,
            road_distance,
            digest,
        };
        assert!(
            result.valid(base, scenario(seed)),
            "invalid eastern expansion topology"
        );
        result
    }
    fn valid(&self, base: &GeneratedRegion, scenario: &Scenario) -> bool {
        let connected = distances(&self.tiles, scenario.entrance);
        let reachable = |p| index(p).is_some_and(|i| connected[i] != u16::MAX);
        if self.road_distance == u16::MAX
            || self.bridges[0].y.abs_diff(self.bridges[1].y) < 5
            || ![ENTRY, EAST_GATE, GROVE, DEPOT].into_iter().all(reachable)
            || !scenario.npcs.iter().all(|npc| {
                reachable(npc.spawn)
                    && reachable(npc.home)
                    && npc.route.iter().copied().all(reachable)
            })
        {
            return false;
        }
        for (i, tile) in self.tiles.iter().enumerate() {
            let p = position(i);
            if tile.terrain.walkable() && connected[i] == u16::MAX {
                return false;
            }
            if p.x < worldgen::WIDTH {
                if let Some(t) = base.tile(p) {
                    if *tile != t {
                        return false;
                    }
                } else if tile.terrain != Terrain::Wall {
                    return false;
                }
            } else {
                let water = p.x == u32::from(self.river[p.y as usize]);
                if water != matches!(tile.terrain, Terrain::River | Terrain::Bridge)
                    || self.bridges.contains(&p) != (tile.terrain == Terrain::Bridge)
                {
                    return false;
                }
            }
        }
        true
    }
    pub fn seed(&self) -> u32 {
        self.seed
    }
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
    pub fn tile(&self, p: Position) -> Option<Tile> {
        index(p).map(|i| self.tiles[i])
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
    pub fn grove_regrow_every(&self) -> u64 {
        self.grove_regrow
    }
    pub fn gate_distance(&self) -> u16 {
        self.road_distance
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn shortest_path(&self, from: Position, to: Position) -> Option<u16> {
        index(to).and_then(|i| {
            let d = distances(&self.tiles, from)[i];
            (d != u16::MAX).then_some(d)
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frontier {
    pub grove_herbs: u32,
    pub depot_herbs: u32,
    pub depot_coins: u32,
    pub regrown: u32,
    pub stocked: u32,
    pub shipped_to_willowford: u32,
    pub player_bought: u32,
    pub player_sold: u32,
}
impl Frontier {
    pub(super) fn new() -> Self {
        Self {
            grove_herbs: GROVE_CAPACITY,
            depot_herbs: DEPOT_INITIAL,
            depot_coins: COIN_BUDGET,
            regrown: 0,
            stocked: 0,
            shipped_to_willowford: 0,
            player_bought: 0,
            player_sold: 0,
        }
    }
    pub fn quote(self) -> MarketQuote {
        let scarce = self.depot_herbs <= DEPOT_RESERVE;
        MarketQuote {
            buy: if scarce { SCARCE_BUY } else { NORMAL_BUY },
            sell: if scarce { SCARCE_SELL } else { NORMAL_SELL },
        }
    }
}

impl World {
    pub fn expanded_region(&self) -> Option<&ExpandedRegion> {
        self.expanded.as_deref()
    }
    pub fn frontier(&self) -> Option<&Frontier> {
        self.frontier.as_ref()
    }
    pub fn terrain_tile(&self, p: Position) -> Option<Tile> {
        if let Some(map) = self.expanded_region() {
            map.tile(p)
        } else {
            self.generated_region()?.tile(p)
        }
    }
    pub(super) fn advance_frontier(&mut self) {
        let Some(cadence) = self
            .expanded_region()
            .map(ExpandedRegion::grove_regrow_every)
        else {
            return;
        };
        let f = self.frontier.as_mut().expect("expanded rules");
        if self.tick.is_multiple_of(cadence) && f.grove_herbs < GROVE_CAPACITY {
            f.grove_herbs += 1;
            f.regrown += 1;
        }
    }
    pub(super) fn gather_frontier(&mut self) -> Result<Outcome, Rejection> {
        self.require_reach(GROVE)?;
        let limit = if self.region.as_ref().unwrap().background == Some(Background::Herbalist) {
            2
        } else {
            1
        };
        let f = self.frontier.as_mut().expect("expanded rules");
        let count = f.grove_herbs.min(limit);
        if count == 0 {
            return Err(Rejection::Empty);
        }
        self.player.herbs = self
            .player
            .herbs
            .checked_add(count)
            .ok_or(Rejection::PackFull)?;
        f.grove_herbs -= count;
        self.record_regional_event(
            super::regional_news::Site::PinefordGrove,
            super::regional_news::EventKind::Harvested,
            Some(super::PLAYER),
            count,
        );
        Ok(if count == 2 {
            Outcome::GatheredPair
        } else {
            Outcome::Gathered
        })
    }
    pub(super) fn trade_frontier(&mut self, buying: bool) -> Result<Outcome, Rejection> {
        self.require_reach(DEPOT)?;
        if self.is_night() {
            return Err(Rejection::MarketClosed);
        }
        let f = self.frontier.as_mut().expect("expanded rules");
        let r = self.region.as_mut().expect("regional rules");
        let quote = f.quote();
        let coins = if buying {
            if f.depot_herbs == 0 {
                return Err(Rejection::Empty);
            }
            if r.coins < quote.buy {
                return Err(Rejection::NotEnoughCoins);
            }
            let herbs = self
                .player
                .herbs
                .checked_add(1)
                .ok_or(Rejection::PackFull)?;
            let treasury = f
                .depot_coins
                .checked_add(quote.buy)
                .ok_or(Rejection::PackFull)?;
            f.depot_herbs -= 1;
            self.player.herbs = herbs;
            r.coins -= quote.buy;
            f.depot_coins = treasury;
            f.player_bought += 1;
            quote.buy
        } else {
            if self.player.herbs == 0 {
                return Err(Rejection::NoCargo);
            }
            if f.depot_coins < quote.sell {
                return Err(Rejection::NotEnoughCoins);
            }
            let stock = f.depot_herbs.checked_add(1).ok_or(Rejection::PackFull)?;
            let purse = r.coins.checked_add(quote.sell).ok_or(Rejection::PackFull)?;
            self.player.herbs -= 1;
            f.depot_herbs = stock;
            f.depot_coins -= quote.sell;
            r.coins = purse;
            f.player_sold += 1;
            quote.sell
        };
        self.record_regional_event(
            super::regional_news::Site::PinefordDepot,
            if buying {
                super::regional_news::EventKind::Bought
            } else {
                super::regional_news::EventKind::Sold
            },
            Some(super::PLAYER),
            1,
        );
        Ok(Outcome::Traded {
            bought: buying,
            coins,
        })
    }
    pub(super) fn frontier_duty(&self, npc: &super::Npc) -> Goal {
        if self.regional_news().is_some() {
            return self.situated_frontier_duty(npc);
        }
        let f = self
            .frontier()
            .expect("frontier actor only in expanded rules");
        match npc.definition.role {
            Role::FrontierGrower if npc.herbs > 0 => Goal::StockFrontierDepot,
            Role::FrontierGrower if f.grove_herbs > 0 && f.depot_herbs < DEPOT_TARGET => {
                Goal::GatherFrontier
            }
            Role::FrontierCarrier if npc.herbs > 0 => Goal::DeliverFrontierFreight,
            Role::FrontierCarrier
                if f.depot_herbs > DEPOT_RESERVE
                    && self.region.as_ref().unwrap().market_herbs
                        < content::MARKET_TARGET + WILLOWFORD_BUFFER =>
            {
                Goal::CollectFrontierFreight
            }
            _ => Goal::Idle,
        }
    }
    pub(super) fn work_frontier(&mut self, id: ActorId, goal: Goal) {
        let Some(npc) = self.npcs.get(&id).filter(|npc| npc.alive()) else {
            return;
        };
        // A stale plan cannot load a dead actor or use another actor's role.
        if !matches!(
            (npc.definition.role, goal),
            (
                Role::FrontierGrower,
                Goal::GatherFrontier | Goal::StockFrontierDepot
            ) | (
                Role::FrontierCarrier,
                Goal::CollectFrontierFreight | Goal::DeliverFrontierFreight
            )
        ) {
            return;
        }
        if npc.herbs > 0 && matches!(goal, Goal::GatherFrontier | Goal::CollectFrontierFreight) {
            return;
        }

        let site = match goal {
            Goal::GatherFrontier => GROVE,
            Goal::StockFrontierDepot | Goal::CollectFrontierFreight => DEPOT,
            Goal::DeliverFrontierFreight => Region::sites().market,
            _ => unreachable!(),
        };
        if !navigation::in_reach(self.npcs[&id].position, site) {
            self.travel(id, site, 1);
            return;
        }
        use super::regional_news::{EventKind, Site};
        // Only the local source is read at commit. The destination quota in
        // revision 29 comes from the carrier's own fresh observation/report.
        let situated = self.regional_news().is_some();
        let known_depot = self.known_stock(id, Site::PinefordDepot);
        let known_market = self.known_stock(id, Site::WillowfordMarket);
        let npc = self.npcs.get_mut(&id).expect("frontier actor");
        let f = self.frontier.as_mut().expect("expanded rules");
        let r = self.region.as_mut().expect("regional rules");
        // All transfers are commit-time, from one physical inventory to another.
        // A shipment already loaded is delivered even if demand has since changed.
        let event = match goal {
            Goal::GatherFrontier => {
                let demand = if situated {
                    known_depot.map_or(0, |stock| DEPOT_TARGET.saturating_sub(stock))
                } else {
                    DEPOT_TARGET.saturating_sub(f.depot_herbs)
                };
                let count = f.grove_herbs.min(CARAVAN_LOAD).min(demand);
                f.grove_herbs -= count;
                npc.herbs += count;
                (Site::PinefordGrove, EventKind::Harvested, count)
            }
            Goal::StockFrontierDepot => {
                f.depot_herbs += npc.herbs;
                f.stocked += npc.herbs;
                let count = npc.herbs;
                npc.herbs = 0;
                (Site::PinefordDepot, EventKind::DepotStocked, count)
            }
            Goal::CollectFrontierFreight => {
                let target = content::MARKET_TARGET + WILLOWFORD_BUFFER;
                let demand = if situated {
                    known_market.map_or(0, |stock| target.saturating_sub(stock))
                } else {
                    target.saturating_sub(r.market_herbs)
                };
                let count = f
                    .depot_herbs
                    .saturating_sub(DEPOT_RESERVE)
                    .min(CARAVAN_LOAD)
                    .min(demand);
                f.depot_herbs -= count;
                npc.herbs += count;
                (Site::PinefordDepot, EventKind::FreightLoaded, count)
            }
            Goal::DeliverFrontierFreight => {
                r.market_herbs += npc.herbs;
                f.shipped_to_willowford += npc.herbs;
                let count = npc.herbs;
                npc.herbs = 0;
                (Site::WillowfordMarket, EventKind::FreightArrived, count)
            }
            _ => unreachable!(),
        };
        if event.2 > 0 {
            self.record_regional_event(event.0, event.1, Some(id), event.2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Intent, PLAYER};

    #[test]
    fn loaded_caravan_death_leaves_physical_cargo_and_no_automatic_delivery() {
        // A targeted domain fixture (not an event-sourced save).
        let mut w = World::expanded_riverlands(7);
        w.tick = 1;
        w.player.position = Position { x: 45, y: 7 };
        w.npcs.get_mut(&CARRIER).unwrap().position = DEPOT;
        let start = w.frontier().unwrap().depot_herbs;
        let market = w.region().unwrap().market_herbs;
        w.work_frontier(CARRIER, Goal::CollectFrontierFreight);
        let cargo = w.npcs[&CARRIER].herbs;
        assert!(cargo > 0);
        assert_eq!(w.frontier().unwrap().depot_herbs, start - cargo);
        w.work_frontier(CARRIER, Goal::CollectFrontierFreight);
        assert_eq!(w.npcs[&CARRIER].herbs, cargo);
        w.damage(PLAYER, CARRIER, 100, false);
        assert!(!w.npcs[&CARRIER].alive());
        let before = w.frontier().unwrap().shipped_to_willowford;
        w.work_frontier(CARRIER, Goal::DeliverFrontierFreight);
        assert_eq!(w.frontier().unwrap().shipped_to_willowford, before);
        assert_eq!(w.region().unwrap().market_herbs, market);
        assert_eq!(w.npcs[&CARRIER].herbs, cargo);
        let player = w.player.herbs;
        assert_eq!(
            w.apply_player(Intent::Take(CARRIER)),
            Ok(Outcome::Took {
                from: CARRIER,
                herbs: cargo
            })
        );
        assert_eq!(w.player.herbs, player + cargo);
        assert_eq!(w.npcs[&CARRIER].herbs, 0);
        assert_eq!(w.apply_player(Intent::Take(CARRIER)), Err(Rejection::Empty));
    }

    #[test]
    fn frontier_revalidates_reach_reserve_role_and_atomic_rejections() {
        let mut w = World::expanded_riverlands(0);
        let original = *w.frontier().unwrap();
        assert_eq!(w.trade_frontier(true), Err(Rejection::OutOfReach));
        assert_eq!(w.gather_frontier(), Err(Rejection::OutOfReach));
        assert_eq!(*w.frontier().unwrap(), original);
        w.player.position = DEPOT;
        w.region.as_mut().unwrap().coins = 0;
        assert_eq!(w.trade_frontier(true), Err(Rejection::NotEnoughCoins));
        assert_eq!(*w.frontier().unwrap(), original);
        w.frontier.as_mut().unwrap().depot_herbs = DEPOT_RESERVE;
        w.work_frontier(CARRIER, Goal::CollectFrontierFreight);
        assert_eq!(w.npcs[&CARRIER].herbs, 0);
        assert_eq!(w.frontier().unwrap().depot_herbs, DEPOT_RESERVE);
        let before = *w.frontier().unwrap();
        w.work_frontier(GROWER, Goal::CollectFrontierFreight);
        assert_eq!(*w.frontier().unwrap(), before);
    }
}
