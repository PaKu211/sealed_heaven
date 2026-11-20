use super::*;

const ENVOY: ActorId = ActorId::new(35);
const WATCH: ActorId = ActorId::new(2);
const STEWARD: ActorId = ActorId::new(36);

fn wait(world: &mut World) {
    world.step(world.tick(), Intent::Wait).unwrap();
}

fn active(world: &mut World, phase: FactionPhase) {
    world.faction_agency.as_mut().unwrap().active = Some(FactionCase {
        opened: world.tick,
        envoy: ENVOY,
        phase,
    });
}

#[test]
fn v10_replays_scouting_on_both_roads_without_upgrading_v9() {
    for seed in [0, 1] {
        let mut world = World::faction_agency(seed);
        for _ in 0..115 {
            wait(&mut world);
        }
        assert!(world.faction_agency_state().unwrap().surveys > 0);
        let bytes = world.checkpoint();
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 10);
        let mut restored = World::restore(&bytes).unwrap();
        assert_eq!(world, restored);
        for _ in 0..25 {
            wait(&mut world);
            wait(&mut restored);
            assert_eq!(world.state_digest(), restored.state_digest());
        }
        let mut previous = World::living_agency(seed);
        for _ in 0..115 {
            wait(&mut previous);
        }
        let old = previous.checkpoint();
        assert_eq!(u16::from_le_bytes([old[12], old[13]]), 9);
        assert_eq!(World::restore(&old).unwrap(), previous);
        assert!(previous.faction_agency_state().is_none());
        assert!(previous.relief().is_some());
    }
}

#[test]
fn envoy_cannot_detect_an_unseen_shortage_or_invent_a_report() {
    let mut world = World::faction_agency(0);
    world.tick = 50;
    world.supply.shrine = 0;
    world.player.position = Position { x: 28, y: 8 };
    world.advance_faction_agency();
    assert_eq!(world.faction_agency_state().unwrap().surveys, 1);
    world.work_faction_agency(ENVOY, Goal::SurveyQinghe);
    assert_eq!(
        world.faction_agency_state().unwrap().active.unwrap().phase,
        FactionPhase::Survey
    );
    assert!(world.faction_agency_state().unwrap().history.is_empty());

    world.npcs.get_mut(&ENVOY).unwrap().position = Position { x: 4, y: 4 };
    world.work_faction_agency(ENVOY, Goal::SurveyQinghe);
    let agency = world.faction_agency_state().unwrap();
    assert_eq!(agency.active.unwrap().phase, FactionPhase::Procuring);
    assert_eq!(agency.history.len(), 1);
    assert_eq!(agency.history[0].kind, FactionEventKind::ShortageSeen);
    assert!(!agency.history[0].witnessed);
}

#[test]
fn clear_shrine_is_observed_locally_and_clears_the_case() {
    let mut world = World::faction_agency(0);
    active(&mut world, FactionPhase::Survey);
    world.supply.shrine = 4;
    world.npcs.get_mut(&ENVOY).unwrap().position = Position { x: 4, y: 4 };
    let coins = world.rivalry().unwrap().lodge_coins;
    world.work_faction_agency(ENVOY, Goal::SurveyQinghe);
    assert!(world.faction_agency_state().unwrap().active.is_none());
    assert_eq!(world.rivalry().unwrap().lodge_coins, coins);
    assert_eq!(
        world.faction_agency_state().unwrap().history[0].kind,
        FactionEventKind::SurveyClear
    );
}

#[test]
fn lodge_purchase_moves_exactly_one_herb_and_real_money() {
    let mut world = World::faction_agency(1);
    active(&mut world, FactionPhase::Procuring);
    world.npcs.get_mut(&ENVOY).unwrap().position = Region::sites().market;
    let stock = world.region().unwrap().market_herbs;
    let coins = world.region().unwrap().market_coins;
    let lodge = world.rivalry().unwrap().lodge_coins;
    let quote = world.market_quote().unwrap().buy;
    world.work_faction_agency(ENVOY, Goal::BuyLodgeAid);
    assert_eq!(world.region().unwrap().market_herbs, stock - 1);
    assert_eq!(world.region().unwrap().market_coins, coins + quote);
    assert_eq!(world.rivalry().unwrap().lodge_coins, lodge - quote);
    assert_eq!(world.npcs[&ENVOY].herbs, 1);
    assert_eq!(
        world.faction_agency_state().unwrap().active.unwrap().phase,
        FactionPhase::Delivering
    );
}

