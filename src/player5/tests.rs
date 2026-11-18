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
