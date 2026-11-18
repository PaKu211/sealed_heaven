use super::*;
use std::collections::{BTreeSet, VecDeque};

fn command(world: &World, intent: Intent) -> Command {
    Command {
        key: Key {
            target_tick: world.tick() + 1,
            actor_id: PLAYER_ID,
            producer_id: 1,
            sequence: world.next_sequence(),
        },
        intent,
    }
}

fn act(world: &mut World, intent: Intent) -> Outcome {
    let c = command(world, intent);
    let events = world.drive(world.tick(), &[c]).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].tick, world.tick());
    assert_eq!(events[0].key, c.key);
    events[0].outcome
}

fn unchanged(world: &mut World, boundary: u64, batch: &[Command], error: Error) {
    let before = world.checkpoint();
    assert_eq!(world.drive(boundary, batch), Err(error));
    assert_eq!(world.checkpoint(), before);
}

#[test]
fn authored_identity_terrain_projection_and_ledger() {
    let mut w = World::settlement();
    assert_eq!(
        DEMO,
        include_bytes!("../../apps/sealed_playable/assets/demo.c03")
    );
    assert_eq!(w.npcs().domain_checkpoint_bytes(), DEMO);
    assert_eq!(w.npcs().next_actor_id(), 21);
    assert_eq!(w.player_position(), START);
    assert_eq!(
        (w.source_herbs(), w.player_herbs(), w.delivered_herbs()),
        (3, 0, 0)
    );
    assert!(!w.requested() && !w.completed());
    assert_eq!(w.npc_position(PLAYER_ID), None);
    for id in 1..=20 {
        assert_eq!(w.npc_position(id), Some(Position { x: 3, y: 3 }));
    }
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            let p = Position { x, y };
            assert_eq!(
                w.is_walkable(p),
                p != PATCH && p != SHRINE && !(x == 6 && (1..=6).contains(&y) && y != 4)
            );
        }
    }
    assert!(!w.is_walkable(Position { x: WIDTH, y: 0 }));
    assert!(!w.is_walkable(Position { x: 0, y: HEIGHT }));
    assert!(w.occupied(PATCH) && w.occupied(SHRINE));
    assert!(!w.occupied(START));
    let a = w.npcs.world3.as_mut().unwrap().actors.get_mut(&1).unwrap();
    a.site = 2;
    assert_eq!(w.npc_position(1), Some(Position { x: 9, y: 5 }));
    w.npcs
        .world3
        .as_mut()
        .unwrap()
        .actors
        .get_mut(&1)
        .unwrap()
        .site = 9;
    assert_eq!(w.npc_position(1), Some(Position { x: 10, y: 7 }));
    let a = w.npcs.world3.as_mut().unwrap().actors.get_mut(&1).unwrap();
    a.locus = 2;
    a.site = 1;
    a.x = 2;
    a.y = 1;
    assert_eq!(w.npc_position(1), Some(Position { x: 5, y: 4 }));
    w.npcs
        .world3
        .as_mut()
        .unwrap()
        .actors
        .get_mut(&1)
        .unwrap()
        .locus = 3;
    assert_eq!(w.npc_position(1), None);
    w.npcs
        .world3
        .as_mut()
        .unwrap()
        .actors
        .get_mut(&1)
        .unwrap()
        .locus = 1;
    w.npcs
        .world3
        .as_mut()
        .unwrap()
        .actors
        .get_mut(&1)
        .unwrap()
        .enabled = 0;
    assert_eq!(w.npc_position(1), None);
    assert!(!adjacent(
        Position {
            x: u32::MAX,
            y: u32::MAX
        },
        START
    ));
}

