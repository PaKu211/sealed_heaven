use super::*;
use crate::living::{
    Goal, Intent, Outcome, Position, Region, Rejection, Role, contracts::ContractPhase,
};
const SHRINE: Need = Need::ShrineMedicine;
const HALL: Need = Need::HallOfferings;
fn officer(w: &World, n: Need) -> ActorId {
    w.contract_requester(n).unwrap()
}
fn place(w: &mut World, a: ActorId, p: Position) {
    let npc = w.npcs.get_mut(&a).unwrap();
    npc.position = p;
    npc.fatigue = 0;
    npc.activity.goal = Goal::Idle;
}
fn board(w: &World, n: Need) -> Board {
    w.governance().unwrap().ledger(n).board
}
fn shrine_fixture() -> World {
    let mut w = World::governance_campaign(3);
    let watch = officer(&w, SHRINE);
    let shrine = SHRINE.site(&w);
    place(&mut w, watch, shrine);
    w.supply.shrine = 1;
    w.tick = 20;
    w.advance_governance();
    w.tick = 36;
    w.advance_governance();
    assert_eq!(board(&w, SHRINE).priority, Priority::Reserve);
    w
}
#[test]
fn samples_are_spaced_bounded_and_hysteretic() {
    let a = ActorId::new(2);
    let mut l = Ledger::default();
    let r = |at, stock| Review {
        actor: a,
        at,
        stock,
    };
    l.review(r(10, 1), true, false);
    let original = l.clone();
    l.review(r(10, 1), true, false);
    l.review(r(25, 1), true, false);
    assert_eq!(l, original);
    l.review(r(26, 1), true, false);
    assert_eq!(l.board.priority, Priority::Reserve);
    assert_eq!(l.board.decision.unwrap().prior, r(10, 1));
    l.review(r(42, 5), false, true);
    assert_eq!(l.board.priority, Priority::Reserve);
    l.review(r(58, 3), false, false);
    assert_eq!(l.streak, 0);
    l.review(r(74, 5), false, true);
    l.review(r(171, 5), false, true);
    assert_eq!(l.streak, -1);
    l.review(r(187, 5), false, true);
    assert_eq!(l.board.priority, Priority::Routine);
    for t in 188..300 {
        l.contribute(t);
    }
    assert_eq!(l.history().len(), HISTORY_CAP);
    assert_eq!(l.board.changes, 2);
    assert_eq!(l.board.player_contributions, 112);
}
#[test]
fn reserve_orders_are_actual_contracts_not_free_goods_or_remote_knowledge() {
    let mut w = shrine_fixture();
    let watch = officer(&w, SHRINE);
    w.tick = 64;
    w.supply.shrine = 3;
    let original = w.supply;
    let hall = w.politics().unwrap().hall_coins;
    let lodge = w.rivalry().unwrap().lodge_coins;
    let mut remote = w.clone();
    place(&mut remote, watch, Region::sites().hall);
    assert!(!remote.governance_need(SHRINE, watch));
    let snapshot = board(&remote, SHRINE);
    remote.advance_governance();
    assert_eq!(board(&remote, SHRINE), snapshot);
    remote.advance_contracts();
    assert!(
        remote
            .contracts()
            .unwrap()
            .active
            .iter()
            .all(|c| c.need != SHRINE)
    );
    let mut control = w.clone();
    control.governance = None;
    control.advance_contracts();
    assert!(
        control
            .contracts()
            .unwrap()
            .active
            .iter()
            .all(|c| c.need != SHRINE)
    );
    w.advance_governance();
    w.advance_contracts();
    let c = w
        .contracts()
        .unwrap()
        .active
        .iter()
        .find(|c| c.need == SHRINE)
        .unwrap();
    assert_eq!((c.requester, c.phase), (watch, ContractPhase::Soliciting));
    let o = board(&w, SHRINE).last_order.unwrap();
    assert!(o.proactive);
    assert_eq!(o.contract, c.id);
    assert_eq!(w.supply, original);
    assert_eq!(w.politics().unwrap().hall_coins, hall);
    assert_eq!(w.rivalry().unwrap().lodge_coins, lodge);
    let count = board(&w, SHRINE).orders;
    w.advance_contracts();
    assert_eq!(board(&w, SHRINE).orders, count);
}
#[test]
fn real_player_delivery_can_prevent_unneeded_reserve_contract() {
    let mut w = shrine_fixture();
    w.tick = 64;
    w.supply.shrine = w.scenario.shrine_target - 1;
    let shrine = SHRINE.site(&w);
    w.player.position = shrine;
    w.player.herbs = 1;
    let mut control = w.clone();
    control.advance_contracts();
    assert!(
        control
            .contracts()
            .unwrap()
            .active
            .iter()
            .any(|c| c.need == SHRINE)
    );
    let outcome = w.apply_player(Intent::Deliver).unwrap();
    assert!(!matches!(outcome, Outcome::Rejected(_)));
    assert_eq!(w.player.herbs, 0);
    assert_eq!(w.supply.shrine, w.scenario.shrine_target);
    assert_eq!(board(&w, SHRINE).player_contributions, 1);
    assert_eq!(w.apply_player(Intent::Deliver), Err(Rejection::NoCargo));
    assert_eq!(board(&w, SHRINE).player_contributions, 1);
    w.advance_governance();
    w.advance_contracts();
    assert!(
        w.contracts()
            .unwrap()
            .active
            .iter()
            .all(|c| c.need != SHRINE)
    );
    w.tick += REVIEW_INTERVAL;
    w.advance_governance();
    assert_eq!(board(&w, SHRINE).priority, Priority::Routine);
}
#[test]
fn inspected_receipts_are_filed_once_and_player_boards_stay_dated() {
    let mut w = World::governance_campaign(3);
    let watch = officer(&w, SHRINE);
    let shrine = SHRINE.site(&w);
    place(&mut w, watch, shrine);
    w.player.position = shrine;
    w.tick = 30;
    w.observe_governance_boards();
    let old = w.known_board(SHRINE).unwrap();
    w.player.position = Region::sites().hall;
    w.retain_contract_receipt(ContractReceipt {
        case_id: 17,
        need: SHRINE,
        requester: watch,
        provider: ActorId::new(36),
        opened: 1,
        closed: 20,
        delivered: true,
        inspected: false,
    });
    assert_eq!(board(&w, SHRINE).delivered, 0);
    w.inspect_contract(watch, SHRINE);
    assert_eq!(board(&w, SHRINE).delivered, 1);
    let filed = board(&w, SHRINE);
    w.inspect_contract(watch, SHRINE);
    assert_eq!(board(&w, SHRINE), filed);
    w.observe_governance_boards();
    assert_eq!(w.known_board(SHRINE), Some(old));
    w.player.position = shrine;
    w.tick += 1;
    w.observe_governance_boards();
    let new = w.known_board(SHRINE).unwrap();
    assert_eq!(new.read_at, w.tick);
    assert_eq!(new.board.delivered, 1);
    assert_eq!(new.board.last_audit.unwrap().contract, 17);
}
#[test]
fn recognized_successor_keeps_the_hall_record_and_can_order_locally() {
    let mut w = World::governance_campaign(3);
    let former = officer(&w, HALL);
    let deputy = w
        .npcs()
        .find(|n| n.definition.role == Role::HallDeputy)
        .unwrap()
        .id();
    let hall = HALL.site(&w);
    place(&mut w, former, hall);
    w.region.as_mut().unwrap().hall_offerings = 0;
    w.rivalry.as_mut().unwrap().pressure = 0;
    w.tick = 64;
    w.advance_governance();
    w.tick = 80;
    w.advance_governance();
    assert_eq!(board(&w, HALL).priority, Priority::Reserve);
    let record = board(&w, HALL).decision.unwrap();
    w.npcs.get_mut(&former).unwrap().hp = 0;
    place(&mut w, deputy, hall);
    w.update_succession();
    w.sync_hierarchy_office();
    assert_eq!(officer(&w, HALL), deputy);
    assert!(w.is_institutional_requester(deputy));
    assert!(!w.is_institutional_requester(former));
    assert_eq!(board(&w, HALL).decision, Some(record));
    w.tick = 96;
    w.advance_governance();
    w.advance_contracts();
    let c = w
        .contracts()
        .unwrap()
        .active
        .iter()
        .find(|c| c.need == HALL)
        .unwrap();
    assert_eq!(c.requester, deputy);
    assert_eq!(c.phase, ContractPhase::Soliciting);
    assert!(board(&w, HALL).last_order.unwrap().proactive);
    assert_eq!(board(&w, HALL).review.unwrap().actor, deputy);
}
#[test]
fn successor_can_inspect_archived_case_only_locally_without_inheriting_personal_regard() {
    let mut w = World::governance_campaign(3);
    let former = officer(&w, HALL);
    let deputy = w
        .npcs()
        .find(|n| n.definition.role == Role::HallDeputy)
        .unwrap()
        .id();
    let provider = ActorId::new(36);
    let hall = HALL.site(&w);
    let shrine = SHRINE.site(&w);
    w.tick = 64;
    place(&mut w, former, hall);
    place(&mut w, deputy, hall);
    let r = ContractReceipt {
        case_id: 17,
        need: HALL,
        requester: former,
        provider,
        opened: 1,
        closed: 20,
        delivered: true,
        inspected: false,
    };
    w.retain_contract_receipt(r);
    assert!(!w.can_inspect_contract_receipt(deputy, &r));
    w.npcs.get_mut(&former).unwrap().hp = 0;
    w.update_succession();
    w.sync_hierarchy_office();
    assert_eq!(officer(&w, HALL), deputy);
    place(&mut w, deputy, shrine);
    assert!(!w.can_inspect_contract_receipt(deputy, &r));
    assert!(w.audit_goal(deputy).is_none());
    w.inspect_contract(deputy, HALL);
    assert!(!w.contract_memory().unwrap().receipts[0].inspected);
    assert_eq!(board(&w, HALL).delivered, 0);
    place(&mut w, deputy, hall);
    assert!(w.can_inspect_contract_receipt(deputy, &r));
    assert_eq!(w.audit_goal(deputy), Some(Goal::AuditContract(HALL)));
    let regard = w.contract_memory().unwrap().regard.clone();
    let history = w.contract_memory().unwrap().history.clone();
    let info = w.information().cloned();
    w.inspect_contract(deputy, HALL);
    assert!(w.contract_memory().unwrap().receipts[0].inspected);
    let a = board(&w, HALL).last_audit.unwrap();
    assert_eq!(
        (a.actor, a.requester, a.provider, a.contract),
        (deputy, former, provider, 17)
    );
    assert_eq!(w.contract_memory().unwrap().regard, regard);
    assert_eq!(w.contract_memory().unwrap().history, history);
    assert_eq!(w.information(), info.as_ref());
    let filed = board(&w, HALL);
    w.inspect_contract(deputy, HALL);
    assert_eq!(board(&w, HALL), filed);
    assert!(w.audit_goal(deputy).is_none());
}
#[test]
fn successor_testimony_preference_requires_local_contact() {
    let mut w = World::governance_campaign(3);
    let former = officer(&w, HALL);
    let deputy = w
        .npcs()
        .find(|n| n.definition.role == Role::HallDeputy)
        .unwrap()
        .id();
    let watch = officer(&w, SHRINE);
    let hall = HALL.site(&w);
    let shrine = SHRINE.site(&w);
    w.tick = 64;
    place(&mut w, former, hall);
    place(&mut w, deputy, hall);
    w.npcs.get_mut(&former).unwrap().hp = 0;
    w.update_succession();
    w.sync_hierarchy_office();
    assert_eq!(officer(&w, HALL), deputy);
    place(&mut w, watch, shrine);
    assert!(!w.institutional_testimony_target(&w.npcs[&watch], &w.npcs[&deputy]));
    place(&mut w, watch, hall);
    assert!(w.institutional_testimony_target(&w.npcs[&watch], &w.npcs[&deputy]));
}
#[test]
fn audit_caution_only_delays_optional_reserve_orders_and_older_receipts_cannot_replace_newer_outcome()
 {
    let mut w = shrine_fixture();
    let watch = officer(&w, SHRINE);
    w.tick = 640;
    w.supply.shrine = 3;
    let provider = ActorId::new(36);
    let r = ContractReceipt {
        case_id: 17,
        need: SHRINE,
        requester: watch,
        provider,
        opened: 1,
        closed: 500,
        delivered: false,
        inspected: false,
    };
    w.retain_contract_receipt(r);
    w.inspect_contract(watch, SHRINE);
    assert_eq!(board(&w, SHRINE).unfulfilled, 1);
    let inspected = board(&w, SHRINE).latest_outcome.unwrap();
    assert_eq!(inspected.finding, AuditFinding::Unfulfilled);
    assert!(!w.governance_need(SHRINE, watch));
    let mut urgent = w.clone();
    urgent.supply.shrine = 1;
    urgent.advance_contracts();
    assert!(
        urgent
            .contracts()
            .unwrap()
            .active
            .iter()
            .any(|c| c.need == SHRINE)
    );
    assert!(!board(&urgent, SHRINE).last_order.unwrap().proactive);
    w.advance_contracts();
    assert!(
        w.contracts()
            .unwrap()
            .active
            .iter()
            .all(|c| c.need != SHRINE)
    );
    w.tick = inspected.at + AUDIT_CAUTION - 1;
    assert!(!w.governance_need(SHRINE, watch));
    w.tick += 1;
    assert!(w.governance_need(SHRINE, watch));
    let mut l = Ledger::default();
    let newer = Audit {
        actor: watch,
        requester: watch,
        provider,
        at: 650,
        contract: 20,
        finding: AuditFinding::Delivered,
    };
    let older = Audit {
        at: 651,
        contract: 19,
        finding: AuditFinding::Unfulfilled,
        ..newer
    };
    l.audit(newer);
    l.audit(older);
    assert_eq!(l.board.last_audit, Some(older));
    assert_eq!(l.board.latest_outcome, Some(newer));
    assert!(l.reserve_retry_ready(651));
}
#[test]
fn natural_campaign_has_material_decisions_and_exact_replay() {
    let mut total_orders = 0;
    let mut differences = 0;
    for seed in [0, 3, 7, 42] {
        let mut w = World::governance_campaign(seed);
        let mut twin = w.clone();
        let mut legacy = World::diplomacy_campaign(seed);
        let mut changed = 0;
        for _ in 0..2000 {
            assert_eq!(
                w.step(w.tick(), Intent::Wait),
                twin.step(twin.tick(), Intent::Wait)
            );
            legacy.step(legacy.tick(), Intent::Wait).unwrap();
            if w.npcs != legacy.npcs || w.contracts() != legacy.contracts() {
                changed += 1;
            }
        }
        assert_eq!(w, twin);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        let a = board(&w, SHRINE);
        let b = board(&w, HALL);
        total_orders += a.proactive + b.proactive;
        differences += changed;
        println!("seed {seed}: shrine {a:?}; hall {b:?}; changed ticks {changed}");
    }
    println!("total reserve orders {total_orders}; changed ticks {differences}");
    assert!(
        total_orders > 0 && differences > 0,
        "C5.9 must affect real decisions naturally; tune before acceptance"
    );
}

