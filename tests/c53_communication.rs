//! Conservation, movement, bounded message provenance and replay.
use sealed_sim::living::{
    ActorId, Direction, Intent, Outcome, PLAYER, Position, World,
    communication::{HISTORY_CAP, Kind, OBSERVED_CAP, REVISION, SEARCH_BUDGET},
    obligations::{REPORT_LIFETIME, ResultKind},
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
fn mixed_actions_preserve_material_resources_and_message_invariants() {
    for seed in [0, 3, 7, 42] {
        let mut w = World::communication_campaign(seed);
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
            let o = w.obligations().unwrap();
            let c = w.communication().unwrap();
            let (created, done, expired, handoffs) = c.counts();
            assert_eq!(created, done + expired + c.messages().len() as u32);
            assert_eq!(done, o.counts().2);
            assert!(expired <= o.counts().3);
            assert!(handoffs <= created);
            assert_eq!(c.messages().len(), o.pending().len());
            assert!(
                c.messages().len() <= 2
                    && c.history().len() <= HISTORY_CAP
                    && c.observed().len() <= OBSERVED_CAP
            );
            let mut holders = BTreeSet::new();
            for (&origin, m) in c.messages() {
                let r = o.pending()[&origin];
                assert_eq!(
                    (origin, m.project, m.origin, m.recipient, m.created),
                    (
                        r.team.carrier,
                        r.team.id,
                        r.team.carrier,
                        r.team.grower,
                        r.at
                    )
                );
                assert!(m.holder != m.recipient && holders.insert(m.holder));
                assert!(m.site < 3 && m.dwell < 2 && m.spent < SEARCH_BUDGET);
                assert!(w.tick() - r.at < REPORT_LIFETIME);
                assert_ne!(r.result, ResultKind::CarrierLost);
            }
            assert!(
                c.observed()
                    .iter()
                    .all(|e| matches!(e.kind, Kind::Direct | Kind::Handoff | Kind::Relayed))
            );
        }
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        println!(
            "seed {seed}: messages {:?}",
            w.communication().unwrap().counts()
        );
    }
}

#[test]
fn c52_revision_stays_distinct_and_forged_revision_is_rejected() {
    let mut w = World::obligations_campaign(42);
    for _ in 0..354 {
        step(&mut w, Intent::Wait);
    }
    assert!(w.communication().is_none());
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
