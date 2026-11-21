use super::*;

const WATCH: ActorId = ActorId::new(2);
const MENTOR: ActorId = ActorId::new(33);
const LODGE: ActorId = ActorId::new(35);
const HALL: ActorId = ActorId::new(36);

fn wait(world: &mut World) {
    world.step(world.tick(), Intent::Wait).unwrap();
}

fn contract(world: &mut World, need: Need, provider: ActorId, phase: ContractPhase) {
    let registry = world.contracts.as_mut().unwrap();
    registry.opened += 1;
    registry.active.push(Contract {
        id: registry.opened,
        need,
        requester: need.requester(),
        opened: world.tick,
        phase,
        next_candidate: 0,
        bids: Vec::new(),
        provider: Some(provider),
    });
}

fn fail_and_audit(world: &mut World, need: Need, provider: ActorId) {
    contract(world, need, provider, ContractPhase::Delivering);
    world.npcs.get_mut(&provider).unwrap().hp = 0;
    world.advance_contracts();
    let opened = world
        .contract_memory()
        .unwrap()
        .receipts
        .last()
        .unwrap()
        .opened;
    world.tick = opened.saturating_add(super::contracts::DEADLINE);
    let site = need.site(world);
    world.npcs.get_mut(&need.requester()).unwrap().position = site;
    world.inspect_contract(need.requester(), need);
}

#[test]
fn firsthand_audit_creates_named_event_with_firsthand_provenance() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let info = world.information().unwrap();
    assert_eq!(info.events.len(), 1);
    let event = info.events[0];
    assert_eq!(event.provider, HALL);
    assert_eq!(event.finding, AuditFinding::Unfulfilled);
    let knowledge = info.knowledge(WATCH, event.id).unwrap();
    assert!(matches!(
        knowledge.provenance,
        Provenance::Firsthand { observer } if observer == WATCH
    ));
}

#[test]
fn unwitnessed_remote_failure_is_not_omniscient_before_audit() {
    let mut world = World::information_disputes(0);
    contract(
        &mut world,
        Need::ShrineMedicine,
        HALL,
        ContractPhase::Delivering,
    );
    world.npcs.get_mut(&HALL).unwrap().hp = 0;
    world.advance_contracts();
    assert!(world.information().unwrap().events.is_empty());
    assert!(world.information().unwrap().known.is_empty());
    assert_eq!(world.contract_regard(WATCH, HALL), 0);
}

#[test]
fn face_to_face_testimony_preserves_origin_and_hops() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, LODGE);
    let event = world.information().unwrap().events[0].id;
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    assert_eq!(world.information().unwrap().pending.len(), 1);
    let report = *world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap();
    assert_eq!(report.sender, WATCH);
    assert_eq!(report.recipient, MENTOR);
    assert_eq!(report.origin, WATCH);
    assert_eq!(report.hops, 1);
    world.tick = report.due;
    world.deliver_info_reports();
    let knowledge = world
        .information()
        .unwrap()
        .knowledge(MENTOR, event)
        .unwrap();
    assert_eq!(
        knowledge.provenance,
        Provenance::Reported {
            sender: WATCH,
            origin: WATCH,
            hops: 1
        }
    );
}

#[test]
fn distant_knowledge_waits_for_physical_delivery_delay() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let event = world.information().unwrap().events[0].id;
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    let due = world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap()
        .due;
    world.tick = due - 1;
    world.deliver_info_reports();
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, event)
            .is_none()
    );
    world.tick = due;
    world.deliver_info_reports();
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, event)
            .is_some()
    );
}

#[test]
fn repeated_testimony_does_not_multiply_dispute_effects() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let event = world.information().unwrap().events[0];
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    for _ in 0..3 {
        world.queue_info_reports();
        if let Some(due) = world
            .information()
            .unwrap()
            .pending
            .values()
            .next()
            .map(|report| report.due)
        {
            world.tick = due;
            world.deliver_info_reports();
        }
        world.learn_information(
            MENTOR,
            event,
            Provenance::Reported {
                sender: WATCH,
                origin: WATCH,
                hops: 1,
            },
            false,
        );
    }
    let info = world.information().unwrap();
    assert_eq!(
        info.disputes
            .iter()
            .filter(|dispute| dispute.institution == MENTOR && dispute.subject == HALL)
            .count(),
        1
    );
    assert_eq!(world.dispute_bid_delta(MENTOR, HALL), -2);
}

