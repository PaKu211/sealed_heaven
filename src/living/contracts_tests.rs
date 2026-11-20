use super::*;

const WATCH: ActorId = ActorId::new(2);
const MENTOR: ActorId = ActorId::new(33);
const LODGE: ActorId = ActorId::new(35);
const HALL: ActorId = ActorId::new(36);

fn wait(world: &mut World) {
    world
        .step(world.tick(), Intent::Wait)
        .expect("canonical turn");
}

fn fixture(world: &mut World, need: Need, phase: ContractPhase, provider: Option<ActorId>) {
    let registry = world.contracts.as_mut().expect("contract campaign");
    registry.opened += 1;
    registry.active.push(Contract {
        id: registry.opened,
        need,
        requester: need.requester(),
        opened: world.tick,
        phase,
        next_candidate: 0,
        bids: Vec::new(),
        provider,
    });
}

#[test]
fn revision_eleven_replays_both_seeds_and_revision_ten_is_unchanged() {
    for seed in [0, 1] {
        let mut current = World::institutional_agency(seed);
        for _ in 0..150 {
            wait(&mut current);
        }
        let bytes = current.checkpoint();
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 11);
        let mut restored = World::restore(&bytes).expect("replayed contract state");
        assert_eq!(current, restored);
        for _ in 0..20 {
            wait(&mut current);
            wait(&mut restored);
            assert_eq!(current.state_digest(), restored.state_digest());
        }
        let mut old = World::faction_agency(seed);
        for _ in 0..150 {
            wait(&mut old);
        }
        let bytes = old.checkpoint();
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 10);
        assert_eq!(World::restore(&bytes).unwrap(), old);
        assert!(old.contracts().is_none());
    }
}

#[test]
fn shrine_request_needs_an_actual_local_observer_not_distant_global_stock() {
    let mut world = World::institutional_agency(0);
    world.tick = 55;
    world.supply.shrine = 0;
    world.player.position = Position { x: 28, y: 8 };
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 20, y: 4 };
    world.advance_contracts();
    assert!(world.contracts().unwrap().active.is_empty());
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 5, y: 4 };
    world.advance_contracts();
    let registry = world.contracts().unwrap();
    assert_eq!(registry.active.len(), 1);
    assert_eq!(registry.active[0].need, Need::ShrineMedicine);
    assert_eq!(registry.history[0].kind, ContractEventKind::Observed);
    assert!(!registry.history[0].witnessed);
}

#[test]
fn hall_requests_use_the_same_contract_protocol_with_a_different_site() {
    let mut world = World::institutional_agency(0);
    world.tick = 80;
    world.supply.shrine = 5;
    world.rivalry.as_mut().unwrap().pressure = 5;
    world.region.as_mut().unwrap().hall_offerings = 0;
    world.npcs.get_mut(&MENTOR).unwrap().position = Region::sites().hall;
    world.advance_contracts();
    assert_eq!(world.contracts().unwrap().active.len(), 1);
    assert_eq!(
        world.contracts().unwrap().active[0].need,
        Need::HallOfferings
    );
    assert_eq!(world.contracts().unwrap().active[0].requester, MENTOR);
}

#[test]
fn face_to_face_bid_required_before_a_sponsor_can_be_awarded() {
    let mut world = World::institutional_agency(0);
    fixture(
        &mut world,
        Need::ShrineMedicine,
        ContractPhase::Soliciting,
        None,
    );
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 5, y: 4 };
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    assert!(world.contracts().unwrap().active[0].bids.is_empty());
    assert_eq!(world.contracts().unwrap().active[0].next_candidate, 0);
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 28, y: 4 };
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    let case = &world.contracts().unwrap().active[0];
    assert_eq!(case.next_candidate, 1);
    assert_eq!(case.bids.len(), 1);
    assert_eq!(case.bids[0].provider, HALL);
    assert_eq!(case.phase, ContractPhase::Soliciting);
}

#[test]
fn collecting_offers_does_not_telepathically_award_the_winner() {
    let mut world = World::institutional_agency(0);
    fixture(
        &mut world,
        Need::ShrineMedicine,
        ContractPhase::Soliciting,
        None,
    );
    {
        let case = world
            .contracts
            .as_mut()
            .unwrap()
            .active
            .first_mut()
            .unwrap();
        case.next_candidate = 1;
        case.bids.push(Bid {
            provider: HALL,
            quoted_coins: 2,
            local_trust: 1,
        });
    }
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 25, y: 6 };
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    let case = &world.contracts().unwrap().active[0];
    assert_eq!(case.phase, ContractPhase::Awarding);
    assert_eq!(case.provider, Some(HALL));
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 3, y: 4 };
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    assert_eq!(
        world.contracts().unwrap().active[0].phase,
        ContractPhase::Awarding
    );
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 28, y: 4 };
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    assert_eq!(
        world.contracts().unwrap().active[0].phase,
        ContractPhase::Procuring
    );
    assert!(
        world
            .contracts()
            .unwrap()
            .history
            .iter()
            .any(|event| event.kind == ContractEventKind::Awarded)
    );
}

