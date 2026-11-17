mod c02;
mod c02_golden;
mod c03_remediation;
mod c03_s1;
mod golden;

use super::*;

const A1: ActorId = ActorId::ONE;
const A2: ActorId = ActorId::TWO;

fn fixture(seed: u64, actor_enabled: [bool; 2]) -> Simulation {
    Simulation::new(FixtureConfig {
        seed,
        actor_enabled,
    })
}

fn claim(tick: u64, actor: ActorId, producer: u32, sequence: u64) -> Command {
    Command::new(
        CommandKey {
            target_tick: tick,
            actor_id: actor,
            producer_id: producer,
            sequence,
        },
        Payload::ClaimSlot,
    )
}

fn probe(command: Command, stream_id: u8, draw_count: u8) -> Command {
    Command::new(
        command.key(),
        Payload::Probe {
            stream_id,
            draw_count,
        },
    )
}

fn contention() -> [Command; 2] {
    [claim(2, A2, 1, 0), claim(2, A1, 1, 0)]
}

fn event(id: u64, command: Command, outcome: Outcome) -> Event {
    Event {
        id,
        key: command.key(),
        outcome,
    }
}

fn words(stream_id: u8, first_word: u32, second_word: Option<u32>) -> Outcome {
    Outcome::Probed {
        stream_id,
        first_word,
        second_word,
    }
}

fn decode_hex(hex: &str) -> Vec<u8> {
    assert!(hex.len().is_multiple_of(2));
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn display_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut hex = String::with_capacity(2 * bytes.len());
    for byte in bytes {
        write!(hex, "{byte:02x}").unwrap();
    }
    hex
}

fn assert_bytes(actual: &[u8], expected: &str) {
    assert_eq!(
        actual,
        decode_hex(expected),
        "observed: {}",
        display_hex(actual)
    );
}

fn assert_checkpoint(sim: &Simulation, vector: golden::Checkpoint) {
    assert_bytes(&sim.checkpoint_bytes(), vector.bytes);
    assert_bytes(&sim.checkpoint_sha256(), vector.sha256);
}

fn assert_rollback(sim: &mut Simulation, boundary: u64, batch: &[Command], error: StepError) {
    let before = sim.clone();
    let bytes = sim.checkpoint_bytes();
    assert_eq!(sim.step(boundary, batch), Err(error));
    assert_eq!(*sim, before);
    assert_eq!(sim.checkpoint_bytes(), bytes);
}

fn command_error(command: Command, reason: InvalidCommand) -> StepError {
    StepError::Command {
        key: command.key(),
        reason,
    }
}

#[test]
fn c01_t01_clock_pending_and_pause() {
    let mut sim = fixture(100, [true, true]);
    let [second, first] = contention();

    assert_eq!(FIXTURE_ID, 1);
    assert_eq!(sim.config().seed, 100);
    assert_eq!(sim.actor_enabled(A1), Some(true));
    assert_eq!(sim.actor_enabled(A2), Some(true));
    assert_eq!(sim.actor_enabled(ActorId::new(3)), None);
    assert_eq!(sim.tick(), 0);
    assert_eq!(sim.slot_owner(), None);
    assert_eq!(sim.pending_commands().len(), 0);
    assert_eq!(sim.next_event_id(), 1);
    assert_bytes(&sim.event_digest(), golden::H0);
    let initial = sim.checkpoint_bytes();
    assert_eq!(sim.checkpoint_bytes(), initial);

    assert!(sim.step(0, &[second, first]).unwrap().is_empty());
    assert_eq!(sim.tick(), 1);
    assert_eq!(sim.pending_commands().collect::<Vec<_>>(), [first, second]);
    assert_eq!(sim.slot_owner(), None);
    assert_eq!(sim.next_event_id(), 1);
    assert_bytes(&sim.event_digest(), golden::H0);
    let paused = sim.checkpoint_bytes();
    assert_eq!(sim.checkpoint_bytes(), paused);

    let events = sim.step(1, &[]).unwrap();
    assert_eq!(sim.tick(), 2);
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|event| event.key().target_tick == 2));
    assert_eq!(sim.pending_commands().len(), 0);
    assert_eq!(sim.slot_owner(), Some(A1));
}

