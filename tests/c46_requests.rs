//! C4.6: exact non-interference, actual choices, conservation, bounded memory.
use sealed_sim::living::{
    ActorId, Direction, Intent, Outcome, PLAYER, Position, World,
    requests::{RECEIPT_CAP, REVISION, Site, Status},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[path = "support/c46_trace.rs"]
mod trace;
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
        + w.requests().map_or(0, |q| q.escrow())
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

#[test]
fn ignoring_requests_preserves_previous_simulation_and_resources_exactly() {
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::emergent_requests_campaign(seed);
        let mut old = World::road_history_campaign(seed);
        let initial = herbs(&w);
        let money = coins(&w);
        assert_eq!(initial, herbs(&old));
        assert_eq!(money, coins(&old));
        let mut max_id = 0;
        for _ in 0..2000 {
            let before = positions(&w);
            assert_eq!(step(&mut w, Intent::Wait), step(&mut old, Intent::Wait));
            assert_eq!(w.player(), old.player());
            assert_eq!(w.supply(), old.supply());
            assert_eq!(w.region(), old.region());
            assert!(w.npcs().eq(old.npcs()));
            assert_eq!(w.social(), old.social());
            assert_eq!(w.regional_news(), old.regional_news());
            assert_eq!(w.contracts(), old.contracts());
            assert_eq!(w.production(), old.production());
            assert_eq!(w.scheduler(), old.scheduler());
            assert_eq!(w.road_history(), old.road_history());
            assert_eq!(w.character_agency(), old.character_agency());
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
            let q = w.requests().unwrap();
            assert_eq!(q.escrow(), 0);
            assert!(q.receipts().len() <= RECEIPT_CAP);
            for site in Site::ALL {
                if let Some(r) = q.board(site).current {
                    max_id = max_id.max(r.id);
                    assert_ne!(r.status, Status::Accepted);
                }
            }
            for (&id, &pos) in &positions(&w) {
                assert!(dist(before[&id], pos) <= 1);
            }
            if matches!(w.tick(), 100 | 500 | 1000 | 2000) {
                assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            }
        }
        assert!(max_id > 0);
        println!("seed={seed} requests_opened={max_id} same gameplay and conserved",);
    }
}
#[test]
fn real_shrine_and_market_journeys_support_completion_decline_and_remote_withdrawal() {
    for site in Site::ALL {
        let j = trace::journey(site);
        let seed = j.offer.world.region().unwrap().seed;
        let original = World::emergent_requests_campaign(seed);
        let money = coins(&original);
        let material = herbs(&original);
        let accepted = j
            .accepted
            .world
            .requests()
            .unwrap()
            .board(site)
            .current
            .unwrap();
        assert_eq!(accepted.status, Status::Accepted);
        assert_eq!(accepted.id, j.id);
        assert_eq!(accepted.escrow, accepted.bonus);
        let done = j
            .completed
            .world
            .requests()
            .unwrap()
            .board(site)
            .current
            .unwrap();
        assert_eq!(done.status, Status::Completed);
        assert!(done.delivered > 0 && done.delivered <= done.required);
        assert_eq!(done.escrow, 0);
        assert_eq!(
            j.completed.world.requests().unwrap().board(site).paid,
            done.bonus
        );
        let declined = trace::decline(&j);
        let abandoned = trace::abandon(&j);
        assert_eq!(
            declined
                .world
                .requests()
                .unwrap()
                .board(site)
                .current
                .unwrap()
                .status,
            Status::Declined
        );
        assert_eq!(
            abandoned
                .world
                .known_requests()
                .into_iter()
                .find(|s| s.request.id == j.id)
                .unwrap()
                .request
                .status,
            Status::Abandoned
        );
        for t in [&j.offer, &j.accepted, &j.completed, &declined, &abandoned] {
            let w = &t.world;
            assert_eq!(herbs(w), material + regrown(w));
            assert_eq!(coins(w), money);
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), *w);
            assert_eq!(t.commands.len() as u64, w.tick());
        }
        println!(
            "site={site:?} seed={seed} id={} accept={} complete={} withdraw={} delivered={} bonus={}",
            j.id,
            j.accepted.world.tick(),
            j.completed.world.tick(),
            abandoned.world.tick(),
            done.delivered,
            done.bonus
        );
    }
}
#[test]
fn mixed_commands_keep_exact_twins_and_bounded_conserved_requests() {
    for seed in [0, 3, 7, 42] {
        let mut w = World::emergent_requests_campaign(seed);
        let mut twin = w.clone();
        let initial = herbs(&w);
        let money = coins(&w);
        let mut rng = u64::from(seed) + 1;
        for _ in 0..1000 {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let candidate = w
                .known_requests()
                .first()
                .map_or(u32::MAX, |s| s.request.id);
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
                Intent::AcceptRequest(candidate),
                Intent::DeclineRequest(candidate),
                Intent::AbandonRequest(candidate),
            ];
            let intent = actions[((rng >> 32) as usize) % actions.len()];
            assert_eq!(step(&mut w, intent), step(&mut twin, intent));
            assert_eq!(w, twin);
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
            assert!(w.requests().unwrap().receipts().len() <= RECEIPT_CAP);
        }
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}