#[test]
fn every_cell_direction_obeys_committed_npc_snapshot() {
    let base = World::settlement();
    let mut next_npcs = base.npcs.clone();
    next_npcs.drive(0, &[]).unwrap();
    let mut found = BTreeSet::new();
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            let from = Position { x, y };
            if !walkable(from) {
                continue;
            }
            for direction in [
                Direction::North,
                Direction::East,
                Direction::South,
                Direction::West,
            ] {
                let mut w = base.clone();
                w.position = from;
                let mut snapshot = w.clone();
                snapshot.npcs = next_npcs.clone();
                let destination = moved(from, direction);
                let expected = match destination {
                    None => Outcome::Rejected(Rejection::OutOfBounds),
                    Some(p) if p.x >= WIDTH || p.y >= HEIGHT => {
                        Outcome::Rejected(Rejection::OutOfBounds)
                    }
                    Some(p) if !walkable(p) => Outcome::Rejected(Rejection::Terrain),
                    Some(p) if snapshot.occupied(p) => Outcome::Rejected(Rejection::Occupied),
                    Some(_) => Outcome::Moved,
                };
                assert_eq!(
                    act(&mut w, Intent::Move(direction)),
                    expected,
                    "{from:?} {direction:?}"
                );
                assert_eq!(
                    w.position,
                    if expected == Outcome::Moved {
                        destination.unwrap()
                    } else {
                        from
                    }
                );
                assert_eq!(w.npcs, next_npcs);
                found.insert(format!("{expected:?}"));
            }
        }
    }
    assert!(found.contains("Moved"));
    assert!(found.contains("Rejected(OutOfBounds)"));
    assert!(found.contains("Rejected(Terrain)"));
    assert!(found.contains("Rejected(Occupied)"));
}

#[test]
fn frozen_npcs_can_overlap_player_and_player_can_leave() {
    let mut w = World::settlement();
    w.position = Position { x: 3, y: 3 };
    w.drive(0, &[]).unwrap();
    assert!(w.occupied(w.position));
    assert_eq!(act(&mut w, Intent::Move(Direction::North)), Outcome::Moved);
    assert_eq!(w.position, Position { x: 3, y: 2 });
}

#[test]
fn interaction_range_request_finite_transfer_and_repeat_rules() {
    let mut w = World::settlement();
    assert_eq!(
        act(&mut w, Intent::Talk(1)),
        Outcome::Rejected(Rejection::OutOfRange)
    );
    assert_eq!(
        act(&mut w, Intent::Talk(2)),
        Outcome::Rejected(Rejection::UnknownTarget)
    );
    assert_eq!(
        act(&mut w, Intent::Gather),
        Outcome::Rejected(Rejection::OutOfRange)
    );
    assert_eq!(
        act(&mut w, Intent::Deliver),
        Outcome::Rejected(Rejection::OutOfRange)
    );
    w.position = Position { x: 8, y: 2 };
    assert!(w.in_range(Target::HerbPatch));
    assert_eq!(
        w.apply(Intent::Gather),
        Outcome::Rejected(Rejection::RequestNotAccepted)
    );
    w.position = Position { x: 2, y: 5 };
    assert_eq!(
        w.apply(Intent::Deliver),
        Outcome::Rejected(Rejection::RequestNotAccepted)
    );
    w.position = Position { x: 2, y: 3 };
    let actor = w.npcs.world3.as_mut().unwrap().actors.get_mut(&1).unwrap();
    actor.locus = 1;
    actor.site = 1;
    assert_eq!(w.apply(Intent::Talk(1)), Outcome::Requested);
    let herbs = w.herbs;
    assert_eq!(w.apply(Intent::Talk(1)), Outcome::Requested);
    assert_eq!(w.herbs, herbs);
    w.position = Position { x: 2, y: 5 };
    assert_eq!(
        w.apply(Intent::Deliver),
        Outcome::Rejected(Rejection::Empty)
    );
    w.position = Position { x: 8, y: 2 };
    for _ in 0..3 {
        assert_eq!(w.apply(Intent::Gather), Outcome::Gathered);
    }
    assert_eq!(w.apply(Intent::Gather), Outcome::Rejected(Rejection::Empty));
    assert_eq!(
        (w.source_herbs(), w.player_herbs(), w.delivered_herbs()),
        (0, 3, 0)
    );
    w.position = Position { x: 2, y: 5 };
    assert_eq!(w.apply(Intent::Deliver), Outcome::Delivered);
    assert_eq!(
        w.apply(Intent::Deliver),
        Outcome::Rejected(Rejection::AlreadyCompleted)
    );
    assert_eq!(
        (w.source_herbs(), w.player_herbs(), w.delivered_herbs()),
        (0, 2, 1)
    );
    assert!(w.completed());
    w.position = Position { x: 8, y: 1 };
    assert!(!w.in_range(Target::HerbPatch)); // diagonals are not adjacent
    assert_eq!(w.target_position(Target::Npc(21)), None);
}

