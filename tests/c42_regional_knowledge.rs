//! Integration acceptance for situated knowledge, regional history and replay.
use sealed_sim::living::{
    ActorId, Background, Direction, Intent, LoadError, Outcome, PLAYER, Position, Region, World,
    expansion::{CARRIER, DEPOT, GROWER},
    regional_news::{self, EVENT_CAP, EventKind, MAX_HOPS, REPORT_DELAY, STOCK_LIFETIME, Site},
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

#[test]
fn regional_history_and_livelihood_remain_causal_and_conserved_on_six_seeds() {
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::regional_knowledge_campaign(seed);
        let old = World::expanded_riverlands(seed);
        assert_eq!(w.expanded_region(), old.expanded_region());
        assert_eq!(w.checkpoint_revision(), 29);
        assert!(old.regional_news().is_none());
        assert!(
            w.regional_news()
                .unwrap()
                .stock(CARRIER, Site::WillowfordMarket)
                .is_none()
        );
        let initial = herbs(&w);
        let initial_coins = coins(&w);
        let mut loaded_crossing = false;
        let mut observed_remote_news = false;
        let mut reported_remote_news = false;
        let mut event_count = 0;
        let mut max_hops = 0;
        let mut loaded = 0;
        let mut arrived = 0;
        let mut stocked = 0;
        for _ in 0..2000 {
            let before = positions(&w);
            let prior = *w.frontier().unwrap();
            let prior_qinghe = w.region().unwrap().shipped_to_qinghe;
            let start = w.regional_news().unwrap().next_event();
            step(&mut w, Intent::Wait);
            let after = positions(&w);
            for (&id, &p) in &after {
                assert!(dist(before[&id], p) <= 1, "actor teleported");
            }
            assert_eq!(
                herbs(&w),
                initial + regrown(&w),
                "seed={seed} t={}",
                w.tick()
            );
            assert_eq!(coins(&w), initial_coins);
            let news = w.regional_news().unwrap();
            assert!(news.events().len() <= EVENT_CAP);
            assert!(news.receipt_count() <= 13 * EVENT_CAP && news.pending_count() <= 13 * 5);
            assert_eq!(
                news.next_event(),
                news.events().back().map_or(0, |e| e.id + 1)
            );
            let mut delivered = 0;
            let mut west_delivered = 0;
            let mut deposited = 0;
            for event in news.events().iter().filter(|event| event.id >= start) {
                event_count += 1;
                assert_eq!(event.at, w.tick());
                match event.kind {
                    EventKind::FreightLoaded => {
                        if event.actor == Some(CARRIER) {
                            assert_eq!(event.site, Site::PinefordDepot);
                            assert!(dist(after[&CARRIER], DEPOT) <= 1);
                            loaded += event.amount;
                        } else {
                            let courier = ActorId::new(32);
                            assert_eq!(event.actor, Some(courier));
                            assert_eq!(event.site, Site::WillowfordMarket);
                            assert!(dist(after[&courier], Region::sites().market) <= 1);
                        }
                    }
                    EventKind::FreightArrived => {
                        if event.actor == Some(CARRIER) {
                            assert_eq!(event.site, Site::WillowfordMarket);
                            assert!(dist(after[&CARRIER], Region::sites().market) <= 1);
                            arrived += event.amount;
                            delivered += event.amount;
                        } else {
                            let courier = ActorId::new(32);
                            assert_eq!(event.actor, Some(courier));
                            assert_eq!(event.site, Site::QingheShrine);
                            assert!(dist(after[&courier], w.scenario().shrine) <= 1);
                            west_delivered += event.amount;
                        }
                    }
                    EventKind::DepotStocked => {
                        assert_eq!(event.actor, Some(GROWER));
                        stocked += event.amount;
                        deposited += event.amount;
                    }
                    _ => {}
                }
                for (&actor, &position) in &after {
                    if let Some(r) = news.knowledge(actor, event.id) {
                        assert_eq!(r.hops, 0);
                        assert_eq!(r.origin, actor);
                        assert!(
                            dist(position, event.site.position(&w)) <= 1
                                || dist(before[&actor], event.site.position(&w)) <= 1
                        );
                    }
                }
            }
            let f = w.frontier().unwrap();
            assert_eq!(
                w.region().unwrap().shipped_to_qinghe - prior_qinghe,
                west_delivered
            );
            assert_eq!(
                f.shipped_to_willowford - prior.shipped_to_willowford,
                delivered
            );
            assert_eq!(f.stocked - prior.stocked, deposited);
            let carrier = w.npc(CARRIER).unwrap();
            loaded_crossing |= carrier.herbs > 0
                && w.expanded_region()
                    .unwrap()
                    .bridges()
                    .contains(&carrier.position);
            for (&actor, &position) in &after {
                for site in Site::ALL {
                    if let Some(k) = news.stock(actor, site) {
                        let r = k.receipt;
                        assert!(
                            r.observed_at <= r.acquired_at
                                && r.acquired_at <= w.tick()
                                && r.hops <= MAX_HOPS
                        );
                        if r.hops == 0 {
                            assert_eq!(r.origin, actor);
                            assert!(r.via.is_none());
                            if r.observed_at == w.tick() {
                                let sampled = if r.phase == 0 {
                                    before[&actor]
                                } else {
                                    position
                                };
                                assert!(dist(sampled, site.position(&w)) <= 1);
                            }
                            observed_remote_news |= actor == CARRIER
                                && site == Site::WillowfordMarket
                                && position.x >= 30;
                        } else {
                            assert!(r.via.is_some());
                            assert!(
                                r.acquired_at >= r.observed_at + u64::from(r.hops) * REPORT_DELAY
                            );
                            reported_remote_news |= dist(position, site.position(&w)) > 8;
                        }
                        max_hops = max_hops.max(r.hops);
                    }
                }
                for (e, r) in news.known_events(actor) {
                    assert_eq!(e.at, r.observed_at);
                    assert!(r.acquired_at <= w.tick() && r.hops <= MAX_HOPS);
                    if let Some(via) = r.via {
                        let previous = news.knowledge(via, e.id).unwrap();
                        assert_eq!(previous.origin, r.origin);
                        assert_eq!(previous.hops + 1, r.hops);
                        assert!(previous.acquired_at + REPORT_DELAY <= r.acquired_at);
                    } else {
                        assert_eq!(r.origin, actor);
                        assert_eq!(r.hops, 0);
                    }
                }
            }
            for site in [
                Site::WillowfordMarket,
                Site::PinefordDepot,
                Site::PinefordGrove,
            ] {
                assert!(
                    news.stock(PLAYER, site).is_none(),
                    "player never visited or talked"
                );
            }
            if matches!(w.tick(), 100 | 500 | 1000 | 2000) {
                let checkpoint = w.checkpoint();
                assert_eq!(World::restore(&checkpoint).unwrap(), w);
            }
        }
        println!(
            "seed={seed} events={event_count} stocked={stocked} loaded={loaded} arrived={arrived} remote_observation={observed_remote_news} remote_report={reported_remote_news} max_hops={max_hops}"
        );
        assert!(loaded_crossing && loaded > 0 && arrived > 0 && stocked > 0 && event_count > 4);
        assert!(observed_remote_news && reported_remote_news && max_hops > 0);
        assert_eq!(w.frontier().unwrap().stocked, stocked);
        assert_eq!(w.frontier().unwrap().shipped_to_willowford, arrived);
        assert!(w.region().unwrap().shipped_to_qinghe > 0);
    }
}

