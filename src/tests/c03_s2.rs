//! S2 needs, decision order, and one Eat completion. Not an S3 movement suite.
use crate::fixture3::{self, Stock, World};
use crate::{ActorId, Command, CommandKey, Payload, Simulation, StepError};

fn start(world: World) -> Simulation {
    fixture3::into_simulation(world)
}

fn food(id: u64, owner: u64, quantity: u64) -> (u64, Stock) {
    (
        id,
        Stock {
            definition_id: 1,
            class: 1,
            quantity,
            owner_kind: 1,
            owner_id: owner,
        },
    )
}

fn hungry(actor: u32, wake: u64) -> World {
    let mut world = World::standard(100);
    world.insert_actor(actor, 0);
    let stored = world.actors.get_mut(&actor).unwrap();
    stored.channels = [1000, 0, 0, 0, 0];
    stored.next_wake = wake;
    let (id, stock) = food(1, u64::from(actor), 1);
    world.stocks.insert(id, stock);
    world.next_stock_id = 2;
    world
}

fn drive_kinds(sim: &mut Simulation, tick: u64) -> Vec<u8> {
    sim.drive(tick, &[])
        .unwrap()
        .resource_events()
        .iter()
        .map(|event| event.kind)
        .collect()
}

fn reload(sim: &Simulation) -> Simulation {
    Simulation::restore_domain_checkpoint(&sim.domain_checkpoint_bytes()).unwrap()
}

#[test]
fn eat_selects_reserves_and_consumes_once() {
    let mut sim = start(hungry(1, 1));
    let decided = drive_kinds(&mut sim, 0);
    assert_eq!(decided, vec![1, 7, 6]);
    let world = sim.world3.as_ref().unwrap();
    let actor = &world.actors[&1];
    assert_eq!(actor.task_id, 1);
    assert_eq!(actor.next_wake, 5);
    assert_eq!(actor.channels, [1000, 0, 0, 0, 0]);
    let task = &world.tasks[&1];
    assert_eq!(
        (task.goal, task.stage, task.created, task.due),
        (1, 4, 1, 3)
    );
    assert_eq!(task.reservations, vec![2]);
    assert_eq!(world.reservations[&2].state, 1);
    assert_eq!(world.stocks[&1].quantity, 1);
    let trace = &world.traces[&1];
    assert_eq!(trace.len(), 320);
    assert_eq!(trace[16], 1);
    assert_eq!(u64::from_le_bytes(trace[17..25].try_into().unwrap()), 1);
    assert_eq!(
        i64::from_le_bytes(trace[28..36].try_into().unwrap()),
        1_000_000
    );
    for index in 1..5 {
        let at = 25 + index * 59 + 3;
        assert_eq!(i64::from_le_bytes(trace[at..at + 8].try_into().unwrap()), 0);
    }

    let mut early = start(hungry(1, 2));
    assert!(drive_kinds(&mut early, 0).is_empty());
    assert_eq!(early.world3.as_ref().unwrap().actors[&1].next_wake, 2);
    assert!(early.world3.as_ref().unwrap().traces.is_empty());

    let saved = sim.domain_checkpoint_bytes();
    let mut restored = reload(&sim);
    assert_eq!(restored.domain_checkpoint_bytes(), saved);
    assert!(
        restored
            .drive(1, &[])
            .unwrap()
            .resource_events()
            .iter()
            .all(|event| event.kind != 6)
    );

    assert_eq!(drive_kinds(&mut sim, 1), vec![7]);
    assert_eq!(sim.world3.as_ref().unwrap().tasks[&1].stage, 5);
    assert_eq!(sim.world3.as_ref().unwrap().stocks[&1].quantity, 1);
    let completing = sim.domain_checkpoint_bytes();
    assert_eq!(
        reload(&sim).domain_checkpoint_bytes(),
        completing,
        "save before completion"
    );

    assert_eq!(drive_kinds(&mut sim, 2), vec![4, 3, 7]);
    let done = sim.world3.as_ref().unwrap();
    assert_eq!(done.tasks[&1].stage, 6);
    assert_eq!(done.actors[&1].task_id, 0);
    assert_eq!(done.stocks[&1].quantity, 0);
    assert_eq!(done.reservations[&2].state, 2);
    let finished = sim.domain_checkpoint_bytes();
    assert_eq!(reload(&sim).domain_checkpoint_bytes(), finished);

    assert!(drive_kinds(&mut sim, 3).is_empty());
    assert_eq!(sim.world3.as_ref().unwrap().stocks[&1].quantity, 0);
    let again = drive_kinds(&mut sim, 4);
    let world = sim.world3.as_ref().unwrap();
    assert_eq!(again, vec![7, 6]);
    assert_eq!(world.stocks[&1].quantity, 0);
    assert_eq!(world.traces[&1][16], 2);
    assert_eq!(world.tasks[&3].goal, 2);
    assert_eq!(world.reservations.len(), 1);
}