#[test]
fn checked_transfers_never_partially_debit() {
    let mut w = World::settlement();
    w.requested = true;
    w.position = Position { x: 8, y: 2 };
    w.herbs[1].quantity = u64::MAX;
    let herbs = w.herbs;
    assert_eq!(
        w.apply(Intent::Gather),
        Outcome::Rejected(Rejection::Overflow)
    );
    assert_eq!(w.herbs, herbs);
    w.position = Position { x: 2, y: 5 };
    w.herbs[2].quantity = u64::MAX;
    let herbs = w.herbs;
    assert_eq!(
        w.apply(Intent::Deliver),
        Outcome::Rejected(Rejection::Overflow)
    );
    assert_eq!(w.herbs, herbs);
    assert!(!w.completed);
}

#[test]
fn idempotence_conflicts_highwater_and_admission_are_atomic() {
    let mut w = World::settlement();
    let c = command(&w, Intent::Move(Direction::East));
    let events = w.drive(0, &[c, c]).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(w.next_sequence(), 1);
    let position = w.position;
    let digest = w.digest;
    w = World::restore(&w.checkpoint()).unwrap();
    let mut autonomous = w.npcs.clone();
    autonomous.drive(1, &[]).unwrap();
    assert!(w.drive(1, &[c, c]).unwrap().is_empty());
    assert_eq!(w.npcs, autonomous);
    assert_eq!(w.position, position);
    assert_eq!(w.digest, digest);
    let conflict = Command {
        intent: Intent::Gather,
        ..c
    };
    unchanged(&mut w, 2, &[conflict], Error::ConflictingIdentity(c.key));
    let new = command(&w, Intent::Move(Direction::East));
    unchanged(
        &mut w,
        2,
        &[
            new,
            Command {
                intent: Intent::Deliver,
                ..new
            },
        ],
        Error::ConflictingIdentity(new.key),
    );
    let mut second = new;
    second.key.sequence += 1;
    unchanged(&mut w, 2, &[second, new], Error::Capacity);
    unchanged(
        &mut w,
        1,
        &[new],
        Error::WrongBoundary {
            expected: 2,
            received: 1,
        },
    );
    for key in [
        Key {
            actor_id: 1,
            ..new.key
        },
        Key {
            producer_id: 2,
            ..new.key
        },
        Key {
            target_tick: 1,
            ..new.key
        },
        Key {
            target_tick: 4,
            ..new.key
        },
        Key {
            sequence: 0,
            ..new.key
        },
    ] {
        unchanged(&mut w, 2, &[Command { key, ..new }], Error::InvalidKey(key));
    }
    let overflow = Command {
        key: Key {
            sequence: u64::MAX,
            ..new.key
        },
        ..new
    };
    unchanged(&mut w, 2, &[overflow], Error::Capacity);
    unchanged(&mut w, 2, &vec![c; MAX_BATCH + 1], Error::Capacity);
    // Sequence gaps are legal; the saved highwater, not registry length, governs reuse.
    let gap = Command {
        key: Key {
            sequence: 99,
            ..new.key
        },
        ..new
    };
    assert_eq!(w.drive(2, &[gap, c, gap]).unwrap().len(), 1);
    assert_eq!(w.next_sequence(), 100);
    assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    let low = Command {
        key: Key {
            sequence: 5,
            target_tick: 4,
            ..new.key
        },
        ..new
    };
    unchanged(&mut w, 3, &[low], Error::InvalidKey(low.key));
}

#[test]
fn all_fatal_capacity_and_npc_failures_preserve_checkpoint() {
    let mut w = World::settlement();
    w.next_event_id = u64::MAX;
    let c = command(&w, Intent::Gather);
    unchanged(&mut w, 0, &[c], Error::Capacity);
    let mut w = World::settlement();
    for sequence in 0..MAX_COMMANDS as u64 {
        w.registry.insert(
            Key { sequence, ..c.key },
            Record {
                intent: c.intent,
                outcome: Outcome::Rejected(Rejection::OutOfRange),
            },
        );
    }
    let c = Command {
        key: Key {
            sequence: 9000,
            ..c.key
        },
        ..c
    };
    unchanged(&mut w, 0, &[c], Error::Capacity);
    // A full registry still permits exact retries and no-op NPC boundaries.
    let retry = Command {
        key: Key {
            sequence: 0,
            ..c.key
        },
        ..c
    };
    assert!(w.drive(0, &[retry]).unwrap().is_empty());
    let mut w = World::settlement();
    w.npcs.tick = u64::MAX;
    w.npcs.world3.as_mut().unwrap().tick = u64::MAX;
    unchanged(&mut w, u64::MAX, &[], Error::Capacity);
    let mut w = World::settlement();
    w.npcs.world3.as_mut().unwrap().next_record_id = u64::MAX;
    let before = w.checkpoint();
    assert!(matches!(w.drive(0, &[]), Err(Error::Npc(_))));
    assert_eq!(w.checkpoint(), before);
}

