use super::*;
use std::collections::BTreeSet;

const WATCH: ActorId = ActorId::new(2);
const STEWARD: ActorId = ActorId::new(36);
const COURIER: ActorId = ActorId::new(32);
const SUPPLIER: ActorId = ActorId::new(3);

fn turn(world: &mut World, intent: Intent) -> Outcome {
    world.step(world.tick(), intent).expect("canonical step")
}

fn shortage(world: &mut World) {
    // Test fixture: earlier medicinal use exhausted the western reserve.
    // This mutation is NOT used as a replayable campaign event.
    world.supply.consumed += world.supply.source + world.supply.shrine;
    world.supply.source = 0;
    world.supply.shrine = 0;
    for actor in [COURIER, SUPPLIER] {
        let herbs = world.npcs[&actor].herbs;
        world.supply.consumed += herbs;
        let hauler = world.npcs.get_mut(&actor).unwrap();
        hauler.herbs = 0;
        hauler.hp = 0; // ordinary logistics has collapsed in this test case
    }
    let region = world.region.as_mut().unwrap();
    region.order_remaining = 0;
    region.order_in_transit = None;
    region.market_herbs = 5;
    world.politics.as_mut().unwrap().hall_coins = 18;
}

#[test]
fn v9_can_replay_both_routes_while_v8_remains_unchanged() {
    for seed in [0, 1] {
        let mut world = World::living_agency(seed);
        for _ in 0..220 {
            turn(&mut world, Intent::Wait);
            let mut positions = BTreeSet::new();
            for npc in world.npcs().filter(|npc| npc.alive()) {
                assert!(positions.insert(npc.position));
            }
            assert!(!positions.contains(&world.player().position));
        }
        let bytes = world.checkpoint();
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 9);
        let mut restored = World::restore(&bytes).unwrap();
        assert_eq!(restored, world);
        for _ in 0..12 {
            assert_eq!(
                turn(&mut world, Intent::Wait),
                turn(&mut restored, Intent::Wait)
            );
            assert_eq!(world.state_digest(), restored.state_digest());
        }
        assert_eq!(world.relief().unwrap().requests, 0);
        let mut previous = World::traffic_courtesy(seed);
        for _ in 0..70 {
            turn(&mut previous, Intent::Wait);
        }
        let previous_bytes = previous.checkpoint();
        assert_eq!(
            u16::from_le_bytes([previous_bytes[12], previous_bytes[13]]),
            8
        );
        assert_eq!(World::restore(&previous_bytes).unwrap(), previous);
        assert!(previous.relief().is_none());
    }
}

#[test]
fn crisis_needs_a_real_local_witness_and_does_not_disclose_remote_news() {
    let mut world = World::living_agency(0);
    world.tick = 51;
    shortage(&mut world);
    world.player.position = Position { x: 28, y: 8 };
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 20, y: 4 };
    world.advance_relief();
    assert_eq!(world.relief().unwrap().requests, 0);
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 5, y: 4 };
    world.advance_relief();
    let relief = world.relief().unwrap();
    assert_eq!(relief.requests, 1);
    assert_eq!(relief.active.unwrap().phase, ReliefPhase::SeekingSponsor);
    assert_eq!(relief.history.len(), 1);
    assert_eq!(relief.history[0].kind, ReliefEventKind::Observed);
    assert!(!relief.history[0].witnessed);
    world.advance_relief();
    assert_eq!(world.relief().unwrap().requests, 1);
}

#[test]
fn steward_purchases_real_stock_and_physically_delivers_without_duplication() {
    let mut world = World::living_agency(0);
    let market = Region::sites().market;
    let shrine = world.scenario.shrine;
    world.relief.as_mut().unwrap().active = Some(ReliefCase {
        opened: 1,
        messenger: WATCH,
        sponsor: STEWARD,
        phase: ReliefPhase::Procuring,
    });
    world.npcs.get_mut(&STEWARD).unwrap().position = market;
    world.npcs.get_mut(&STEWARD).unwrap().herbs = 0;
    let quote = world.market_quote().unwrap().buy;
    let initial_herbs = world.region().unwrap().market_herbs;
    let initial_market_cash = world.region().unwrap().market_coins;
    let initial_hall_cash = world.politics().unwrap().hall_coins;
    world.work_relief(STEWARD, Goal::BuyRelief);
    assert_eq!(world.region().unwrap().market_herbs, initial_herbs - 1);
    assert_eq!(
        world.region().unwrap().market_coins,
        initial_market_cash + quote
    );
    assert_eq!(
        world.politics().unwrap().hall_coins,
        initial_hall_cash - quote
    );
    assert_eq!(world.npcs[&STEWARD].herbs, 1);
    assert_eq!(
        world.relief().unwrap().active.unwrap().phase,
        ReliefPhase::Delivering
    );
    assert_eq!(
        world.relief().unwrap().history[0].kind,
        ReliefEventKind::Purchased
    );
    world.npcs.get_mut(&STEWARD).unwrap().position = shrine;
    let delivered = world.supply.npc_deliveries;
    let stocked = world.supply.shrine;
    world.work_relief(STEWARD, Goal::DeliverRelief);
    assert_eq!(world.supply.shrine, stocked + 1);
    assert_eq!(world.supply.npc_deliveries, delivered + 1);
    assert_eq!(world.npcs[&STEWARD].herbs, 0);
    assert_eq!(world.relief().unwrap().deliveries, 1);
    assert!(world.relief().unwrap().active.is_none());
    assert_eq!(
        world.relief().unwrap().history.last().unwrap().kind,
        ReliefEventKind::Delivered
    );
    // A transfer changed ownership, not the amount of medicine in existence.
    assert_eq!(
        initial_herbs + stocked,
        world.region().unwrap().market_herbs + world.supply.shrine
    );
}

