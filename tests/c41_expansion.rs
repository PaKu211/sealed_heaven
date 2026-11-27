//! Public C4.1 acceptance: the expansion must be physically playable.
use sealed_sim::living::{
    ActorId, Background, Direction, Intent, LoadError, Outcome, Position, Region, Rejection, World,
    content,
    expansion::{self, CARRIER, DEPOT, ExpandedRegion, GROVE, GROWER},
    worldgen::{GeneratedRegion, Terrain},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[path = "../build_support/expansion.rs"]
mod compiler;
const PACK: &str = include_str!("../content/expansion/v1/pineford.json");
const BASE: &str = include_str!("../content/v1/sealed_heaven.json");

fn p(x: u32, y: u32) -> Position {
    Position { x, y }
}
fn dist(a: Position, b: Position) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}
fn step(w: &mut World, i: Intent) -> Outcome {
    w.step(w.tick(), i).unwrap()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

#[test]
fn overlay_compiler_is_strict_and_pinned() {
    let compiled = compiler::compile(PACK, BASE, content::CONTENT_SHA256).unwrap();
    assert_eq!(compiled.sha256, expansion::CONTENT_SHA256);
    assert_eq!(compiled.manifest, expansion::manifest_json());
    assert_eq!(expansion::DEFINITIONS.len(), 5);
    for d in expansion::DEFINITIONS {
        assert_eq!(content::definition_for_revision(28, d.id), Some(d));
        assert!(content::definition_for_revision(27, d.id).is_none());
        assert!(content::definition(d.id).is_none());
    }
    assert_eq!(
        content::definition_for_revision(28, "sealed_heaven:li_wen")
            .unwrap()
            .wire_id,
        Some(37)
    );
    assert_eq!(
        content::definition_for_revision(29, "sealed_heaven:li_wen"),
        content::definition_for_revision(28, "sealed_heaven:li_wen")
    );
    assert_eq!(
        content::definition_for_revision(30, "sealed_heaven:li_wen"),
        content::definition_for_revision(29, "sealed_heaven:li_wen")
    );
    assert_eq!(
        content::definition_for_revision(31, "sealed_heaven:li_wen"),
        content::definition_for_revision(30, "sealed_heaven:li_wen")
    );
    assert!(content::definition_for_revision(32, "sealed_heaven:li_wen").is_none());
    let original: serde_json::Value = serde_json::from_str(PACK).unwrap();
    let equivalent =
        compiler::compile(&original.to_string(), BASE, content::CONTENT_SHA256).unwrap();
    assert_eq!(equivalent.sha256, compiled.sha256);
    assert_eq!(equivalent.rust, compiled.rust);
    for (path, value) in [
        ("/schema_version", serde_json::json!(2)),
        ("/base_sha256", serde_json::json!("wrong")),
        ("/width", serde_json::json!(30)),
        ("/grove", serde_json::json!([31, 4])),
        ("/depot_reserve", serde_json::json!(99)),
        ("/normal_sell", serde_json::json!(99)),
        ("/actors/0/wire_id", serde_json::json!(1)),
        ("/actors/1/wire_id", serde_json::json!(37)),
        ("/actors/1/id", serde_json::json!("sealed_heaven:li_wen")),
        ("/actors/1/glyph", serde_json::json!("Y")),
        ("/actors/1/role", serde_json::json!("Medic")),
        ("/actors/1/home", serde_json::json!([0, 0])),
    ] {
        let mut edited = original.clone();
        *edited.pointer_mut(path).unwrap() = value;
        assert!(
            compiler::compile(&edited.to_string(), BASE, content::CONTENT_SHA256).is_err(),
            "{path}"
        );
    }
    let mut unknown = original.clone();
    unknown["script"] = serde_json::json!("anything");
    assert!(compiler::compile(&unknown.to_string(), BASE, content::CONTENT_SHA256).is_err());
    let mut duplicate = PACK.to_owned();
    duplicate.insert_str(1, "\"schema_version\":1,");
    assert!(compiler::compile(&duplicate, BASE, content::CONTENT_SHA256).is_err());
    let mut changed = original;
    changed["coin_budget"] = serde_json::json!(21);
    assert_ne!(
        compiler::compile(&changed.to_string(), BASE, content::CONTENT_SHA256)
            .unwrap()
            .sha256,
        compiled.sha256
    );
    assert!(
        content::identity_for_revision(27)
            .unwrap()
            .overlay_sha256
            .is_none()
    );
    assert_eq!(
        content::identity_for_revision(28).unwrap().overlay_sha256,
        Some(expansion::CONTENT_SHA256)
    );
}

#[test]
fn expanded_map_is_deterministic_connected_and_preserves_every_old_tile() {
    let mut shapes = BTreeSet::new();
    let mut distances = BTreeSet::new();
    let mut cadences = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    for seed in (0..2048).chain([u32::MAX, u32::MAX - 1, 0x80000000]) {
        let map = ExpandedRegion::generate(seed);
        let base = GeneratedRegion::generate(seed);
        assert_eq!(map, ExpandedRegion::generate(seed));
        assert_eq!(map.tiles().len(), 810);
        assert_eq!(map.seed(), seed);
        assert!(map.tile(p(u32::MAX, 0)).is_none() && !map.walkable(p(0, u32::MAX)));
        for y in 0..9 {
            for x in 0..30 {
                assert_eq!(map.tile(p(x, y)), base.tile(p(x, y)));
            }
        }
        for y in 9..15 {
            for x in 0..30 {
                assert!(!map.walkable(p(x, y)));
            }
        }
        let [a, b] = map.bridges();
        assert!(a.y.abs_diff(b.y) >= 5);
        for y in 0..15 {
            let x = map.river()[y as usize];
            assert!((36..=38).contains(&x));
            if y > 0 {
                assert!(x.abs_diff(map.river()[y as usize - 1]) <= 1);
            }
            let water = p(u32::from(x), y);
            assert_eq!(map.walkable(water), map.bridges().contains(&water));
        }
        let distance = map.shortest_path(p(29, 4), p(43, 7)).unwrap();
        assert_eq!(distance, map.gate_distance());
        assert!(map.shortest_path(p(0, 4), GROVE).is_some());
        assert!(map.shortest_path(p(0, 4), DEPOT).is_some());
        assert!([10, 14, 18, 22].contains(&map.grove_regrow_every()));
        assert!(hashes.insert(map.digest()));
        shapes.insert(*map.river());
        distances.insert(distance);
        cadences.insert(map.grove_regrow_every());
    }
    assert!(shapes.len() > 100 && distances.len() >= 3 && cadences.len() >= 3);
    println!(
        "seeds={} eastern_rivers={} gate_distances={distances:?} regrowth={cadences:?}",
        hashes.len(),
        shapes.len()
    );
}

fn herb_total(w: &World) -> u32 {
    let f = w.frontier().unwrap();
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
        + f.grove_herbs
        + f.depot_herbs
}
fn coins(w: &World) -> u32 {
    let r = w.region().unwrap();
    let agency = w.character_agency().unwrap();
    r.coins
        + r.market_coins
        + w.frontier().unwrap().depot_coins
        + w.politics().unwrap().hall_coins
        + w.rivalry().unwrap().lodge_coins
        + agency.clinic_coins
        + agency.people.values().map(|v| v.coins).sum::<u32>()
}
fn production(w: &World) -> u32 {
    w.supply().regrown + w.region().unwrap().garden_regrown + w.frontier().unwrap().regrown
}
fn positions(w: &World) {
    let mut set = BTreeSet::new();
    assert_eq!(w.npcs().len(), 12);
    if w.player().alive() {
        assert!(w.walkable(w.player().position));
        set.insert(w.player().position);
    }
    for n in w.npcs().filter(|n| n.alive()) {
        assert!(w.walkable(n.position) && set.insert(n.position));
    }
}

#[test]
fn actual_three_settlement_freight_conserves_material_and_coin_on_one_clock() {
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::expanded_riverlands(seed);
        assert_eq!(w.checkpoint_revision(), 28);
        assert_eq!((w.scenario().width, w.scenario().height), (54, 15));
        let legacy = World::generated_riverlands(seed);
        let identities = w
            .npcs()
            .map(|n| (n.id(), n.definition.name))
            .collect::<BTreeMap<_, _>>();
        for n in legacy.npcs() {
            assert_eq!(identities[&n.id()], n.definition.name);
        }
        assert_eq!(identities[&CARRIER], "Li Wen");
        assert_eq!(identities[&GROWER], "Han Yu");
        assert_eq!(w.character_agency().unwrap().people.len(), 12);
        assert_eq!(w.meridians().unwrap().people.len(), 13);
        assert_eq!(w.martial().unwrap().skills.len(), 13);
        let initial = herb_total(&w);
        let money = coins(&w);
        let map = w.expanded_region().unwrap().clone();
        let mut carrier_crossed_loaded = false;
        let mut grower_deliveries = 0;
        let mut freight_deliveries = 0;
        let mut old_crossed = false;
        for _ in 0..2000 {
            let before = *w.frontier().unwrap();
            let old_positions = w
                .npcs()
                .map(|n| (n.id(), n.position))
                .collect::<BTreeMap<_, _>>();
            let carrier_before = *w.npc(CARRIER).unwrap();
            let grower_before = *w.npc(GROWER).unwrap();
            step(&mut w, Intent::Wait);
            positions(&w);
            let f = w.frontier().unwrap();
            let carrier = w.npc(CARRIER).unwrap();
            let grower = w.npc(GROWER).unwrap();
            assert_eq!(
                herb_total(&w),
                initial + production(&w),
                "seed {seed}, turn {}",
                w.tick()
            );
            assert_eq!(coins(&w), money);
            assert_eq!(
                f.regrown - before.regrown,
                u32::from(
                    w.tick().is_multiple_of(map.grove_regrow_every())
                        && before.grove_herbs < expansion::GROVE_CAPACITY
                )
            );
            for n in w.npcs() {
                assert!(
                    dist(old_positions[&n.id()], n.position) <= 1,
                    "actor teleport"
                );
            }
            if f.stocked > before.stocked {
                grower_deliveries += 1;
                assert!(dist(grower.position, DEPOT) <= 1);
                assert_eq!(f.stocked - before.stocked, grower_before.herbs);
                assert_eq!(grower.herbs, 0);
            }
            if f.shipped_to_willowford > before.shipped_to_willowford {
                freight_deliveries += 1;
                assert!(dist(carrier.position, Region::sites().market) <= 1);
                assert_eq!(
                    f.shipped_to_willowford - before.shipped_to_willowford,
                    carrier_before.herbs
                );
                assert_eq!(carrier.herbs, 0);
            }
            carrier_crossed_loaded |=
                map.bridges().contains(&carrier.position) && carrier.herbs > 0;
            old_crossed |= w
                .generated_region()
                .unwrap()
                .bridges()
                .contains(&w.npc(ActorId::new(32)).unwrap().position);
            if matches!(w.tick(), 100 | 500 | 1000 | 2000) {
                assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
                assert_eq!(w.expanded_region().unwrap(), &map);
            }
        }
        let f = w.frontier().unwrap();
        println!(
            "seed={seed} stocked={} shipped={} regrown={} old_shipped={} grower_deliveries={grower_deliveries} freight_deliveries={freight_deliveries} loaded_crossing={carrier_crossed_loaded}",
            f.stocked,
            f.shipped_to_willowford,
            f.regrown,
            w.region().unwrap().shipped_to_qinghe
        );
        assert!(
            carrier_crossed_loaded
                && old_crossed
                && grower_deliveries > 0
                && freight_deliveries > 0,
            "seed={seed}"
        );
        assert!(w.region().unwrap().shipped_to_qinghe > 0 && f.regrown > 0);
    }
}