#[test]
fn c01_t01_empty_steps_only_advance_the_clock() {
    let mut sim = fixture(100, [false, true]);
    assert_eq!(sim.actor_enabled(A1), Some(false));
    assert_eq!(sim.actor_enabled(A2), Some(true));
    let initial = sim.clone();
    for boundary in 0..2 {
        assert!(sim.step(boundary, &[]).unwrap().is_empty());
        assert_eq!(
            sim,
            Simulation {
                tick: boundary + 1,
                ..initial.clone()
            }
        );
    }
}

#[test]
fn c01_t02_commit_revalidates_contention_and_disabled_actors() {
    let [second, first] = contention();
    let occupied = Outcome::Rejected(Rejection::OccupiedSlot);
    let disabled = Outcome::Rejected(Rejection::DisabledActor);
    for batch in [[first, second], [second, first]] {
        let mut sim = fixture(100, [true, true]);
        assert!(sim.step(0, &batch).unwrap().is_empty());
        assert_eq!(
            sim.step(1, &[]).unwrap(),
            [
                event(1, first, Outcome::Claimed),
                event(2, second, occupied)
            ]
        );
        let repeat = claim(3, A1, 1, 0);
        assert_eq!(
            sim.step(2, &[repeat]).unwrap(),
            [event(3, repeat, occupied)]
        );
        assert_eq!(sim.slot_owner(), Some(A1));
    }

    let mut sim = fixture(100, [false, true]);
    assert!(sim.step(0, &[second, first]).unwrap().is_empty());
    assert_eq!(
        sim.step(1, &[]).unwrap(),
        [
            event(1, first, disabled),
            event(2, second, Outcome::Claimed)
        ]
    );
    assert_eq!(sim.slot_owner(), Some(A2));
    let mut sim = fixture(100, [false, true]);
    let draw = probe(claim(1, A1, 1, 0), 1, 1);
    let events = sim.step(0, &[draw]).unwrap();
    assert_eq!(events, [event(1, draw, disabled)]);
    assert_eq!(&events[0].canonical_bytes()[32..], &[2, 1]);
    assert_eq!(sim.ordinals, [[0; 2]; 2]);
}

#[test]
fn c01_t02_orders_numeric_producer_before_sequence() {
    for (first_producer, first_sequence, second_producer, second_sequence) in [
        (1, 9, 2, 0),
        (1, 0, 1, 1),
        (1, 9, 256, 0),
        (1, 0, u32::MAX, 0),
    ] {
        let first = claim(1, A1, first_producer, first_sequence);
        let second = claim(1, A1, second_producer, second_sequence);
        let mut sim = fixture(100, [true, true]);
        let events = sim.step(0, &[second, first]).unwrap();
        assert_eq!(
            events,
            [
                event(1, first, Outcome::Claimed),
                event(2, second, Outcome::Rejected(Rejection::OccupiedSlot))
            ],
        );
        assert_eq!(sim.slot_owner(), Some(A1));
    }
}

#[test]
fn c01_t03_deduplicates_admitted_and_resubmitted_commands() {
    let mut sim = fixture(100, [true, true]);
    let work = claim(3, A1, 1, 0);
    for boundary in 0..2 {
        assert!(sim.step(boundary, &[work, work]).unwrap().is_empty());
        assert_eq!(sim.pending_commands().collect::<Vec<_>>(), [work]);
    }
    assert_eq!(
        sim.step(2, &[]).unwrap(),
        [event(1, work, Outcome::Claimed)]
    );
    assert_eq!(sim.pending_commands().len(), 0);
    let valid = claim(4, A2, 1, 0);
    assert_rollback(
        &mut sim,
        3,
        &[valid, work],
        command_error(work, InvalidCommand::LateTarget),
    );
    assert_eq!(
        sim.step(3, &[valid]).unwrap(),
        [event(2, valid, Outcome::Rejected(Rejection::OccupiedSlot))]
    );
}

