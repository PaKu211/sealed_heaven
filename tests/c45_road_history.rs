//! Public-API conservation, natural encounters, replay and immutable prehistory.
use sealed_sim::living::{
    ActorId, Background, Direction, Intent, LoadError, Outcome, PLAYER, Position, Rejection, World,
    road_history::{Method, REVISION},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn step(w: &mut World, intent: Intent) -> Outcome {
    w.step(w.tick(), intent).unwrap()
}
fn dist(a: Position, b: Position) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn herbs(w: &World) -> u32 {
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
        + w.road_history()
            .map_or(0, |h| h.caches().iter().map(|c| c.remaining).sum::<u32>())
}
fn coins(w: &World) -> u32 {
    let r = w.region().unwrap();
    let a = w.character_agency().unwrap();
    r.coins
        + r.market_coins
        + w.frontier().unwrap().depot_coins
        + w.politics().unwrap().hall_coins
        + w.rivalry().unwrap().lodge_coins
        + a.clinic_coins
        + a.people.values().map(|p| p.coins).sum::<u32>()
}
fn regrown(w: &World) -> u32 {
    w.supply().regrown + w.region().unwrap().garden_regrown + w.frontier().unwrap().regrown
}
fn positions(w: &World) -> BTreeMap<ActorId, Position> {
    let mut occupied = BTreeSet::new();
    let mut result = BTreeMap::new();
    if w.player().alive() {
        assert!(w.walkable(w.player().position));
        occupied.insert(w.player().position);
        result.insert(PLAYER, w.player().position);
    }
    for n in w.npcs().filter(|n| n.alive()) {
        assert!(w.walkable(n.position) && occupied.insert(n.position));
        result.insert(n.id(), n.position);
    }
    result
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
fn walk_to(w: &mut World, id: u32) -> bool {
    let target = w
        .road_history()
        .unwrap()
        .cache(id)
        .unwrap()
        .deposit
        .position;
    for _ in 0..250 {
        if !w.player().alive() {
            return false;
        }
        if dist(w.player().position, target) <= 1 {
            return true;
        }
        let action = direction(w, target).map_or(Intent::Wait, Intent::Move);
        step(w, action);
    }
    false
}
fn rechecksum(bytes: &mut [u8]) {
    let at = bytes.len() - 32;
    let digest = Sha256::digest(&bytes[..at]);
    bytes[at..].copy_from_slice(&digest);
}

#[test]
fn six_seeds_passive_history_and_resources_are_conserved_and_replay_exact() {
    let mut autonomous = 0;
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::road_history_campaign(seed);
        let mut twin = World::road_history_campaign(seed);
        let prior = World::settlement_production_campaign(seed);
        let initial = herbs(&w);
        let money = coins(&w);
        assert_eq!(initial, herbs(&prior));
        assert_eq!(money, coins(&prior));
        let history = w.road_history().unwrap().initial_digest();
        let mut collected = 0;
        for _ in 0..2000 {
            let before = positions(&w);
            let h = w.road_history().unwrap().clone();
            assert_eq!(step(&mut w, Intent::Wait), step(&mut twin, Intent::Wait));
            assert_eq!(w, twin);
            assert_eq!(w.road_history().unwrap().initial_digest(), history);
            assert_eq!(
                herbs(&w),
                initial + regrown(&w),
                "material seed={seed} tick={}",
                w.tick()
            );
            assert_eq!(coins(&w), money);
            for (&actor, &pos) in &positions(&w) {
                assert!(dist(before[&actor], pos) <= 1, "teleport");
            }
            for (a, b) in h.caches().iter().zip(w.road_history().unwrap().caches()) {
                assert_eq!(a.deposit, b.deposit);
                if a.resolution.is_some() {
                    assert_eq!(a.resolution, b.resolution);
                }
                if a.remaining != b.remaining {
                    assert!(a.remaining > 0 && b.remaining == 0);
                    let res = b.resolution.unwrap();
                    assert_eq!(res.at, w.tick());
                    assert_eq!(res.method, Method::Collected);
                    assert_eq!(res.recipient, b.deposit.route.recipient());
                    assert!(dist(before[&res.recipient], b.deposit.position) <= 1);
                    assert_eq!(
                        w.npc(res.recipient).unwrap().position,
                        before[&res.recipient]
                    );
                    collected += 1;
                }
            }
            if matches!(w.tick(), 100 | 500 | 1000 | 2000) {
                assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            }
        }
        println!(
            "seed={seed} auto_collected={collected} material={} herbs_grown={} west_shipments={} east_shipments={}",
            initial,
            regrown(&w),
            w.region().unwrap().shipped_to_qinghe,
            w.frontier().unwrap().shipped_to_willowford
        );
        autonomous += collected;
    }
    assert!(
        autonomous > 0,
        "no autonomous roadside encounter in reference campaigns"
    );
}

#[test]
fn actual_player_journey_supports_recovery_and_entrustment_as_distinct_outcomes() {
    let mut taken = None;
    let mut entrusted = None;
    for seed in [0, 1, 2, 7, 42, u32::MAX, 3, 4, 5, 6, 8, 9] {
        let mut w = World::road_history_campaign(seed);
        let initial = herbs(&w);
        let money = coins(&w);
        step(&mut w, Intent::ChooseBackground(Background::Herbalist));
        if !walk_to(&mut w, 1) {
            continue;
        }
        if taken.is_none()
            && w.local_caches()
                .iter()
                .any(|c| c.deposit.id == 1 && c.can_take)
        {
            let mut branch = w.clone();
            let amount = branch.road_history().unwrap().cache(1).unwrap().remaining;
            let out = step(&mut branch, Intent::RecoverCache(1));
            assert_eq!(
                out,
                Outcome::CacheRecovered {
                    id: 1,
                    herbs: amount
                }
            );
            assert_eq!(
                branch.road_history().unwrap().cache(1).unwrap().remaining,
                0
            );
            assert_eq!(herbs(&branch), initial + regrown(&branch));
            assert_eq!(coins(&branch), money);
            assert_eq!(World::restore(&branch.checkpoint()).unwrap(), branch);
            assert_eq!(
                step(&mut branch, Intent::RecoverCache(1)),
                Outcome::Rejected(Rejection::Empty)
            );
            assert_eq!(herbs(&branch), initial + regrown(&branch));
            println!("actual take seed={seed} tick={}", branch.tick());
            taken = Some(seed);
        }
        if entrusted.is_none() {
            for _ in 0..300 {
                let Some(c) = w.local_caches().into_iter().find(|c| c.deposit.id == 1) else {
                    break;
                };
                if c.remaining == 0 {
                    break;
                }
                if c.can_entrust {
                    let mut take = w.clone();
                    let mut handover = w.clone();
                    let actor = c.deposit.route.recipient();
                    let relation = w.social().relationship(actor, PLAYER).regard;
                    assert_eq!(
                        step(&mut take, Intent::RecoverCache(1)),
                        Outcome::CacheRecovered {
                            id: 1,
                            herbs: c.remaining
                        }
                    );
                    assert_eq!(
                        step(&mut handover, Intent::EntrustCache(1)),
                        Outcome::CacheEntrusted {
                            id: 1,
                            to: actor,
                            herbs: c.remaining
                        }
                    );
                    assert!(handover.social().relationship(actor, PLAYER).regard >= relation);
                    assert_ne!(take.state_digest(), handover.state_digest());
                    for branch in [&take, &handover] {
                        assert_eq!(herbs(branch), initial + regrown(branch));
                        assert_eq!(coins(branch), money);
                        assert_eq!(World::restore(&branch.checkpoint()).unwrap(), *branch);
                    }
                    println!("actual entrust seed={seed} tick={}", handover.tick());
                    entrusted = Some(seed);
                    break;
                }
                step(&mut w, Intent::Wait);
            }
        }
        if taken.is_some() && entrusted.is_some() {
            break;
        }
    }
    assert!(taken.is_some(), "no reachable take encounter");
    assert!(entrusted.is_some(), "no reachable handover encounter");
}

#[test]
fn revision_32_binds_history_and_preserves_old_checkpoint_and_wire() {
    let mut w = World::road_history_campaign(7);
    let original = w.road_history().unwrap().initial_digest();
    for _ in 0..1000 {
        step(&mut w, Intent::Wait);
    }
    assert_eq!(w.road_history().unwrap().initial_digest(), original);
    let bytes = w.checkpoint();
    assert_eq!(&bytes[12..14], &REVISION.to_le_bytes());
    assert_eq!(World::restore(&bytes).unwrap(), w);
    let mut bad = bytes.clone();
    let at = bad.len() - 64;
    bad[at] ^= 1;
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::StateMismatch));
    let old = World::settlement_production_campaign(7).checkpoint();
    let mut bad = old.clone();
    bad[12..14].copy_from_slice(&REVISION.to_le_bytes());
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::ScenarioMismatch));
    let mut bad = bytes;
    bad[12..14].copy_from_slice(&31u16.to_le_bytes());
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::ScenarioMismatch));
    assert_eq!(World::restore(&old).unwrap().checkpoint(), old);
}

