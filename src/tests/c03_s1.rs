use crate::fixture3::{self, Stock, World};
use crate::{
    ActorId, Command, CommandKey, InvalidCommand, Leg, Payload, RestoreError, Simulation,
    StepError, encoding,
};

fn command(tick: u64, actor: u32, sequence: u64, payload: Payload) -> Command {
    Command::new(
        CommandKey {
            target_tick: tick,
            actor_id: ActorId::new(actor),
            producer_id: 1,
            sequence,
        },
        payload,
    )
}

fn reserve(actor: u32, task: u64, expiry: u64) -> Command {
    command(
        1,
        actor,
        1,
        Payload::ReserveStock {
            stock_id: 1,
            quantity: 1,
            expiry_tick: expiry,
            task_id: task,
        },
    )
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha(bytes: &[u8]) -> String {
    encoding::sha256(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn zero_leg() -> Leg {
    Leg {
        op: 0,
        stock_id: 0,
        definition_id: 0,
        class: 0,
        quantity: 0,
        owner_kind_after: 0,
        owner_id_after: 0,
    }
}

fn legs(items: &[Leg]) -> [Leg; 4] {
    let mut stored = [zero_leg(); 4];
    for (index, leg) in items.iter().enumerate() {
        stored[index] = *leg;
    }
    stored
}

#[test]
fn c03_t01a_two_claimants_match_independent_blob() {
    let mut sim = Simulation::two_claimants();
    assert_eq!(sim.step(0, &[]), Err(StepError::UseDrive));
    assert_eq!(sim.tick(), 0);
    let first = reserve(1, 1, 5);
    let second = reserve(2, 2, 5);
    for batch in [[second, first], [first, second]] {
        let mut sim = Simulation::two_claimants();
        let output = sim.drive(0, &batch).unwrap();
        let events = output.resource_events();
        assert_eq!(events.len(), 4, "{:?}", events[0].canonical_bytes());
        assert_eq!(
            events.iter().map(|event| event.kind).collect::<Vec<_>>(),
            [1, 7, 2, 7]
        );
        assert_eq!(events[1].payload.last().copied(), Some(2));
        assert_eq!(events[3].payload.last().copied(), Some(4));
        assert_eq!(sim.tick(), 1);
        assert_eq!(sim.next_event_id(), 5);
        let mut digest = encoding::sha256(b"SH:C03:EVENT-INIT:v1");
        let expected = [
            "33dafb9c588fd7299c5742a873e8356e1cbda54998c3d9b2b55a4f89a2311e91",
            "40076c534936ba6eaf9fa061d5b6e3e2f00a79123e92987e036b82ab18735d62",
            "8b04d4b8805008055104ddfde0bfa51810a5f1adcdfed010e9208dfa8b97cf37",
            "5fdbc5707211fbfc309f5ed75e21eaa83ebf3d7307da60cc442a3cd7f8340908",
        ];
        for (event, digest_hex) in events.iter().zip(expected) {
            digest = encoding::sha256(
                &[
                    b"SH:C03:EVENT:v1".as_slice(),
                    &digest,
                    &event.canonical_bytes(),
                ]
                .concat(),
            );
            assert_eq!(
                hex_of(&digest),
                digest_hex,
                "{}",
                hex_of(&event.canonical_bytes())
            );
        }
        assert_eq!(sim.event_digest(), digest);
        assert_eq!(
            sha(&sim.domain_checkpoint_bytes()),
            "11f9aae46d7f0f04b6c3efead0a25c3086bc56904877d237c0a79ae032a293d9"
        );
        let restored =
            Simulation::restore_domain_checkpoint(&sim.domain_checkpoint_bytes()).unwrap();
        assert_eq!(
            restored.domain_checkpoint_bytes(),
            sim.domain_checkpoint_bytes()
        );
    }
}

#[test]
fn c03_t01d_disable_after_acquire() {
    let mut world = World::standard(100);
    world.insert_actor(1, 1);
    world.insert_scripted_task(1, 1);
    world.stocks.insert(
        1,
        Stock {
            definition_id: 1,
            class: 1,
            quantity: 1,
            owner_kind: 3,
            owner_id: 1,
        },
    );
    world.next_stock_id = 2;
    world.next_record_id = 3;
    let mut sim = fixture3::into_simulation(world);
    let events = sim.drive(0, &[reserve(1, 1, 4)]).unwrap();
    assert_eq!(events.resource_events().len(), 3);
    assert_eq!(sim.next_event_id(), 4);
    let quiet = sim
        .drive(1, &[command(3, 1, 1, Payload::SetEnabled(0))])
        .unwrap();
    assert!(quiet.resource_events().is_empty());
    assert_eq!(sim.tick(), 2);
    assert_eq!(sim.next_event_id(), 4);
    let failed = sim.drive(2, &[]).unwrap();
    assert_eq!(
        failed
            .resource_events()
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [3, 7, 5]
    );
    assert_eq!(sim.next_event_id(), 7);
    assert_eq!(sim.tick(), 3);
}

#[test]
fn c03_t01b_expiry_and_t01c_cancel() {
    let mut world = World::standard(100);
    world.insert_actor(1, 1);
    world.insert_actor(2, 0);
    world.tick = 2;
    world.insert_scripted_task(1, 1);
    let task = world.tasks.get_mut(&1).unwrap();
    task.stage = 5;
    task.due = 3;
    task.progress = 2;
    task.reservations = vec![3];
    world.reservations.insert(
        3,
        fixture3::Reservation {
            stock_id: 1,
            claimant: 1,
            quantity: 1,
            acquired_tick: 1,
            expiry_tick: 3,
            task_id: 1,
            state: 1,
        },
    );
    world.next_record_id = 4;
    let mut sim = fixture3::into_simulation(world);
    let expired = sim.drive(2, &[]).unwrap();
    assert_eq!(
        expired
            .resource_events()
            .iter()
            .map(|event| (event.kind, *event.payload.last().unwrap()))
            .collect::<Vec<_>>(),
        [(3, 3), (7, 8), (5, 3)]
    );
    assert!(sim.drive(3, &[]).unwrap().resource_events().is_empty());

    let mut world = World::standard(100);
    world.insert_actor(1, 1);
    world.insert_actor(2, 0);
    world.tick = 2;
    world.insert_scripted_task(1, 1);
    let task = world.tasks.get_mut(&1).unwrap();
    task.stage = 5;
    task.due = 9;
    task.reservations = vec![3];
    world.reservations.insert(
        3,
        fixture3::Reservation {
            stock_id: 1,
            claimant: 1,
            quantity: 1,
            acquired_tick: 1,
            expiry_tick: 9,
            task_id: 1,
            state: 1,
        },
    );
    world.next_record_id = 4;
    let before = fixture3::into_simulation(world.clone());
    let mut sim = fixture3::into_simulation(world);
    let wrong = command(3, 2, 1, Payload::CancelReservation(3));
    assert_eq!(
        sim.drive(2, &[wrong]),
        Err(StepError::Command {
            key: wrong.key(),
            reason: InvalidCommand::InvalidPayload,
        })
    );
    assert_eq!(
        sim.domain_checkpoint_bytes(),
        before.domain_checkpoint_bytes()
    );
    let cancelled = sim
        .drive(2, &[command(3, 1, 1, Payload::CancelReservation(3))])
        .unwrap();
    assert_eq!(
        cancelled
            .resource_events()
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [3, 7, 5]
    );
    let again = sim
        .drive(3, &[command(4, 1, 1, Payload::CancelReservation(3))])
        .unwrap();
    assert!(again.resource_events().is_empty());
}

#[test]
fn c03_t01e_completion_replay_is_exactly_once() {
    let mut world = World::standard(100);
    world.insert_actor(1, 0);
    world.stocks.insert(
        10,
        Stock {
            definition_id: 1,
            class: 1,
            quantity: 1,
            owner_kind: 1,
            owner_id: 1,
        },
    );
    world.next_stock_id = 11;
    world.next_record_id = 5;
    let mut sim = fixture3::into_simulation(world);
    let debit = Leg {
        op: 1,
        stock_id: 10,
        definition_id: 1,
        class: 1,
        quantity: 1,
        owner_kind_after: 0,
        owner_id_after: 0,
    };
    let begin = command(
        1,
        1,
        1,
        Payload::BeginTask {
            goal: 0,
            stage: 5,
            recipe_id: 0,
            target_locus: 1,
            target_site: 1,
            target_x: 0,
            target_y: 0,
            due_tick: 2,
            leg_count: 1,
            legs: legs(&[debit]),
        },
    );
    let complete = command(1, 1, 2, Payload::CompleteTask(0));
    let created = sim.drive(0, &[complete, begin]).unwrap();
    assert_eq!(created.resource_events().len(), 1);
    assert_eq!(created.resource_events()[0].kind, 7);
    let applied = sim.drive(1, &[]).unwrap();
    assert_eq!(applied.resource_events()[0].kind, 4);
    assert_eq!(applied.resource_events().last().unwrap().kind, 7);
    let cursor = sim.next_event_id();
    let digest = sim.event_digest();
    let replay = command(3, 1, 2, Payload::CompleteTask(0));
    // The stored key is target 1, sequence 2. Replay that key.
    let same = command(1, 1, 2, Payload::CompleteTask(0));
    let quiet = sim.drive(2, &[same]).unwrap();
    assert!(quiet.resource_events().is_empty());
    assert_eq!(sim.next_event_id(), cursor);
    assert_eq!(sim.event_digest(), digest);
    assert_eq!(sim.tick(), 3);
    let _conflict = command(4, 1, 2, Payload::CompleteTask(5));
    // Registry key is the original key (target 1), not a new target.
    let conflict = command(1, 1, 2, Payload::CompleteTask(5));
    let before = sim.domain_checkpoint_bytes();
    assert_eq!(
        sim.drive(3, &[conflict]),
        Err(StepError::Command {
            key: conflict.key(),
            reason: InvalidCommand::ConflictingIdentity,
        })
    );
    assert_eq!(sim.domain_checkpoint_bytes(), before);
    let _ = replay;
}

#[test]
fn c03_t01f_later_leg_reservation_and_credit_roll_back() {
    let mut world = World::standard(100);
    world.insert_actor(1, 1);
    world.tick = 9;
    world.stocks.insert(
        14,
        Stock {
            definition_id: 2,
            class: 1,
            quantity: 1,
            owner_kind: 3,
            owner_id: 2,
        },
    );
    world.next_stock_id = 15;
    world.insert_scripted_task(1, 1);
    let task = world.tasks.get_mut(&1).unwrap();
    task.stage = 5;
    task.created = 8;
    task.due = 10;
    task.legs = vec![
        Leg {
            op: 1,
            stock_id: 14,
            definition_id: 2,
            class: 1,
            quantity: 1,
            owner_kind_after: 0,
            owner_id_after: 0,
        },
        Leg {
            op: 2,
            stock_id: 0,
            definition_id: 2,
            class: 1,
            quantity: 1,
            owner_kind_after: 3,
            owner_id_after: 0,
        },
    ];
    let mut sim = fixture3::into_simulation(world);
    let before = sim.domain_checkpoint_bytes();
    assert_eq!(sim.drive(9, &[]), Err(StepError::InvalidLeg));
    assert_eq!(sim.domain_checkpoint_bytes(), before);
    assert_eq!(sim.tick(), 9);

    let mut world = World::standard(100);
    world.insert_actor(1, 1);
    world.insert_actor(2, 2);
    world.tick = 1;
    world.stocks.insert(
        10,
        Stock {
            definition_id: 1,
            class: 1,
            quantity: 2,
            owner_kind: 1,
            owner_id: 1,
        },
    );
    world.next_stock_id = 11;
    world.insert_scripted_task(1, 1);
    world.insert_scripted_task(2, 2);
    world.tasks.get_mut(&1).unwrap().stage = 5;
    world.tasks.get_mut(&1).unwrap().created = 0;
    world.tasks.get_mut(&1).unwrap().due = 2;
    world.tasks.get_mut(&1).unwrap().reservations = vec![3];
    world.tasks.get_mut(&1).unwrap().legs = vec![Leg {
        op: 1,
        stock_id: 10,
        definition_id: 1,
        class: 1,
        quantity: 1,
        owner_kind_after: 0,
        owner_id_after: 0,
    }];
    world.tasks.get_mut(&2).unwrap().reservations = vec![4];
    world.reservations.insert(
        3,
        fixture3::Reservation {
            stock_id: 10,
            claimant: 1,
            quantity: 1,
            acquired_tick: 1,
            expiry_tick: 9,
            task_id: 1,
            state: 1,
        },
    );
    world.reservations.insert(
        4,
        fixture3::Reservation {
            stock_id: 10,
            claimant: 2,
            quantity: 1,
            acquired_tick: 1,
            expiry_tick: 9,
            task_id: 2,
            state: 1,
        },
    );
    world.next_record_id = 5;
    let mut sim = fixture3::into_simulation(world.clone());
    assert!(sim.drive(1, &[]).is_ok());
    assert_eq!(sim.resource_stock_quantity(10), Some(1));
    let mut too_much = world;
    too_much.tasks.get_mut(&1).unwrap().legs[0].quantity = 2;
    let mut sim = fixture3::into_simulation(too_much);
    let before = sim.domain_checkpoint_bytes();
    assert_eq!(sim.drive(1, &[]), Err(StepError::InvalidLeg));
    assert_eq!(sim.domain_checkpoint_bytes(), before);

    let mut world = World::standard(100);
    world.insert_actor(1, 1);
    world.tick = 1;
    world.stocks.insert(
        8,
        Stock {
            definition_id: 7,
            class: 1,
            quantity: 1,
            owner_kind: 1,
            owner_id: 1,
        },
    );
    world.stocks.insert(
        20,
        Stock {
            definition_id: 7,
            class: 1,
            quantity: 1,
            owner_kind: 3,
            owner_id: 1,
        },
    );
    world.next_stock_id = 21;
    world.insert_scripted_task(1, 1);
    let task = world.tasks.get_mut(&1).unwrap();
    task.stage = 5;
    task.created = 0;
    task.due = 2;
    task.legs = vec![
        Leg {
            op: 1,
            stock_id: 8,
            definition_id: 7,
            class: 1,
            quantity: 1,
            owner_kind_after: 0,
            owner_id_after: 0,
        },
        Leg {
            op: 2,
            stock_id: 20,
            definition_id: 7,
            class: 1,
            quantity: 1,
            owner_kind_after: 3,
            owner_id_after: 1,
        },
    ];
    let mut sim = fixture3::into_simulation(world.clone());
    assert!(sim.drive(1, &[]).is_ok());
    assert_eq!(sim.resource_stock_quantity(20), Some(2));
    for (definition, class, owner) in [(8, 1, 1), (7, 2, 1), (7, 1, 2)] {
        let mut bad = world.clone();
        let credit = &mut bad.tasks.get_mut(&1).unwrap().legs[1];
        credit.definition_id = definition;
        credit.class = class;
        credit.owner_id_after = owner;
        let mut sim = fixture3::into_simulation(bad);
        let before = sim.domain_checkpoint_bytes();
        assert_eq!(sim.drive(1, &[]), Err(StepError::InvalidLeg));
        assert_eq!(sim.domain_checkpoint_bytes(), before);
    }
}

#[test]
fn c03_t01h_hostile_checkpoint_and_bundle_atomicity() {
    let sim = Simulation::two_claimants();
    let blob = sim.domain_checkpoint_bytes();
    assert_eq!(
        Simulation::restore_domain_checkpoint(&blob[..10]),
        Err(RestoreError::Truncated)
    );
    assert_eq!(
        fixture3::restore(b"SH:C02:STATE-not-a-real-blob"),
        Err(RestoreError::UnsupportedFormat)
    );
    let mut bad_checksum = blob.clone();
    let last = bad_checksum.len() - 1;
    bad_checksum[last] ^= 1;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&bad_checksum),
        Err(RestoreError::ChecksumMismatch)
    );
    let mut revision = blob.clone();
    revision[12] = 2;
    // Rewriting revision invalidates the trailer, which is checked after the revision.
    assert!(matches!(
        Simulation::restore_domain_checkpoint(&revision),
        Err(RestoreError::UnsupportedRevision { found: 2 })
    ));

    let mut world = World::standard(100);
    world.insert_actor(1, 1);
    world.insert_scripted_task(1, 1);
    world.stocks.insert(
        1,
        Stock {
            definition_id: 1,
            class: 1,
            quantity: 1,
            owner_kind: 3,
            owner_id: 1,
        },
    );
    world.stocks.insert(
        2,
        Stock {
            definition_id: 4,
            class: 2,
            quantity: 1,
            owner_kind: 3,
            owner_id: 1,
        },
    );
    world.next_stock_id = 3;
    world.next_record_id = 2;
    let mut sim = fixture3::into_simulation(world);
    let before = sim.domain_checkpoint_bytes();
    let bundle = command(
        1,
        1,
        1,
        Payload::ReserveBundle {
            task_id: 1,
            expiry_tick: 5,
            pair_count: 2,
            stocks: [1, 2, 0, 0],
            quantities: [1, 2, 0, 0],
        },
    );
    assert_eq!(sim.drive(0, &[bundle]), Err(StepError::InvalidState));
    assert_eq!(sim.domain_checkpoint_bytes(), before);
    let zero = command(
        1,
        1,
        1,
        Payload::ReserveStock {
            stock_id: 1,
            quantity: 0,
            expiry_tick: 5,
            task_id: 1,
        },
    );
    assert_eq!(
        sim.drive(0, &[zero]),
        Err(StepError::Command {
            key: zero.key(),
            reason: InvalidCommand::InvalidPayload,
        })
    );
}