#[test]
fn c01_t03_conflicts_roll_back_in_any_input_order() {
    let mut sim = fixture(100, [true, true]);
    let due = claim(1, A2, 2, 0);
    let existing = claim(3, A1, 1, 0);
    let conflicted = probe(existing, 2, 1);
    let higher_conflict = claim(3, A2, 1, 0);
    let higher_probe = probe(higher_conflict, 1, 2);
    let batch = [due, existing, conflicted, higher_conflict, higher_probe];
    let mut reversed = batch;
    reversed.reverse();
    let error = command_error(existing, InvalidCommand::ConflictingIdentity);
    assert_rollback(&mut sim, 0, &batch, error);
    assert_rollback(&mut sim, 0, &reversed, error);
    let malformed = claim(4, A1, 0, 0);
    assert_rollback(
        &mut sim,
        0,
        &[existing, conflicted, due, malformed],
        command_error(malformed, InvalidCommand::InvalidProducer),
    );
    assert!(sim.step(0, &[existing]).unwrap().is_empty());

    let due = claim(2, A2, 2, 0);
    assert_rollback(&mut sim, 1, &[due, conflicted], error);
    assert_rollback(&mut sim, 1, &[conflicted, due], error);
    assert_eq!(sim.pending_commands().collect::<Vec<_>>(), [existing]);
    assert_eq!(
        sim.step(1, &[due]).unwrap(),
        [event(1, due, Outcome::Claimed)]
    );
    assert_eq!(
        sim.step(2, &[]).unwrap(),
        [event(
            2,
            existing,
            Outcome::Rejected(Rejection::OccupiedSlot)
        )]
    );
}

#[test]
fn c01_t03_preflight_selects_boundary_key_and_diagnostic_in_order() {
    let mut sim = fixture(100, [true, true]);
    let valid = claim(1, A1, 1, 9);
    let late = probe(claim(0, ActorId::new(9), 0, 0), 0, 0);
    let unknown = probe(claim(1, ActorId::new(9), 0, 0), 0, 0);
    let zero = probe(claim(1, A1, 0, 0), 0, 0);
    let zero_only = claim(1, A1, 0, 0);
    assert_rollback(
        &mut sim,
        8,
        &[valid, late],
        StepError::WrongBoundary {
            expected: 0,
            received: 8,
        },
    );
    for (invalid, reason) in [
        (late, InvalidCommand::LateTarget),
        (unknown, InvalidCommand::UnknownActor),
        (zero, InvalidCommand::InvalidProducer),
        (zero_only, InvalidCommand::InvalidProducer),
        (probe(valid, 0, 1), InvalidCommand::InvalidPayload),
        (probe(valid, 3, 1), InvalidCommand::InvalidPayload),
        (probe(valid, 1, 0), InvalidCommand::InvalidPayload),
        (probe(valid, 2, 3), InvalidCommand::InvalidPayload),
    ] {
        let error = command_error(invalid, reason);
        assert_rollback(&mut sim, 0, &[valid, invalid], error);
        assert_rollback(&mut sim, 0, &[invalid, valid], error);
    }

    let earlier_key = probe(claim(1, A1, 1, 0), 3, 1);
    let later_key = claim(1, A2, 0, 0);
    let error = command_error(earlier_key, InvalidCommand::InvalidPayload);
    assert_rollback(&mut sim, 0, &[later_key, valid, earlier_key], error);
    assert_rollback(&mut sim, 0, &[earlier_key, valid, later_key], error);
    assert_eq!(
        sim.step(0, &[valid]).unwrap(),
        [event(1, valid, Outcome::Claimed)]
    );
}

