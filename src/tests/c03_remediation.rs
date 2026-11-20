//! Focused regressions for the owner-authorized independent S1 findings.
use crate::fixture3::{self, Completion, PathStep, RegistryRecord, Reservation, Stock, World};
use crate::{
    ActorId, Command, CommandKey, InvalidCommand, Leg, Payload, ProcessingMode, RestoreError,
    Simulation, StepError, encoding,
};

fn key(tick: u64, actor: u32, sequence: u64) -> CommandKey {
    CommandKey {
        target_tick: tick,
        actor_id: ActorId::new(actor),
        producer_id: 1,
        sequence,
    }
}
fn cmd(tick: u64, actor: u32, sequence: u64, payload: Payload) -> Command {
    Command::new(key(tick, actor, sequence), payload)
}
fn reserve(sequence: u64, quantity: u64) -> Command {
    cmd(
        1,
        1,
        sequence,
        Payload::ReserveStock {
            stock_id: 1,
            quantity,
            expiry_tick: 9,
            task_id: 1,
        },
    )
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
fn begin(tick: u64, sequence: u64) -> Command {
    cmd(
        tick,
        1,
        sequence,
        Payload::BeginTask {
            goal: 0,
            stage: 5,
            recipe_id: 0,
            target_locus: 1,
            target_site: 1,
            target_x: 0,
            target_y: 0,
            due_tick: tick + 2,
            leg_count: 0,
            legs: [zero_leg(); 4],
        },
    )
}
fn idle() -> World {
    let mut w = World::standard(100);
    w.insert_actor(1, 0);
    w
}
fn assert_invalid(w: World, expected: RestoreError) {
    let bytes = fixture3::into_simulation(w).domain_checkpoint_bytes();
    assert_eq!(Simulation::restore_domain_checkpoint(&bytes), Err(expected));
}
fn edit_blob(bytes: &[u8], offset: usize, value: &[u8]) -> Vec<u8> {
    let mut body = bytes[..bytes.len() - 32].to_vec();
    body[offset..offset + value.len()].copy_from_slice(value);
    let hash = encoding::sha256(&body);
    body.extend(hash);
    body
}
fn assert_rollback(sim: &mut Simulation, batch: &[Command], expected: StepError) {
    let before = sim.domain_checkpoint_bytes();
    assert_eq!(sim.drive(sim.tick(), batch), Err(expected));
    assert_eq!(sim.domain_checkpoint_bytes(), before);
}

#[test]
fn b01_cursors_links_and_domains_are_rejected_before_publication() {
    for field in 0..6 {
        let mut w = fixture3::two_claimants();
        match field {
            0 => w.next_record_id = 2,
            1 => w.next_actor_id = 0,
            2 => w.next_stock_id = 1,
            3 => w.next_transaction_id = 0,
            4 => w.next_event_id = 0,
            _ => w.actors.get_mut(&1).unwrap().task_id = 99,
        }
        assert_invalid(w, RestoreError::InvalidState);
    }
    for field in 0..7 {
        let mut w = fixture3::two_claimants();
        match field {
            0 => w.tasks.get_mut(&1).unwrap().actor_id = 99,
            1 => w.actors.get_mut(&1).unwrap().channels[0] = 1001,
            2 => w.actors.get_mut(&1).unwrap().enabled = 2,
            3 => w.actors.get_mut(&1).unwrap().mode = 9,
            4 => w.stocks.get_mut(&1).unwrap().owner_id = 99,
            5 => w.stocks.get_mut(&1).unwrap().class = 9,
            _ => w.tasks.get_mut(&1).unwrap().recipe_id = 99,
        }
        assert_invalid(w, RestoreError::InvalidState);
    }
    let mut w = fixture3::two_claimants();
    w.actors.get_mut(&1).unwrap().locus = 9;
    assert_invalid(w, RestoreError::Corrupt);
}

#[test]
fn b01_duplicate_and_unsorted_tables_are_not_normalized() {
    let blob = Simulation::two_claimants().domain_checkpoint_bytes();
    // Actor width is 131 bytes; the scenario ends at offset 403.
    let mut body = blob[..blob.len() - 32].to_vec();
    let first = body[411..542].to_vec();
    let second = body[542..673].to_vec();
    body[411..542].copy_from_slice(&second);
    body[542..673].copy_from_slice(&first);
    let checksum = encoding::sha256(&body);
    body.extend(checksum);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&body),
        Err(RestoreError::InvalidState)
    );
    let duplicate = edit_blob(&blob, 542, &1u32.to_le_bytes());
    assert_eq!(
        Simulation::restore_domain_checkpoint(&duplicate),
        Err(RestoreError::InvalidState)
    );
}