#[test]
fn conflicting_delivery_testimony_restores_trust_without_erasing_failure() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, LODGE);
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    world.tick = world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap()
        .due;
    world.deliver_info_reports();
    assert!(world.information().unwrap().disputes.iter().any(|dispute| {
        dispute.institution == MENTOR
            && dispute.subject == LODGE
            && dispute.phase == DisputePhase::Open
    }));
    let delivered_event = InfoEvent {
        id: EventId(world.information().unwrap().events.len() as u32),
        tick: world.tick,
        case_id: 99,
        need: Need::ShrineMedicine,
        requester: WATCH,
        provider: LODGE,
        finding: AuditFinding::Delivered,
        site: shrine,
    };
    world
        .information
        .as_mut()
        .unwrap()
        .events
        .push(delivered_event);
    world.learn_information(
        MENTOR,
        delivered_event,
        Provenance::Reported {
            sender: WATCH,
            origin: WATCH,
            hops: 1,
        },
        false,
    );
    let failure = world.information().unwrap().events[0];
    assert_eq!(failure.finding, AuditFinding::Unfulfilled);
    assert!(world.information().unwrap().disputes.iter().any(|dispute| {
        dispute.institution == MENTOR
            && dispute.subject == LODGE
            && dispute.event == failure.id
            && dispute.phase == DisputePhase::Restored
    }));
    assert_eq!(world.dispute_bid_delta(MENTOR, LODGE), 0);
    assert!(!world.provider_suspended(MENTOR, LODGE));
}

#[test]
fn firsthand_failure_suspends_the_provider_for_the_auditing_institution() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    assert!(world.provider_suspended(WATCH, HALL));
    assert_eq!(world.dispute_bid_delta(WATCH, HALL), -8);
    assert!(!world.provider_suspended(MENTOR, HALL));
}

#[test]
fn dispute_investigation_promotes_secondhand_claims() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, LODGE);
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    world.tick = world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap()
        .due;
    world.deliver_info_reports();
    let dispute_id = world
        .information()
        .unwrap()
        .disputes
        .iter()
        .find(|dispute| dispute.institution == MENTOR)
        .unwrap()
        .id;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.investigate_dispute(MENTOR, dispute_id);
    let phase = world
        .information()
        .unwrap()
        .disputes
        .iter()
        .find(|dispute| dispute.id == dispute_id)
        .unwrap()
        .phase;
    assert_eq!(phase, DisputePhase::Suspended);
}

#[test]
fn suspended_provider_is_excluded_from_later_contract_awards() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    assert!(world.provider_suspended(WATCH, HALL));
    contract(
        &mut world,
        Need::ShrineMedicine,
        HALL,
        ContractPhase::Soliciting,
    );
    let case = world.contracts.as_mut().unwrap().active.last_mut().unwrap();
    case.provider = None;
    case.next_candidate = 2;
    case.bids = vec![
        Bid {
            provider: HALL,
            quoted_coins: 1,
            local_trust: 8,
        },
        Bid {
            provider: LODGE,
            quoted_coins: 2,
            local_trust: 0,
        },
    ];
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    assert_eq!(world.contracts().unwrap().active[0].provider, Some(LODGE));
}

#[test]
fn mentorship_suspension_can_refuse_the_only_hall_offerings_provider() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    world.tick = world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap()
        .due;
    world.deliver_info_reports();
    let dispute_id = world
        .information()
        .unwrap()
        .disputes
        .iter()
        .find(|dispute| dispute.institution == MENTOR)
        .unwrap()
        .id;
    world.investigate_dispute(MENTOR, dispute_id);
    assert!(world.provider_suspended(MENTOR, HALL));
    contract(
        &mut world,
        Need::HallOfferings,
        HALL,
        ContractPhase::Soliciting,
    );
    let case = world.contracts.as_mut().unwrap().active.last_mut().unwrap();
    case.provider = None;
    case.next_candidate = 1;
    case.bids = vec![Bid {
        provider: HALL,
        quoted_coins: 1,
        local_trust: 8,
    }];
    world.work_contract(MENTOR, Goal::NegotiateContract(Need::HallOfferings));
    assert!(world.contracts().unwrap().active.is_empty());
}

#[test]
fn dead_reporter_blocks_pending_testimony_delivery() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let event = world.information().unwrap().events[0].id;
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    world.npcs.get_mut(&WATCH).unwrap().hp = 0;
    world.tick = world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap()
        .due;
    world.deliver_info_reports();
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, event)
            .is_none()
    );
    assert!(world.information().unwrap().pending.is_empty());
}