#[test]
fn c01_t03_tick_cursor_and_ordinal_overflow_roll_back() {
    let capacity = StepError::CapacityExceeded;
    let mut clock = fixture(100, [true, true]);
    clock.tick = u64::MAX;
    assert_rollback(&mut clock, u64::MAX, &[], capacity);

    let first = claim(1, A1, 1, 0);
    let second = probe(claim(1, A2, 1, 0), 1, 2);
    let future = claim(2, A2, 1, 0);
    let mut cursor = fixture(100, [true, true]);
    cursor.next_event_id = u64::MAX - 1;
    assert_rollback(&mut cursor, 0, &[future, second, first], capacity);
    assert_eq!(
        cursor.step(0, &[first]).unwrap(),
        [event(u64::MAX - 1, first, Outcome::Claimed)]
    );
    assert_eq!(cursor.next_event_id(), u64::MAX);
    assert_rollback(&mut cursor, 1, &[future], capacity);

    let mut ordinal = fixture(100, [true, true]);
    ordinal.ordinals[0][1] = u64::MAX;
    assert_rollback(&mut ordinal, 0, &[second, future, first], capacity);
    assert_eq!(
        ordinal.step(0, &[first]).unwrap(),
        [event(1, first, Outcome::Claimed)]
    );
    assert_eq!(ordinal.next_ordinal(1, A2), Some(u64::MAX));

    let mut disabled = fixture(100, [true, false]);
    disabled.ordinals[0][1] = u64::MAX;
    assert_eq!(
        disabled.step(0, &[second]).unwrap(),
        [event(
            1,
            second,
            Outcome::Rejected(Rejection::DisabledActor)
        )]
    );
    assert_eq!(disabled.next_ordinal(1, A2), Some(u64::MAX));
}

#[test]
fn c01_t04_checkpoint_and_event_chain_golden_vectors() {
    let mut sim = fixture(100, [true, true]);
    assert_bytes(b"SH:C01:FIXTURE:v1\n", golden::CONTENT_BYTES);
    assert_bytes(&Simulation::fixture_content_hash(), golden::CONTENT_HASH);
    assert_bytes(&sim.event_digest(), golden::H0);
    assert_checkpoint(&sim, golden::INITIAL);
    println!(
        "checkpoint.initial.sha256={}",
        display_hex(&sim.checkpoint_sha256())
    );

    assert!(sim.step(0, &contention()).unwrap().is_empty());
    assert_checkpoint(&sim, golden::PENDING);
    println!(
        "checkpoint.pending.sha256={}",
        display_hex(&sim.checkpoint_sha256())
    );

    let prefix = sim.event_digest();
    let events = sim.step(1, &[]).unwrap();
    assert_eq!(events.len(), 2);
    assert_bytes(&events[0].canonical_bytes(), golden::EVENT_1);
    assert_bytes(&events[1].canonical_bytes(), golden::EVENT_2);
    let first_digest = encoding::extend_event_digest(prefix, events[0]);
    assert_bytes(&first_digest, golden::H1);
    assert_bytes(
        &encoding::extend_event_digest(first_digest, events[1]),
        golden::H2,
    );
    assert_bytes(&sim.event_digest(), golden::H2);
    assert_checkpoint(&sim, golden::CONTENTION_END);
    println!(
        "checkpoint.contention.sha256={}",
        display_hex(&sim.checkpoint_sha256())
    );
    println!(
        "event.1.bytes={}",
        display_hex(&events[0].canonical_bytes())
    );
    println!(
        "event.2.bytes={}",
        display_hex(&events[1].canonical_bytes())
    );
    println!(
        "event.contention.digest={}",
        display_hex(&sim.event_digest())
    );

    let mut sim = fixture(100, [true, true]);
    let draw = probe(claim(1, A1, 1, 0), 1, 2);
    let events = sim.step(0, &[draw]).unwrap();
    assert_eq!(events.len(), 1);
    assert_bytes(&events[0].canonical_bytes(), golden::PROBE_EVENT);
    assert_bytes(&sim.event_digest(), golden::PROBE_DIGEST);
    assert_checkpoint(&sim, golden::PROBE_END);
    println!(
        "checkpoint.probe.sha256={}",
        display_hex(&sim.checkpoint_sha256())
    );
    println!("event.probe.digest={}", display_hex(&sim.event_digest()));
}