#[test]
fn h01_counts_header_grid_and_parse_before_semantics() {
    let blob = Simulation::two_claimants().domain_checkpoint_bytes();
    for length in 0..14 {
        assert_eq!(
            Simulation::restore_domain_checkpoint(&blob[..length]),
            Err(RestoreError::Truncated)
        );
    }
    assert_eq!(
        Simulation::restore_domain_checkpoint(&edit_blob(&blob, 403, &u64::MAX.to_le_bytes())),
        Err(RestoreError::Corrupt)
    );
    for (width, height, cells) in [(4097, 1, vec![1; 4097]), (0, 3, vec![])] {
        let mut w = fixture3::two_claimants();
        w.grid_w = width;
        w.grid_h = height;
        w.grid = cells;
        assert_invalid(w, RestoreError::Corrupt);
    }
    let mut invalid = edit_blob(&blob, 74, &0u64.to_le_bytes());
    invalid.truncate(invalid.len() - 32);
    invalid.push(0);
    let checksum = encoding::sha256(&invalid);
    invalid.extend(checksum);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&invalid),
        Err(RestoreError::Corrupt)
    );
}

#[test]
fn b01_reservations_and_registry_are_validated() {
    let mut w = fixture3::two_claimants();
    w.reservations.insert(
        3,
        Reservation {
            stock_id: 1,
            claimant: 1,
            quantity: 2,
            acquired_tick: 0,
            expiry_tick: 9,
            task_id: 1,
            state: 1,
        },
    );
    w.tasks.get_mut(&1).unwrap().reservations.push(3);
    w.next_record_id = 4;
    assert_invalid(w, RestoreError::InvalidState);
    for resolved in [0, 99] {
        let mut w = fixture3::two_claimants();
        w.registry.insert(
            key(0, 1, 1),
            RegistryRecord {
                submitted: 0,
                resolved,
                role: 2,
            },
        );
        assert_invalid(w, RestoreError::InvalidState);
    }
    let mut w = fixture3::two_claimants();
    w.tasks.get_mut(&1).unwrap().unconsumed = vec![
        Completion {
            key: key(0, 1, 1),
            submitted: 1,
            resolved: 1
        };
        2
    ];
    assert_invalid(w, RestoreError::InvalidState);
}

#[test]
fn h02_same_phase_and_future_begin_cannot_orphan_tasks() {
    let mut sim = fixture3::into_simulation(idle());
    assert_rollback(
        &mut sim,
        &[begin(1, 1), begin(1, 2)],
        StepError::InvalidState,
    );
    let mut sim = fixture3::into_simulation(idle());
    sim.drive(0, &[begin(2, 1), begin(3, 2)]).unwrap();
    sim.drive(1, &[]).unwrap();
    assert_rollback(&mut sim, &[], StepError::InvalidState);
}

#[test]
fn h03_zero_completion_requires_an_actual_same_phase_begin() {
    let mut sim = Simulation::two_claimants();
    let completion = cmd(1, 1, 1, Payload::CompleteTask(0));
    assert_rollback(
        &mut sim,
        &[completion],
        StepError::Command {
            key: completion.key(),
            reason: InvalidCommand::InvalidPayload,
        },
    );
    let mut sim = fixture3::into_simulation(idle());
    let completion = cmd(2, 1, 2, Payload::CompleteTask(0));
    assert_rollback(
        &mut sim,
        &[begin(1, 1), completion],
        StepError::Command {
            key: completion.key(),
            reason: InvalidCommand::InvalidPayload,
        },
    );
}

#[test]
fn h04_own_reservations_share_capacity_and_checked_aggregate() {
    let mut sim = Simulation::two_claimants();
    let out = sim.drive(0, &[reserve(2, 1), reserve(1, 1)]).unwrap();
    assert_eq!(
        out.resource_events()
            .iter()
            .map(|e| e.kind)
            .collect::<Vec<_>>(),
        [1, 7, 2, 7]
    );
    assert_eq!(sim.next_record_id(), 4);
    let w = sim.world3.as_ref().unwrap();
    assert_eq!(w.reservations.len(), 1);
    let mut w = fixture3::two_claimants();
    w.stocks.get_mut(&1).unwrap().quantity = u64::MAX;
    let mut sim = fixture3::into_simulation(w);
    assert_rollback(
        &mut sim,
        &[reserve(1, u64::MAX), reserve(2, u64::MAX)],
        StepError::CapacityExceeded,
    );
}

#[test]
fn h05_producer_and_actor_checks_precede_duplicate_and_late_keys() {
    let good = reserve(1, 1);
    let bad = Command::new(
        CommandKey {
            producer_id: 0,
            ..good.key()
        },
        good.payload(),
    );
    let mut sim = Simulation::two_claimants();
    assert_rollback(
        &mut sim,
        &[bad, bad],
        StepError::Command {
            key: bad.key(),
            reason: InvalidCommand::InvalidProducer,
        },
    );
    let late = cmd(0, 1, 1, Payload::SetEnabled(1));
    assert_rollback(
        &mut sim,
        &[late, bad],
        StepError::Command {
            key: bad.key(),
            reason: InvalidCommand::InvalidProducer,
        },
    );
}