#[test]
fn dead_recipient_drops_en_route_institutional_reports() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let event = world.information().unwrap().events[0].id;
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    world.npcs.get_mut(&MENTOR).unwrap().hp = 0;
    world.tick = world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap()
        .due;
    world.deliver_info_reports();
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, event)
            .is_none()
    );
}

#[test]
fn multiple_audit_events_propagate_independently() {
    let mut world = World::information_disputes(0);
    let shrine = world.scenario.shrine;
    for (case_id, provider) in [(1u32, HALL), (2, LODGE)] {
        let event = InfoEvent {
            id: EventId(case_id - 1),
            tick: world.tick,
            case_id,
            need: Need::ShrineMedicine,
            requester: WATCH,
            provider,
            finding: AuditFinding::Unfulfilled,
            site: shrine,
        };
        world.information.as_mut().unwrap().events.push(event);
        world.learn_information(
            WATCH,
            event,
            Provenance::Firsthand { observer: WATCH },
            false,
        );
    }
    assert_eq!(world.information().unwrap().events.len(), 2);
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    assert_eq!(world.information().unwrap().pending.len(), 1);
    world.tick = world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap()
        .due;
    world.deliver_info_reports();
    world.queue_info_reports();
    assert_eq!(world.information().unwrap().pending.len(), 1);
    world.tick = world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap()
        .due;
    world.deliver_info_reports();
    assert_eq!(
        world
            .information()
            .unwrap()
            .known
            .get(&MENTOR)
            .map(|known| known.len())
            .unwrap_or(0),
        2
    );
}

#[test]
fn information_memory_stays_bounded() {
    let mut world = World::information_disputes(0);
    for case_id in 1..=120 {
        let event = InfoEvent {
            id: EventId(case_id - 1),
            tick: u64::from(case_id),
            case_id,
            need: Need::ShrineMedicine,
            requester: WATCH,
            provider: HALL,
            finding: AuditFinding::Unfulfilled,
            site: world.scenario.shrine,
        };
        if world.information.as_ref().unwrap().events.len() < 96 {
            world.information.as_mut().unwrap().events.push(event);
        }
        world.learn_information(
            WATCH,
            event,
            Provenance::Firsthand { observer: WATCH },
            false,
        );
    }
    assert!(world.information().unwrap().events.len() <= 96);
    assert!(
        world
            .information()
            .unwrap()
            .known
            .get(&WATCH)
            .unwrap()
            .len()
            <= 48
    );
    assert!(world.information().unwrap().disputes.len() <= 32);
}

#[test]
fn unresolved_disputes_and_pending_reports_enter_revision_thirteen_state() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, LODGE);
    assert!(world.information().unwrap().disputes.iter().any(|dispute| {
        dispute.institution == WATCH && dispute.phase == DisputePhase::Suspended
    }));
    let after_dispute = world.state_digest();
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    assert!(!world.information().unwrap().pending.is_empty());
    assert_ne!(world.state_digest(), after_dispute);
    let bytes = world.checkpoint();
    assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 13);
}

#[test]
fn stepped_campaign_save_load_keeps_information_ledger() {
    let mut world = World::information_disputes(0);
    for _ in 0..80 {
        wait(&mut world);
    }
    // Force an audited failure into the live campaign, then continue turning so the
    // journal captures later motion while the ledger remains in the digest.
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    for _ in 0..6 {
        wait(&mut world);
    }
    assert!(!world.information().unwrap().events.is_empty());
    assert!(
        world
            .information()
            .unwrap()
            .disputes
            .iter()
            .any(|dispute| { dispute.institution == WATCH && dispute.subject == HALL })
    );
    // Event-sourced restore only replays the journal; unjournaled mutations are
    // covered by digest membership above. Canonical stepped worlds still round-trip.
    let mut clean = World::information_disputes(1);
    for _ in 0..100 {
        wait(&mut clean);
    }
    let bytes = clean.checkpoint();
    assert_eq!(World::restore(&bytes).unwrap(), clean);
}

