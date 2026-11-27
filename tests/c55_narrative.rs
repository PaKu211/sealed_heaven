//! Conservation, bounded narrative memory and replay.
use sealed_sim::living::{
    ActorId, Direction, Intent, Outcome, PLAYER, Position, World,
    narrative::{INQUIRY_HISTORY_CAP, KNOWN_CAP, PROJECT_HISTORY_CAP, REVISION},
    rumors::{HISTORY_CAP, MAX_HOPS, Status, VERIFY_BUDGET},
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
fn mixed_actions_preserve_material_resources_and_narrative_invariants() {
    for seed in [0, 3, 7, 42] {
        let mut w = World::narrative_campaign(seed);
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
            let r = w.rumors().unwrap();
            let actor_count = w.npcs().count() + 1;
            assert!(r.local().len() <= actor_count * 4);
            assert!(r.reports().len() <= actor_count * 8);
            assert!(r.attempts().len() <= actor_count * 4);
            assert!(r.history().len() <= HISTORY_CAP);
            for (&(id, site), sample) in r.local() {
                assert_eq!((id, site), (sample.origin, sample.site));
                assert!(sample.fresh(w.tick()));
            }
            for (&(id, site, scarce), account) in r.reports() {
                assert_eq!((site, scarce), (account.sample.site, account.sample.scarce));
                assert_ne!(id, account.sample.origin);
                assert!(account.via.is_some() && account.hops > 0 && account.hops <= MAX_HOPS);
                assert!(
                    account.sample.observed_at <= account.acquired_at
                        && account.acquired_at <= w.tick()
                );
                assert!(account.credibility <= 100 - 15 * account.hops && account.fresh(w.tick()));
            }
            for a in r.attempts().values() {
                assert!(a.spent < VERIFY_BUDGET);
            }
            let n = w.narrative().unwrap();
            let c = n.counts();
            let o = w.obligations().unwrap().counts();
            assert_eq!(c.0, o.0);
            assert_eq!(c.1, o.2 + o.3);
            assert_eq!(c.0, c.1 + n.active_projects().len() as u32);
            assert_eq!(c.2, c.3 + c.4 + n.active_inquiries().len() as u32);
            assert!(n.active_projects().len() <= 2);
            assert!(n.project_history().len() <= PROJECT_HISTORY_CAP);
            assert!(n.inquiry_history().len() <= INQUIRY_HISTORY_CAP);
            assert!(
                n.known_projects().len() <= KNOWN_CAP && n.known_inquiries().len() <= KNOWN_CAP
            );
            assert!(n.active_inquiries().len() <= actor_count * 4);
            for p in n
                .active_projects()
                .values()
                .chain(n.project_history().iter())
            {
                if let Some(r) = p.outcome {
                    assert_eq!(r.team, p.team);
                    assert!(p.team.opened <= r.at);
                }
                if let Some(m) = p.handoff {
                    assert_eq!(m.project, p.team.id);
                    assert!(p.outcome.unwrap().at <= m.at);
                }
                if let Some(e) = p.end {
                    assert!(p.outcome.unwrap().at <= e.at);
                    if let Some(m) = p.handoff {
                        assert!(m.at <= e.at);
                    }
                }
            }
            for (&(actor, site), i) in n.active_inquiries() {
                assert_eq!((actor, site), (i.actor, i.site));
                assert!(i.end.is_none());
                assert!(matches!(
                    r.view(actor, site, w.tick()).status,
                    Status::Reported | Status::Contested
                ));
            }
            assert!(w.known_story_inquiries().all(|i| i.actor == PLAYER));
        }
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        println!("seed {seed}: stories {:?}", w.narrative().unwrap().counts());
    }
}

#[test]
fn c54_revision_stays_distinct_and_forged_revision_is_rejected() {
    let mut w = World::rumor_campaign(42);
    for _ in 0..354 {
        step(&mut w, Intent::Wait);
    }
    assert!(w.narrative().is_none());
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