#[test]
fn no_award_is_filed_at_office_and_only_delays_reserve_orders() {
    let mut w = shrine_fixture();
    let watch = officer(&w, SHRINE);
    w.tick = 64;
    w.supply.shrine = 3;
    w.advance_contracts();
    let c = w
        .contracts
        .as_mut()
        .unwrap()
        .active
        .iter_mut()
        .find(|c| c.need == SHRINE)
        .unwrap();
    let id = c.id;
    c.next_candidate = 2;
    assert!(c.bids.is_empty());
    let shrine = SHRINE.site(&w);
    w.player.position = shrine;
    w.observe_governance_boards();
    let prior = w.known_board(SHRINE);
    let hall = HALL.site(&w);
    place(&mut w, watch, hall);
    w.tick += 1;
    w.work_contract(watch, Goal::NegotiateContract(SHRINE));
    assert!(
        w.contracts()
            .unwrap()
            .active
            .iter()
            .all(|c| c.need != SHRINE)
    );
    let ledger = w.governance().unwrap().ledger(SHRINE);
    assert_eq!(ledger.board.no_award, 0);
    assert_eq!(ledger.pending().len(), 1);
    assert_eq!(ledger.pending()[0].contract, id);
    w.advance_governance();
    w.observe_governance_boards();
    assert_eq!(board(&w, SHRINE).no_award, 0);
    assert_eq!(w.known_board(SHRINE).unwrap().board, prior.unwrap().board);
    let failed = w.tick;
    w.tick += 96;
    place(&mut w, watch, shrine);
    w.advance_governance();
    let b = board(&w, SHRINE);
    assert_eq!(b.no_award, 1);
    assert_eq!(b.last_negotiation.unwrap().filed_at, w.tick);
    assert_eq!(b.no_award_until, failed + NO_AWARD_CAUTION);
    w.advance_governance();
    assert_eq!(board(&w, SHRINE), b);
    assert!(!w.governance_need(SHRINE, watch));
    w.advance_contracts();
    assert!(
        w.contracts()
            .unwrap()
            .active
            .iter()
            .all(|c| c.need != SHRINE)
    );
    let mut urgent = w.clone();
    urgent.supply.shrine = 1;
    urgent.advance_contracts();
    assert!(
        urgent
            .contracts()
            .unwrap()
            .active
            .iter()
            .any(|c| c.need == SHRINE)
    );
    w.tick = failed + NO_AWARD_CAUTION;
    assert!(w.governance_need(SHRINE, watch));
}