#[test]
fn concurrent_hall_and_lodge_orders_cannot_double_spend_one_market_herb() {
    let mut world = World::faction_agency(0);
    active(&mut world, FactionPhase::Procuring);
    world.relief.as_mut().unwrap().active = Some(ReliefCase {
        opened: world.tick,
        messenger: WATCH,
        sponsor: STEWARD,
        phase: ReliefPhase::Procuring,
    });
    world.npcs.get_mut(&ENVOY).unwrap().position = Region::sites().market;
    world.npcs.get_mut(&STEWARD).unwrap().position = Position { x: 26, y: 4 };
    world.region.as_mut().unwrap().market_herbs = 1;
    world.work_faction_agency(ENVOY, Goal::BuyLodgeAid);
    world.work_relief(STEWARD, Goal::BuyRelief);
    assert_eq!(world.region().unwrap().market_herbs, 0);
    assert_eq!(world.npcs[&ENVOY].herbs + world.npcs[&STEWARD].herbs, 1);
    assert_eq!(
        world.relief().unwrap().active.unwrap().phase,
        ReliefPhase::Procuring
    );
}

#[test]
fn empty_market_or_insufficient_lodge_funds_does_not_fabricate_cargo() {
    let mut world = World::faction_agency(0);
    active(&mut world, FactionPhase::Procuring);
    world.npcs.get_mut(&ENVOY).unwrap().position = Region::sites().market;
    world.region.as_mut().unwrap().market_herbs = 0;
    let money = world.rivalry().unwrap().lodge_coins;
    world.work_faction_agency(ENVOY, Goal::BuyLodgeAid);
    assert_eq!(world.npcs[&ENVOY].herbs, 0);
    assert_eq!(world.rivalry().unwrap().lodge_coins, money);
    world.region.as_mut().unwrap().market_herbs = 2;
    world.rivalry.as_mut().unwrap().lodge_coins = 0;
    world.work_faction_agency(ENVOY, Goal::BuyLodgeAid);
    assert_eq!(world.region().unwrap().market_herbs, 2);
    assert_eq!(world.npcs[&ENVOY].herbs, 0);
}

#[test]
fn both_factions_reserve_emergency_money_instead_of_exhausting_it_on_routine_trade() {
    let mut world = World::faction_agency(0);
    let price = world.market_quote().unwrap().buy;
    world.rivalry.as_mut().unwrap().pressure = 1;
    world.rivalry.as_mut().unwrap().lodge_coins = price + 3;
    world.politics.as_mut().unwrap().hall_coins = price + 3;
    assert!(!world.rival_can_buy());
    assert!(!world.hall_can_buy());
    active(&mut world, FactionPhase::Procuring);
    world.npcs.get_mut(&ENVOY).unwrap().position = Region::sites().market;
    world.work_faction_agency(ENVOY, Goal::BuyLodgeAid);
    assert_eq!(world.rivalry().unwrap().lodge_coins, 3);
    assert_eq!(world.npcs[&ENVOY].herbs, 1);
    let previous = World::living_agency(0);
    assert!(previous.rival_can_buy());
}

#[test]
fn physical_delivery_updates_pressure_and_only_local_witnesses_give_credit() {
    let mut world = World::faction_agency(0);
    active(&mut world, FactionPhase::Delivering);
    world.npcs.get_mut(&ENVOY).unwrap().position = world.scenario.shrine;
    world.npcs.get_mut(&ENVOY).unwrap().herbs = 1;
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 5, y: 4 };
    let pressure = world.rivalry().unwrap().pressure;
    let shrine = world.supply().shrine;
    world.work_faction_agency(ENVOY, Goal::DeliverLodgeAid);
    assert_eq!(world.supply().shrine, shrine + 1);
    assert_eq!(world.npcs[&ENVOY].herbs, 0);
    assert_eq!(world.supply().npc_deliveries, 1);
    assert_eq!(world.rivalry().unwrap().pressure, pressure + 1);
    assert_eq!(world.faction_agency_state().unwrap().lodge_deliveries, 1);
    assert_eq!(world.faction_agency_state().unwrap().public_standing, -1);
    assert!(world.social.relationship(WATCH, ENVOY).regard > 0);
    // A distant delivery with no local witness still moves material but
    // cannot change Qinghe's social standing.
    active(&mut world, FactionPhase::Delivering);
    world.npcs.get_mut(&ENVOY).unwrap().herbs = 1;
    world.npcs.get_mut(&WATCH).unwrap().position = Region::sites().hall;
    world.work_faction_agency(ENVOY, Goal::DeliverLodgeAid);
    assert_eq!(world.faction_agency_state().unwrap().public_standing, -1);
}