#[test]
fn requests_v1_golden_checkpoints_are_exact() {
    let mut rows = Vec::new();
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::emergent_requests_campaign(seed);
        let initial = hash(&w.checkpoint());
        for _ in 0..1000 {
            step(&mut w, Intent::Wait);
        }
        rows.push(format!(
            "{seed},{initial},{},{}",
            hash(&w.checkpoint()),
            hex(&w.state_digest())
        ));
    }
    assert_eq!(
        rows,
        include_str!("fixtures/living/c46_requests_v1.csv")
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
    );
}
#[test]
fn fixed_request_branch_goldens_replay_six_actual_choices() {
    use sealed_sim::living::Background;
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/living/c46_branch_trace_v1.json")).unwrap();
    assert_eq!(fixture["revision"], REVISION);
    for r in fixture["rows"].as_array().unwrap() {
        let seed = r["seed"].as_u64().unwrap().try_into().unwrap();
        let id = r["id"].as_u64().unwrap().try_into().unwrap();
        let mut w = World::emergent_requests_campaign(seed);
        for command in r["commands"].as_array().unwrap() {
            let command = command.as_str().unwrap();
            let intent = match command {
                "background herbalist" => Intent::ChooseBackground(Background::Herbalist),
                "north" => Intent::Move(Direction::North),
                "east" => Intent::Move(Direction::East),
                "south" => Intent::Move(Direction::South),
                "west" => Intent::Move(Direction::West),
                "wait" => Intent::Wait,
                "gather" => Intent::Gather,
                "deliver" => Intent::Deliver,
                "sell" => Intent::SellHerb,
                other => {
                    let (cmd, arg) = other.split_once(' ').unwrap();
                    let value = arg.parse::<u32>().unwrap();
                    assert_eq!(value, id);
                    match cmd {
                        "acceptquest" | "declinequest" => {
                            assert_eq!(hash(&w.checkpoint()), r["offer_sha256"].as_str().unwrap());
                            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
                            if cmd == "acceptquest" {
                                Intent::AcceptRequest(id)
                            } else {
                                Intent::DeclineRequest(id)
                            }
                        }
                        "abandonquest" => Intent::AbandonRequest(id),
                        _ => panic!("invalid command {other}"),
                    }
                }
            };
            step(&mut w, intent);
        }
        assert_eq!(w.tick(), r["tick"].as_u64().unwrap());
        assert_eq!(
            hash(&w.checkpoint()),
            r["checkpoint_sha256"].as_str().unwrap()
        );
        assert_eq!(hex(&w.state_digest()), r["state_sha256"].as_str().unwrap());
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}