#[test]
fn personal_refusal_at_award_can_switch_to_a_second_physical_bid() {
    let mut world = World::institutional_agency(0);
    fixture(
        &mut world,
        Need::ShrineMedicine,
        ContractPhase::Awarding,
        Some(HALL),
    );
    world.contracts.as_mut().unwrap().active[0].bids = vec![
        Bid {
            provider: HALL,
            quoted_coins: 2,
            local_trust: 1,
        },
        Bid {
            provider: LODGE,
            quoted_coins: 2,
            local_trust: 0,
        },
    ];
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 28, y: 4 };
    world
        .social
        .relations
        .entry((HALL, WATCH))
        .or_default()
        .regard = -3;
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    let case = &world.contracts().unwrap().active[0];
    assert_eq!(case.provider, Some(LODGE));
    assert_eq!(case.phase, ContractPhase::Awarding);
    assert_eq!(case.bids.len(), 1);
    assert_eq!(world.contracts().unwrap().failed, 0);
}

#[test]
fn occupied_provider_is_not_awarded_a_second_job() {
    let mut world = World::institutional_agency(0);
    fixture(
        &mut world,
        Need::HallOfferings,
        ContractPhase::Procuring,
        Some(HALL),
    );
    fixture(
        &mut world,
        Need::ShrineMedicine,
        ContractPhase::Soliciting,
        None,
    );
    {
        let case = world.contracts.as_mut().unwrap().active.last_mut().unwrap();
        case.next_candidate = 2;
        case.bids = vec![
            Bid {
                provider: HALL,
                quoted_coins: 0,
                local_trust: 8,
            },
            Bid {
                provider: LODGE,
                quoted_coins: 2,
                local_trust: 0,
            },
        ];
    }
    world.work_contract(WATCH, Goal::NegotiateContract(Need::ShrineMedicine));
    assert_eq!(world.contracts().unwrap().active[1].provider, Some(LODGE));
}

#[test]
fn two_competing_awards_can_purchase_only_one_remaining_market_herb() {
    let mut world = World::institutional_agency(0);
    fixture(
        &mut world,
        Need::HallOfferings,
        ContractPhase::Procuring,
        Some(HALL),
    );
    fixture(
        &mut world,
        Need::ShrineMedicine,
        ContractPhase::Procuring,
        Some(LODGE),
    );
    world.npcs.get_mut(&HALL).unwrap().position = Region::sites().market;
    world.npcs.get_mut(&LODGE).unwrap().position = Position { x: 26, y: 4 };
    world.region.as_mut().unwrap().market_herbs = 1;
    world.work_contract(LODGE, Goal::ProcureContract(Need::ShrineMedicine));
    world.work_contract(HALL, Goal::ProcureContract(Need::HallOfferings));
    assert_eq!(world.region().unwrap().market_herbs, 0);
    assert_eq!(world.npcs[&LODGE].herbs + world.npcs[&HALL].herbs, 1);
    assert_eq!(
        world.contracts().unwrap().active[0].phase,
        ContractPhase::Procuring
    );
}

#[test]
fn a_purchase_moves_exact_price_and_one_existing_herb() {
    let mut world = World::institutional_agency(0);
    fixture(
        &mut world,
        Need::HallOfferings,
        ContractPhase::Procuring,
        Some(HALL),
    );
    world.npcs.get_mut(&HALL).unwrap().position = Region::sites().market;
    let before_stock = world.region().unwrap().market_herbs;
    let before_cash = world.region().unwrap().market_coins;
    let before_hall = world.politics().unwrap().hall_coins;
    let price = world.market_quote().unwrap().buy;
    world.work_contract(HALL, Goal::ProcureContract(Need::HallOfferings));
    assert_eq!(world.region().unwrap().market_herbs, before_stock - 1);
    assert_eq!(world.region().unwrap().market_coins, before_cash + price);
    assert_eq!(world.politics().unwrap().hall_coins, before_hall - price);
    assert_eq!(world.npcs[&HALL].herbs, 1);
    assert_eq!(
        world.contracts().unwrap().active[0].phase,
        ContractPhase::Delivering
    );
}

#[test]
fn no_market_stock_or_funds_means_no_fabricated_shipment() {
    let mut world = World::institutional_agency(0);
    fixture(
        &mut world,
        Need::HallOfferings,
        ContractPhase::Procuring,
        Some(HALL),
    );
    world.npcs.get_mut(&HALL).unwrap().position = Region::sites().market;
    world.region.as_mut().unwrap().market_herbs = 0;
    world.work_contract(HALL, Goal::ProcureContract(Need::HallOfferings));
    assert_eq!(world.npcs[&HALL].herbs, 0);
    world.region.as_mut().unwrap().market_herbs = 3;
    world.politics.as_mut().unwrap().hall_coins = 0;
    world.work_contract(HALL, Goal::ProcureContract(Need::HallOfferings));
    assert_eq!(world.npcs[&HALL].herbs, 0);
    assert_eq!(world.region().unwrap().market_herbs, 3);
}

