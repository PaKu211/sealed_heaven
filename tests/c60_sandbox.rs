//! Conservation, bounded situated diplomacy and replay.
use sealed_sim::living::{
    ActorId, Direction, Intent, Outcome, PLAYER, Position, World,
    circles::{HISTORY_CAP as CIRCLE_CAP, Kind, MEMBER_CAP, OBSERVED_CAP},
    development::{HISTORY_CAP as DEVELOPMENT_CAP, MAX_POINTS},
    diplomacy::{
        HISTORY_CAP as DIPLOMATIC_CAP, Kind as DiplomaticKind, MAX_REPUTATION,
        OBSERVED_CAP as DIPLOMATIC_OBSERVED, Party,
    },
    exploration::REVISION,
    governance::{HISTORY_CAP as GOVERNANCE_CAP, Priority, REVIEW_GAP, REVIEW_INTERVAL},
    narrative::{INQUIRY_HISTORY_CAP, KNOWN_CAP, PROJECT_HISTORY_CAP},
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
fn mixed_actions_preserve_material_resources_and_chart_invariants() {
    for seed in [0, 3, 7, 42] {
        let mut w = World::sandbox_campaign(seed);
        let mut twin = w.clone();
        let initial = herbs(&w);
        let money = coins(&w);
        let mut rng = u64::from(seed) + 1;
        for _ in 0..3000 {
            let previous = positions(&w);
            let before_chart = w.exploration().unwrap().clone();
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
            let chart = w.exploration().unwrap();
            assert!(before_chart.cells().all(|p| chart.knows(p)));
            assert!(chart.count() <= w.scenario().width * w.scenario().height);
            for l in w.known_landmarks() {
                assert!(chart.knows(l.position(&w)));
            }

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
            let d = w.development().unwrap();
            assert_eq!(d.profiles().len(), actor_count - 1);
            assert!(d.history().len() <= DEVELOPMENT_CAP && d.known().len() < actor_count);
            assert!(d.partners().len() < actor_count * actor_count);
            assert!(d.efforts().len() <= d.profiles().len() * 4);
            for (&actor, p) in d.profiles() {
                assert!(p.inquiry_points <= MAX_POINTS && p.partnership_points <= MAX_POINTS);
                assert!(
                    p.extended_ticks <= p.practice_ticks && p.practice_ticks <= w.tick() as u32
                );
                assert!(p.last_credit.iter().flatten().all(|&at| at <= w.tick()));
                assert!(p.checked <= n.career(actor).checked);
                assert!(p.attempted_setbacks <= n.career(actor).lapsed);
            }
            for (&(actor, site), e) in d.efforts() {
                assert!(e.ticks > 0 && e.extended_ticks <= e.ticks);
                assert_eq!(n.active_inquiries()[&(actor, site)].id, e.inquiry);
            }
            for (&(a, b), p) in d.partners() {
                assert_ne!(a, b);
                assert!(p.learned_at <= w.tick());
                assert!(p.successful + p.disrupted > 0);
                assert!(p.last_project < c.0);
            }
            assert!(
                d.known()
                    .values()
                    .all(|p| p.at <= w.tick() && d.profiles().contains_key(&p.actor))
            );
            let c = w.circles().unwrap();
            assert_eq!(
                c.count(Kind::Formed),
                c.groups().len() as u32 + c.count(Kind::Dissolved)
            );
            assert_eq!(
                c.membership().len(),
                c.groups().values().map(|g| g.members.len()).sum::<usize>()
            );
            assert!(c.membership().len() < actor_count);
            assert!(
                c.history().len() <= CIRCLE_CAP
                    && c.observed().len() <= OBSERVED_CAP
                    && c.known().len() < actor_count
            );
            assert_eq!(
                c.count(Kind::Lead),
                c.leads().len() as u32 + c.count(Kind::Checked) + c.count(Kind::Lapsed)
            );
            assert_eq!(
                c.count(Kind::Project),
                c.projects().len() as u32 + c.count(Kind::Delivered) + c.count(Kind::Disrupted)
            );
            for (&id, g) in c.groups() {
                assert_eq!(id, g.id);
                assert!(!g.members.is_empty() && g.members.len() <= MEMBER_CAP);
                assert!(g.coordinator.is_none_or(|a| g.members.contains_key(&a)));
                for (&a, m) in &g.members {
                    assert_eq!(c.membership()[&a], id);
                    assert!(m.joined <= m.met && m.met <= w.tick());
                }
            }
            for (&a, l) in c.leads() {
                assert_eq!(a, l.actor);
                assert_eq!(c.membership()[&a], l.circle);
                assert_eq!(n.active_inquiries()[&(a, l.site)].id, l.inquiry);
                assert!(l.accepted <= w.tick() && w.tick() < l.until);
            }
            for (&id, p) in c.projects() {
                assert_eq!(id, p.team.id);
                assert_eq!(n.active_projects()[&id].team, p.team);
                assert!(n.active_projects()[&id].outcome.is_none());
            }
            assert!(
                c.observed()
                    .iter()
                    .all(|e| matches!(e.kind, Kind::Formed | Kind::Joined | Kind::Elected))
            );
            let gov = w.governance().unwrap();
            for need in [
                sealed_sim::living::Need::ShrineMedicine,
                sealed_sim::living::Need::HallOfferings,
            ] {
                let ledger = gov.ledger(need);
                let b = ledger.board;
                assert!((-2..=2).contains(&ledger.streak));
                assert!(ledger.history().len() <= GOVERNANCE_CAP);
                assert!(b.proactive <= b.orders);
                assert!(ledger.pending().len() <= GOVERNANCE_CAP);
                assert!(b.no_award + b.restored_locally + b.delivered + b.unfulfilled <= b.orders);
                for n in ledger.pending() {
                    assert!(n.at <= w.tick());
                }
                if let Some(n) = b.last_negotiation {
                    assert!(n.fact.at <= n.filed_at && n.filed_at <= w.tick());
                }

                if let Some(r) = b.review {
                    assert!(r.at <= w.tick());
                }
                if let Some(decision) = b.decision {
                    assert_eq!(decision.priority, b.priority);
                    let interval = decision.current.at - decision.prior.at;
                    assert!((REVIEW_INTERVAL..=REVIEW_GAP).contains(&interval));
                    assert!(decision.current.at <= w.tick());
                } else {
                    assert_eq!(b.priority, Priority::Routine);
                }
                if let Some(order) = b.last_order {
                    assert!(order.at <= w.tick());
                    assert!(!order.proactive || order.priority == Priority::Reserve);
                }
                if let Some(audit) = b.last_audit {
                    assert!(audit.at <= w.tick());
                }
                if let Some(latest) = b.latest_outcome {
                    assert!(latest.at <= w.tick());
                    assert!(b.last_audit.is_some_and(|a| a.contract <= latest.contract));
                }
                assert_eq!(b.latest_outcome.is_some(), b.delivered + b.unfulfilled > 0);
                if let Some(known) = gov.known(need) {
                    assert!(known.read_at <= w.tick());
                    assert!(
                        known.board.orders <= b.orders
                            && known.board.changes <= b.changes
                            && known.board.player_contributions <= b.player_contributions
                    );
                }
            }
            let d = w.diplomacy().unwrap();
            assert!(
                d.exclusive_ticks() <= d.effort_ticks()
                    && u64::from(d.effort_ticks()) <= w.tick() * actor_count as u64
            );
            assert!(
                d.history().len() <= DIPLOMATIC_CAP
                    && d.observed().len() <= DIPLOMATIC_OBSERVED
                    && d.known().len() < actor_count
            );
            assert!(
                d.pacts().len() < actor_count * actor_count
                    && d.assessments().len() < actor_count * actor_count
                    && d.replies().len() < actor_count * actor_count
            );
            assert!(d.cases().len() < actor_count && d.petitions().len() < actor_count);
            assert_eq!(
                d.count(DiplomaticKind::Accord),
                d.pacts().len() as u32
                    + d.count(DiplomaticKind::Expired)
                    + d.count(DiplomaticKind::Withdrawn)
            );
            assert_eq!(
                d.count(DiplomaticKind::Received),
                d.cases().len() as u32
                    + d.count(DiplomaticKind::Confirmed)
                    + d.count(DiplomaticKind::Superseded)
                    + d.count(DiplomaticKind::Lapsed)
            );
            for (&(a, b), p) in d.pacts() {
                assert!(
                    a < b
                        && p.a != p.b
                        && p.opened <= p.last
                        && p.last <= w.tick()
                        && w.tick() < p.until
                );
            }
            for (&a, c) in d.cases() {
                assert_eq!(a, c.recipient);
                assert_ne!(a, c.reporter);
                assert_eq!(c.report.via, Some(c.reporter));
                assert_eq!(c.report.acquired_at, c.accepted);
                assert!(c.accepted <= w.tick() && w.tick() < c.until);
                assert_eq!(
                    n.active_inquiries()[&(a, c.report.sample.site)].id,
                    c.inquiry
                );
            }
            for (&(a, b), r) in d.replies() {
                assert_eq!((a, b), (r.case.recipient, r.case.reporter));
                assert!(r.case.contested);
                assert_eq!(r.checked.origin, a);
                assert!(r.checked.observed_at <= w.tick() && w.tick() < r.until);
            }
            for (&a, p) in d.petitions() {
                assert_eq!(a, p.case.recipient);
                assert_eq!(p.case.to, Party::Qinghe);
                assert_eq!(p.checked.origin, a);
                assert!(p.checked.scarce && w.tick() < p.until);
            }
            for (&(a, b), r) in d.assessments() {
                assert_ne!(a, b);
                assert!(r.score <= MAX_REPUTATION && u32::from(r.score) <= r.confirmed);
            }
            assert!(d.observed().iter().all(|e| matches!(
                e.kind,
                DiplomaticKind::Accord
                    | DiplomaticKind::Disputed
                    | DiplomaticKind::Reconciled
                    | DiplomaticKind::Returned
                    | DiplomaticKind::Withdrawn
            )));
            assert!(
                d.known()
                    .values()
                    .all(|x| x.event.at <= x.heard && x.heard <= w.tick())
            );
        }
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        println!(
            "seed {seed}: diplomatic accords {:?}",
            w.diplomacy().unwrap().count(DiplomaticKind::Accord)
        );
    }
}

#[test]
fn c59_revision_stays_distinct_and_forged_revision_is_rejected() {
    let mut w = World::governance_campaign(42);
    for _ in 0..354 {
        step(&mut w, Intent::Wait);
    }
    assert!(w.exploration().is_none());
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

#[test]
fn full_campaign_horizon_is_bounded_conservative_and_replayable() {
    use sealed_sim::living::{MAX_SAVE_BYTES, MAX_TICKS, StepError};
    let mut w = World::sandbox_campaign(3);
    let initial = herbs(&w);
    let money = coins(&w);
    let chart = w.exploration().unwrap().clone();
    for t in 0..MAX_TICKS {
        step(&mut w, Intent::Wait);
        if t % 1024 == 0 || t == MAX_TICKS - 1 {
            assert_eq!(herbs(&w), initial + regrown(&w));
            assert_eq!(coins(&w), money);
            positions(&w);
            assert_eq!(w.exploration(), Some(&chart));
            assert!(
                w.governance()
                    .unwrap()
                    .ledger(sealed_sim::living::Need::HallOfferings)
                    .history()
                    .len()
                    <= GOVERNANCE_CAP
            );
        }
    }
    let b = w.checkpoint();
    assert!(b.len() <= MAX_SAVE_BYTES);
    assert_eq!(World::restore(&b).unwrap(), w);
    assert_eq!(
        w.step(w.tick(), Intent::Wait),
        Err(StepError::HorizonReached)
    );
    assert_eq!(w.checkpoint(), b);
    println!(
        "full horizon {} ticks; checkpoint {} bytes (cap {})",
        w.tick(),
        b.len(),
        MAX_SAVE_BYTES
    );
}