#[test]
fn h06_invalidation_orders_actors_not_record_ids() {
    let mut w = fixture3::two_claimants();
    w.tasks.get_mut(&1).unwrap().actor_id = 2;
    w.tasks.get_mut(&2).unwrap().actor_id = 1;
    w.actors.get_mut(&1).unwrap().task_id = 2;
    w.actors.get_mut(&2).unwrap().task_id = 1;
    let mut sim = fixture3::into_simulation(w);
    let out = sim
        .drive(
            0,
            &[
                cmd(1, 2, 1, Payload::SetEnabled(0)),
                cmd(1, 1, 1, Payload::SetEnabled(0)),
            ],
        )
        .unwrap();
    assert_eq!(
        out.resource_events()
            .iter()
            .map(|e| (e.actor_id, e.kind))
            .collect::<Vec<_>>(),
        [(1, 7), (1, 5), (2, 7), (2, 5)]
    );
}

#[test]
fn h07_schema_invalid_begin_and_new_stock_class_do_not_publish() {
    let mut sim = fixture3::into_simulation(idle());
    let Payload::BeginTask {
        stage,
        target_locus,
        target_site,
        target_x,
        target_y,
        due_tick,
        leg_count,
        legs,
        ..
    } = begin(1, 1).payload()
    else {
        unreachable!()
    };
    let bad = cmd(
        1,
        1,
        1,
        Payload::BeginTask {
            goal: 255,
            stage,
            recipe_id: 99,
            target_locus,
            target_site,
            target_x,
            target_y,
            due_tick,
            leg_count,
            legs,
        },
    );
    assert_rollback(
        &mut sim,
        &[bad],
        StepError::Command {
            key: bad.key(),
            reason: InvalidCommand::InvalidPayload,
        },
    );
    let mut w = idle();
    w.insert_scripted_task(1, 1);
    w.actors.get_mut(&1).unwrap().task_id = 1;
    w.tick = 1;
    let task = w.tasks.get_mut(&1).unwrap();
    task.stage = 5;
    task.due = 2;
    task.legs.push(Leg {
        op: 2,
        stock_id: 0,
        definition_id: 1,
        class: 9,
        quantity: 1,
        owner_kind_after: 3,
        owner_id_after: 1,
    });
    let mut sim = fixture3::into_simulation(w);
    assert_rollback(&mut sim, &[], StepError::InvalidLeg);
}

#[test]
fn h08_supported_scenarios_roundtrip_unsupported_consistent_content_is_rejected() {
    for wake in [4, 100] {
        let mut w = idle();
        w.wake_interval = wake;
        let blob = fixture3::into_simulation(w).domain_checkpoint_bytes();
        assert_eq!(
            Simulation::restore_domain_checkpoint(&blob)
                .unwrap()
                .domain_checkpoint_bytes(),
            blob
        );
    }
    let blob = Simulation::two_claimants().domain_checkpoint_bytes();
    let mut body = blob[..blob.len() - 32].to_vec();
    body[106..114].copy_from_slice(&0i64.to_le_bytes());
    let content = encoding::sha256(&body[98..403]);
    body[18..50].copy_from_slice(&content);
    let checksum = encoding::sha256(&body);
    body.extend(checksum);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&body),
        Err(RestoreError::ContentMismatch)
    );
}

#[test]
fn h09_applied_noncompletion_replay_is_late_and_identity_free() {
    let mut sim = Simulation::two_claimants();
    let applied = cmd(1, 1, 1, Payload::SetEnabled(1));
    let out = sim.drive(0, &[applied]).unwrap();
    assert!(out.resource_events().is_empty());
    let applied_bytes = sim.domain_checkpoint_bytes();

    let replay = cmd(1, 1, 1, Payload::CancelReservation(0));
    let late = StepError::Command {
        key: replay.key(),
        reason: InvalidCommand::LateTarget,
    };
    assert_rollback(&mut sim, &[replay], late);
    assert_eq!(sim.domain_checkpoint_bytes(), applied_bytes);

    let mut restored = Simulation::restore_domain_checkpoint(&applied_bytes).unwrap();
    assert_rollback(&mut restored, &[replay], late);
    assert_eq!(restored.domain_checkpoint_bytes(), applied_bytes);

    // A no-op applied command leaves no identity behind: its canonical bytes
    // equal an empty boundary, and both histories produce the same replay.
    let mut empty = Simulation::two_claimants();
    assert!(empty.drive(0, &[]).unwrap().resource_events().is_empty());
    assert_eq!(empty.domain_checkpoint_bytes(), applied_bytes);
    assert_rollback(&mut empty, &[replay], late);
    assert_eq!(empty.domain_checkpoint_bytes(), applied_bytes);
}