// Independent path oracle; avoid ever directly observing Willowford market.
fn next(w: &World, target: Position, reach: u32) -> Option<Direction> {
    let start = w.player().position;
    let mut q = VecDeque::from([(start, None)]);
    let mut seen = BTreeSet::from([start]);
    while let Some((at, first)) = q.pop_front() {
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
            let p = Position { x, y };
            if !w.walkable(p)
                || dist(p, Region::sites().market) <= 1
                || w.npcs().any(|n| n.alive() && n.position == p)
                || !seen.insert(p)
            {
                continue;
            }
            q.push_back((p, first.or(Some(dir))));
        }
    }
    None
}

#[test]
fn actual_player_conversation_transfers_distant_knowledge_only_after_two_turns() {
    for seed in [0, 7] {
        let mut w = World::regional_knowledge_campaign(seed);
        step(&mut w, Intent::ChooseBackground(Background::Wanderer));
        let mut contacted = false;
        for _ in 0..1000 {
            assert!(
                w.regional_news()
                    .unwrap()
                    .stock(PLAYER, Site::WillowfordMarket)
                    .is_none()
            );
            let carrier = w.npc(CARRIER).unwrap();
            let receipt = w
                .regional_news()
                .unwrap()
                .stock(CARRIER, Site::WillowfordMarket);
            if carrier.position.x >= 30
                && dist(w.player().position, carrier.position) <= 1
                && let Some(k) = receipt.filter(|k| {
                    k.receipt.hops < MAX_HOPS && k.receipt.fresh(w.tick() + 3, STOCK_LIFETIME)
                })
            {
                let sent = w.tick() + 1;
                assert_eq!(step(&mut w, Intent::Talk(CARRIER)), Outcome::Spoke(CARRIER));
                assert!(
                    w.regional_news()
                        .unwrap()
                        .stock(PLAYER, Site::WillowfordMarket)
                        .is_none()
                );
                let pending = w.checkpoint();
                let mut restored = World::restore(&pending).unwrap();
                step(&mut w, Intent::Wait);
                step(&mut restored, Intent::Wait);
                assert!(
                    w.regional_news()
                        .unwrap()
                        .stock(PLAYER, Site::WillowfordMarket)
                        .is_none()
                );
                step(&mut w, Intent::Wait);
                step(&mut restored, Intent::Wait);
                let learned = w
                    .regional_news()
                    .unwrap()
                    .stock(PLAYER, Site::WillowfordMarket)
                    .unwrap();
                assert_eq!(learned.amount, k.amount);
                assert_eq!(learned.receipt.via, Some(CARRIER));
                assert_eq!(learned.receipt.origin, k.receipt.origin);
                assert_eq!(learned.receipt.hops, k.receipt.hops + 1);
                assert_eq!(learned.receipt.acquired_at, sent + 2);
                assert_eq!(learned.receipt.observed_at, k.receipt.observed_at);
                assert_eq!(w, restored);
                println!(
                    "seed={seed} spoken_at={sent} amount={} observed_at={} position={:?}",
                    learned.amount,
                    learned.receipt.observed_at,
                    w.player().position
                );
                contacted = true;
                break;
            }
            let target = if carrier.position.x >= 30 && receipt.is_some() {
                carrier.position
            } else {
                Position { x: 43, y: 7 }
            };
            let intent = next(&w, target, 1).map_or(Intent::Wait, Intent::Move);
            step(&mut w, intent);
        }
        assert!(contacted, "carrier contact failed for seed {seed}");
    }
}