#[test]
fn history_v1_golden_checkpoints_and_immutable_deposits_are_exact() {
    let mut rows = Vec::new();
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::road_history_campaign(seed);
        let initial = hash(&w.checkpoint());
        let history = hex(&w.road_history().unwrap().initial_digest());
        for _ in 0..1000 {
            step(&mut w, Intent::Wait);
        }
        let collected = w
            .road_history()
            .unwrap()
            .caches()
            .iter()
            .filter(|c| c.resolution.is_some())
            .count();
        rows.push(format!(
            "{seed},{initial},{},{},{history},{collected}",
            hash(&w.checkpoint()),
            hex(&w.state_digest())
        ));
    }
    assert_eq!(
        rows,
        include_str!("fixtures/living/c45_history_v1.csv")
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
    );
}

#[test]
fn mixed_player_commands_preserve_material_and_deterministic_encounter_history() {
    let actions = [
        Intent::Move(Direction::North),
        Intent::Move(Direction::East),
        Intent::Move(Direction::South),
        Intent::Move(Direction::West),
        Intent::Wait,
        Intent::Gather,
        Intent::Deliver,
        Intent::BuyHerb,
        Intent::SellHerb,
        Intent::RecoverCache(1),
        Intent::RecoverCache(2),
        Intent::EntrustCache(1),
        Intent::EntrustCache(2),
    ];
    for seed in [0, 3, 7, 42] {
        let mut w = World::road_history_campaign(seed);
        let mut twin = w.clone();
        let initial = herbs(&w);
        let money = coins(&w);
        let mut rng = u64::from(seed) + 1;
        for _ in 0..1000 {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let intent = actions[((rng >> 32) as usize) % actions.len()];
            assert_eq!(step(&mut w, intent), step(&mut twin, intent));
            assert_eq!(w, twin);
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
        }
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}

#[test]
fn fixed_branch_goldens_replay_both_actual_player_choices() {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/living/c45_branch_trace_v1.json")).unwrap();
    assert_eq!(data["revision"], 32);
    for row in data["rows"].as_array().unwrap() {
        let seed = row["seed"].as_u64().unwrap().try_into().unwrap();
        let mut w = World::road_history_campaign(seed);
        let commands = row["commands"].as_array().unwrap();
        for (index, command) in commands.iter().enumerate() {
            if index + 1 == commands.len() {
                assert_eq!(
                    hash(&w.checkpoint()),
                    row["pre_choice_sha256"].as_str().unwrap()
                );
                assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            }
            let intent = match command.as_str().unwrap() {
                "background herbalist" => Intent::ChooseBackground(Background::Herbalist),
                "north" => Intent::Move(Direction::North),
                "east" => Intent::Move(Direction::East),
                "south" => Intent::Move(Direction::South),
                "west" => Intent::Move(Direction::West),
                "wait" => Intent::Wait,
                "recovercache" => Intent::RecoverCache(1),
                "entrustcache" => Intent::EntrustCache(1),
                other => panic!("invalid fixture command {other}"),
            };
            step(&mut w, intent);
        }
        assert_eq!(w.tick(), row["tick"].as_u64().unwrap());
        assert_eq!(
            hash(&w.checkpoint()),
            row["checkpoint_sha256"].as_str().unwrap()
        );
        assert_eq!(
            hex(&w.state_digest()),
            row["state_sha256"].as_str().unwrap()
        );
        assert!(w.known_cache_resolution(1).is_some());
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}