#[test]
fn h09_pending_duplicate_and_future_target_follow_first_match() {
    let mut sim = Simulation::two_claimants();
    let applied = cmd(1, 1, 1, Payload::SetEnabled(1));
    sim.drive(0, &[applied]).unwrap();
    let applied_bytes = sim.domain_checkpoint_bytes();
    let replay = cmd(1, 1, 1, Payload::CancelReservation(0));

    let mut producer = Simulation::restore_domain_checkpoint(&applied_bytes).unwrap();
    let bad_producer = Command::new(
        CommandKey {
            producer_id: 0,
            ..replay.key()
        },
        replay.payload(),
    );
    assert_rollback(
        &mut producer,
        &[bad_producer],
        StepError::Command {
            key: bad_producer.key(),
            reason: InvalidCommand::InvalidProducer,
        },
    );

    let mut unknown = Simulation::restore_domain_checkpoint(&applied_bytes).unwrap();
    let bad_actor = Command::new(
        CommandKey {
            actor_id: ActorId::new(99),
            ..replay.key()
        },
        replay.payload(),
    );
    assert_rollback(
        &mut unknown,
        &[bad_actor],
        StepError::Command {
            key: bad_actor.key(),
            reason: InvalidCommand::UnknownActor,
        },
    );

    let mut duplicate = Simulation::restore_domain_checkpoint(&applied_bytes).unwrap();
    assert_eq!(
        duplicate.drive(1, &[replay, replay]),
        Err(StepError::Command {
            key: replay.key(),
            reason: InvalidCommand::ConflictingIdentity,
        })
    );
    assert_eq!(duplicate.domain_checkpoint_bytes(), applied_bytes);

    let pending_future = cmd(3, 1, 1, Payload::SetMode(ProcessingMode::Scheduled));
    let mut pending = Simulation::restore_domain_checkpoint(&applied_bytes).unwrap();
    assert!(
        pending
            .drive(1, &[pending_future])
            .unwrap()
            .resource_events()
            .is_empty()
    );
    let pending_bytes = pending.domain_checkpoint_bytes();
    let conflict = StepError::Command {
        key: pending_future.key(),
        reason: InvalidCommand::ConflictingIdentity,
    };
    assert_rollback(&mut pending, &[pending_future], conflict);
    let mut pending_restored = Simulation::restore_domain_checkpoint(&pending_bytes).unwrap();
    assert_rollback(&mut pending_restored, &[pending_future], conflict);

    // With no durable non-completion identity, a future target is an ordinary
    // new command, not a historical replay.
    let future = cmd(2, 1, 1, Payload::SetMode(ProcessingMode::Scheduled));
    let mut future_sim = Simulation::restore_domain_checkpoint(&applied_bytes).unwrap();
    assert!(
        future_sim
            .drive(1, &[future])
            .unwrap()
            .resource_events()
            .is_empty()
    );
    assert_eq!(
        future_sim.world3.as_ref().unwrap().actors[&1].mode,
        ProcessingMode::Scheduled as u8
    );
    let future_bytes = future_sim.domain_checkpoint_bytes();

    let mut future_restored = Simulation::restore_domain_checkpoint(&applied_bytes).unwrap();
    assert!(
        future_restored
            .drive(1, &[future])
            .unwrap()
            .resource_events()
            .is_empty()
    );
    assert_eq!(future_restored.domain_checkpoint_bytes(), future_bytes);

    assert_rollback(
        &mut future_sim,
        &[future],
        StepError::Command {
            key: future.key(),
            reason: InvalidCommand::LateTarget,
        },
    );
    assert_eq!(future_sim.domain_checkpoint_bytes(), future_bytes);
}

#[test]
fn h10_goal_zero_request_advances_once_then_applies_after_reload() {
    let mut w = fixture3::two_claimants();
    w.stocks.get_mut(&1).unwrap().quantity = 0;
    let mut sim = fixture3::into_simulation(w);
    sim.drive(0, &[]).unwrap();
    {
        let task = sim.world3.as_mut().unwrap().tasks.get_mut(&1).unwrap();
        task.stage = 4;
        task.due = 4;
    }
    let out = sim
        .drive(1, &[cmd(2, 1, 1, Payload::CompleteTask(1))])
        .unwrap();
    assert_eq!(
        out.resource_events()
            .iter()
            .map(|e| e.kind)
            .collect::<Vec<_>>(),
        [7]
    );
    assert_eq!(sim.world3.as_ref().unwrap().tasks[&1].stage, 5);
    let mut sim = Simulation::restore_domain_checkpoint(&sim.domain_checkpoint_bytes()).unwrap();
    let out = sim.drive(2, &[]).unwrap();
    assert_eq!(
        out.resource_events()
            .iter()
            .map(|e| e.kind)
            .collect::<Vec<_>>(),
        [4, 7]
    );
    assert_eq!(sim.world3.as_ref().unwrap().registry.len(), 1);
}