#[test]
fn v13_replays_canonical_worlds_and_keeps_v12_campaign_exact() {
    for seed in [0, 1] {
        let mut current = World::information_disputes(seed);
        assert!(current.information().is_some());
        for _ in 0..120 {
            wait(&mut current);
        }
        let bytes = current.checkpoint();
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 13);
        let mut restored = World::restore(&bytes).unwrap();
        assert_eq!(current, restored);
        for _ in 0..20 {
            wait(&mut current);
            wait(&mut restored);
            assert_eq!(current.state_digest(), restored.state_digest());
        }
        let mut previous = World::contract_accountability(seed);
        for _ in 0..120 {
            wait(&mut previous);
        }
        let checkpoint = previous.checkpoint();
        assert_eq!(u16::from_le_bytes([checkpoint[12], checkpoint[13]]), 12);
        assert_eq!(World::restore(&checkpoint).unwrap(), previous);
        assert!(previous.information().is_none());
        assert!(previous.contract_memory().is_some());
    }
}

#[test]
fn legacy_revision_eleven_stays_without_information_or_memory() {
    let mut world = World::institutional_agency(0);
    for _ in 0..40 {
        wait(&mut world);
    }
    let bytes = world.checkpoint();
    assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 11);
    let restored = World::restore(&bytes).unwrap();
    assert!(restored.information().is_none());
    assert!(restored.contract_memory().is_none());
    assert!(restored.contracts().is_some());
}

#[test]
fn player_facing_history_marks_only_witnessed_transfers() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.player.position = Position { x: 0, y: 0 };
    world.queue_info_reports();
    assert!(
        world
            .information()
            .unwrap()
            .history
            .iter()
            .filter(|event| event.kind == InfoLogKind::TestimonyQueued)
            .all(|event| !event.witnessed)
    );
    world.player.position = shrine;
    // Clear and re-queue while the player stands with both speakers.
    world.information.as_mut().unwrap().pending.clear();
    world.queue_info_reports();
    assert!(
        world
            .information()
            .unwrap()
            .history
            .iter()
            .rev()
            .find(|event| event.kind == InfoLogKind::TestimonyQueued)
            .unwrap()
            .witnessed
    );
}

#[test]
fn information_state_is_part_of_revision_thirteen_digest() {
    let mut world = World::information_disputes(0);
    let before = world.state_digest();
    world.information.as_mut().unwrap().next_dispute = 7;
    assert_ne!(world.state_digest(), before);
}

#[test]
fn hop_two_allegations_are_remembered_without_opening_disputes() {
    let mut world = World::information_disputes(0);
    let shrine = world.scenario.shrine;
    let event = InfoEvent {
        id: EventId(0),
        tick: world.tick,
        case_id: 1,
        need: Need::ShrineMedicine,
        requester: WATCH,
        provider: HALL,
        finding: AuditFinding::Unfulfilled,
        site: shrine,
    };
    world.information.as_mut().unwrap().events.push(event);
    world.learn_information(
        MENTOR,
        event,
        Provenance::Reported {
            sender: ActorId::new(32),
            origin: WATCH,
            hops: 2,
        },
        false,
    );
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, event.id)
            .is_some()
    );
    assert!(world.information().unwrap().disputes.is_empty());
    assert_eq!(world.dispute_bid_delta(MENTOR, HALL), 0);
    assert!(!world.provider_suspended(MENTOR, HALL));
}

#[test]
fn later_hop_one_testimony_upgrades_hop_two_knowledge_and_opens_one_dispute() {
    let mut world = World::information_disputes(0);
    let shrine = world.scenario.shrine;
    let event = InfoEvent {
        id: EventId(0),
        tick: world.tick,
        case_id: 1,
        need: Need::ShrineMedicine,
        requester: WATCH,
        provider: HALL,
        finding: AuditFinding::Unfulfilled,
        site: shrine,
    };
    world.information.as_mut().unwrap().events.push(event);
    world.learn_information(
        MENTOR,
        event,
        Provenance::Reported {
            sender: ActorId::new(32),
            origin: WATCH,
            hops: 2,
        },
        false,
    );
    world.learn_information(
        MENTOR,
        event,
        Provenance::Reported {
            sender: WATCH,
            origin: WATCH,
            hops: 1,
        },
        false,
    );
    let knowledge = world
        .information()
        .unwrap()
        .knowledge(MENTOR, event.id)
        .unwrap();
    assert_eq!(
        knowledge.provenance,
        Provenance::Reported {
            sender: WATCH,
            origin: WATCH,
            hops: 1
        }
    );
    assert_eq!(
        world
            .information()
            .unwrap()
            .disputes
            .iter()
            .filter(|dispute| dispute.institution == MENTOR && dispute.subject == HALL)
            .count(),
        1
    );
    assert_eq!(world.dispute_bid_delta(MENTOR, HALL), -2);
}

