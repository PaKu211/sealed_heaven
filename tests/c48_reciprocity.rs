//! C4.8 deterministic reciprocity, material conservation and canonical choices.
use sealed_sim::living::{
    ActorId, Direction, Intent, Outcome, PLAYER, Position, World,
    reciprocity::{BALANCE_CAP, HISTORY_CAP, Kind, OBSERVED_CAP, REVISION},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[path = "support/c48_trace.rs"]
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
fn passive_turns_preserve_old_simulation_and_conserve_resources() {
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::reciprocal_bonds_campaign(seed);
        let mut old = World::personal_favors_campaign(seed);
        let initial = herbs(&w);
        let money = coins(&w);
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
            assert_eq!(w.injuries(), old.injuries());
            assert_eq!(w.requests(), old.requests());
            assert_eq!(w.company(), old.company());
            assert_eq!(w.favors(), old.favors());
            assert!(w.known_bonds().is_empty());
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
            for (&id, &pos) in &positions(&w) {
                assert!(dist(before[&id], pos) <= 1);
            }
            if matches!(w.tick(), 100 | 500 | 1000 | 2000) {
                assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            }
        }
        println!(
            "seed {seed}: passive parity, {} aid events retained",
            w.reciprocity().unwrap().history().len()
        );
    }
}
#[test]
fn real_companion_returns_surplus_releases_and_keeps_obligations_after_parting() {
    let j = trace::journey();
    let actor = trace::TRAVELER;
    let net = |t: &trace::Trace| t.world.reciprocity().unwrap().net(PLAYER, actor);
    assert_eq!(net(&j.one), 1);
    assert_eq!(net(&j.two), 2);
    assert_eq!(net(&j.returned), 1);
    assert_eq!(net(&j.waived), 0);
    assert_eq!(net(&j.parted), 2);
    assert_eq!(net(&j.reserve), 1);
    assert_eq!(j.returned.world.npc(actor).unwrap().packed_herbs, 1);
    assert_eq!(
        j.returned.world.player().herbs,
        j.two.world.player().herbs + 1
    );
    assert_eq!(
        j.returned.world.known_aid().last().unwrap().kind,
        Kind::ReturnedHerb
    );
    assert_eq!(
        j.waived.world.known_aid().last().unwrap().kind,
        Kind::Released
    );
    assert_eq!(
        j.returned.world.social().relationship(actor, PLAYER).regard,
        j.two.world.social().relationship(actor, PLAYER).regard
    );
    assert_eq!(
        j.waived.world.social().relationship(actor, PLAYER).regard,
        j.returned.world.social().relationship(actor, PLAYER).regard
    );
    assert!(j.parted.world.company().unwrap().journey.is_none());
    assert_eq!(j.reserve.world.npc(actor).unwrap().packed_herbs, 1);
    let initial = World::reciprocal_bonds_campaign(j.one.world.region().unwrap().seed);
    for t in [
        &j.one,
        &j.two,
        &j.returned,
        &j.waived,
        &j.parted,
        &j.reserve,
    ] {
        assert_eq!(coins(&t.world), coins(&initial));
        assert_eq!(herbs(&t.world), herbs(&initial) + regrown(&t.world));
        assert_eq!(World::restore(&t.world.checkpoint()).unwrap(), t.world);
        assert_eq!(t.commands.len() as u64, t.world.tick());
    }
    println!(
        "real branches: one={} two={} return={} waive={} part={} reserve={}",
        j.one.world.tick(),
        j.two.world.tick(),
        j.returned.world.tick(),
        j.waived.world.tick(),
        j.parted.world.tick(),
        j.reserve.world.tick()
    );
}
#[test]
fn mixed_actions_are_deterministic_and_bounded() {
    for seed in [0, 3, 7, 42] {
        let mut w = World::reciprocal_bonds_campaign(seed);
        let mut twin = w.clone();
        let initial = herbs(&w);
        let money = coins(&w);
        let mut rng = u64::from(seed) + 1;
        for _ in 0..1000 {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let id = w.known_favors().first().map_or(u32::MAX, |s| s.favor.id);
            let actor = trace::TRAVELER;
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
                Intent::Talk(actor),
                Intent::Invite(actor),
                Intent::Provision(actor),
                Intent::Part(actor),
                Intent::AcceptFavor(id),
                Intent::DeclineFavor(id),
                Intent::WithdrawFavor(id),
                Intent::FundFavor(id),
                Intent::RequestAid(actor),
                Intent::RepayAid(actor),
                Intent::WaiveAid(actor),
            ];
            let intent = actions[((rng >> 32) as usize) % actions.len()];
            assert_eq!(step(&mut w, intent), step(&mut twin, intent));
            assert_eq!(w, twin);
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
            let r = w.reciprocity().unwrap();
            assert!(
                r.balances()
                    .all(|((a, b), n)| a < b && *n != 0 && n.abs() <= BALANCE_CAP)
            );
            assert!(r.history().len() <= HISTORY_CAP && r.observed().len() <= OBSERVED_CAP);
        }
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}

