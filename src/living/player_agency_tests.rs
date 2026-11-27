use super::*;

const WATCH: ActorId = ActorId::new(2);
const MENTOR: ActorId = ActorId::new(33);
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

fn fail_and_audit(world: &mut World) {
    contract(world, Need::ShrineMedicine, HALL, ContractPhase::Delivering);
    world.npcs.get_mut(&HALL).unwrap().hp = 0;
    world.advance_contracts();
    let opened = world
        .contract_memory()
        .unwrap()
        .receipts
        .last()
        .unwrap()
        .opened;
    world.tick = opened.saturating_add(super::contracts::DEADLINE);
    let site = Need::ShrineMedicine.site(world);
    world.npcs.get_mut(&WATCH).unwrap().position = site;
    world.inspect_contract(WATCH, Need::ShrineMedicine);
}

fn far_from_shrine(world: &mut World) {
    world.player.position = Position { x: 29, y: 8 };
}

fn at_shrine(world: &mut World) {
    world.player.position = world.scenario.shrine;
}

fn beside(world: &mut World, actor: ActorId) {
    world.player.position = world.npcs[&actor].position;
}

fn deliver_pending(world: &mut World) {
    for _ in 0..3 {
        wait(world);
    }
}

#[test]
fn player_beside_an_audit_learns_it_firsthand() {
    let mut world = World::player_agency(0);
    at_shrine(&mut world);
    fail_and_audit(&mut world);
    let event = world.information().unwrap().events[0].id;
    let knowledge = world
        .information()
        .unwrap()
        .knowledge(PLAYER, event)
        .unwrap();
    assert_eq!(
        knowledge.provenance,
        Provenance::Firsthand { observer: PLAYER }
    );
    assert_eq!(
        world.information().unwrap().event(event).unwrap().finding,
        AuditFinding::Unfulfilled
    );
}

#[test]
fn a_distant_player_does_not_learn_an_audit() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    let event = world.information().unwrap().events[0].id;
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(PLAYER, event)
            .is_none()
    );
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(WATCH, event)
            .is_some()
    );
}

#[test]
fn examining_the_site_records_checked_evidence() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    at_shrine(&mut world);
    assert_eq!(
        world.step(world.tick(), Intent::Examine).unwrap(),
        Outcome::Learned(EventId(0))
    );
    assert_eq!(
        world
            .information()
            .unwrap()
            .knowledge(PLAYER, EventId(0))
            .unwrap()
            .provenance,
        Provenance::Reported {
            sender: PLAYER,
            origin: WATCH,
            hops: 1
        }
    );
}

#[test]
fn telling_checked_evidence_updates_the_institution_with_one_hop() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    at_shrine(&mut world);
    world.step(world.tick(), Intent::Examine).unwrap();
    beside(&mut world, MENTOR);
    assert_eq!(
        world
            .step(
                world.tick(),
                Intent::Testify {
                    to: MENTOR,
                    event: EventId(0)
                }
            )
            .unwrap(),
        Outcome::Testified {
            to: MENTOR,
            event: EventId(0)
        }
    );
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, EventId(0))
            .is_none()
    );
    deliver_pending(&mut world);
    assert_eq!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, EventId(0))
            .unwrap()
            .provenance,
        Provenance::Reported {
            sender: PLAYER,
            origin: WATCH,
            hops: 1
        }
    );
    assert!(world.dispute_bid_delta(MENTOR, HALL) < 0);
}

#[test]
fn unknown_testimony_is_refused_and_grants_nothing() {
    let mut world = World::player_agency(0);
    beside(&mut world, MENTOR);
    assert_eq!(
        world
            .step(
                world.tick(),
                Intent::Testify {
                    to: MENTOR,
                    event: EventId(4)
                }
            )
            .unwrap(),
        Outcome::Rejected(Rejection::Refused)
    );
    assert!(world.information().unwrap().pending.is_empty());
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, EventId(4))
            .is_none()
    );
}