/// Test-only path search. Each action is explicit and submitted through the public
/// API; prediction uses one frozen NPC step, never a second rules implementation.
fn approach(w: &World, target: Target, intent: Intent) -> Intent {
    let mut snapshot = w.clone();
    snapshot.npcs.drive(w.tick(), &[]).unwrap();
    let Some(goal) = snapshot.target_position(target) else {
        return intent;
    };
    if adjacent(w.position, goal) {
        return intent;
    }
    let mut queue = VecDeque::from([(w.position, None)]);
    let mut visited = BTreeSet::from([w.position]);
    while let Some((p, first)) = queue.pop_front() {
        if adjacent(p, goal) {
            return Intent::Move(first.unwrap());
        }
        for d in [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ] {
            if let Some(next) = moved(p, d)
                && snapshot.is_walkable(next)
                && !snapshot.occupied(next)
                && visited.insert(next)
            {
                queue.push_back((next, first.or(Some(d))));
            }
        }
    }
    intent // publishes a deterministic rejection instead of inventing a wait action
}

#[test]
fn scripted_full_loop_save_every_tick_and_npc_coexistence() {
    let mut live = World::settlement();
    let mut replay = World::restore(&live.checkpoint()).unwrap();
    let mut npcs = live.npcs.clone();
    let mut accepted = Vec::new();
    let mut saw_empty = false;
    for step in 0..200 {
        let intent = if !live.requested() {
            approach(&live, Target::Npc(1), Intent::Talk(1))
        } else if live.source_herbs() > 0 {
            approach(&live, Target::HerbPatch, Intent::Gather)
        } else if !saw_empty {
            saw_empty = true;
            Intent::Gather
        } else {
            approach(&live, Target::Shrine, Intent::Deliver)
        };
        let c = command(&live, intent);
        let mut batch = vec![c, c];
        if let Some(&retry) = accepted.first() {
            batch.insert(0, retry);
        }
        let expected = live.drive(step, &batch).unwrap();
        batch.reverse();
        assert_eq!(replay.drive(step, &batch).unwrap(), expected);
        npcs.drive(step, &[]).unwrap();
        assert_eq!(
            live.npcs().domain_checkpoint_bytes(),
            npcs.domain_checkpoint_bytes()
        );
        assert_eq!(live.tick(), step + 1);
        assert_eq!(live.checkpoint(), replay.checkpoint());
        assert_eq!(live.hash(), replay.hash());
        assert_eq!(live.event_digest(), replay.event_digest());
        assert_eq!(
            live.source_herbs() + live.player_herbs() + live.delivered_herbs(),
            3
        );
        replay = World::restore(&replay.checkpoint()).unwrap();
        accepted.push(c);
        if live.completed() {
            break;
        }
    }
    assert!(
        live.completed(),
        "script must finish the actual player loop"
    );
    assert!(saw_empty);
    assert_eq!(
        (
            live.source_herbs(),
            live.player_herbs(),
            live.delivered_herbs()
        ),
        (0, 2, 1)
    );
    assert_eq!(
        act(&mut live, Intent::Deliver),
        Outcome::Rejected(Rejection::AlreadyCompleted)
    );
    let restored = World::restore(&live.checkpoint()).unwrap();
    assert_eq!(restored, live);
    assert_eq!(restored.checkpoint(), live.checkpoint());
}

fn resign(bytes: &mut [u8]) {
    let end = bytes.len() - 32;
    let digest = sha256(&bytes[..end]);
    bytes[end..].copy_from_slice(&digest);
}

