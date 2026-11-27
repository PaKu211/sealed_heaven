//! Resource, occupancy, knowledge and project lifecycle invariants.
use sealed_sim::living::{
    ActorId, Direction, Intent, Outcome, PLAYER, Position, World,
    cooperation::{Evidence, HISTORY_CAP, OBSERVED_CAP, REGISTER_CAP, REVISION, Route},
};
use std::collections::{BTreeMap, BTreeSet};
fn step(w: &mut World, intent: Intent) -> Outcome {
    w.step(w.tick(), intent).unwrap()
}
fn dist(a: Position, b: Position) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
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
fn mixed_actions_conserve_real_resources_and_bound_project_state() {
    for seed in [0, 3, 7, 42] {
        let mut w = World::cooperation_campaign(seed);
        let mut twin = w.clone();
        let initial = herbs(&w);
        let money = coins(&w);
        let mut rng = u64::from(seed) + 1;
        for _ in 0..1500 {
            let previous = positions(&w);
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let actor = ActorId::new(22);
            let id = w.known_favors().first().map_or(u32::MAX, |s| s.favor.id);
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
            let action = actions[((rng >> 32) as usize) % actions.len()];
            assert_eq!(step(&mut w, action), step(&mut twin, action));
            assert_eq!(w, twin);
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
            for (&id, &pos) in &positions(&w) {
                assert!(dist(previous[&id], pos) <= 1);
            }
            let c = w.cooperation().unwrap();
            let (opened, delivered, failed) = c.counts();
            assert_eq!(opened, delivered + failed + c.active().len() as u32);
            assert!(
                c.active().len() <= 2
                    && c.history().len() <= HISTORY_CAP
                    && c.observed().len() <= OBSERVED_CAP
            );
            let mut members = BTreeSet::new();
            for (&carrier, t) in c.active() {
                assert_eq!(carrier, t.carrier);
                assert!(members.insert(t.grower) && members.insert(t.carrier));
                assert!(
                    t.contributed > 0
                        && t.contributed <= t.shipment
                        && t.opened <= w.tick()
                        && w.tick() < t.deadline
                );
                assert_eq!(w.npc(carrier).unwrap().herbs, t.shipment);
            }
            for route in [Route::Qinghe, Route::Willowford] {
                assert!(c.register(route).count() <= REGISTER_CAP);
                assert!(
                    c.register(route)
                        .all(|r| r.delivered && r.evidence == Evidence::Register)
                );
            }
        }
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        println!("seed {seed}: teams {:?}", w.cooperation().unwrap().counts());
    }
}

#[test]
fn legacy_revision_has_no_collective_state_and_forgery_is_rejected() {
    let mut w = World::social_network_campaign(3);
    for _ in 0..120 {
        step(&mut w, Intent::Wait);
    }
    assert!(w.cooperation().is_none());
    let old = w.checkpoint();
    assert_eq!(World::restore(&old).unwrap(), w);
    let mut forged = old.clone();
    forged[12..14].copy_from_slice(&REVISION.to_le_bytes());
    use sha2::{Digest, Sha256};
    let end = forged.len() - 32;
    let digest = Sha256::digest(&forged[..end]);
    forged[end..].copy_from_slice(&digest);
    assert_eq!(
        World::restore(&forged),
        Err(sealed_sim::living::LoadError::ScenarioMismatch)
    );
    assert_eq!(w.checkpoint(), old);
}