#[test]
fn decision_orders_actors_and_breaks_score_ties_toward_eat() {
    let mut world = World::standard(100);
    world.insert_actor(1, 0);
    world.insert_actor(2, 0);
    for actor in [1, 2] {
        let stored = world.actors.get_mut(&actor).unwrap();
        stored.next_wake = 1;
        stored.channels = [1, 1, 0, 0, 0];
    }
    let (left, left_stock) = food(1, 1, 1);
    let (right, right_stock) = food(2, 2, 1);
    world.stocks.insert(left, left_stock);
    world.stocks.insert(right, right_stock);
    world.next_stock_id = 3;
    let mut sim = start(world);
    let events = sim.drive(0, &[]).unwrap();
    let events = events.resource_events();
    assert_eq!(
        events
            .iter()
            .map(|event| event.actor_id)
            .collect::<Vec<_>>(),
        vec![1, 1, 1, 2, 2, 2]
    );
    assert_eq!(events[2].kind, 6);
    assert_eq!(
        events[2].payload[0], 1,
        "equal eat and rest scores keep eat"
    );
    assert_eq!(events[5].payload[0], 1);
    assert_eq!(sim.world3.as_ref().unwrap().actors[&1].task_id, 1);
    assert_eq!(sim.world3.as_ref().unwrap().actors[&2].task_id, 3);
}

#[test]
fn disabled_actor_records_winner_zero_and_overflow_rolls_back() {
    let mut world = hungry(1, 1);
    world.actors.get_mut(&1).unwrap().enabled = 0;
    let mut sim = start(world);
    let events = sim.drive(0, &[]).unwrap();
    let event = &events.resource_events()[0];
    assert_eq!(event.kind, 6);
    assert_eq!(event.payload[0], 0);
    assert_eq!(sim.world3.as_ref().unwrap().actors[&1].task_id, 0);
    assert!(sim.world3.as_ref().unwrap().tasks.is_empty());
    assert_eq!(sim.world3.as_ref().unwrap().actors[&1].next_wake, 5);

    let mut world = hungry(1, 1);
    world.actors.get_mut(&1).unwrap().roles[0] = i64::MAX;
    let mut sim = start(world);
    let before = sim.domain_checkpoint_bytes();
    assert_eq!(sim.drive(0, &[]), Err(StepError::CapacityExceeded));
    assert_eq!(sim.domain_checkpoint_bytes(), before);
    assert!(sim.world3.as_ref().unwrap().traces.is_empty());
}

#[test]
fn insufficient_food_and_foreign_hold_do_not_reserve() {
    let mut world = hungry(1, 1);
    world.actors.get_mut(&1).unwrap().home = 99;
    world.stocks.get_mut(&1).unwrap().quantity = 0;
    let mut sim = start(world);
    let events = sim.drive(0, &[]).unwrap();
    assert_eq!(
        events
            .resource_events()
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        vec![6]
    );
    assert_eq!(events.resource_events()[0].payload[0], 0);
    assert_eq!(sim.world3.as_ref().unwrap().stocks[&1].quantity, 0);

    let mut world = hungry(1, 1);
    world.actors.get_mut(&1).unwrap().home = 99;
    world.insert_actor(2, 3);
    world.insert_scripted_task(3, 2);
    world.reservations.insert(
        4,
        fixture3::Reservation {
            stock_id: 1,
            claimant: 2,
            quantity: 1,
            acquired_tick: 0,
            expiry_tick: 9,
            task_id: 3,
            state: 1,
        },
    );
    world.next_record_id = 5;
    let mut sim = start(world);
    let before = sim.domain_checkpoint_bytes();
    let events = sim.drive(0, &[]).unwrap();
    assert_eq!(events.resource_events()[0].kind, 6);
    assert_eq!(events.resource_events()[0].payload[0], 0);
    assert_eq!(sim.world3.as_ref().unwrap().stocks[&1].quantity, 1);
    assert!(sim.domain_checkpoint_bytes().len() > before.len());
}