#[test]
fn same_fulfillment_engine_routes_cargo_to_either_real_destination() {
    for need in Need::ALL {
        let mut world = World::institutional_agency(0);
        fixture(&mut world, need, ContractPhase::Delivering, Some(HALL));
        let site = need.site(&world);
        world.npcs.get_mut(&HALL).unwrap().position = site;
        world.npcs.get_mut(&HALL).unwrap().herbs = 1;
        world.npcs.get_mut(&need.requester()).unwrap().position = Position {
            x: site.x.saturating_sub(1),
            y: site.y,
        };
        let old_stock = world.supply.shrine;
        let old_offerings = world.region().unwrap().hall_offerings;
        world.work_contract(HALL, Goal::FulfillContract(need));
        assert!(world.contracts().unwrap().active.is_empty());
        assert_eq!(world.contracts().unwrap().completed, 1);
        assert_eq!(world.npcs[&HALL].herbs, 0);
        match need {
            Need::ShrineMedicine => {
                assert_eq!(world.supply.shrine, old_stock + 1);
                assert_eq!(world.region().unwrap().hall_offerings, old_offerings);
            }
            Need::HallOfferings => {
                assert_eq!(world.supply.shrine, old_stock);
                assert_eq!(world.region().unwrap().hall_offerings, old_offerings + 1);
            }
        }
        assert!(world.social.relationship(need.requester(), HALL).regard > 0);
    }
}

#[test]
fn dead_or_companion_sponsor_cannot_complete_a_contract_or_erase_cargo() {
    let mut dead = World::institutional_agency(0);
    fixture(
        &mut dead,
        Need::ShrineMedicine,
        ContractPhase::Delivering,
        Some(HALL),
    );
    dead.npcs.get_mut(&HALL).unwrap().herbs = 1;
    dead.npcs.get_mut(&HALL).unwrap().hp = 0;
    dead.advance_contracts();
    assert!(dead.contracts().unwrap().active.is_empty());
    assert_eq!(dead.contracts().unwrap().failed, 1);
    assert_eq!(dead.npcs[&HALL].herbs, 1);

    let mut companion = World::institutional_agency(0);
    fixture(
        &mut companion,
        Need::ShrineMedicine,
        ContractPhase::Delivering,
        Some(LODGE),
    );
    companion.npcs.get_mut(&LODGE).unwrap().herbs = 1;
    companion.company.as_mut().unwrap().journey = Some(Journey {
        actor: LODGE,
        last_seen: companion.player.position,
        seen_at: 0,
    });
    companion.advance_contracts();
    assert!(companion.contracts().unwrap().active.is_empty());
    assert_eq!(companion.npcs[&LODGE].herbs, 1);
    assert_eq!(
        companion.contracts().unwrap().history[0].kind,
        ContractEventKind::Withdrawn
    );
}

#[test]
fn autonomous_shrine_response_completes_after_a_real_crisis_on_both_roads() {
    for seed in [0, 1] {
        let mut world = World::institutional_agency(seed);
        for actor in [ActorId::new(3), ActorId::new(32)] {
            world.npcs.get_mut(&actor).unwrap().hp = 0;
        }
        world.region.as_mut().unwrap().order_remaining = 0;
        world.region.as_mut().unwrap().order_in_transit = None;
        for _ in 0..600 {
            wait(&mut world);
            if world.contracts().unwrap().history.iter().any(|event| {
                event.kind == ContractEventKind::Delivered && event.need == Need::ShrineMedicine
            }) {
                break;
            }
        }
        assert!(
            world
                .contracts()
                .unwrap()
                .history
                .iter()
                .any(|event| event.kind == ContractEventKind::Observed
                    && event.need == Need::ShrineMedicine),
            "no local Qinghe request on seed {seed}"
        );
        assert!(
            world
                .contracts()
                .unwrap()
                .history
                .iter()
                .any(|event| event.kind == ContractEventKind::Delivered
                    && event.need == Need::ShrineMedicine),
            "no institution completed real Qinghe delivery on seed {seed}"
        );
        assert!(world.supply().shrine > 0);
    }
}

#[test]
fn contracts_still_tick_after_the_player_dies() {
    let mut world = World::institutional_agency(0);
    for actor in [ActorId::new(3), ActorId::new(32)] {
        world.npcs.get_mut(&actor).unwrap().hp = 0;
    }
    world.player.hp = 0;
    for _ in 0..52 {
        wait(&mut world);
    }
    assert_eq!(world.contracts().unwrap().opened, 1);
    assert!(
        world
            .contracts()
            .unwrap()
            .active
            .iter()
            .any(|case| case.need == Need::ShrineMedicine)
    );
}