#[test]
fn c01_t04_named_rng_golden_vectors_and_stream_isolation() {
    for vector in golden::RNG {
        let key = encoding::probe_seed(vector.seed, vector.stream_id, A1, vector.ordinal);
        assert_bytes(&key, vector.key);
        assert_eq!(encoding::sha256(&decode_hex(vector.preimage)), key);
        let mut rng = ChaCha12Rng::from_seed(key);
        let actual = [rng.next_u32(), rng.next_u32()];
        assert_eq!(actual, vector.words);
        println!(
            "rng.seed={}/stream={}/ordinal={} words={:08x},{:08x}",
            vector.seed, vector.stream_id, vector.ordinal, actual[0], actual[1],
        );
    }

    let [first_a, first_b, next_a, _] = golden::RNG.map(|vector| vector.words);
    for draw_count in 1..=2 {
        let mut sim = fixture(100, [true, true]);
        let a = probe(claim(1, A1, 1, 0), 1, draw_count);
        let b = probe(claim(2, A1, 1, 0), 2, 2);
        let a_next = probe(claim(3, A1, 1, 0), 1, 2);
        let second_word = (draw_count == 2).then_some(first_a[1]);
        let events = sim.step(0, &[a]).unwrap();
        assert_eq!(events, [event(1, a, words(1, first_a[0], second_word))]);
        if draw_count == 1 {
            assert_bytes(&events[0].canonical_bytes()[32..], "030101325b9111");
        }
        assert_eq!(sim.ordinals, [[1, 0], [0, 0]]);
        assert_eq!(
            sim.step(1, &[b]).unwrap(),
            [event(2, b, words(2, first_b[0], Some(first_b[1])))]
        );
        assert_eq!(sim.ordinals, [[1, 0], [1, 0]]);
        assert_eq!(
            sim.step(2, &[a_next]).unwrap(),
            [event(3, a_next, words(1, next_a[0], Some(next_a[1])))]
        );
        assert_eq!(sim.ordinals, [[2, 0], [1, 0]]);
    }
}

#[test]
fn c01_t04_checkpoint_covers_each_mutable_field_group() {
    let sim = fixture(100, [true, true]);
    let mut pending = sim.clone();
    let command = claim(1, A1, 1, 0);
    pending.pending.insert(command.key(), command.payload());
    let mut ordinal = sim.clone();
    ordinal.ordinals[0][0] = 1;
    let mut digest = sim.event_digest;
    digest[0] ^= 1;

    for alternate in [
        Simulation {
            tick: 1,
            ..sim.clone()
        },
        fixture(101, [true, true]),
        fixture(100, [false, true]),
        Simulation {
            slot_owner: Some(A1),
            ..sim.clone()
        },
        pending,
        ordinal,
        Simulation {
            next_event_id: 2,
            ..sim.clone()
        },
        Simulation {
            event_digest: digest,
            ..sim.clone()
        },
    ] {
        assert_ne!(alternate.checkpoint_bytes(), sim.checkpoint_bytes());
        assert_ne!(alternate.checkpoint_sha256(), sim.checkpoint_sha256());
    }
}

#[test]
fn c01_t04_queue_encoding_uses_numeric_order_independent_of_insertion() {
    let mut forward = fixture(100, [true, true]);
    let mut reverse = forward.clone();
    let first = claim(2, A1, 1, 9);
    let second = claim(2, A1, 256, 0);
    for (sim, batch) in [
        (&mut forward, [first, second]),
        (&mut reverse, [second, first]),
    ] {
        assert!(sim.step(0, &batch).unwrap().is_empty());
        assert_eq!(sim.pending_commands().collect::<Vec<_>>(), [first, second]);
    }
    let bytes = forward.checkpoint_bytes();
    assert_eq!(bytes, reverse.checkpoint_bytes());
    assert_eq!(forward.checkpoint_sha256(), reverse.checkpoint_sha256());
    // Record positions for the vacant two-actor fixture in encoding revision 1.
    let pending_start = 12 + 2 + 4 + 32 + 8 + 8 + 8 + 2 * 5 + 1 + 8;
    let producer_start = pending_start + 8 + 4;
    let claim_width = 8 + 4 + 4 + 8 + 1;
    assert_eq!(&bytes[producer_start..][..4], &[1, 0, 0, 0]);
    assert_eq!(&bytes[producer_start + claim_width..][..4], &[0, 1, 0, 0]);
}