#[test]
fn disable_cancel_and_expiry_release_the_eat_hold_once() {
    let mut sim = start(hungry(1, 1));
    assert_eq!(drive_kinds(&mut sim, 0), vec![1, 7, 6]);
    let disable = Command::new(
        CommandKey {
            target_tick: 2,
            actor_id: ActorId::ONE,
            producer_id: 1,
            sequence: 1,
        },
        Payload::SetEnabled(0),
    );
    let events = sim.drive(1, &[disable]).unwrap();
    assert_eq!(
        events
            .resource_events()
            .iter()
            .map(|event| (event.kind, event.payload.last().copied()))
            .collect::<Vec<_>>(),
        vec![(3, Some(1)), (7, Some(8)), (5, Some(1))]
    );
    assert_eq!(sim.world3.as_ref().unwrap().stocks[&1].quantity, 1);

    let mut sim = start(hungry(1, 1));
    drive_kinds(&mut sim, 0);
    let cancel = Command::new(
        CommandKey {
            target_tick: 2,
            actor_id: ActorId::ONE,
            producer_id: 1,
            sequence: 1,
        },
        Payload::CancelReservation(0),
    );
    let events = sim.drive(1, &[cancel]).unwrap();
    assert_eq!(events.resource_events()[0].kind, 3);
    assert_eq!(events.resource_events()[0].payload.last().copied(), Some(2));
    assert_eq!(sim.world3.as_ref().unwrap().stocks[&1].quantity, 1);
    let repeat = Command::new(
        CommandKey {
            target_tick: 2,
            actor_id: ActorId::ONE,
            producer_id: 1,
            sequence: 1,
        },
        Payload::CancelReservation(2),
    );
    assert!(
        sim.drive(2, &[repeat])
            .unwrap()
            .resource_events()
            .is_empty()
    );

    let mut sim = start(hungry(1, 1));
    drive_kinds(&mut sim, 0);
    sim.world3
        .as_mut()
        .unwrap()
        .reservations
        .get_mut(&2)
        .unwrap()
        .expiry_tick = 2;
    let before = sim.domain_checkpoint_bytes();
    let events = sim.drive(1, &[]).unwrap();
    assert_eq!(events.resource_events()[0].kind, 3);
    assert_eq!(events.resource_events()[0].payload.last().copied(), Some(3));
    assert_eq!(sim.world3.as_ref().unwrap().stocks[&1].quantity, 1);
    assert_ne!(sim.domain_checkpoint_bytes(), before);
}

#[test]
fn same_tick_enable_suppresses_decision_and_future_begin_still_rolls_back() {
    let mut world = hungry(1, 1);
    world.actors.get_mut(&1).unwrap().enabled = 0;
    let mut sim = start(world);
    let enable = Command::new(
        CommandKey {
            target_tick: 1,
            actor_id: ActorId::ONE,
            producer_id: 1,
            sequence: 1,
        },
        Payload::SetEnabled(1),
    );
    assert!(
        sim.drive(0, &[enable])
            .unwrap()
            .resource_events()
            .is_empty()
    );
    assert_eq!(sim.world3.as_ref().unwrap().actors[&1].enabled, 1);
    assert_eq!(sim.world3.as_ref().unwrap().actors[&1].next_wake, 5);
    assert!(sim.world3.as_ref().unwrap().tasks.is_empty());

    let mut world = World::standard(100);
    world.insert_actor(1, 0);
    world.actors.get_mut(&1).unwrap().task_id = 0;
    world.tasks.clear();
    let mut sim = start(world);
    let begin = |tick, sequence| {
        Command::new(
            CommandKey {
                target_tick: tick,
                actor_id: ActorId::ONE,
                producer_id: 1,
                sequence,
            },
            Payload::BeginTask {
                goal: 0,
                stage: 1,
                recipe_id: 0,
                target_locus: 1,
                target_site: 1,
                target_x: 0,
                target_y: 0,
                due_tick: 9,
                leg_count: 0,
                legs: [crate::Leg {
                    op: 0,
                    stock_id: 0,
                    definition_id: 0,
                    class: 0,
                    quantity: 0,
                    owner_kind_after: 0,
                    owner_id_after: 0,
                }; 4],
            },
        )
    };
    sim.drive(0, &[begin(2, 1), begin(3, 2)]).unwrap();
    sim.drive(1, &[]).unwrap();
    let stuck = sim.domain_checkpoint_bytes();
    assert_eq!(sim.drive(2, &[]), Err(StepError::InvalidState));
    assert_eq!(sim.domain_checkpoint_bytes(), stuck);
}