/// Mutate the final recorded outcome and recompute both unkeyed hashes exactly
/// as an external save editor can. No private simulation fields are changed.
fn forge_last_outcome(
    world: &World,
    event: Event,
    previous_digest: [u8; 32],
    encoded_outcome: [u8; 2],
    player_x: u32,
    requested: bool,
) -> Vec<u8> {
    let mut bytes = world.checkpoint();
    let record = 159 + (event.id as usize - 1) * 31;
    bytes[record + 29..record + 31].copy_from_slice(&encoded_outcome);
    bytes[58..62].copy_from_slice(&player_x.to_le_bytes());
    bytes[105] = u8::from(requested);
    let mut chain = b"SH:C05:EVENT:v1".to_vec();
    chain.extend(previous_digest);
    chain.extend(event.id.to_le_bytes());
    chain.extend(event.tick.to_le_bytes());
    chain.extend(&bytes[record..record + 24]);
    chain.extend(encoded_outcome);
    bytes[123..155].copy_from_slice(&sha256(&chain));
    resign(&mut bytes);
    assert_eq!(
        sha256(&bytes[..bytes.len() - 32]),
        bytes[bytes.len() - 32..]
    );
    bytes
}

#[test]
fn audit_regression_remote_talk_cannot_restore_as_requested() {
    let mut world = World::settlement();
    let previous_digest = world.event_digest();
    let c = command(&world, Intent::Talk(1));
    let event = world.drive(0, &[c]).unwrap()[0];
    assert_eq!(event.outcome, Outcome::Rejected(Rejection::OutOfRange));
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
    let forged = forge_last_outcome(&world, event, previous_digest, [2, 0], 0, true);
    assert_eq!(
        World::restore(&forged).map(|_| ()),
        Err(Error::Restore(RestoreError::InvalidState))
    );
}

#[test]
fn audit_regression_clear_cell_cannot_restore_as_occupied_rejection() {
    let mut world = World::settlement();
    let previous_digest = world.event_digest();
    let c = command(&world, Intent::Move(Direction::East));
    let event = world.drive(0, &[c]).unwrap()[0];
    assert_eq!(event.outcome, Outcome::Moved);
    assert!(!world.occupied(Position { x: 1, y: 4 }));
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
    let forged = forge_last_outcome(&world, event, previous_digest, [5, 3], 0, false);
    assert_eq!(
        World::restore(&forged).map(|_| ()),
        Err(Error::Restore(RestoreError::InvalidState))
    );
}

#[test]
fn audit_regression_occupied_entry_cannot_restore_as_moved() {
    let mut world = World::settlement();
    for direction in [Direction::East, Direction::East, Direction::North] {
        assert_eq!(act(&mut world, Intent::Move(direction)), Outcome::Moved);
    }
    let previous_digest = world.event_digest();
    let c = command(&world, Intent::Move(Direction::East));
    let event = world.drive(3, &[c]).unwrap()[0];
    assert_eq!(event.outcome, Outcome::Rejected(Rejection::Occupied));
    assert!(world.occupied(Position { x: 3, y: 3 }));
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
    let forged = forge_last_outcome(&world, event, previous_digest, [1, 0], 3, false);
    assert_eq!(
        World::restore(&forged).map(|_| ()),
        Err(Error::Restore(RestoreError::InvalidState))
    );
}

#[test]
fn nearby_requests_and_legal_occupancy_rejections_replay_exactly() {
    let mut world = World::settlement();
    for direction in [Direction::East, Direction::East, Direction::North] {
        assert_eq!(act(&mut world, Intent::Move(direction)), Outcome::Moved);
    }
    // Exercise command gaps: replay must advance NPCs to each target, not once
    // per registry record. A nearby NPC1 remains a live target after the gap.
    for _ in 0..3 {
        world.drive(world.tick(), &[]).unwrap();
    }
    let previous_digest = world.event_digest();
    let c = command(&world, Intent::Talk(1));
    let event = world.drive(world.tick(), &[c]).unwrap()[0];
    assert_eq!(event.outcome, Outcome::Requested);
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
    let forged = forge_last_outcome(&world, event, previous_digest, [5, 4], 2, false);
    assert_eq!(
        World::restore(&forged).map(|_| ()),
        Err(Error::Restore(RestoreError::InvalidState))
    );
    assert_eq!(act(&mut world, Intent::Talk(1)), Outcome::Requested);
    assert_eq!(world.player_herbs(), 0);
    assert_eq!(
        act(&mut world, Intent::Move(Direction::East)),
        Outcome::Rejected(Rejection::Occupied)
    );
    // A retry-only boundary is absent from the registry but must still replay.
    assert!(world.drive(world.tick(), &[c]).unwrap().is_empty());
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
}

