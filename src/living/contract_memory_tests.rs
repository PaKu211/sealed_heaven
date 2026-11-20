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

fn delivered(world: &mut World, need: Need, provider: ActorId) {
    contract(world, need, provider, ContractPhase::Delivering);
    let site = need.site(world);
    world.npcs.get_mut(&provider).unwrap().position = site;
    world.npcs.get_mut(&provider).unwrap().herbs = 1;
    world.work_contract(provider, Goal::FulfillContract(need));
}

#[test]
fn v12_replays_canonical_worlds_and_keeps_v11_campaign_exact() {
    for seed in [0, 1] {
        let mut current = World::contract_accountability(seed);
        assert!(current.contract_memory().is_some());
        for _ in 0..200 {
            wait(&mut current);
        }
        let bytes = current.checkpoint();
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 12);
        let mut restored = World::restore(&bytes).unwrap();
        assert_eq!(current, restored);
        for _ in 0..30 {
            wait(&mut current);
            wait(&mut restored);
            assert_eq!(current.state_digest(), restored.state_digest());
        }
        let mut old = World::institutional_agency(seed);
        for _ in 0..200 {
            wait(&mut old);
        }
        let checkpoint = old.checkpoint();
        assert_eq!(u16::from_le_bytes([checkpoint[12], checkpoint[13]]), 11);
        assert_eq!(World::restore(&checkpoint).unwrap(), old);
        assert!(old.contract_memory().is_none());
    }
}

#[test]
fn no_remote_success_knowledge_and_only_requester_learns_on_site() {
    let mut world = World::contract_accountability(0);
    delivered(&mut world, Need::ShrineMedicine, HALL);
    let entry = world.contract_memory().unwrap().receipts[0];
    assert!(entry.delivered);
    assert!(!entry.inspected);
    assert_eq!(world.contract_regard(WATCH, HALL), 0);
    assert_eq!(
        world.audit_goal(WATCH),
        Some(Goal::AuditContract(Need::ShrineMedicine))
    );
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 27, y: 5 };
    world.inspect_contract(WATCH, Need::ShrineMedicine);
    assert_eq!(world.contract_regard(WATCH, HALL), 0);
    assert_eq!(world.contract_memory().unwrap().history.len(), 0);
    world.npcs.get_mut(&WATCH).unwrap().position = world.scenario.shrine;
    world.player.position = Position { x: 26, y: 4 };
    world.inspect_contract(WATCH, Need::ShrineMedicine);
    assert_eq!(world.contract_regard(WATCH, HALL), 2);
    assert_eq!(world.contract_regard(MENTOR, HALL), 0);
    assert_eq!(world.audit_goal(WATCH), None);
    assert_eq!(world.contract_memory().unwrap().history.len(), 1);
    assert!(!world.contract_memory().unwrap().history[0].witnessed);
    world.inspect_contract(WATCH, Need::ShrineMedicine);
    assert_eq!(world.contract_regard(WATCH, HALL), 2);
    assert_eq!(world.contract_memory().unwrap().history.len(), 1);
}

#[test]
fn witnessed_audit_is_logged_only_when_player_is_physically_present() {
    let mut world = World::contract_accountability(0);
    delivered(&mut world, Need::ShrineMedicine, LODGE);
    let site = world.scenario.shrine;
    world.npcs.get_mut(&WATCH).unwrap().position = site;
    world.player.position = site;
    world.inspect_contract(WATCH, Need::ShrineMedicine);
    let event = world.contract_memory().unwrap().history[0];
    assert_eq!(event.finding, AuditFinding::Delivered);
    assert!(event.witnessed);
    assert_eq!(event.provider, LODGE);
}

#[test]
fn a_canonical_player_wait_commits_the_npc_audit_on_the_same_world_tick() {
    let mut world = World::contract_accountability(0);
    delivered(&mut world, Need::ShrineMedicine, LODGE);
    world.npcs.get_mut(&WATCH).unwrap().position = world.scenario.shrine;
    world.player.position = Position { x: 25, y: 4 };
    assert_eq!(world.contract_regard(WATCH, LODGE), 0);
    wait(&mut world);
    assert_eq!(world.tick(), 1);
    assert_eq!(world.contract_regard(WATCH, LODGE), 2);
    assert_eq!(world.contract_memory().unwrap().history[0].at, 1);
    assert_eq!(
        world.npcs[&WATCH].activity.goal,
        Goal::AuditContract(Need::ShrineMedicine)
    );
}

#[test]
fn remote_death_is_not_told_to_requester_and_late_missing_delivery_is_local() {
    let mut world = World::contract_accountability(1);
    contract(
        &mut world,
        Need::ShrineMedicine,
        HALL,
        ContractPhase::Delivering,
    );
    world.npcs.get_mut(&HALL).unwrap().hp = 0;
    world.advance_contracts();
    assert!(world.contracts().unwrap().active.is_empty());
    assert_eq!(world.contract_memory().unwrap().receipts.len(), 1);
    assert_eq!(world.audit_goal(WATCH), None);
    world.npcs.get_mut(&WATCH).unwrap().position = world.scenario.shrine;
    world.inspect_contract(WATCH, Need::ShrineMedicine);
    assert_eq!(world.contract_regard(WATCH, HALL), 0);
    world.tick = super::contracts::DEADLINE;
    assert_eq!(
        world.audit_goal(WATCH),
        Some(Goal::AuditContract(Need::ShrineMedicine))
    );
    world.inspect_contract(WATCH, Need::ShrineMedicine);
    assert_eq!(world.contract_regard(WATCH, HALL), -3);
    assert_eq!(
        world.contract_memory().unwrap().history[0].finding,
        AuditFinding::Unfulfilled
    );
    assert_eq!(world.contract_memory().unwrap().history.len(), 1);
}