#[test]
fn no_market_stock_means_no_herb_is_created() {
    let mut world = World::living_agency(0);
    world.relief.as_mut().unwrap().active = Some(ReliefCase {
        opened: 1,
        messenger: WATCH,
        sponsor: STEWARD,
        phase: ReliefPhase::Procuring,
    });
    world.npcs.get_mut(&STEWARD).unwrap().position = Region::sites().market;
    world.region.as_mut().unwrap().market_herbs = 0;
    let before = world.politics().unwrap().hall_coins;
    world.work_relief(STEWARD, Goal::BuyRelief);
    assert_eq!(world.npcs[&STEWARD].herbs, 0);
    assert_eq!(world.politics().unwrap().hall_coins, before);
    assert_eq!(world.relief().unwrap().history.len(), 0);
    assert_eq!(
        world.relief().unwrap().active.unwrap().phase,
        ReliefPhase::Procuring
    );
    world.region.as_mut().unwrap().market_herbs = 4;
    world.politics.as_mut().unwrap().hall_coins = 0;
    world.work_relief(STEWARD, Goal::BuyRelief);
    assert_eq!(world.region().unwrap().market_herbs, 4);
    assert_eq!(world.npcs[&STEWARD].herbs, 0);
}

#[test]
fn hostile_steward_can_refuse_the_messenger_after_actual_contact() {
    let mut world = World::living_agency(0);
    world.relief.as_mut().unwrap().active = Some(ReliefCase {
        opened: 1,
        messenger: WATCH,
        sponsor: STEWARD,
        phase: ReliefPhase::SeekingSponsor,
    });
    let sponsor = world.npcs[&STEWARD].position;
    world.npcs.get_mut(&WATCH).unwrap().position = Position {
        x: sponsor.x.saturating_sub(1),
        y: sponsor.y,
    };
    world
        .social
        .relations
        .entry((STEWARD, WATCH))
        .or_default()
        .regard = -3;
    let coins = world.politics().unwrap().hall_coins;
    world.work_relief(WATCH, Goal::RequestRelief);
    assert_eq!(world.relief().unwrap().failures, 1);
    assert!(world.relief().unwrap().active.is_none());
    assert_eq!(world.politics().unwrap().hall_coins, coins);
    assert_eq!(
        world.relief().unwrap().history[0].kind,
        ReliefEventKind::Failed
    );
}

#[test]
fn a_dead_sponsor_ends_the_mission_without_erasing_its_cargo() {
    let mut world = World::living_agency(0);
    world.relief.as_mut().unwrap().active = Some(ReliefCase {
        opened: 1,
        messenger: WATCH,
        sponsor: STEWARD,
        phase: ReliefPhase::Delivering,
    });
    world.npcs.get_mut(&STEWARD).unwrap().herbs = 1;
    world.npcs.get_mut(&STEWARD).unwrap().hp = 0;
    let stock = world.supply.shrine;
    world.advance_relief();
    assert_eq!(world.relief().unwrap().failures, 1);
    assert!(world.relief().unwrap().active.is_none());
    assert_eq!(world.npcs[&STEWARD].herbs, 1);
    assert_eq!(world.supply.shrine, stock);
}

#[test]
fn a_replenished_shrine_cancels_an_unfunded_request() {
    let mut world = World::living_agency(0);
    world.tick = 80;
    world.relief.as_mut().unwrap().active = Some(ReliefCase {
        opened: 60,
        messenger: WATCH,
        sponsor: STEWARD,
        phase: ReliefPhase::SeekingSponsor,
    });
    world.supply.shrine = 2;
    let herbs = world.region().unwrap().market_herbs;
    world.advance_relief();
    assert!(world.relief().unwrap().active.is_none());
    assert_eq!(world.relief().unwrap().last_closed, 80);
    assert_eq!(
        world.relief().unwrap().history[0].kind,
        ReliefEventKind::Withdrawn
    );
    assert_eq!(world.region().unwrap().market_herbs, herbs);
}

#[test]
fn unassisted_observation_can_trigger_a_physical_regional_relief_chain() {
    for seed in [0, 1] {
        let mut world = World::living_agency(seed);
        for _ in 0..50 {
            turn(&mut world, Intent::Wait);
        }
        shortage(&mut world);
        let mut saw_request = false;
        let mut finished = false;
        for _ in 0..220 {
            turn(&mut world, Intent::Wait);
            saw_request |= world.relief().unwrap().requests > 0;
            if world.relief().unwrap().deliveries > 0 {
                finished = true;
                break;
            }
        }
        assert!(
            saw_request,
            "no guard learned about the shortage on seed {seed}"
        );
        assert!(
            finished,
            "Hall never completed physical relief on seed {seed}"
        );
        let history = &world.relief().unwrap().history;
        assert!(
            history
                .iter()
                .any(|event| event.kind == ReliefEventKind::Reported)
        );
        assert!(history.iter().any(|event| matches!(
            event.kind,
            ReliefEventKind::Purchased | ReliefEventKind::Redirected
        )));
        assert!(
            history
                .iter()
                .any(|event| event.kind == ReliefEventKind::Delivered)
        );
    }
}