#[test]
fn authentic_c03_structure_is_insufficient_without_exact_npc_evolution() {
    for with_command in [false, true] {
        let mut world = World::settlement();
        if with_command {
            act(&mut world, Intent::Move(Direction::East));
        }
        for _ in 0..8 {
            world.drive(world.tick(), &[]).unwrap();
        }
        let bytes = world.checkpoint();
        assert_eq!(World::restore(&bytes).unwrap(), world);
        let count = u32::from_le_bytes(bytes[155..159].try_into().unwrap()) as usize;
        let npc_start = 159 + count * 31 + 4;
        let npc_end = bytes.len() - 32;
        // C03 digest corruption and a non-exhausted, replay-inconsistent event
        // cursor are not reachable by exact NPC replay.
        for digest_forgery in [false, true] {
            let mut forged = bytes.clone();
            if digest_forgery {
                forged[npc_end - 64] ^= 1;
            } else {
                let cursor =
                    u64::from_le_bytes(forged[npc_end - 72..npc_end - 64].try_into().unwrap());
                forged[npc_end - 72..npc_end - 64]
                    .copy_from_slice(&cursor.checked_add(1).unwrap().to_le_bytes());
            }
            resign(&mut forged[npc_start..npc_end]);
            resign(&mut forged);
            assert_eq!(
                World::restore(&forged).map(|_| ()),
                Err(Error::Restore(RestoreError::InvalidState))
            );
        }
    }
}

#[test]
fn exhausted_authentic_npc_event_cursor_restores_and_fails_drive_atomically() {
    let mut initial = World::settlement();
    initial.drive(0, &[]).unwrap();
    let mut bytes = initial.checkpoint();
    let npc_start = 163;
    let npc_len = u32::from_le_bytes(bytes[159..163].try_into().unwrap()) as usize;
    let npc_end = npc_start + npc_len;
    bytes[npc_end - 72..npc_end - 64].copy_from_slice(&u64::MAX.to_le_bytes());
    resign(&mut bytes[npc_start..npc_end]);
    resign(&mut bytes);
    let mut restored = World::restore(&bytes).unwrap();
    assert_eq!(restored.checkpoint(), bytes);

    for _ in 0..50 {
        let before = restored.checkpoint();
        let command = command(&restored, Intent::Move(Direction::East));
        match restored.drive(restored.tick(), &[command]) {
            Err(Error::Npc(_)) => {
                assert_eq!(restored.checkpoint(), before);
                return;
            }
            Ok(_) => {}
            Err(error) => panic!("unexpected NPC cursor error: {error:?}"),
        }
    }
    panic!("exhausted NPC event cursor did not fail within 50 boundaries");
}

#[test]
fn finite_horizon_admission_and_huge_clock_rejection_are_bounded() {
    assert_eq!(MAX_TICKS, 4096);
    assert_eq!(MAX_TICKS as usize, MAX_COMMANDS);
    let mut initial = World::settlement();
    unchanged(&mut initial, MAX_TICKS, &[], Error::Capacity);
    unchanged(&mut initial, u64::MAX, &[], Error::Capacity);
    // A private near-horizon clock isolates the drive admission guard without
    // replaying thousands of NPC ticks in every ordinary test run. The ignored
    // budget test below separately exercises an authentic maximum-length save.
    let mut world = World::settlement();
    world.npcs.tick = MAX_TICKS - 1;
    world.npcs.world3.as_mut().unwrap().tick = MAX_TICKS - 1;
    let c = command(&world, Intent::Talk(99));
    assert_eq!(
        world.drive(MAX_TICKS - 1, &[c]).unwrap()[0].outcome,
        Outcome::Rejected(Rejection::UnknownTarget)
    );
    assert_eq!(world.tick(), MAX_TICKS);
    unchanged(&mut world, MAX_TICKS, &[], Error::Capacity);
    unchanged(&mut world, MAX_TICKS, &[c], Error::Capacity);
    let new = command(&world, Intent::Gather);
    unchanged(&mut world, MAX_TICKS, &[new], Error::Capacity);
    let original = World::settlement().checkpoint();
    let start = std::time::Instant::now();
    for tick in [MAX_TICKS + 1, u64::MAX] {
        let mut forged = original.clone();
        forged[46..54].copy_from_slice(&tick.to_le_bytes());
        // Match the inner C03 tick and recompute both checksums too.
        let npc_start = 163;
        let npc_end = forged.len() - 32;
        forged[npc_start + 50..npc_start + 58].copy_from_slice(&tick.to_le_bytes());
        resign(&mut forged[npc_start..npc_end]);
        resign(&mut forged);
        assert_eq!(
            World::restore(&forged).map(|_| ()),
            Err(Error::Restore(RestoreError::InvalidState))
        );
        // Even without the remainder of the file, the header must reject on the
        // clock, proving the bound is checked before candidate parsing/replay.
        assert_eq!(
            World::restore(&forged[..54]).map(|_| ()),
            Err(Error::Restore(RestoreError::InvalidState))
        );
    }
    eprintln!(
        "over-horizon/hostile-u64 header rejection: {:?}",
        start.elapsed()
    );
}