fn debit(stock: u64, quantity: u64) -> Leg {
    Leg {
        op: 1,
        stock_id: stock,
        definition_id: 1,
        class: 1,
        quantity,
        owner_kind_after: 0,
        owner_id_after: 0,
    }
}
fn credit(owner_kind: u8, owner_id: u64) -> Leg {
    Leg {
        op: 2,
        stock_id: 0,
        definition_id: 1,
        class: 1,
        quantity: 1,
        owner_kind_after: owner_kind,
        owner_id_after: owner_id,
    }
}
fn executing(goal: u8, due: u64) -> World {
    let mut w = idle();
    w.tick = 1;
    w.insert_scripted_task(1, 1);
    w.actors.get_mut(&1).unwrap().task_id = 1;
    let t = w.tasks.get_mut(&1).unwrap();
    t.goal = goal;
    t.stage = 4;
    t.due = due;
    w
}
fn created_completing(legs: &[Leg], requests: u64) -> Simulation {
    let mut storage = [zero_leg(); 4];
    storage[..legs.len()].copy_from_slice(legs);
    let mut sim = fixture3::into_simulation(idle());
    let begin = cmd(
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
            due_tick: 3,
            leg_count: legs.len() as u8,
            legs: storage,
        },
    );
    let mut batch = vec![begin];
    for sequence in 2..2 + requests {
        batch.push(cmd(1, 1, sequence, Payload::CompleteTask(0)));
    }
    sim.drive(0, &batch).unwrap();
    sim
}
fn restored(sim: &Simulation) -> Simulation {
    let blob = sim.domain_checkpoint_bytes();
    let restored = Simulation::restore_domain_checkpoint(&blob).unwrap();
    assert_eq!(restored.domain_checkpoint_bytes(), blob);
    restored
}
fn kinds(out: &crate::DriveOutput) -> Vec<u8> {
    out.resource_events().iter().map(|e| e.kind).collect()
}

#[test]
fn b01_completion_registry_transactions_and_all_roles_roundtrip() {
    let sim = created_completing(&[credit(3, 1)], 2);
    assert_eq!(sim.world3.as_ref().unwrap().tasks[&1].unconsumed.len(), 2);
    let mut sim = restored(&sim);
    assert_eq!(kinds(&sim.drive(1, &[]).unwrap()), [4, 7]);
    let mut sim = restored(&sim);
    let w = sim.world3.as_ref().unwrap();
    assert_eq!(
        w.registry.values().map(|r| r.role).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(w.transactions.len(), 1);
    let event = (sim.next_event_id(), sim.event_digest());
    for sequence in [2, 3] {
        let replay = cmd(1, 1, sequence, Payload::CompleteTask(0));
        assert!(
            sim.drive(sim.tick(), &[replay])
                .unwrap()
                .resource_events()
                .is_empty()
        );
        assert_eq!((sim.next_event_id(), sim.event_digest()), event);
        let conflict = cmd(1, 1, sequence, Payload::CompleteTask(1));
        assert_rollback(
            &mut sim,
            &[conflict],
            StepError::Command {
                key: conflict.key(),
                reason: InvalidCommand::ConflictingIdentity,
            },
        );
    }
    for mutation in 0..7 {
        let mut w = sim.world3.clone().unwrap();
        match mutation {
            0 => w.registry.get_mut(&key(1, 1, 2)).unwrap().role = 2,
            1 => w.registry.get_mut(&key(1, 1, 3)).unwrap().role = 1,
            2 => w.transactions.get_mut(&1).unwrap().applied = 0,
            3 => w.transactions.get_mut(&1).unwrap().key.producer_id = 0,
            4 => w.transactions.get_mut(&1).unwrap().task_id = 99,
            5 => w.transactions.get_mut(&1).unwrap().legs[0].stock_id = 99,
            _ => w.registry.get_mut(&key(1, 1, 2)).unwrap().resolved = 0,
        }
        assert_invalid(w, RestoreError::InvalidState);
    }
}

#[test]
fn b01_duplicate_stock_registry_pending_and_namespace_collision() {
    fn duplicate(blob: &[u8], record: &[u8]) -> Vec<u8> {
        let mut body = blob[..blob.len() - 32].to_vec();
        let starts: Vec<_> = body
            .windows(record.len())
            .enumerate()
            .filter(|(_, s)| *s == record)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(starts.len(), 1);
        let at = starts[0];
        let count = u64::from_le_bytes(body[at - 8..at].try_into().unwrap());
        body[at - 8..at].copy_from_slice(&(count + 1).to_le_bytes());
        body.splice(at..at, record.iter().copied());
        let hash = encoding::sha256(&body);
        body.extend(hash);
        body
    }
    let sim = Simulation::two_claimants();
    let record = [
        1u64.to_le_bytes().as_slice(),
        1u64.to_le_bytes().as_slice(),
        &[1],
        1u64.to_le_bytes().as_slice(),
        &[3],
        1u64.to_le_bytes().as_slice(),
    ]
    .concat();
    assert_eq!(
        Simulation::restore_domain_checkpoint(&duplicate(&sim.domain_checkpoint_bytes(), &record)),
        Err(RestoreError::InvalidState)
    );
    let mut sim = created_completing(&[], 1);
    sim.drive(1, &[]).unwrap();
    let record = [
        1u64.to_le_bytes().as_slice(),
        1u32.to_le_bytes().as_slice(),
        1u32.to_le_bytes().as_slice(),
        2u64.to_le_bytes().as_slice(),
        0u64.to_le_bytes().as_slice(),
        1u64.to_le_bytes().as_slice(),
        &[1],
    ]
    .concat();
    assert_eq!(record.len(), 41);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&duplicate(&sim.domain_checkpoint_bytes(), &record)),
        Err(RestoreError::InvalidState)
    );
    let mut sim = Simulation::two_claimants();
    sim.drive(0, &[cmd(3, 1, 17, Payload::SetEnabled(0))])
        .unwrap();
    let record = [
        3u64.to_le_bytes().as_slice(),
        1u32.to_le_bytes().as_slice(),
        1u32.to_le_bytes().as_slice(),
        17u64.to_le_bytes().as_slice(),
        &[3, 0],
    ]
    .concat();
    assert_eq!(
        Simulation::restore_domain_checkpoint(&duplicate(&sim.domain_checkpoint_bytes(), &record)),
        Err(RestoreError::InvalidState)
    );
    let mut w = fixture3::two_claimants();
    w.reservations.insert(
        1,
        Reservation {
            stock_id: 1,
            claimant: 1,
            quantity: 1,
            acquired_tick: 0,
            expiry_tick: 3,
            task_id: 1,
            state: 1,
        },
    );
    assert_invalid(w, RestoreError::InvalidState);
}