#[test]
fn local_stock_restoration_is_not_a_failed_or_awarded_delivery() {
    let mut w = World::governance_campaign(3);
    let mentor = officer(&w, HALL);
    let hall = HALL.site(&w);
    place(&mut w, mentor, hall);
    w.region.as_mut().unwrap().hall_offerings = 0;
    w.rivalry.as_mut().unwrap().pressure = 0;
    w.tick = 64;
    w.advance_governance();
    w.tick = 80;
    w.advance_governance();
    w.advance_contracts();
    let c = w
        .contracts()
        .unwrap()
        .active
        .iter()
        .find(|c| c.need == HALL)
        .unwrap();
    let id = c.id;
    assert_eq!(c.phase, ContractPhase::Soliciting);
    let mut awarded = w.clone();
    let c = awarded
        .contracts
        .as_mut()
        .unwrap()
        .active
        .iter_mut()
        .find(|c| c.need == HALL)
        .unwrap();
    c.phase = ContractPhase::Delivering;
    c.provider = Some(ActorId::new(36));
    awarded.region.as_mut().unwrap().hall_offerings = 1;
    awarded.tick += 1;
    awarded.advance_contracts();
    assert!(
        awarded
            .contracts()
            .unwrap()
            .active
            .iter()
            .any(|c| c.id == id)
    );
    w.tick += 1;
    w.region.as_mut().unwrap().hall_offerings = 1;
    let failed = w.contracts().unwrap().failed;
    w.advance_contracts();
    assert!(w.contracts().unwrap().active.iter().all(|c| c.need != HALL));
    assert_eq!(w.contracts().unwrap().failed, failed);
    assert_eq!(board(&w, HALL).restored_locally, 0);
    w.advance_governance();
    let b = board(&w, HALL);
    assert_eq!(b.restored_locally, 1);
    assert_eq!(b.no_award, 0);
    assert_eq!(b.no_award_until, 0);
    assert_eq!(b.last_negotiation.unwrap().fact.contract, id);
}