fn next(w: &World, target: Position, reach: u32) -> Option<Direction> {
    let start = w.player().position;
    let mut queue = VecDeque::from([(start, None)]);
    let mut seen = BTreeSet::from([start]);
    while let Some((at, first)) = queue.pop_front() {
        if dist(at, target) <= reach {
            return first;
        }
        for (dir, x, y) in [
            (Direction::North, Some(at.x), at.y.checked_sub(1)),
            (Direction::East, at.x.checked_add(1), Some(at.y)),
            (Direction::South, Some(at.x), at.y.checked_add(1)),
            (Direction::West, at.x.checked_sub(1), Some(at.y)),
        ] {
            let (Some(x), Some(y)) = (x, y) else {
                continue;
            };
            let n = p(x, y);
            if !w.walkable(n) || w.npcs().any(|a| a.alive() && a.position == n) || !seen.insert(n) {
                continue;
            }
            queue.push_back((n, first.or(Some(dir))));
        }
    }
    None
}
fn walk(
    w: &mut World,
    target: Position,
    reach: u32,
    crossings: &mut BTreeSet<u8>,
    trace: &mut Vec<Intent>,
) {
    for _ in 0..400 {
        if dist(w.player().position, target) <= reach {
            return;
        }
        let intent = next(w, target, reach).map_or(Intent::Wait, Intent::Move);
        trace.push(intent);
        step(w, intent);
        positions(w);
        assert!(w.player().alive());
        let at = w.player().position;
        if w.terrain_tile(at).unwrap().terrain == Terrain::Bridge {
            crossings.insert(if at.x < 30 { 1 } else { 2 });
        }
    }
    panic!("cannot reach {target:?} from {:?}", w.player().position);
}
fn record(w: &mut World, trace: &mut Vec<Intent>, intent: Intent) -> Outcome {
    trace.push(intent);
    step(w, intent)
}