#[test]
fn h01_truncations_bad_framing_and_content_precedence_never_publish() {
    let blob = Simulation::two_claimants().domain_checkpoint_bytes();
    for end in 0..blob.len() {
        assert!(
            Simulation::restore_domain_checkpoint(&blob[..end]).is_err(),
            "prefix {end}"
        );
    }
    for offset in 0..blob.len() {
        let mut mutant = blob.clone();
        mutant[offset] ^= 0x80;
        assert!(
            Simulation::restore_domain_checkpoint(&mutant).is_err(),
            "mutation {offset}"
        );
    }
    for offset in [14, 18] {
        let mut body = blob[..blob.len() - 32].to_vec();
        body[offset] ^= 0x80;
        body.push(0);
        let hash = encoding::sha256(&body);
        body.extend(hash);
        assert_eq!(
            Simulation::restore_domain_checkpoint(&body),
            Err(RestoreError::Corrupt)
        );
    }
    let mut w = fixture3::two_claimants();
    w.actors.get_mut(&1).unwrap().locus = 9;
    w.transactions.insert(
        1,
        fixture3::Transaction {
            task_id: 1,
            source: 2,
            key: key(0, 1, 1),
            applied: 0,
            legs: vec![],
        },
    );
    w.next_transaction_id = 2;
    assert_invalid(w, RestoreError::Corrupt); // actor locus precedes transaction semantics.
}

#[test]
fn b01_codec_preserves_variable_width_paths_and_validates_shape_without_navigation() {
    let mut w = fixture3::two_claimants();
    w.tasks.get_mut(&1).unwrap().steps = vec![PathStep::Edge(1), PathStep::Grid(2)];
    w.tasks.get_mut(&1).unwrap().path_index = 1;
    let sim = fixture3::into_simulation(w.clone());
    restored(&sim);
    w.tasks.get_mut(&1).unwrap().path_index = 3;
    assert_invalid(w.clone(), RestoreError::InvalidState);
    w.tasks.get_mut(&1).unwrap().path_index = 1;
    w.tasks.get_mut(&1).unwrap().steps[0] = PathStep::Edge(99);
    assert_invalid(w, RestoreError::InvalidState);
    let mut w = idle();
    let mut trace = vec![0; 320];
    for i in 0..5 {
        trace[25 + i * 59] = i as u8 + 1;
    }
    w.traces.insert(1, trace);
    restored(&fixture3::into_simulation(w.clone()));
    w.traces.get_mut(&1).unwrap()[16] = 6;
    assert_invalid(w, RestoreError::InvalidState);
}

#[test]
fn h03_future_same_phase_dependency_survives_restore_and_requires_successful_creation() {
    let mut sim = fixture3::into_simulation(idle());
    sim.drive(0, &[cmd(2, 1, 2, Payload::CompleteTask(0)), begin(2, 1)])
        .unwrap();
    let mut sim = restored(&sim);
    assert_eq!(kinds(&sim.drive(1, &[]).unwrap()), [7]);
    let mut sim = restored(&sim);
    assert_eq!(kinds(&sim.drive(2, &[]).unwrap()), [4, 7]);
    let mut w = idle();
    w.actors.get_mut(&1).unwrap().enabled = 0;
    let mut sim = fixture3::into_simulation(w);
    let complete = cmd(1, 1, 2, Payload::CompleteTask(0));
    assert_rollback(
        &mut sim,
        &[begin(1, 1), complete],
        StepError::Command {
            key: complete.key(),
            reason: InvalidCommand::InvalidPayload,
        },
    );
    assert_eq!(kinds(&sim.drive(0, &[begin(1, 1)]).unwrap()), [2]);
    assert_eq!(sim.next_record_id(), 1);
}