#[test]
fn later_successful_delivery_restores_trust_without_erasing_failure_history() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    assert!(world.provider_suspended(WATCH, HALL));
    let failure_id = world.information().unwrap().events[0].id;
    let success = InfoEvent {
        id: EventId(1),
        tick: world.tick,
        case_id: 99,
        need: Need::ShrineMedicine,
        requester: WATCH,
        provider: HALL,
        finding: AuditFinding::Delivered,
        site: world.scenario.shrine,
    };
    world.information.as_mut().unwrap().events.push(success);
    world.learn_information(
        WATCH,
        success,
        Provenance::Firsthand { observer: WATCH },
        false,
    );
    let failure = *world.information().unwrap().event(failure_id).unwrap();
    assert_eq!(failure.finding, AuditFinding::Unfulfilled);
    assert!(
        world.information().unwrap().disputes.iter().any(|dispute| {
            dispute.event == failure_id && dispute.phase == DisputePhase::Restored
        })
    );
    assert!(!world.provider_suspended(WATCH, HALL));
    assert_eq!(world.dispute_bid_delta(WATCH, HALL), 0);
}

#[test]
fn aged_suspension_restores_after_review_period_without_refuting_the_case() {
    let mut world = World::information_disputes(0);
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let failure_id = world.information().unwrap().events[0].id;
    let since = world
        .information()
        .unwrap()
        .disputes
        .iter()
        .find(|dispute| dispute.event == failure_id)
        .unwrap()
        .resolved_at
        .unwrap();
    world.tick = since + 960;
    world.advance_information_recovery();
    assert!(
        world.information().unwrap().disputes.iter().any(|dispute| {
            dispute.event == failure_id && dispute.phase == DisputePhase::Restored
        })
    );
    assert_eq!(
        world
            .information()
            .unwrap()
            .event(failure_id)
            .unwrap()
            .finding,
        AuditFinding::Unfulfilled
    );
    assert!(!world.provider_suspended(WATCH, HALL));
}

#[test]
fn full_event_ledger_recycles_settled_slots_instead_of_ignoring_new_findings() {
    let mut world = World::information_disputes(0);
    let shrine = world.scenario.shrine;
    for case_id in 1..=96 {
        let event = InfoEvent {
            id: EventId(case_id - 1),
            tick: u64::from(case_id),
            case_id,
            need: Need::ShrineMedicine,
            requester: WATCH,
            provider: HALL,
            finding: AuditFinding::Delivered,
            site: shrine,
        };
        world.information.as_mut().unwrap().events.push(event);
        world.learn_information(
            WATCH,
            event,
            Provenance::Firsthand { observer: WATCH },
            false,
        );
    }
    assert_eq!(world.information().unwrap().events.len(), 96);
    world.record_audit_information(
        200,
        Need::ShrineMedicine,
        WATCH,
        LODGE,
        AuditFinding::Unfulfilled,
        shrine,
        false,
    );
    assert_eq!(world.information().unwrap().events.len(), 96);
    assert!(
        world
            .information()
            .unwrap()
            .events
            .iter()
            .any(|event| event.case_id == 200 && event.finding == AuditFinding::Unfulfilled)
    );
    assert!(world.provider_suspended(WATCH, LODGE));
}

#[test]
fn revision_twelve_audits_never_create_information_or_suspension() {
    let mut world = World::contract_accountability(0);
    assert!(world.information().is_none());
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    assert!(world.information().is_none());
    assert_eq!(world.contract_regard(WATCH, HALL), -3);
    assert_eq!(world.dispute_bid_delta(WATCH, HALL), 0);
    assert!(!world.provider_suspended(WATCH, HALL));
    assert_eq!(
        u16::from_le_bytes(world.checkpoint()[12..14].try_into().unwrap()),
        12
    );
}

#[test]
fn private_information_history_is_not_marked_witnessed_when_player_is_absent() {
    let mut world = World::information_disputes(0);
    world.player.position = Position { x: 0, y: 0 };
    fail_and_audit(&mut world, Need::ShrineMedicine, HALL);
    let shrine = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = shrine;
    world.npcs.get_mut(&MENTOR).unwrap().position = shrine;
    world.queue_info_reports();
    world.tick = world
        .information()
        .unwrap()
        .pending
        .values()
        .next()
        .unwrap()
        .due;
    world.deliver_info_reports();
    assert!(
        world
            .information()
            .unwrap()
            .history
            .iter()
            .all(|event| !event.witnessed),
        "absent players must not receive witnessed information history"
    );
}