#[derive(Debug, Eq, PartialEq)]
struct ReplayEvidence {
    checkpoints: Vec<(Vec<u8>, [u8; 32])>,
    event_bytes: Vec<Vec<u8>>,
}

fn run_contention_replay(
    groups: &[usize],
    reverse_input: bool,
    rejected_attempt: bool,
) -> ReplayEvidence {
    let mut sim = fixture(100, [true, true]);
    let mut batch = contention();
    if reverse_input {
        batch.reverse();
    }
    let log: [(u64, &[Command]); 2] = [(0, &batch), (1, &[])];
    let mut input = log.into_iter();
    let mut checkpoints = vec![(sim.checkpoint_bytes(), sim.checkpoint_sha256())];
    let mut event_bytes = Vec::new();
    for &group_size in groups {
        for _ in 0..group_size {
            let (boundary, commands) = input.next().unwrap();
            if rejected_attempt && boundary == 1 {
                let invalid = claim(2, A1, 0, 0);
                assert_rollback(
                    &mut sim,
                    boundary,
                    &[invalid],
                    command_error(invalid, InvalidCommand::InvalidProducer),
                );
            }
            let events = sim.step(boundary, commands).unwrap();
            match boundary {
                0 => {
                    assert!(events.is_empty());
                    assert_eq!(sim.slot_owner(), None);
                }
                1 => {
                    let [second, first] = contention();
                    assert_eq!(
                        events,
                        [
                            event(1, first, Outcome::Claimed),
                            event(2, second, Outcome::Rejected(Rejection::OccupiedSlot))
                        ]
                    );
                    assert_eq!(sim.slot_owner(), Some(A1));
                }
                _ => unreachable!(),
            }
            checkpoints.push((sim.checkpoint_bytes(), sim.checkpoint_sha256()));
            event_bytes.extend(events.into_iter().map(Event::canonical_bytes));
        }
    }
    assert!(input.next().is_none());
    ReplayEvidence {
        checkpoints,
        event_bytes,
    }
}

#[test]
fn c01_t05_replay_matches_every_checkpoint_and_concatenated_event_stream() {
    let single_steps = run_contention_replay(&[1, 1], false, false);
    let grouped_steps = run_contention_replay(&[2], true, false);
    let retried = run_contention_replay(&[1, 1], false, true);
    assert_eq!(single_steps, grouped_steps);
    assert_eq!(single_steps, retried);
    for ((bytes, checksum), vector) in single_steps.checkpoints.iter().zip([
        golden::INITIAL,
        golden::PENDING,
        golden::CONTENTION_END,
    ]) {
        assert_bytes(bytes, vector.bytes);
        assert_bytes(checksum, vector.sha256);
    }
    assert_eq!(
        single_steps.event_bytes.concat(),
        decode_hex(&[golden::EVENT_1, golden::EVENT_2].concat())
    );
}

#[test]
fn c01_t05_seed_101_smoke() {
    let mut sim = fixture(101, [true, true]);
    let command = probe(claim(1, A1, 1, 0), 1, 1);
    let events = sim.step(0, &[command]).unwrap();
    assert_eq!(
        events,
        [event(1, command, words(1, golden::RNG[3].words[0], None))]
    );
    assert_eq!(sim.tick(), 1);
    assert_eq!(sim.next_event_id(), 2);
    assert_eq!(sim.ordinals, [[1, 0], [0, 0]]);
    assert_eq!(sim.slot_owner(), None);
}