#[test]
fn h04_multiple_own_holds_fit_then_debit_and_release_exactly_once() {
    let mut w = fixture3::two_claimants();
    w.stocks.get_mut(&1).unwrap().quantity = 2;
    w.tasks.get_mut(&1).unwrap().legs = vec![debit(1, 2)];
    let mut sim = fixture3::into_simulation(w);
    assert_eq!(
        kinds(&sim.drive(0, &[reserve(2, 1), reserve(1, 1)]).unwrap()),
        [1, 7, 1, 7]
    );
    assert_eq!(sim.next_record_id(), 5);
    assert_eq!(sim.resource_stock_quantity(1), Some(2));
    assert_eq!(
        kinds(
            &sim.drive(1, &[cmd(2, 1, 1, Payload::CompleteTask(1))])
                .unwrap()
        ),
        [7]
    );
    let mut sim = restored(&sim);
    assert_eq!(kinds(&sim.drive(2, &[]).unwrap()), [4, 3, 3, 7]);
    assert_eq!(sim.resource_stock_quantity(1), Some(0));
    let mut sim = restored(&sim);
    let prefix = (sim.next_event_id(), sim.event_digest());
    assert!(
        sim.drive(3, &[cmd(1, 1, 1, Payload::CancelReservation(3))])
            .unwrap()
            .resource_events()
            .is_empty()
    );
    assert_eq!((sim.next_event_id(), sim.event_digest()), prefix);
}

#[test]
fn h04_bundle_contention_has_one_rejection_no_partial_allocation() {
    let mut w = fixture3::two_claimants();
    w.next_stock_id = 3;
    w.next_record_id = 4;
    w.stocks.insert(
        2,
        Stock {
            definition_id: 4,
            class: 2,
            quantity: 1,
            owner_kind: 3,
            owner_id: 1,
        },
    );
    w.reservations.insert(
        3,
        Reservation {
            stock_id: 2,
            claimant: 2,
            quantity: 1,
            acquired_tick: 0,
            expiry_tick: 9,
            task_id: 2,
            state: 1,
        },
    );
    w.tasks.get_mut(&2).unwrap().reservations = vec![3];
    let mut sim = fixture3::into_simulation(w);
    restored(&sim);
    let bundle = cmd(
        1,
        1,
        1,
        Payload::ReserveBundle {
            task_id: 1,
            expiry_tick: 9,
            pair_count: 2,
            stocks: [1, 2, 0, 0],
            quantities: [1, 1, 0, 0],
        },
    );
    let out = sim.drive(0, &[bundle]).unwrap();
    assert_eq!(kinds(&out), [2]);
    assert_eq!(
        out.resource_events()[0].payload,
        [
            2u64.to_le_bytes().as_slice(),
            1u64.to_le_bytes().as_slice(),
            &[4]
        ]
        .concat()
    );
    assert_eq!(sim.next_record_id(), 4);
    assert_eq!(sim.world3.as_ref().unwrap().reservations.len(), 1);
    restored(&sim);
}

#[test]
fn h06_disable_cancel_expiry_preempt_completion_and_replays_remain_silent() {
    let mut w = fixture3::two_claimants();
    w.stocks.get_mut(&1).unwrap().class = 2;
    let mut sim = fixture3::into_simulation(w);
    let reservation = cmd(
        1,
        1,
        1,
        Payload::ReserveStock {
            stock_id: 1,
            quantity: 1,
            expiry_tick: 2,
            task_id: 1,
        },
    );
    sim.drive(0, &[reservation]).unwrap();
    let batch = [
        cmd(2, 1, 3, Payload::CompleteTask(1)),
        cmd(2, 1, 2, Payload::CancelReservation(3)),
        cmd(2, 1, 1, Payload::SetEnabled(0)),
    ];
    let out = sim.drive(1, &batch).unwrap();
    assert_eq!(kinds(&out), [3, 7, 5]);
    assert_eq!(out.resource_events()[0].payload.last(), Some(&1));
    let mut sim = restored(&sim);
    assert_eq!(sim.world3.as_ref().unwrap().registry[&key(2, 1, 3)].role, 2);
    let prefix = (sim.next_event_id(), sim.event_digest());
    assert!(
        sim.drive(2, &[batch[0], batch[1]])
            .unwrap()
            .resource_events()
            .is_empty()
    );
    assert_eq!((sim.next_event_id(), sim.event_digest()), prefix);
}

#[test]
fn h10_deadline_request_has_one_transition_no_extension_and_invalidation_can_win() {
    let mut sim = fixture3::into_simulation(executing(0, 2));
    let out = sim
        .drive(1, &[cmd(2, 1, 1, Payload::CompleteTask(1))])
        .unwrap();
    assert_eq!(kinds(&out), [7]);
    assert_eq!(sim.world3.as_ref().unwrap().tasks[&1].due, 2);
    let mut sim = restored(&sim);
    assert_rollback(&mut sim, &[], StepError::InvalidState);
    let out = sim
        .drive(2, &[cmd(3, 1, 1, Payload::SetEnabled(0))])
        .unwrap();
    assert_eq!(kinds(&out), [7, 5]);
    assert_eq!(sim.world3.as_ref().unwrap().registry[&key(2, 1, 1)].role, 2);
    restored(&sim);
}