#[test]
fn player_travels_to_pineford_buys_sells_gathers_and_delivers_to_qinghe() {
    for seed in [0, 1, 7, 42] {
        let mut w = World::expanded_riverlands(seed);
        let initial = herb_total(&w);
        let money = coins(&w);
        let mut trace = Vec::new();
        let mut crossings = BTreeSet::new();
        record(
            &mut w,
            &mut trace,
            Intent::ChooseBackground(Background::Peddler),
        );
        assert!(matches!(
            record(&mut w, &mut trace, Intent::BuyHerb),
            Outcome::Rejected(Rejection::OutOfReach)
        ));
        walk(&mut w, DEPOT, 1, &mut crossings, &mut trace);
        assert_eq!(crossings, BTreeSet::from([1, 2]));
        assert!(w.player_sees(DEPOT));
        assert!(!w.player_sees(p(0, 4)));
        let bought = record(&mut w, &mut trace, Intent::BuyHerb);
        assert!(
            matches!(bought, Outcome::Traded { bought: true, .. }),
            "seed={seed} {bought:?}"
        );
        assert!(matches!(
            record(&mut w, &mut trace, Intent::SellHerb),
            Outcome::Traded { bought: false, .. }
        ));
        assert_eq!(w.frontier().unwrap().player_bought, 1);
        assert_eq!(w.frontier().unwrap().player_sold, 1);
        walk(&mut w, GROVE, 1, &mut crossings, &mut trace);
        assert!(matches!(
            record(&mut w, &mut trace, Intent::Gather),
            Outcome::Gathered
        ));
        let mid = w.checkpoint();
        let mut restored = World::restore(&mid).unwrap();
        assert_eq!(w, restored);
        let split = trace.len();
        let mut returned = BTreeSet::new();
        walk(&mut w, p(3, 5), 1, &mut returned, &mut trace);
        assert_eq!(returned, BTreeSet::from([1, 2]));
        assert!(matches!(
            record(&mut w, &mut trace, Intent::Deliver),
            Outcome::Delivered | Outcome::Contribution { .. }
        ));
        for &i in &trace[split..] {
            step(&mut restored, i);
        }
        assert_eq!(restored.checkpoint(), w.checkpoint());
        assert_eq!(herb_total(&w), initial + production(&w));
        assert_eq!(coins(&w), money);
        let mut fresh = World::expanded_riverlands(seed);
        for i in trace {
            step(&mut fresh, i);
        }
        assert_eq!(fresh, w);
    }
}
fn rechecksum(bytes: &mut [u8]) {
    let at = bytes.len() - 32;
    let digest = Sha256::digest(&bytes[..at]);
    bytes[at..].copy_from_slice(&digest);
}
#[test]
fn new_save_binds_seed_map_overlay_and_rejects_foreign_revision() {
    let mut w = World::expanded_riverlands(7);
    for _ in 0..100 {
        step(&mut w, Intent::Wait);
    }
    let bytes = w.checkpoint();
    assert_eq!(&bytes[12..14], &28u16.to_le_bytes());
    assert_eq!(World::restore(&bytes).unwrap(), w);
    for at in [14, 50] {
        let mut bad = bytes.clone();
        bad[at] ^= 1;
        rechecksum(&mut bad);
        assert_eq!(World::restore(&bad), Err(LoadError::ScenarioMismatch));
    }
    let mut bad = bytes.clone();
    bad[12..14].copy_from_slice(&27u16.to_le_bytes());
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::ScenarioMismatch));
    let mut state = bytes.clone();
    let at = state.len() - 64;
    state[at] ^= 1;
    rechecksum(&mut state);
    assert_eq!(World::restore(&state), Err(LoadError::StateMismatch));
    let legacy = World::generated_riverlands(7).checkpoint();
    let mut relabeled = legacy.clone();
    relabeled[12..14].copy_from_slice(&28u16.to_le_bytes());
    rechecksum(&mut relabeled);
    assert_eq!(World::restore(&relabeled), Err(LoadError::ScenarioMismatch));
    assert_eq!(World::restore(&legacy).unwrap().checkpoint(), legacy);
}
#[test]
fn expansion_v1_golden_checkpoints_and_maps_are_exact() {
    let mut rows = Vec::new();
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::expanded_riverlands(seed);
        let map = w.expanded_region().unwrap();
        let row = format!(
            "{seed},{},{},{},{},{}",
            hex(&w.generated_region().unwrap().digests().world),
            hex(&map.digest()),
            map.gate_distance(),
            map.grove_regrow_every(),
            hash(&w.checkpoint())
        );
        for _ in 0..1000 {
            step(&mut w, Intent::Wait);
        }
        rows.push(format!("{row},{}", hash(&w.checkpoint())));
    }
    assert_eq!(
        rows,
        include_str!("fixtures/living/c41_expansion_v1.csv")
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
    );
}