#[test]
fn testimony_requires_a_face_to_face_meeting() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    at_shrine(&mut world);
    world.step(world.tick(), Intent::Examine).unwrap();
    far_from_shrine(&mut world);
    assert_eq!(
        world
            .step(
                world.tick(),
                Intent::Testify {
                    to: MENTOR,
                    event: EventId(0)
                }
            )
            .unwrap(),
        Outcome::Rejected(Rejection::OutOfReach)
    );
    assert!(
        !world
            .information()
            .unwrap()
            .pending
            .contains_key(&(MENTOR, EventId(0)))
    );
}

#[test]
fn repeating_testimony_does_not_open_a_second_dispute() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    at_shrine(&mut world);
    world.step(world.tick(), Intent::Examine).unwrap();
    beside(&mut world, MENTOR);
    world
        .step(
            world.tick(),
            Intent::Testify {
                to: MENTOR,
                event: EventId(0),
            },
        )
        .unwrap();
    assert_eq!(
        world
            .step(
                world.tick(),
                Intent::Testify {
                    to: MENTOR,
                    event: EventId(0),
                },
            )
            .unwrap(),
        Outcome::Rejected(Rejection::Refused)
    );
    deliver_pending(&mut world);
    assert_eq!(
        world
            .information()
            .unwrap()
            .disputes
            .iter()
            .filter(|dispute| dispute.institution == MENTOR && dispute.event == EventId(0))
            .count(),
        1
    );
}

#[test]
fn a_later_success_restores_cooperation_without_erasing_the_failure() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    at_shrine(&mut world);
    world.step(world.tick(), Intent::Examine).unwrap();
    beside(&mut world, MENTOR);
    world
        .step(
            world.tick(),
            Intent::Testify {
                to: MENTOR,
                event: EventId(0),
            },
        )
        .unwrap();
    deliver_pending(&mut world);
    let shrine = world.scenario.shrine;
    world.record_audit_information(
        90,
        Need::ShrineMedicine,
        WATCH,
        HALL,
        AuditFinding::Delivered,
        shrine,
        false,
    );
    at_shrine(&mut world);
    let learned = world.step(world.tick(), Intent::Examine).unwrap();
    let Outcome::Learned(success) = learned else {
        panic!("expected the delivery evidence, got {learned:?}");
    };
    beside(&mut world, MENTOR);
    world
        .step(
            world.tick(),
            Intent::Testify {
                to: MENTOR,
                event: success,
            },
        )
        .unwrap();
    deliver_pending(&mut world);
    assert_eq!(
        world
            .information()
            .unwrap()
            .event(EventId(0))
            .unwrap()
            .finding,
        AuditFinding::Unfulfilled
    );
    assert!(world.information().unwrap().disputes.iter().any(|dispute| {
        dispute.institution == MENTOR
            && dispute.event == EventId(0)
            && dispute.phase == DisputePhase::Restored
    }));
    assert_eq!(world.dispute_bid_delta(MENTOR, HALL), 0);
}

#[test]
fn telling_and_withholding_diverge_from_the_same_world() {
    let mut reported = World::player_agency(1);
    far_from_shrine(&mut reported);
    fail_and_audit(&mut reported);
    at_shrine(&mut reported);
    reported.step(reported.tick(), Intent::Examine).unwrap();
    beside(&mut reported, MENTOR);
    let silent = reported.clone();
    reported
        .step(
            reported.tick(),
            Intent::Testify {
                to: MENTOR,
                event: EventId(0),
            },
        )
        .unwrap();
    let mut silent = silent;
    wait(&mut silent);
    deliver_pending(&mut reported);
    deliver_pending(&mut silent);
    assert!(reported.dispute_bid_delta(MENTOR, HALL) < 0);
    assert_eq!(silent.dispute_bid_delta(MENTOR, HALL), 0);
    assert!(
        silent
            .information()
            .unwrap()
            .knowledge(MENTOR, EventId(0))
            .is_none()
    );
    assert_eq!(
        reported
            .information()
            .unwrap()
            .event(EventId(0))
            .unwrap()
            .finding,
        silent
            .information()
            .unwrap()
            .event(EventId(0))
            .unwrap()
            .finding
    );
    for _ in 0..8 {
        wait(&mut silent);
    }
    assert!(silent.tick() > reported.tick().saturating_sub(8));
    assert_eq!(silent.dispute_bid_delta(MENTOR, HALL), 0);
}