#[test]
fn h10_goal_zero_waiting_and_goals_one_through_five_have_distinct_rules() {
    let mut sim = fixture3::into_simulation(executing(0, 3));
    assert!(sim.drive(1, &[]).unwrap().resource_events().is_empty());
    assert!(sim.drive(2, &[]).unwrap().resource_events().is_empty());
    assert_rollback(&mut sim, &[], StepError::InvalidState);
    for goal in 1..=5 {
        let mut sim = fixture3::into_simulation(executing(goal, 3));
        assert_eq!(kinds(&sim.drive(1, &[]).unwrap()), [7]);
        assert_eq!(kinds(&sim.drive(2, &[]).unwrap()), [4, 7]);
        assert_eq!(sim.world3.as_ref().unwrap().transactions[&1].source, 2);
        restored(&sim);
    }
}

#[test]
fn h10_creation_skip_due_path_and_late_request_do_not_bypass_deadline() {
    let mut sim = created_completing(&[], 0);
    assert_eq!(sim.world3.as_ref().unwrap().transactions.len(), 0);
    assert!(sim.drive(1, &[]).unwrap().resource_events().is_empty());
    assert_eq!(kinds(&sim.drive(2, &[]).unwrap()), [4, 7]);
    restored(&sim);
    let mut sim = fixture3::into_simulation(executing(0, 1));
    let late = cmd(1, 1, 1, Payload::CompleteTask(1));
    assert_rollback(
        &mut sim,
        &[late],
        StepError::Command {
            key: late.key(),
            reason: InvalidCommand::LateTarget,
        },
    );
    assert_rollback(
        &mut sim,
        &[cmd(2, 1, 1, Payload::CompleteTask(1))],
        StepError::InvalidState,
    );
}

#[test]
fn overflow_after_staged_legs_events_and_ids_rolls_back_every_byte() {
    for field in 0..5 {
        let mut w = fixture3::two_claimants();
        let batch = if field <= 1 {
            vec![reserve(1, 1)]
        } else {
            vec![]
        };
        match field {
            0 => w.next_record_id = u64::MAX,
            1 => w.next_event_id = u64::MAX - 1,
            2 => w.tick = u64::MAX,
            _ => {
                w.tick = 1;
                let task = w.tasks.get_mut(&1).unwrap();
                task.stage = 5;
                task.due = 2;
                task.legs = vec![debit(1, 1), credit(3, 1)];
                if field == 3 {
                    w.next_transaction_id = u64::MAX;
                } else {
                    w.next_stock_id = u64::MAX;
                }
            }
        }
        let mut sim = fixture3::into_simulation(w);
        restored(&sim);
        assert_rollback(&mut sim, &batch, StepError::CapacityExceeded);
    }
    let mut w = executing(0, 3);
    let task = w.tasks.get_mut(&1).unwrap();
    task.progress = u64::MAX;
    let mut sim = fixture3::into_simulation(w);
    assert_rollback(
        &mut sim,
        &[cmd(2, 1, 1, Payload::CompleteTask(1))],
        StepError::CapacityExceeded,
    );
}

#[test]
fn later_task_failure_undoes_earlier_transaction_and_owner_changes() {
    let mut w = fixture3::two_claimants();
    w.tick = 1;
    for task in w.tasks.values_mut() {
        task.stage = 5;
        task.due = 2;
    }
    w.tasks.get_mut(&1).unwrap().legs = vec![credit(2, 20)];
    w.tasks.get_mut(&2).unwrap().legs = vec![debit(1, 1), credit(3, 0)];
    let mut sim = fixture3::into_simulation(w);
    restored(&sim);
    assert_rollback(&mut sim, &[], StepError::InvalidLeg);
}

#[test]
fn reordered_batches_and_loaded_prefix_match_at_every_boundary() {
    let mut w = fixture3::two_claimants();
    w.tasks.get_mut(&1).unwrap().legs = vec![debit(1, 1)];
    let first = [
        reserve(1, 1),
        cmd(
            1,
            2,
            1,
            Payload::ReserveStock {
                stock_id: 1,
                quantity: 1,
                expiry_tick: 9,
                task_id: 2,
            },
        ),
        cmd(3, 2, 2, Payload::SetEnabled(0)),
    ];
    let second = [cmd(2, 1, 1, Payload::CompleteTask(1))];
    let batches = [first.to_vec(), second.to_vec(), vec![]];
    let mut direct = fixture3::into_simulation(w.clone());
    let mut loaded = fixture3::into_simulation(w);
    for batch in batches {
        let tick = direct.tick();
        let a = direct.drive(tick, &batch).unwrap();
        let mut reversed = batch;
        reversed.reverse();
        loaded = restored(&loaded);
        let b = loaded.drive(tick, &reversed).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            direct.domain_checkpoint_bytes(),
            loaded.domain_checkpoint_bytes()
        );
    }
}
