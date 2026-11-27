//! C4.7 actual personal promises, resource/trust effects and passive parity.
use sealed_sim::living::{
    ActorId, Direction, Intent, Outcome, PLAYER, Position, World,
    favors::{RECEIPT_CAP, REVISION, Status},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[path = "support/c47_trace.rs"]
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
fn no_conversation_means_no_new_favors_or_passive_simulation_interference() {
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::personal_favors_campaign(seed);
        let mut old = World::emergent_requests_campaign(seed);
        let initial = herbs(&w);
        let money = coins(&w);
        let mut injured = BTreeSet::new();
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
            assert_eq!(w.favors().unwrap().records().count(), 0);
            assert!(w.known_favors().is_empty() && w.favors().unwrap().receipts().is_empty());
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
            for (&id, &pos) in &positions(&w) {
                assert!(dist(before[&id], pos) <= 1);
            }
            for n in w.npcs() {
                if w.injuries().unwrap().people[&n.id()].injured() {
                    injured.insert(n.id());
                }
            }
            if matches!(w.tick(), 100 | 500 | 1000 | 2000) {
                assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            }
        }
        println!("seed={seed} passive_wounded_npcs={injured:?} parity and conservation");
    }
}
#[test]
fn real_companion_favors_have_five_voluntary_branches_and_directional_trust() {
    let j = trace::journey();
    let seed = j.offer.world.region().unwrap().seed;
    let original = World::personal_favors_campaign(seed);
    let money = coins(&original);
    let material = herbs(&original);
    let actor = trace::TRAVELER;
    let initial = j.accepted.world.social().relationship(actor, PLAYER).regard;
    let initial_reverse = j.accepted.world.social().relationship(PLAYER, actor).regard;
    let completed = j
        .fulfilled
        .world
        .favors()
        .unwrap()
        .record(actor)
        .unwrap()
        .current
        .unwrap();
    assert_eq!(completed.status, Status::Fulfilled);
    assert_eq!(completed.regard_change, (initial + 1).min(8) - initial);
    assert_eq!(
        j.fulfilled
            .world
            .social()
            .relationship(actor, PLAYER)
            .regard,
        initial + completed.regard_change
    );
    assert_eq!(
        j.fulfilled
            .world
            .social()
            .relationship(PLAYER, actor)
            .regard,
        initial_reverse
    );
    assert_eq!(
        j.fulfilled.world.npc(actor).unwrap().packed_herbs,
        j.accepted.world.npc(actor).unwrap().packed_herbs + 1
    );
    assert_eq!(
        j.fulfilled.world.player().herbs,
        j.accepted.world.player().herbs - 1
    );
    assert_eq!(
        j.fulfilled
            .world
            .favors()
            .unwrap()
            .record(actor)
            .unwrap()
            .fulfilled,
        1
    );
    let d = trace::decline(&j);
    let w = trace::withdraw(&j);
    let p = trace::part(&j);
    let u = trace::unpromised(&j);
    for (t, status) in [
        (&d, Status::Declined),
        (&w, Status::Withdrawn),
        (&p, Status::Changed),
        (&u, Status::Changed),
    ] {
        let r = t.world.favors().unwrap().record(actor).unwrap();
        assert_eq!(r.current.unwrap().status, status);
        assert_eq!(r.seen.unwrap().favor.status, status);
        assert_eq!(
            t.world.favor_receipts().last().unwrap().favor.status,
            status
        );
        assert_eq!(r.fulfilled, 0);
    }
    assert_eq!(
        d.world.social().relationship(actor, PLAYER).regard,
        j.offer.world.social().relationship(actor, PLAYER).regard
    );
    assert_eq!(
        u.world.social().relationship(actor, PLAYER).regard,
        j.offer.world.social().relationship(actor, PLAYER).regard
    );
    assert!(p.world.company().unwrap().journey.is_none());
    assert_eq!(
        u.world.npc(actor).unwrap().packed_herbs,
        j.offer.world.npc(actor).unwrap().packed_herbs + 1
    );
    for t in [&j.offer, &j.accepted, &j.fulfilled, &d, &w, &p, &u] {
        assert_eq!(herbs(&t.world), material + regrown(&t.world));
        assert_eq!(coins(&t.world), money);
        assert_eq!(World::restore(&t.world.checkpoint()).unwrap(), t.world);
        assert_eq!(t.commands.len() as u64, t.world.tick());
    }
    println!(
        "seed={seed} actor={} id={} offer={} accepted={} fulfilled={} decline={} withdraw={} parted={} unpromised={} trust_delta={}",
        actor.get(),
        j.id,
        j.offer.world.tick(),
        j.accepted.world.tick(),
        j.fulfilled.world.tick(),
        d.world.tick(),
        w.world.tick(),
        p.world.tick(),
        u.world.tick(),
        completed.regard_change
    );
}
#[test]
fn mixed_commands_keep_deterministic_twins_and_no_resource_creation() {
    for seed in [0, 3, 7, 42] {
        let mut w = World::personal_favors_campaign(seed);
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
            ];
            let intent = actions[((rng >> 32) as usize) % actions.len()];
            assert_eq!(step(&mut w, intent), step(&mut twin, intent));
            assert_eq!(w, twin);
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
            assert!(w.favors().unwrap().records().count() <= w.npcs().len());
            assert!(w.favors().unwrap().receipts().len() <= RECEIPT_CAP);
        }
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}