#[test]
#[ignore = "explicit maximum-horizon latency probe; avoid repeated 4096-tick replay in the default suite"]
fn maximum_valid_horizon_budget_roundtrip() {
    let mut world = World::settlement();
    let drive_start = std::time::Instant::now();
    for _ in 0..MAX_TICKS - 1 {
        world.drive(world.tick(), &[]).unwrap();
        if world.tick().is_multiple_of(1024) {
            eprintln!(
                "maximum-horizon fixture generation: tick {}, elapsed {:?}",
                world.tick(),
                drive_start.elapsed()
            );
        }
    }
    let c = command(&world, Intent::Talk(99));
    assert_eq!(
        world.drive(MAX_TICKS - 1, &[c]).unwrap()[0].outcome,
        Outcome::Rejected(Rejection::UnknownTarget)
    );
    let drive_elapsed = drive_start.elapsed();
    let bytes = world.checkpoint();
    eprintln!(
        "maximum-horizon fixture generated: {} bytes, drive {:?}; starting restore",
        bytes.len(),
        drive_elapsed
    );
    let restore_start = std::time::Instant::now();
    let restored = World::restore(&bytes).unwrap();
    let restore_elapsed = restore_start.elapsed();
    assert_eq!(restored.checkpoint(), bytes);
    assert_eq!(restored, world);
    unchanged(&mut world, MAX_TICKS, &[], Error::Capacity);
    unchanged(&mut world, MAX_TICKS, &[c], Error::Capacity);
    eprintln!(
        "maximum valid horizon: {} ticks, {} checkpoint bytes, drive {:?}, restore {:?}",
        MAX_TICKS,
        bytes.len(),
        drive_elapsed,
        restore_elapsed
    );
}