fn rechecksum(bytes: &mut [u8]) {
    let at = bytes.len() - 32;
    let checksum = Sha256::digest(&bytes[..at]);
    bytes[at..].copy_from_slice(&checksum);
}
#[test]
fn revision_29_binds_news_state_and_preserves_revision_28() {
    let mut w = World::regional_knowledge_campaign(7);
    for _ in 0..400 {
        step(&mut w, Intent::Wait);
    }
    let bytes = w.checkpoint();
    assert_eq!(&bytes[12..14], &regional_news::REVISION.to_le_bytes());
    assert_eq!(World::restore(&bytes).unwrap(), w);
    let mut relabeled = bytes.clone();
    relabeled[12..14].copy_from_slice(&28u16.to_le_bytes());
    rechecksum(&mut relabeled);
    assert_eq!(World::restore(&relabeled), Err(LoadError::ScenarioMismatch));
    let old = World::expanded_riverlands(7).checkpoint();
    let mut wrong = old.clone();
    wrong[12..14].copy_from_slice(&29u16.to_le_bytes());
    rechecksum(&mut wrong);
    assert_eq!(World::restore(&wrong), Err(LoadError::ScenarioMismatch));
    assert_eq!(World::restore(&old).unwrap().checkpoint(), old);
    let mut wrong = bytes.clone();
    let at = wrong.len() - 64;
    wrong[at] ^= 1;
    rechecksum(&mut wrong);
    assert_eq!(World::restore(&wrong), Err(LoadError::StateMismatch));
}

#[test]
fn regional_v1_golden_checkpoints_and_history_are_exact() {
    let mut rows = Vec::new();
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::regional_knowledge_campaign(seed);
        let initial = hash(&w.checkpoint());
        let map = hex(&w.expanded_region().unwrap().digest());
        for _ in 0..1000 {
            step(&mut w, Intent::Wait);
        }
        let news = w.regional_news().unwrap();
        rows.push(format!(
            "{seed},{map},{initial},{},{},{},{}",
            hash(&w.checkpoint()),
            hex(&w.state_digest()),
            news.next_event(),
            news.receipt_count()
        ));
    }
    assert_eq!(
        rows,
        include_str!("fixtures/living/c42_regional_v1.csv")
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
    );
}