#[test]
fn favors_v1_passive_checkpoints_are_exact() {
    let mut rows = Vec::new();
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::personal_favors_campaign(seed);
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
        include_str!("fixtures/living/c47_favors_v1.csv")
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
    );
}
#[test]
fn frozen_five_branch_goldens_bind_actual_choices_and_local_regard() {
    use sealed_sim::living::Background;
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/living/c47_branch_trace_v1.json")).unwrap();
    assert_eq!(fixture["revision"], REVISION);
    for r in fixture["rows"].as_array().unwrap() {
        let seed = r["seed"].as_u64().unwrap().try_into().unwrap();
        let actor = ActorId::new(r["actor"].as_u64().unwrap().try_into().unwrap());
        let id = r["id"].as_u64().unwrap().try_into().unwrap();
        let mut w = World::personal_favors_campaign(seed);
        for command in r["commands"].as_array().unwrap() {
            if w.tick() == r["offer_tick"].as_u64().unwrap() {
                assert_eq!(hash(&w.checkpoint()), r["offer_sha256"].as_str().unwrap());
                assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            }
            let command = command.as_str().unwrap();
            let intent = match command {
                "background herbalist" => Intent::ChooseBackground(Background::Herbalist),
                "north" => Intent::Move(Direction::North),
                "east" => Intent::Move(Direction::East),
                "south" => Intent::Move(Direction::South),
                "west" => Intent::Move(Direction::West),
                "wait" => Intent::Wait,
                "gather" => Intent::Gather,
                "talk 22" => Intent::Talk(actor),
                "invite 22" => Intent::Invite(actor),
                "pack 22" => Intent::Provision(actor),
                "part 22" => Intent::Part(actor),
                other => {
                    let (cmd, arg) = other.split_once(' ').unwrap();
                    let value = arg.parse::<u32>().unwrap();
                    assert_eq!(value, id);
                    match cmd {
                        "acceptfavor" => Intent::AcceptFavor(id),
                        "declinefavor" => Intent::DeclineFavor(id),
                        "withdrawfavor" => Intent::WithdrawFavor(id),
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
        let f = w.favors().unwrap().record(actor).unwrap().current.unwrap();
        assert_eq!(format!("{:?}", f.status), r["status"].as_str().unwrap());
        assert_eq!(
            i64::from(f.regard_change),
            r["regard_change"].as_i64().unwrap()
        );
        assert_eq!(
            i64::from(w.social().relationship(actor, PLAYER).regard),
            r["regard"].as_i64().unwrap()
        );
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}
