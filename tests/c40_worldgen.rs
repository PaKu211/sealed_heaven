//! C4.0 public contract: generated geography must influence the actual world.
use sealed_sim::living::{
    ActorId, Direction, Intent, LoadError, Outcome, Position, Region, Rejection, World, content,
    worldgen::{self, GeneratedRegion, Terrain},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, VecDeque};

fn p(x: u32, y: u32) -> Position {
    Position { x, y }
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn generator_is_total_connected_and_varied_across_many_seeds() {
    let mut rivers = BTreeSet::new();
    let mut crossings = BTreeSet::new();
    let mut path_lengths = BTreeSet::new();
    let mut west_cadences = BTreeSet::new();
    let mut east_cadences = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    for seed in (0..4096).chain([u32::MAX, u32::MAX - 1, 0x8000_0000]) {
        let map = GeneratedRegion::generate(seed);
        assert_eq!(map, GeneratedRegion::generate(seed));
        assert_eq!(map.seed(), seed);
        assert_eq!(map.tiles().len(), 270);
        assert!(map.tile(p(u32::MAX, 0)).is_none());
        assert!(!map.walkable(p(0, u32::MAX)));
        let [a, b] = map.bridges();
        assert!(a.y.abs_diff(b.y) >= 3);
        for bridge in [a, b] {
            assert_eq!(map.tile(bridge).unwrap().terrain, Terrain::Bridge);
        }
        for y in 0..worldgen::HEIGHT {
            let x = u32::from(map.river()[y as usize]);
            assert!((15..=17).contains(&x));
            if y > 0 {
                assert!(x.abs_diff(u32::from(map.river()[(y - 1) as usize])) <= 1);
            }
            let river = p(x, y);
            assert_eq!(map.walkable(river), map.bridges().contains(&river));
        }
        let d = map.shortest_path(p(11, 4), p(21, 4)).unwrap();
        assert_eq!(d, map.gate_distance());
        assert!((10..=26).contains(&d));
        let sites = Region::sites();
        for site in [
            p(0, 4),
            p(9, 2),
            p(3, 5),
            p(3, 3),
            sites.hall,
            sites.market,
            sites.garden,
        ] {
            assert!(map.shortest_path(p(0, 4), site).is_some());
        }
        assert!([8, 12, 16, 20].contains(&map.west_regrow_every()));
        assert!([12, 16, 20, 24].contains(&map.garden_regrow_every()));
        rivers.insert(*map.river());
        crossings.insert((a.y, b.y));
        path_lengths.insert(d);
        west_cadences.insert(map.west_regrow_every());
        east_cadences.insert(map.garden_regrow_every());
        assert!(hashes.insert(map.digests().world));
    }
    assert!(rivers.len() > 100);
    assert!(crossings.len() >= 10);
    assert!(path_lengths.len() >= 3);
    assert!(west_cadences.len() >= 3 && east_cadences.len() >= 3);
    println!(
        "seeds={} river_shapes={} bridge_patterns={} distances={:?} west={:?} east={:?}",
        hashes.len(),
        rivers.len(),
        crossings.len(),
        path_lengths,
        west_cadences,
        east_cadences
    );
}

#[test]
fn generated_world_inherits_living_systems_and_stable_content_identity() {
    let world = World::generated_riverlands(7);
    assert_eq!(world.checkpoint_revision(), 27);
    assert_eq!(world.npcs().len(), 10);
    assert!(world.region().is_some() && world.school().is_some() && world.company().is_some());
    assert!(world.traffic().is_some() && world.rivalry().is_some() && world.politics().is_some());
    // The original relief and faction-agency experiments were superseded
    // by contracts in the latest historical revision.
    assert!(world.relief().is_none() && world.faction_agency_state().is_none());
    assert!(world.contracts().is_some());
    assert!(world.contract_memory().is_some() && world.information().is_some());
    assert!(
        world.contested().is_some() && world.martial().is_some() && world.meridians().is_some()
    );
    assert!(
        world.injuries().is_some()
            && world.character_agency().is_some()
            && world.sect_hierarchy().is_some()
    );
    assert!(world.is_player_agency() && world.is_consequences() && world.is_violence_logistics());
    assert!(
        world.is_player_reporting() && world.is_physical_recovery() && world.is_sect_cultivation()
    );
    assert_eq!(
        content::identity_for_revision(27),
        content::identity_for_revision(26)
    );
    let legacy = World::duty_coherence_campaign(7);
    assert!(legacy.generated_region().is_none());
    assert_eq!(
        world.npcs().map(|n| n.id()).collect::<Vec<_>>(),
        legacy.npcs().map(|n| n.id()).collect::<Vec<_>>()
    );
    assert_eq!(
        world.generated_region().unwrap(),
        &GeneratedRegion::generate(7)
    );
    for y in 0..worldgen::HEIGHT {
        for x in 0..worldgen::WIDTH {
            assert_eq!(
                world.walkable(p(x, y)),
                world.generated_region().unwrap().walkable(p(x, y))
            );
            if x <= 11 || x >= 21 {
                assert_eq!(world.walkable(p(x, y)), legacy.walkable(p(x, y)));
            }
        }
    }
}

fn step(world: &mut World, intent: Intent) -> Outcome {
    world.step(world.tick(), intent).unwrap()
}

fn assert_positions(world: &World) {
    let player = world.player();
    let mut occupied = BTreeSet::new();
    if player.alive() {
        assert!(world.walkable(player.position));
        occupied.insert((player.position.x, player.position.y));
    }
    for npc in world.npcs().filter(|npc| npc.alive()) {
        assert!(world.walkable(npc.position));
        assert!(occupied.insert((npc.position.x, npc.position.y)));
    }
    assert_eq!(world.npcs().len(), 10);
}

/// Independent breadth-first path oracle over the exposed world collision map.
/// Ignore currently occupied cells and replan after each committed turn.
fn direction_to(world: &World, target: Position, reach: u32) -> Option<Direction> {
    let start = world.player().position;
    let distance = |a: Position, b: Position| a.x.abs_diff(b.x) + a.y.abs_diff(b.y);
    let mut queue = VecDeque::from([(start, None)]);
    let mut seen = BTreeSet::from([(start.x, start.y)]);
    while let Some((at, first)) = queue.pop_front() {
        if distance(at, target) <= reach {
            return first;
        }
        for (direction, x, y) in [
            (Direction::North, Some(at.x), at.y.checked_sub(1)),
            (Direction::East, at.x.checked_add(1), Some(at.y)),
            (Direction::South, Some(at.x), at.y.checked_add(1)),
            (Direction::West, at.x.checked_sub(1), Some(at.y)),
        ] {
            let (Some(x), Some(y)) = (x, y) else {
                continue;
            };
            let next = p(x, y);
            if !world.walkable(next)
                || world.npcs().any(|n| n.alive() && n.position == next)
                || !seen.insert((x, y))
            {
                continue;
            }
            queue.push_back((next, first.or(Some(direction))));
        }
    }
    None
}

fn walk_to(world: &mut World, target: Position, reach: u32, crossed: &mut bool) {
    for _ in 0..300 {
        let at = world.player().position;
        if at.x.abs_diff(target.x) + at.y.abs_diff(target.y) <= reach {
            return;
        }
        let intent = direction_to(world, target, reach).map_or(Intent::Wait, Intent::Move);
        step(world, intent);
        assert!(world.player().alive());
        assert_positions(world);
        *crossed |= world
            .generated_region()
            .unwrap()
            .tile(world.player().position)
            .unwrap()
            .terrain
            == Terrain::Bridge;
    }
    panic!(
        "failed to reach {target:?} from {:?}",
        world.player().position
    );
}

#[test]
fn player_can_traverse_generated_bridges_trade_and_return_with_same_actor_namespace() {
    for seed in [0, 1, 7, 42] {
        let mut world = World::generated_riverlands(seed);
        let mut crossed = false;
        step(
            &mut world,
            Intent::ChooseBackground(sealed_sim::living::Background::Peddler),
        );
        walk_to(&mut world, Region::sites().market, 1, &mut crossed);
        assert!(crossed);
        let mut bought = false;
        for _ in 0..16 {
            match step(&mut world, Intent::BuyHerb) {
                Outcome::Traded { bought: true, .. } => {
                    bought = true;
                    break;
                }
                _ => {
                    step(&mut world, Intent::Wait);
                }
            }
        }
        assert!(bought, "market inaccessible or unavailable on seed {seed}");
        let before = world.checkpoint();
        let mut restored = World::restore(&before).unwrap();
        assert_eq!(world, restored);
        let mut returned = false;
        walk_to(&mut world, p(3, 5), 1, &mut returned);
        assert!(returned);
        assert!(matches!(
            step(&mut world, Intent::Deliver),
            Outcome::Contribution { .. } | Outcome::Delivered
        ));
        walk_to(&mut restored, p(3, 5), 1, &mut false);
        step(&mut restored, Intent::Deliver);
        assert_eq!(world.checkpoint(), restored.checkpoint());
        assert_positions(&world);
    }
}

#[test]
fn water_collision_and_line_of_sight_are_the_same_for_player_and_world() {
    let mut world = World::generated_riverlands(12);
    let map = world.generated_region().unwrap().clone();
    let mut crossed = false;
    walk_to(&mut world, p(11, 4), 0, &mut crossed);
    // Search a reachable river edge in the western bank.
    let edge = (0..worldgen::HEIGHT)
        .find_map(|y| {
            let water = p(u32::from(map.river()[y as usize]), y);
            let from = p(water.x - 1, y);
            (map.tile(water)?.terrain == Terrain::River && map.walkable(from))
                .then_some((from, water))
        })
        .expect("generated river has a land edge");
    walk_to(&mut world, edge.0, 0, &mut crossed);
    assert!(!world.can_see(edge.0, edge.1));
    assert!(!world.can_see(edge.1, edge.0));
    assert_eq!(
        step(&mut world, Intent::Move(Direction::East)),
        Outcome::Rejected(Rejection::Impassable)
    );
    assert_eq!(world.player().position, edge.0);
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
}

fn rechecksum(bytes: &mut [u8]) {
    let at = bytes.len() - 32;
    let digest = Sha256::digest(&bytes[..at]);
    bytes[at..].copy_from_slice(&digest);
}

#[test]
fn checkpoint_binds_generated_map_seed_version_and_original_rules() {
    let mut world = World::generated_riverlands(7);
    for _ in 0..40 {
        step(&mut world, Intent::Wait);
    }
    let original = world.checkpoint();
    assert_eq!(&original[12..14], &27u16.to_le_bytes());
    assert_eq!(World::restore(&original).unwrap(), world);
    let mut wrong_seed = original.clone();
    wrong_seed[50..54].copy_from_slice(&9u32.to_le_bytes());
    rechecksum(&mut wrong_seed);
    assert_eq!(
        World::restore(&wrong_seed),
        Err(LoadError::ScenarioMismatch)
    );
    let mut wrong_geometry = original.clone();
    wrong_geometry[14] ^= 1;
    rechecksum(&mut wrong_geometry);
    assert_eq!(
        World::restore(&wrong_geometry),
        Err(LoadError::ScenarioMismatch)
    );
    let mut wrong_rev = original.clone();
    wrong_rev[12..14].copy_from_slice(&26u16.to_le_bytes());
    rechecksum(&mut wrong_rev);
    assert_eq!(World::restore(&wrong_rev), Err(LoadError::ScenarioMismatch));
    let mut future = original.clone();
    future[12..14].copy_from_slice(&35u16.to_le_bytes());
    rechecksum(&mut future);
    assert_eq!(
        World::restore(&future),
        Err(LoadError::UnsupportedRevision(35))
    );
    let mut broken = original.clone();
    broken[50] ^= 1;
    assert_eq!(World::restore(&broken), Err(LoadError::ChecksumMismatch));
    assert_eq!(world.checkpoint(), original);
    let legacy = World::duty_coherence_campaign(7).checkpoint();
    let mut relabeled = legacy.clone();
    relabeled[12..14].copy_from_slice(&27u16.to_le_bytes());
    rechecksum(&mut relabeled);
    assert_eq!(World::restore(&relabeled), Err(LoadError::ScenarioMismatch));
    assert_eq!(World::restore(&legacy).unwrap().checkpoint(), legacy);
}

#[test]
fn autonomous_trade_regrowth_and_replay_are_conserved_over_long_runs() {
    // The production ledger covers all existing stocks, actor cargo, physical
    // recovery, consumed herbs, recipes, sect offerings and training.
    fn herbs(w: &World) -> u32 {
        let r = w.region().unwrap();
        let s = w.supply();
        w.player().herbs
            + s.source
            + s.shrine
            + s.consumed
            + w.npcs()
                .map(|n| n.herbs + u32::from(n.packed_herbs))
                .sum::<u32>()
            + w.social().recovery_cargo_total()
            + r.market_herbs
            + r.garden_herbs
            + r.hall_offerings
            + (r.remedies + r.remedies_used) * 2
            + w.rivalry().unwrap().consumed_herbs
            + w.rivalry().unwrap().accord_offerings
            + w.company().unwrap().herbs_used
            + u32::from(w.breath_progress())
            + u32::from(w.school().unwrap().form_learned) * 2
            + u32::from(w.school().unwrap().duty == sealed_sim::living::Duty::Carrying)
    }
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut world = World::generated_riverlands(seed);
        let initial = herbs(&world);
        let map = world.generated_region().unwrap().clone();
        let mut courier_crossed = false;
        for _ in 0..1500 {
            let source_before = world.supply();
            let region_before = *world.region().unwrap();
            step(&mut world, Intent::Wait);
            assert_positions(&world);
            let r = world.region().unwrap();
            let t = world.tick();
            assert_eq!(
                world.supply().regrown - source_before.regrown,
                u32::from(t.is_multiple_of(map.west_regrow_every()) && source_before.source < 12)
            );
            assert_eq!(
                r.garden_regrown - region_before.garden_regrown,
                u32::from(
                    t.is_multiple_of(map.garden_regrow_every()) && region_before.garden_herbs < 10
                )
            );
            assert_eq!(
                herbs(&world),
                initial + world.supply().regrown + r.garden_regrown
            );
            courier_crossed |= map
                .bridges()
                .contains(&world.npc(ActorId::new(32)).unwrap().position);
            if matches!(world.tick(), 40 | 500 | 1000 | 1500) {
                let bytes = world.checkpoint();
                assert_eq!(World::restore(&bytes).unwrap(), world);
                assert_eq!(world.generated_region().unwrap(), &map);
            }
        }
        println!(
            "seed={seed} shipped={} garden_regrown={} source_regrown={} crossing={courier_crossed}",
            world.region().unwrap().shipped_to_qinghe,
            world.region().unwrap().garden_regrown,
            world.supply().regrown
        );
        assert!(courier_crossed);
        assert!(
            world.region().unwrap().shipped_to_qinghe > 0,
            "no intersettlement delivery on seed {seed}"
        );
        assert!(world.region().unwrap().garden_regrown > 0);
    }
}

#[test]
fn generator_v1_stage_and_checkpoint_golden_are_exact() {
    let mut rows = Vec::new();
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::generated_riverlands(seed);
        let map = w.generated_region().unwrap();
        let d = map.digests();
        let initial = format!(
            "{seed},{},{},{},{},{},{},{},{},{}",
            hex(d.geography),
            hex(d.hydrology),
            hex(d.ecology),
            hex(d.roads),
            hex(d.world),
            map.gate_distance(),
            map.west_regrow_every(),
            map.garden_regrow_every(),
            hash(&w.checkpoint())
        );
        for _ in 0..1000 {
            step(&mut w, Intent::Wait);
        }
        rows.push(format!("{initial},{}", hash(&w.checkpoint())));
    }
    let fixture = include_str!("fixtures/living/c40_generator_v1.csv");
    assert_eq!(rows, fixture.lines().skip(1).collect::<Vec<_>>());
}