#[test]
fn player_death_leaves_the_finding_in_the_world() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    at_shrine(&mut world);
    world.step(world.tick(), Intent::Examine).unwrap();
    world.player.hp = 0;
    beside(&mut world, MENTOR);
    assert_eq!(
        world
            .step(
                world.tick(),
                Intent::Testify {
                    to: MENTOR,
                    event: EventId(0),
                },
            )
            .unwrap(),
        Outcome::Rejected(Rejection::Dead)
    );
    assert!(world.information().unwrap().event(EventId(0)).is_some());
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(WATCH, EventId(0))
            .is_some()
    );
    wait(&mut world);
    assert!(world.tick() > 0);
}

#[test]
fn a_dying_listener_never_receives_queued_testimony() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    at_shrine(&mut world);
    world.step(world.tick(), Intent::Examine).unwrap();
    beside(&mut world, MENTOR);
    world
        .step(
            world.tick(),
            Intent::Testify {
                to: MENTOR,
                event: EventId(0),
            },
        )
        .unwrap();
    world.npcs.get_mut(&MENTOR).unwrap().hp = 0;
    deliver_pending(&mut world);
    assert!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, EventId(0))
            .is_none()
    );
    assert!(world.information().unwrap().pending.is_empty());
}

#[test]
fn pending_testimony_is_in_the_digest_and_matching_copies_agree() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    at_shrine(&mut world);
    world.step(world.tick(), Intent::Examine).unwrap();
    beside(&mut world, MENTOR);
    let before = world.state_digest();
    let mut twin = world.clone();
    world
        .step(
            world.tick(),
            Intent::Testify {
                to: MENTOR,
                event: EventId(0),
            },
        )
        .unwrap();
    assert_ne!(world.state_digest(), before);
    assert!(
        world
            .information()
            .unwrap()
            .pending
            .contains_key(&(MENTOR, EventId(0)))
    );
    twin.step(
        twin.tick(),
        Intent::Testify {
            to: MENTOR,
            event: EventId(0),
        },
    )
    .unwrap();
    assert_eq!(twin.checkpoint(), world.checkpoint());
    assert_eq!(twin.information(), world.information());
    deliver_pending(&mut world);
    deliver_pending(&mut twin);
    assert_eq!(twin, world);
    assert!(world.dispute_bid_delta(MENTOR, HALL) < 0);

    let mut clean = World::player_agency(1);
    for _ in 0..24 {
        wait(&mut clean);
    }
    let digest = clean.state_digest();
    assert_eq!(World::restore(&clean.checkpoint()).unwrap(), clean);
    clean.information.as_mut().unwrap().known.insert(
        PLAYER,
        [(
            EventId(7),
            InfoKnowledge {
                acquired: 3,
                provenance: Provenance::Firsthand { observer: PLAYER },
            },
        )]
        .into_iter()
        .collect(),
    );
    assert_ne!(clean.state_digest(), digest);
    assert!(World::restore(&clean.checkpoint()).is_err());
}

#[test]
fn both_seeds_replay_a_player_agency_campaign() {
    for seed in [0, 1] {
        let mut world = World::player_agency(seed);
        assert!(world.is_player_agency());
        for _ in 0..40 {
            wait(&mut world);
        }
        let saved = world.checkpoint();
        assert_eq!(u16::from_le_bytes(saved[12..14].try_into().unwrap()), 14);
        let restored = World::restore(&saved).unwrap();
        assert_eq!(
            restored.information().unwrap().next_event,
            world.information().unwrap().next_event
        );
        assert_eq!(restored, world);
    }
}