#[test]
fn learned_history_changes_the_next_real_bid_decision() {
    let mut world = World::contract_accountability(0);
    delivered(&mut world, Need::ShrineMedicine, HALL);
    world.npcs.get_mut(&WATCH).unwrap().position = world.scenario.shrine;
    world.inspect_contract(WATCH, Need::ShrineMedicine);
    assert_eq!(world.contract_regard(WATCH, HALL), 2);
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
            quoted_coins: 2,
            local_trust: 0,
        },
        Bid {
            provider: LODGE,
            quoted_coins: 2,
            local_trust: 1,
        },
    ];
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    assert_eq!(world.contracts().unwrap().active[0].provider, Some(HALL));
    assert_eq!(
        world.contracts().unwrap().active[0].phase,
        ContractPhase::Awarding
    );
}

#[test]
fn a_missed_obligation_can_reverse_supplier_preference_but_not_fabricate_cargo() {
    let mut world = World::contract_accountability(0);
    contract(
        &mut world,
        Need::ShrineMedicine,
        HALL,
        ContractPhase::Delivering,
    );
    world.npcs.get_mut(&HALL).unwrap().hp = 0;
    world.advance_contracts();
    world.tick = super::contracts::DEADLINE;
    world.npcs.get_mut(&WATCH).unwrap().position = world.scenario.shrine;
    world.inspect_contract(WATCH, Need::ShrineMedicine);
    assert_eq!(world.contract_regard(WATCH, HALL), -3);
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
            local_trust: 1,
        },
        Bid {
            provider: LODGE,
            quoted_coins: 2,
            local_trust: 0,
        },
    ];
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    assert_eq!(world.contracts().unwrap().active[0].provider, Some(LODGE));
    assert_eq!(world.npcs[&LODGE].herbs, 0);
}

#[test]
fn requesters_do_not_blame_an_institution_without_an_awarded_provider() {
    let mut world = World::contract_accountability(0);
    let registry = world.contracts.as_mut().unwrap();
    registry.opened = 1;
    registry.active.push(Contract {
        id: 1,
        need: Need::ShrineMedicine,
        requester: WATCH,
        opened: 0,
        phase: ContractPhase::Soliciting,
        next_candidate: 2,
        bids: Vec::new(),
        provider: None,
    });
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    assert!(world.contracts().unwrap().active.is_empty());
    assert!(world.contract_memory().unwrap().receipts.is_empty());
    assert!(world.contract_memory().unwrap().regard.is_empty());
}

#[test]
fn an_unconfirmed_candidate_is_not_mistaken_for_a_signed_supplier() {
    let mut world = World::contract_accountability(0);
    contract(
        &mut world,
        Need::ShrineMedicine,
        HALL,
        ContractPhase::Awarding,
    );
    world.npcs.get_mut(&WATCH).unwrap().hp = 0;
    world.advance_contracts();
    assert!(world.contracts().unwrap().active.is_empty());
    assert_eq!(world.contracts().unwrap().failed, 1);
    assert!(world.contract_memory().unwrap().receipts.is_empty());
    assert!(world.contract_memory().unwrap().regard.is_empty());
}

#[test]
fn dead_requesters_cannot_witness_or_perform_account_audits() {
    let mut world = World::contract_accountability(0);
    delivered(&mut world, Need::ShrineMedicine, HALL);
    world.npcs.get_mut(&WATCH).unwrap().hp = 0;
    world.npcs.get_mut(&WATCH).unwrap().position = world.scenario.shrine;
    for _ in 0..10 {
        wait(&mut world);
    }
    assert_eq!(world.contract_regard(WATCH, HALL), 0);
    assert!(!world.contract_memory().unwrap().receipts[0].inspected);
}

#[test]
fn bounded_account_history_avoids_unlimited_campaign_growth() {
    let mut world = World::contract_accountability(0);
    for case_id in 1..=120 {
        world.retain_contract_receipt(ContractReceipt {
            case_id,
            need: Need::ShrineMedicine,
            requester: WATCH,
            provider: HALL,
            opened: 0,
            closed: u64::from(case_id),
            delivered: true,
            inspected: false,
        });
    }
    let memory = world.contract_memory().unwrap();
    assert_eq!(memory.receipts.len(), 96);
    assert_eq!(memory.receipts[0].case_id, 25);
    assert_eq!(memory.receipts.last().unwrap().case_id, 120);
}

#[test]
fn many_site_audits_keep_a_bounded_history_and_clamped_ratings() {
    let mut world = World::contract_accountability(0);
    world.npcs.get_mut(&WATCH).unwrap().position = world.scenario.shrine;
    for case_id in 1..=175 {
        world.retain_contract_receipt(ContractReceipt {
            case_id,
            need: Need::ShrineMedicine,
            requester: WATCH,
            provider: HALL,
            opened: 0,
            closed: u64::from(case_id),
            delivered: true,
            inspected: false,
        });
        world.inspect_contract(WATCH, Need::ShrineMedicine);
    }
    let memory = world.contract_memory().unwrap();
    assert_eq!(memory.history.len(), 160);
    assert_eq!(memory.history[0].case_id, 16);
    assert_eq!(memory.history.last().unwrap().case_id, 175);
    assert_eq!(world.contract_regard(WATCH, HALL), 6);
}

#[test]
fn rating_changes_are_part_of_revision_twelve_state_integrity() {
    let mut world = World::contract_accountability(0);
    let before = world.state_digest();
    world
        .contract_memory
        .as_mut()
        .unwrap()
        .regard
        .insert((WATCH, HALL), 2);
    assert_ne!(world.state_digest(), before);
    assert_eq!(world.contracts().unwrap().completed, 0);
    assert_eq!(world.supply().shrine, 0);
}