#[test]
fn reciprocity_v1_passive_checkpoints_are_exact() {
    let mut rows = Vec::new();
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::reciprocal_bonds_campaign(seed);
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
        include_str!("fixtures/living/c48_reciprocity_v1.csv")
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
    );
}
#[test]
fn actual_clinic_repayment_journey_uses_existing_health_and_coin_ledgers() {
    let t = trace::clinic_journey().expect("seed-0 actual clinic repayment path");
    assert_eq!(t.world.reciprocity().unwrap().net(trace::MEDIC, PLAYER), 0);
    assert_eq!(t.world.known_aid().last().unwrap().kind, Kind::RepaidCoin);
    let original = World::reciprocal_bonds_campaign(t.world.region().unwrap().seed);
    assert_eq!(coins(&t.world), coins(&original));
    assert_eq!(herbs(&t.world), herbs(&original) + regrown(&t.world));
    assert_eq!(World::restore(&t.world.checkpoint()).unwrap(), t.world);
    println!(
        "natural clinic repayment seed={} tick={}",
        t.world.region().unwrap().seed,
        t.world.tick()
    );
}
#[test]
fn frozen_seven_branch_goldens_bind_real_material_and_social_outcomes() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/living/c48_branch_trace_v1.json")).unwrap();
    assert_eq!(f["revision"], REVISION);
    for row in f["rows"].as_array().unwrap() {
        let seed = row["seed"].as_u64().unwrap().try_into().unwrap();
        let actor = ActorId::new(row["actor"].as_u64().unwrap().try_into().unwrap());
        let mut w = World::reciprocal_bonds_campaign(seed);
        let initial = herbs(&w);
        let money = coins(&w);
        for c in row["commands"].as_array().unwrap() {
            let before = w.tick();
            step(&mut w, trace::command(c.as_str().unwrap()));
            assert_eq!(w.tick(), before + 1);
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
        assert_eq!(
            i64::from(w.reciprocity().unwrap().net(PLAYER, actor)),
            row["balance"].as_i64().unwrap()
        );
        assert_eq!(
            u64::from(w.npc(actor).unwrap().packed_herbs),
            row["packed"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(w.player().herbs),
            row["player_herbs"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(w.region().unwrap().coins),
            row["purse"].as_u64().unwrap()
        );
        assert_eq!(
            format!("{:?}", w.latest_outcome()),
            row["outcome"].as_str().unwrap()
        );
        assert_eq!(
            w.known_aid().last().map(|e| format!("{:?}", e.kind)),
            row["last_aid"].as_str().map(str::to_owned)
        );
        assert_eq!(herbs(&w), initial + regrown(&w));
        assert_eq!(coins(&w), money);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}