#[test]
fn death_and_companion_promise_close_mission_without_destroying_cargo() {
    let mut world = World::faction_agency(0);
    active(&mut world, FactionPhase::Delivering);
    world.npcs.get_mut(&ENVOY).unwrap().herbs = 1;
    world.npcs.get_mut(&ENVOY).unwrap().hp = 0;
    world.advance_faction_agency();
    assert!(world.faction_agency_state().unwrap().active.is_none());
    assert_eq!(world.npcs[&ENVOY].herbs, 1);
    assert_eq!(
        world.faction_agency_state().unwrap().history[0].kind,
        FactionEventKind::Failed
    );
    let mut other = World::faction_agency(0);
    active(&mut other, FactionPhase::Delivering);
    other.npcs.get_mut(&ENVOY).unwrap().herbs = 1;
    other.company.as_mut().unwrap().journey = Some(Journey {
        actor: ENVOY,
        last_seen: other.player.position,
        seen_at: 0,
    });
    other.advance_faction_agency();
    assert!(other.faction_agency_state().unwrap().active.is_none());
    assert_eq!(other.npcs[&ENVOY].herbs, 1);
    assert_eq!(
        other.faction_agency_state().unwrap().history[0].kind,
        FactionEventKind::Withdrawn
    );
}

#[test]
fn hall_success_is_counted_only_in_revision_ten_and_without_global_social_credit() {
    let mut world = World::faction_agency(0);
    world.npcs.get_mut(&WATCH).unwrap().position = Region::sites().hall;
    world.credit_hall_relief(STEWARD, world.scenario.shrine);
    assert_eq!(world.faction_agency_state().unwrap().hall_deliveries, 1);
    assert_eq!(world.faction_agency_state().unwrap().public_standing, 0);
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 5, y: 4 };
    world.credit_hall_relief(STEWARD, world.scenario.shrine);
    assert_eq!(world.faction_agency_state().unwrap().hall_deliveries, 2);
    assert_eq!(world.faction_agency_state().unwrap().public_standing, 1);
    let old = World::living_agency(0);
    assert!(old.faction_agency_state().is_none());
}

#[test]
fn stonebank_can_complete_an_unassisted_physical_trip_on_both_roads() {
    for seed in [0, 1] {
        let mut world = World::faction_agency(seed);
        // A collapsed village hauler and absent Hall sponsor make the
        // shortage durable. The crisis fixture is not a replayable intent.
        for actor in [ActorId::new(3), ActorId::new(32), STEWARD] {
            world.npcs.get_mut(&actor).unwrap().hp = 0;
        }
        world.region.as_mut().unwrap().order_remaining = 0;
        world.region.as_mut().unwrap().order_in_transit = None;
        let mut saw_shortage = false;
        let mut finished = false;
        for _ in 0..340 {
            wait(&mut world);
            let agency = world.faction_agency_state().unwrap();
            saw_shortage |= agency
                .history
                .iter()
                .any(|e| e.kind == FactionEventKind::ShortageSeen);
            if agency.lodge_deliveries > 0 {
                finished = true;
                break;
            }
        }
        assert!(
            saw_shortage,
            "envoy did not witness scarcity on seed {seed}"
        );
        assert!(
            finished,
            "Stonebank did not deliver physical aid on seed {seed}"
        );
        let history = &world.faction_agency_state().unwrap().history;
        assert!(history.iter().any(|event| matches!(
            event.kind,
            FactionEventKind::Purchased | FactionEventKind::Redirected
        )));
        assert!(
            history
                .iter()
                .any(|e| e.kind == FactionEventKind::Delivered)
        );
        assert!(world.supply().shrine > 0);
    }
}

#[test]
fn faction_decisions_continue_even_when_the_protagonist_is_dead() {
    let mut world = World::faction_agency(0);
    world.player.hp = 0;
    for _ in 0..30 {
        wait(&mut world);
    }
    assert_eq!(world.faction_agency_state().unwrap().surveys, 1);
    assert!(world.faction_agency_state().unwrap().active.is_some());
}
