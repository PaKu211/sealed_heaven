//! C5.0 resource/occupancy invariants and autonomous graph stress.
use sealed_sim::living::{
    ActorId, Direction, Intent, Outcome, PLAYER, Position, World,
    reciprocity::BALANCE_CAP,
    social_network::{HISTORY_CAP, OBSERVED_CAP, REVISION},
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
fn mixed_actions_conserve_goods_and_coins_and_replay_new_social_graph() {
    for seed in [0, 3, 7, u32::MAX] {
        let mut w = World::social_network_campaign(seed);
        let mut twin = w.clone();
        let initial = herbs(&w);
        let money = coins(&w);
        let mut rng = u64::from(seed) + 1;
        for _ in 0..1000 {
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
            let intent = actions[((rng >> 32) as usize) % actions.len()];
            assert_eq!(step(&mut w, intent), step(&mut twin, intent));
            assert_eq!(w, twin);
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
            for (&id, &pos) in &positions(&w) {
                assert!(dist(previous[&id], pos) <= 1);
            }
            let n = w.social_network().unwrap();
            assert!(n.history().len() <= HISTORY_CAP && n.observed().len() <= OBSERVED_CAP);
            assert!(n.pairs().all(|(&(a, b), m)| a < b
                && a != PLAYER
                && b != PLAYER
                && m.first_at <= m.last_at
                && m.last_at <= w.tick()));
            assert!(
                w.reciprocity()
                    .unwrap()
                    .balances()
                    .all(|(&(a, b), &v)| a < b && v != 0 && v.abs() <= BALANCE_CAP)
            );
        }
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        let n = w.social_network().unwrap();
        println!(
            "seed {seed}: {} pairs, {} greetings, {} retained acts, {} witnessed",
            n.pairs().len(),
            n.pairs().map(|(_, m)| m.greetings).sum::<u32>(),
            n.history().len(),
            n.observed().len()
        );
    }
}

#[test]
fn new_revision_preserves_old_checkpoint_and_cannot_be_forged_by_relabeling() {
    let mut prior = World::encounter_campaign(7);
    for _ in 0..80 {
        step(&mut prior, Intent::Wait);
    }
    assert!(prior.social_network().is_none());
    let old = prior.checkpoint();
    assert_eq!(World::restore(&old).unwrap(), prior);
    let mut wrong = old.clone();
    wrong[12..14].copy_from_slice(&REVISION.to_le_bytes());
    use sha2::{Digest, Sha256};
    let end = wrong.len() - 32;
    let checksum = Sha256::digest(&wrong[..end]);
    wrong[end..].copy_from_slice(&checksum);
    assert_eq!(
        World::restore(&wrong),
        Err(sealed_sim::living::LoadError::ScenarioMismatch)
    );
    assert_eq!(prior.checkpoint(), old);
}