#[test]
fn save_before_decision_matches_a_continuous_eat() {
    let mut continuous = start(hungry(1, 1));
    let parked = continuous.domain_checkpoint_bytes();
    drive_kinds(&mut continuous, 0);
    drive_kinds(&mut continuous, 1);
    drive_kinds(&mut continuous, 2);
    let expected = continuous.domain_checkpoint_bytes();

    let mut resumed = Simulation::restore_domain_checkpoint(&parked).unwrap();
    assert!(
        resumed
            .drive(0, &[])
            .unwrap()
            .resource_events()
            .iter()
            .any(|event| event.kind == 6)
    );
    resumed.drive(1, &[]).unwrap();
    resumed.drive(2, &[]).unwrap();
    assert_eq!(resumed.domain_checkpoint_bytes(), expected);
    assert_eq!(resumed.world3.as_ref().unwrap().stocks[&1].quantity, 0);
}

#[test]
fn eat_event_contract_and_every_resume_split_match() {
    // Pack the seven specified events directly; do not use the production encoder
    // to produce the expected stream or digest.
    let payloads = [
        [2u64.to_le_bytes(), 1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(),
        [1u64.to_le_bytes().as_slice(), &[4]].concat(),
        [
            &[1],
            1u64.to_le_bytes().as_slice(),
            5u64.to_le_bytes().as_slice(),
            1_000_000i64.to_le_bytes().as_slice(),
            &[0; 32],
        ]
        .concat(),
        [1u64.to_le_bytes().as_slice(), &[5]].concat(),
        [1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(),
        [2u64.to_le_bytes().as_slice(), &[8]].concat(),
        [1u64.to_le_bytes().as_slice(), &[6]].concat(),
    ];
    let kinds = [1, 7, 6, 7, 4, 3, 7];
    let ticks = [1u64, 1, 1, 2, 3, 3, 3];
    let expected: Vec<Vec<u8>> = payloads
        .iter()
        .enumerate()
        .map(|(index, payload)| {
            [
                (index as u64 + 1).to_le_bytes().as_slice(),
                ticks[index].to_le_bytes().as_slice(),
                1u32.to_le_bytes().as_slice(),
                &[kinds[index]],
                payload,
            ]
            .concat()
        })
        .collect();
    let mut baseline = start(hungry(1, 1));
    let mut actual = Vec::new();
    let mut checkpoints = vec![baseline.domain_checkpoint_bytes()];
    for tick in 0..3 {
        actual.extend(
            baseline
                .drive(tick, &[])
                .unwrap()
                .resource_events()
                .iter()
                .map(|e| e.canonical_bytes()),
        );
        checkpoints.push(baseline.domain_checkpoint_bytes());
    }
    assert_eq!(actual, expected);
    assert_eq!(
        crate::encoding::sha256(&actual.concat())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "36a7ff5be4036b5eb39236d26a5bf2a21f7fe8c8aa118431ed460dd018d72bf8"
    );
    let mut digest = crate::encoding::sha256(b"SH:C03:EVENT-INIT:v1");
    for event in &expected {
        digest = crate::encoding::sha256(&[b"SH:C03:EVENT:v1".as_slice(), &digest, event].concat());
    }
    assert_eq!(baseline.event_digest(), digest);
    for split in 0..=3 {
        let mut resumed = Simulation::restore_domain_checkpoint(&checkpoints[split]).unwrap();
        let mut suffix = Vec::new();
        for tick in split..3 {
            suffix.extend(
                resumed
                    .drive(tick as u64, &[])
                    .unwrap()
                    .resource_events()
                    .iter()
                    .map(|e| e.canonical_bytes()),
            );
            assert_eq!(resumed.domain_checkpoint_bytes(), checkpoints[tick + 1]);
        }
        let prefix_len = [0, 3, 4, 7][split];
        assert_eq!(suffix, expected[prefix_len..]);
    }
}