#[test]
fn player_agency_keeps_event_ids_unique_past_the_ledger_cap() {
    let mut world = World::player_agency(0);
    let shrine = world.scenario.shrine;
    let mut issued = std::collections::BTreeSet::new();
    for case_id in 1..=120 {
        world.tick = u64::from(case_id);
        world.record_audit_information(
            case_id,
            Need::ShrineMedicine,
            WATCH,
            HALL,
            AuditFinding::Delivered,
            shrine,
            false,
        );
        let id = world
            .information()
            .unwrap()
            .events
            .iter()
            .find(|event| event.case_id == case_id)
            .map(|event| event.id)
            .unwrap();
        assert!(issued.insert(id));
    }
    let info = world.information().unwrap();
    assert_eq!(info.events.len(), 96);
    assert_eq!(info.next_event, 120);
    assert!(info.history.len() <= 160);
    assert!(info.pending.len() <= 64);
}

#[test]
fn revision_thirteen_rejects_player_agency_actions() {
    let mut world = World::information_disputes(0);
    far_from_shrine(&mut world);
    at_shrine(&mut world);
    // Revision 13 has no Examine tag. The old tick-consuming rejection
    // created an invalid journal; the structural refusal is now atomic.
    let before = world.checkpoint();
    let tick = world.tick();
    assert_eq!(
        world.step(tick, Intent::Examine),
        Err(StepError::UnrecordableIntent)
    );
    assert_eq!(world.tick(), tick);
    assert_eq!(world.checkpoint(), before);
    assert!(!world.information().unwrap().known.contains_key(&PLAYER));
    let mut clean = World::information_disputes(0);
    let before = clean.checkpoint();
    assert_eq!(
        clean.step(clean.tick(), Intent::Examine),
        Err(StepError::UnrecordableIntent)
    );
    assert_eq!(clean.checkpoint(), before);
    wait(&mut clean);
    assert_eq!(
        World::restore(&clean.checkpoint()).unwrap().checkpoint(),
        clean.checkpoint()
    );
    assert_eq!(
        u16::from_le_bytes(world.checkpoint()[12..14].try_into().unwrap()),
        13
    );
    let mut older = World::contract_accountability(0);
    for _ in 0..12 {
        wait(&mut older);
    }
    let bytes = older.checkpoint();
    assert_eq!(u16::from_le_bytes(bytes[12..14].try_into().unwrap()), 12);
    assert!(World::restore(&bytes).unwrap().information().is_none());
}

#[test]
fn a_long_campaign_stays_within_the_information_caps() {
    let mut world = World::player_agency(0);
    for _ in 0..200 {
        wait(&mut world);
    }
    let info = world.information().unwrap();
    assert!(info.events.len() <= 96);
    assert!(info.history.len() <= 160);
    assert!(info.pending.len() <= 64);
    assert!(info.disputes.len() <= 32);
    let saved = world.checkpoint();
    assert_eq!(World::restore(&saved).unwrap(), world);
}

#[test]
fn asking_a_witness_queues_their_account_without_borrowing_its_weight() {
    let mut world = World::player_agency(0);
    far_from_shrine(&mut world);
    fail_and_audit(&mut world);
    beside(&mut world, WATCH);
    assert_eq!(
        world.step(world.tick(), Intent::Ask(WATCH)).unwrap(),
        Outcome::Briefed {
            from: WATCH,
            event: EventId(0)
        }
    );
    deliver_pending(&mut world);
    let heard = world
        .information()
        .unwrap()
        .knowledge(PLAYER, EventId(0))
        .unwrap();
    assert_eq!(
        heard.provenance,
        Provenance::Reported {
            sender: WATCH,
            origin: WATCH,
            hops: 1
        }
    );
    beside(&mut world, MENTOR);
    assert_eq!(
        world
            .step(
                world.tick(),
                Intent::Testify {
                    to: MENTOR,
                    event: EventId(0),
                },
            )
            .unwrap(),
        Outcome::Testified {
            to: MENTOR,
            event: EventId(0)
        }
    );
    deliver_pending(&mut world);
    assert_eq!(
        world
            .information()
            .unwrap()
            .knowledge(MENTOR, EventId(0))
            .unwrap()
            .provenance,
        Provenance::Reported {
            sender: PLAYER,
            origin: WATCH,
            hops: 2
        }
    );
    assert_eq!(world.dispute_bid_delta(MENTOR, HALL), 0);
}