#[test]
fn checkpoint_bounds_corruption_identity_and_full_candidate_validation() {
    let w = World::settlement();
    let bytes = w.checkpoint();
    for end in 0..bytes.len() {
        assert!(World::restore(&bytes[..end]).is_err(), "prefix {end}");
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(World::restore(&trailing).is_err());
    let mut version = bytes.clone();
    version[12] = 2;
    assert_eq!(
        World::restore(&version),
        Err(Error::Restore(RestoreError::UnsupportedRevision {
            found: 2
        }))
    );
    let mut content = bytes.clone();
    content[14] ^= 1;
    resign(&mut content);
    assert_eq!(
        World::restore(&content),
        Err(Error::Restore(RestoreError::ContentMismatch))
    );
    let mut checksum = bytes.clone();
    *checksum.last_mut().unwrap() ^= 1;
    assert_eq!(
        World::restore(&checksum),
        Err(Error::Restore(RestoreError::ChecksumMismatch))
    );
    let mut excessive = bytes.clone();
    excessive[155..159].copy_from_slice(&u32::MAX.to_le_bytes());
    resign(&mut excessive);
    assert!(World::restore(&excessive).is_err());
    assert!(World::restore(&vec![0; MAX_NPC_BYTES + MAX_COMMANDS * 31 + 257]).is_err());
    // Private corruption exercises semantic checks with otherwise authentic encoding/checksum.
    let mut candidates = Vec::new();
    let mut bad = w.clone();
    bad.position.x = WIDTH;
    candidates.push(bad);
    let mut bad = w.clone();
    bad.position = Position { x: 1, y: 4 };
    candidates.push(bad);
    let mut bad = w.clone();
    bad.herbs[0].quantity = 4;
    candidates.push(bad);
    let mut bad = w.clone();
    bad.herbs[0].quantity = u64::MAX;
    candidates.push(bad);
    let mut bad = w.clone();
    bad.herbs.swap(0, 1);
    candidates.push(bad);
    let mut bad = w.clone();
    bad.requested = true;
    candidates.push(bad);
    let mut bad = w.clone();
    bad.completed = true;
    candidates.push(bad);
    let mut bad = w.clone();
    bad.next_sequence = 1;
    candidates.push(bad);
    let mut bad = w.clone();
    bad.next_event_id = 2;
    candidates.push(bad);
    let mut bad = w.clone();
    bad.digest[0] ^= 1;
    candidates.push(bad);
    let mut bad = w.clone();
    bad.npcs
        .world3
        .as_mut()
        .unwrap()
        .actors
        .get_mut(&1)
        .unwrap()
        .enabled = 0;
    candidates.push(bad);
    for bad in candidates {
        assert!(World::restore(&bad.checkpoint()).is_err());
    }
    assert!(World::restore(DEMO).is_err());
    assert!(Simulation::restore_domain_checkpoint(&bytes).is_err());
    assert_eq!(World::restore(&bytes).unwrap(), w);
}

#[test]
fn registry_codec_rejects_payload_outcome_order_cursor_and_tick_corruption() {
    let mut w = World::settlement();
    act(&mut w, Intent::Move(Direction::East));
    act(&mut w, Intent::Move(Direction::East));
    let bytes = w.checkpoint();
    assert_eq!(World::restore(&bytes).unwrap(), w);
    // Fixed record start 159; keys 24, intent 5, outcome 2.
    for offset in [159 + 8, 159 + 24, 159 + 29, 159 + 30] {
        let mut bad = bytes.clone();
        bad[offset] = 255;
        resign(&mut bad);
        assert!(World::restore(&bad).is_err(), "offset {offset}");
    }
    let mut unsorted = bytes.clone();
    let first = unsorted[159..190].to_vec();
    let second = unsorted[190..221].to_vec();
    unsorted[159..190].copy_from_slice(&second);
    unsorted[190..221].copy_from_slice(&first);
    resign(&mut unsorted);
    assert!(World::restore(&unsorted).is_err());
    let mut bad = w.clone();
    let key = *bad.registry.first_key_value().unwrap().0;
    bad.registry.get_mut(&key).unwrap().outcome = Outcome::Delivered;
    assert!(World::restore(&bad.checkpoint()).is_err());
    let mut bad = bytes.clone();
    bad[46..54].copy_from_slice(&9u64.to_le_bytes());
    resign(&mut bad);
    assert!(World::restore(&bad).is_err());
}

#[test]
fn rejected_events_consume_identity_chain_and_roundtrip_across_empty_boundaries() {
    let mut w = World::settlement();
    let mut before_digest = w.event_digest();
    for intent in [
        Intent::Move(Direction::West),
        Intent::Talk(99),
        Intent::Talk(1),
        Intent::Gather,
        Intent::Deliver,
    ] {
        let key = command(&w, intent).key;
        let events = w.drive(w.tick(), &[Command { key, intent }]).unwrap();
        assert!(matches!(events[0].outcome, Outcome::Rejected(_)));
        assert_eq!(events[0].id, key.sequence + 1);
        assert_eq!(w.next_sequence(), key.sequence + 1);
        assert_ne!(w.event_digest(), before_digest);
        before_digest = w.event_digest();
        w = World::restore(&w.checkpoint()).unwrap();
        assert!(w.drive(w.tick(), &[]).unwrap().is_empty());
        assert_eq!(w.event_digest(), before_digest);
        w = World::restore(&w.checkpoint()).unwrap();
    }
    let bytes = w.checkpoint();
    let mut bad = w.clone();
    // A plausible rejection tag alone is insufficient: east from the start is
    // not an out-of-bounds move. The unchanged outcomes retain the same event
    // chain, isolating command-history semantic validation from checksum checks.
    let key = *bad.registry.first_key_value().unwrap().0;
    bad.registry.get_mut(&key).unwrap().intent = Intent::Move(Direction::East);
    assert!(World::restore(&bad.checkpoint()).is_err());
    assert_eq!(w.checkpoint(), bytes);
}
