use sha2::{Digest, Sha256};

use super::c02_golden::*;
use super::*;

const EVENT_3: &str = "03000000000000000300000000000000020000000100000001000000000000000202";
const DIGEST_3: &str = "1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1";
const PROBE_2: &str =
    "0200000000000000020000000000000001000000010000000000000000000000030101d5e13bdf";
const PROBE_2_DIGEST: &str = "b5dfc10d13778e742283d07e2c7a2973fc2f399e0bff4211064236b7a579e344";
const MINT_1: &str = "0100000000000000010000000000000001000000020a0000000000000001";
const MINT_1_DIGEST: &str = "39cbd921dff575948405544eea3efa8ce08739359aba260decb5e670c2a65fae";
const MINT_2: &str = "0200000000000000020000000000000001000000020b0000000000000001";
const MINT_2_DIGEST: &str = "c0aedb6c21860638585bd07ad6e1b3fbc279db0b0b93ba9ce437458cde1cf7da";
const TASK_1: &str = "01000000000000000300000000000000010000000106000000000000000800000000000000040000000000000001000000000000000200000000000000";
const TASK_1_DIGEST: &str = "f9c54cce1eed252da770ffb4798d065cb255961a0fddd019d98d6168d776b70f";
const TASK_2: &str = "02000000000000000300000000000000020000000107000000000000000900000000000000050000000000000001000000000000000300000000000000";
const TASK_2_DIGEST: &str = "c48936ae6b6e4245f63212280cb07659eab2e9dd2c189c2cf46d7e70d3a3dd67";
const WITNESS_MINT: &str = "0100000000000000020000000000000001000000020a0000000000000002";
const WITNESS_MINT_DIGEST: &str =
    "7a0f89b2b1ff28e4586615339b6bb757916f9ef141b27c9df1cd825a2c0f8ffb";
const WITNESS_TASK_1: &str = "02000000000000000200000000000000010000000106000000000000000800000000000000040000000000000001000000000000000200000000000000";
const WITNESS_TASK_2: &str = "03000000000000000200000000000000020000000107000000000000000900000000000000050000000000000001000000000000000300000000000000";
const WITNESS_FINAL_DIGEST: &str =
    "4dfb4804407daa5cc8bffe951d11ca8830e2fae4c6aa1b01f9e372956f9dfb93";
const DISABLED_EVENT: &str = "01000000000000000100000000000000010000000301";
const DISABLED_DIGEST: &str = "55735de709c23c26eebe840a8c10b4fb560a19789232a740cdfcb6a3e69e60af";
const F2_H0: &str = "413408f703da057f8b39023e580c8c65649376824d1a77be0a4d24aaf048dd0b";

fn decode(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
        .collect()
}

fn digest_of(text: &str) -> [u8; 32] {
    decode(text).try_into().unwrap()
}

fn reseal(body: &[u8]) -> Vec<u8> {
    let mut bytes = body.to_vec();
    bytes.extend(Sha256::digest(body));
    bytes
}

fn body_of(blob: &[u8]) -> &[u8] {
    &blob[..blob.len() - 32]
}

fn assert_blob(sim: &Simulation, expected: &str) {
    let bytes = decode(expected);
    assert_eq!(sim.domain_checkpoint_bytes(), bytes);
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    assert_eq!(sim.domain_checkpoint_sha256(), digest);
}

fn claim_at(tick: u64, actor: ActorId, sequence: u64) -> Command {
    Command::new(
        CommandKey {
            target_tick: tick,
            actor_id: actor,
            producer_id: 1,
            sequence,
        },
        Payload::ClaimSlot,
    )
}

fn set_mode(tick: u64, actor: ActorId, sequence: u64, mode: ProcessingMode) -> Command {
    Command::new(
        CommandKey {
            target_tick: tick,
            actor_id: actor,
            producer_id: 1,
            sequence,
        },
        Payload::SetMode(mode),
    )
}

fn mint(tick: u64, actor: ActorId, sequence: u64) -> Command {
    Command::new(
        CommandKey {
            target_tick: tick,
            actor_id: actor,
            producer_id: 1,
            sequence,
        },
        Payload::MintRecord,
    )
}

fn scheduler() -> Simulation {
    Simulation::scheduler(100, 3, [true, true]).unwrap()
}

fn primary_batch() -> [Command; 4] {
    [
        set_mode(2, ActorId::ONE, 0, ProcessingMode::Scheduled),
        set_mode(2, ActorId::TWO, 0, ProcessingMode::Scheduled),
        set_mode(4, ActorId::ONE, 0, ProcessingMode::Active),
        set_mode(4, ActorId::TWO, 0, ProcessingMode::Active),
    ]
}

#[test]
fn c02_t01a_slot_prefix_resume() {
    let first = claim_at(2, ActorId::TWO, 0);
    let second = claim_at(2, ActorId::ONE, 0);
    let future = claim_at(3, ActorId::TWO, 1);
    let mut sim = Simulation::new(FixtureConfig {
        seed: 100,
        actor_enabled: [true, true],
    });
    assert!(sim.step(0, &[first, second]).unwrap().is_empty());
    let admitted = sim.step(1, &[future]).unwrap();
    assert_eq!(
        admitted[0].canonical_bytes(),
        decode(super::golden::EVENT_1)
    );
    assert_eq!(
        admitted[1].canonical_bytes(),
        decode(super::golden::EVENT_2)
    );
    assert_blob(&sim, T01_SAVE);
    assert_eq!(sim.checkpoint_bytes(), decode(C01_T01_IMAGE));
    assert_eq!(
        Simulation::restore_domain_checkpoint(&decode(C01_T01_IMAGE)),
        Err(RestoreError::UnsupportedFormat)
    );
    assert_eq!(sim.tick(), 2);
    assert_eq!(sim.slot_owner(), Some(ActorId::ONE));
    assert_eq!(sim.pending_commands().collect::<Vec<_>>(), [future]);
    assert_eq!(sim.next_ordinal(1, ActorId::ONE), Some(0));
    assert_eq!(sim.next_ordinal(1, ActorId::TWO), Some(0));
    assert_eq!(sim.next_ordinal(2, ActorId::ONE), Some(0));
    assert_eq!(sim.next_ordinal(2, ActorId::TWO), Some(0));
    assert_eq!(sim.next_actor_id(), 3);
    assert_eq!(sim.next_record_id(), 1);
    assert_eq!(
        sim.processing_mode(ActorId::ONE),
        Some(ProcessingMode::Active)
    );
    assert_eq!(
        sim.processing_mode(ActorId::TWO),
        Some(ProcessingMode::Active)
    );
    assert_eq!(sim.location_ids().len(), 0);
    assert_eq!(sim.event_digest(), digest_of(super::golden::H2));

    let mut restored =
        Simulation::restore_domain_checkpoint(&sim.domain_checkpoint_bytes()).unwrap();
    assert_eq!(
        restored.domain_checkpoint_bytes(),
        sim.domain_checkpoint_bytes()
    );
    let events = restored.step(2, &[]).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].canonical_bytes(), decode(EVENT_3));
    assert_eq!(
        events[0].outcome(),
        Outcome::Rejected(Rejection::OccupiedSlot)
    );
    assert_eq!(events[0].key().target_tick, 3);
    assert_eq!(events[0].key().actor_id, ActorId::TWO);
    assert_eq!(events[0].key().producer_id, 1);
    assert_eq!(events[0].key().sequence, 1);
    assert_eq!(restored.event_digest(), digest_of(DIGEST_3));
    assert_eq!(restored.next_event_id(), 4);
    assert_blob(&restored, T01_FINAL);

    let mut whole = Simulation::new(FixtureConfig {
        seed: 100,
        actor_enabled: [true, true],
    });
    whole.step(0, &[second, first]).unwrap();
    whole.step(1, &[future]).unwrap();
    assert_eq!(whole.domain_checkpoint_bytes(), decode(T01_SAVE));
    let direct = whole.step(2, &[]).unwrap();
    assert_eq!(direct[0].canonical_bytes(), events[0].canonical_bytes());
    assert_eq!(
        whole.domain_checkpoint_bytes(),
        restored.domain_checkpoint_bytes()
    );
    let saved_digest = restored.event_digest();
    let saved_cursor = restored.next_event_id();
    assert!(restored.step(3, &[]).unwrap().is_empty());
    assert_eq!(restored.event_digest(), saved_digest);
    assert_eq!(restored.next_event_id(), saved_cursor);
    assert_eq!(restored.slot_owner(), Some(ActorId::ONE));

    let before = sim.domain_checkpoint_bytes();
    let mode = set_mode(4, ActorId::ONE, 0, ProcessingMode::Scheduled);
    assert_eq!(
        sim.step(2, &[mode]),
        Err(StepError::Command {
            key: mode.key(),
            reason: InvalidCommand::InvalidPayload,
        })
    );
    assert_eq!(sim.domain_checkpoint_bytes(), before);
}

#[test]
fn c02_t01b_ordinal_resume() {
    let mut sim = Simulation::new(FixtureConfig {
        seed: 100,
        actor_enabled: [true, true],
    });
    let first = Command::new(
        CommandKey {
            target_tick: 1,
            actor_id: ActorId::ONE,
            producer_id: 1,
            sequence: 0,
        },
        Payload::Probe {
            stream_id: 1,
            draw_count: 2,
        },
    );
    sim.step(0, &[first]).unwrap();
    assert_blob(&sim, PROBE_SAVE);
    assert_eq!(
        sim.checkpoint_bytes(),
        decode(super::golden::PROBE_END.bytes)
    );
    let mut restored = Simulation::restore_domain_checkpoint(&decode(PROBE_SAVE)).unwrap();
    let second = Command::new(
        CommandKey {
            target_tick: 2,
            actor_id: ActorId::ONE,
            producer_id: 1,
            sequence: 0,
        },
        Payload::Probe {
            stream_id: 1,
            draw_count: 1,
        },
    );
    let events = restored.step(1, &[second]).unwrap();
    match events[0].outcome() {
        Outcome::Probed {
            first_word,
            second_word: None,
            ..
        } => assert_eq!(first_word, 0xdf3b_e1d5),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(events[0].canonical_bytes(), decode(PROBE_2));
    assert_eq!(restored.event_digest(), digest_of(PROBE_2_DIGEST));
    assert_eq!(restored.next_ordinal(1, ActorId::ONE), Some(2));
    assert_eq!(restored.next_ordinal(1, ActorId::TWO), Some(0));
    assert_eq!(restored.next_ordinal(2, ActorId::ONE), Some(0));
    assert_eq!(restored.next_ordinal(2, ActorId::TWO), Some(0));
    assert_blob(&restored, PROBE_FINAL);
    sim.step(1, &[second]).unwrap();
    assert_eq!(
        sim.domain_checkpoint_bytes(),
        restored.domain_checkpoint_bytes()
    );
}

#[test]
fn c02_t01c_cursor_continuity() {
    assert_eq!(
        Simulation::scheduler(100, 0, [true, true]),
        Err(FixtureError::InvalidDueTick)
    );
    let mut sim = scheduler();
    let first = mint(1, ActorId::ONE, 0);
    let output = sim.drive(0, &[first]).unwrap();
    assert_eq!(output.events()[0].canonical_bytes(), decode(MINT_1));
    assert_eq!(sim.event_digest(), digest_of(MINT_1_DIGEST));
    assert_eq!(sim.marker_ids().collect::<Vec<_>>(), [10]);
    assert_eq!(sim.marker_mode(10), Some(ProcessingMode::Active));
    assert_eq!(sim.next_record_id(), 11);
    assert_blob(&sim, MINT_SAVE);
    let mut restored = Simulation::restore_domain_checkpoint(&decode(MINT_SAVE)).unwrap();
    let second = mint(2, ActorId::ONE, 0);
    let output = restored.drive(1, &[second]).unwrap();
    match output.events()[0] {
        DomainEvent::SchedulerCommand(event) => {
            assert_eq!(event.key(), second.key());
            assert_eq!(
                event.outcome(),
                SchedulerCommandOutcome::RecordMinted {
                    marker_id: 11,
                    mode_at_mint: ProcessingMode::Active,
                }
            );
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(output.events()[0].canonical_bytes(), decode(MINT_2));
    assert_eq!(restored.event_digest(), digest_of(MINT_2_DIGEST));
    assert_eq!(restored.next_record_id(), 12);
    assert_eq!(restored.marker_ids().collect::<Vec<_>>(), [10, 11]);
    assert_eq!(restored.location_ids().collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(restored.cargo_owner(4), Some(1));
    assert_eq!(restored.cargo_owner(5), Some(1));
    assert_eq!(
        restored.reservation_state(8),
        Some(ReservationState::Active)
    );
    assert_eq!(
        restored.reservation_state(9),
        Some(ReservationState::Active)
    );
    assert!(!restored.task_view(6).unwrap().completed);
    assert_eq!(restored.task_view(6).unwrap().due_tick, 3);
    assert!(!restored.task_view(7).unwrap().completed);
    assert_blob(&restored, MINT_FINAL);
    sim.drive(1, &[second]).unwrap();
    assert_eq!(
        sim.domain_checkpoint_bytes(),
        restored.domain_checkpoint_bytes()
    );
}

#[test]
fn c02_t01d_failure_matrix() {
    let save = decode(T01_SAVE);
    let initial = decode(super::golden::INITIAL.bytes);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&[]),
        Err(RestoreError::Truncated)
    );
    assert_eq!(
        Simulation::restore_domain_checkpoint(&save[..10]),
        Err(RestoreError::Truncated)
    );
    assert_eq!(
        Simulation::restore_domain_checkpoint(&initial),
        Err(RestoreError::UnsupportedFormat)
    );
    assert_eq!(
        Simulation::restore_domain_checkpoint(&decode(C01_T01_IMAGE)),
        Err(RestoreError::UnsupportedFormat)
    );

    let mut revision = save.clone();
    revision[12] = 2;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&revision),
        Err(RestoreError::UnsupportedRevision { found: 2 })
    );
    let mut tag = save.clone();
    tag[0] ^= 0xff;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&tag),
        Err(RestoreError::UnsupportedFormat)
    );
    let mut flipped = save.clone();
    flipped[14] ^= 0xff;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&flipped),
        Err(RestoreError::ChecksumMismatch)
    );
    let mut trailing = save.clone();
    trailing.push(0);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&trailing),
        Err(RestoreError::ChecksumMismatch)
    );

    let mut leftover = body_of(&save).to_vec();
    leftover.push(0);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&leftover)),
        Err(RestoreError::Corrupt)
    );
    let mut mode = body_of(&save).to_vec();
    assert_eq!(mode[95], 1);
    mode[95] = 9;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&mode)),
        Err(RestoreError::Corrupt)
    );
    let mut unknown = body_of(&save).to_vec();
    unknown[14] = 99;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&unknown)),
        Err(RestoreError::UnknownFixture { found: 99 })
    );
    let mut hash = body_of(&save).to_vec();
    hash[18..50].fill(0);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&hash)),
        Err(RestoreError::ContentMismatch)
    );

    let tick1 = decode(SCHED_1);
    let mut unsorted = body_of(&tick1).to_vec();
    assert_eq!(&unsorted[103..111], &4u64.to_le_bytes());
    let first = unsorted[111..137].to_vec();
    let second = unsorted[137..163].to_vec();
    unsorted[111..137].copy_from_slice(&second);
    unsorted[137..163].copy_from_slice(&first);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&unsorted)),
        Err(RestoreError::InvalidState)
    );

    let mut overdue = Simulation::restore_domain_checkpoint(&decode(SCHED_0)).unwrap();
    overdue.tasks.get_mut(&6).unwrap().due_tick = 0;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&overdue.domain_checkpoint_bytes()),
        Err(RestoreError::InvalidState)
    );
    let mut dangling = Simulation::restore_domain_checkpoint(&decode(SCHED_0)).unwrap();
    dangling.cargo.insert(4, 99);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&dangling.domain_checkpoint_bytes()),
        Err(RestoreError::InvalidState)
    );
    let mut cursor = body_of(&decode(SCHED_0)).to_vec();
    cursor[74..82].copy_from_slice(&9u64.to_le_bytes());
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&cursor)),
        Err(RestoreError::InvalidState)
    );
    assert!(Simulation::restore_domain_checkpoint(&save).is_ok());
}

#[test]
fn c02_t01f_fixture_invariants() {
    let mut scheduled = body_of(&decode(T01_SAVE)).to_vec();
    scheduled[95] = 2;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&scheduled)),
        Err(RestoreError::InvalidState)
    );
    let mut cursor = body_of(&decode(T01_SAVE)).to_vec();
    cursor[74..82].copy_from_slice(&2u64.to_le_bytes());
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&cursor)),
        Err(RestoreError::InvalidState)
    );

    let mut foreign = Simulation::restore_domain_checkpoint(&decode(T01_SAVE)).unwrap();
    let key = foreign.pending_commands().next().unwrap().key();
    foreign.pending.insert(key, Payload::MintRecord);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&foreign.domain_checkpoint_bytes()),
        Err(RestoreError::InvalidState)
    );

    for (stream_id, draw_count) in [(3, 1), (1, 3)] {
        let mut probe = Simulation::restore_domain_checkpoint(&decode(PROBE_SAVE)).unwrap();
        probe.pending.insert(
            CommandKey {
                target_tick: 2,
                actor_id: ActorId::ONE,
                producer_id: 1,
                sequence: 0,
            },
            Payload::Probe {
                stream_id,
                draw_count,
            },
        );
        assert_eq!(
            Simulation::restore_domain_checkpoint(&probe.domain_checkpoint_bytes()),
            Err(RestoreError::InvalidState)
        );
    }

    let mut event_cursor = body_of(&decode(T01_SAVE)).to_vec();
    let digest = digest_of(super::golden::H2);
    let digest_at = event_cursor
        .windows(32)
        .position(|window| window == digest)
        .unwrap();
    event_cursor[digest_at - 8..digest_at].copy_from_slice(&0u64.to_le_bytes());
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&event_cursor)),
        Err(RestoreError::InvalidState)
    );

    let mut disabled_owner = body_of(&decode(T01_SAVE)).to_vec();
    assert_eq!(disabled_owner[94], 1);
    disabled_owner[94] = 0;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&disabled_owner)),
        Err(RestoreError::InvalidState)
    );

    let mut occupied = body_of(&decode(SCHED_0)).to_vec();
    assert_eq!(occupied[102], 0);
    occupied[102] = 1;
    occupied.splice(103..103, [1, 0, 0, 0]);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&occupied)),
        Err(RestoreError::InvalidState)
    );
    let mut drawn = body_of(&decode(SCHED_0)).to_vec();
    let ordinal_at = drawn
        .windows(13)
        .position(|window| window[0] == 1 && window[1] == 1)
        .unwrap();
    drawn[ordinal_at + 5] = 1;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&drawn)),
        Err(RestoreError::InvalidState)
    );

    let mut claim = body_of(&decode(SCHED_1)).to_vec();
    let tag_at = 111 + 24;
    assert_eq!(claim[tag_at], 3);
    claim[tag_at] = 1;
    claim.remove(tag_at + 1);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&claim)),
        Err(RestoreError::InvalidState)
    );

    let mut split = Simulation::restore_domain_checkpoint(&decode(SCHED_0)).unwrap();
    let task = split.tasks.get_mut(&6).unwrap();
    task.completed = true;
    task.mode_at_completion = 1;
    split.cargo.insert(4, 2);
    split.reservations.get_mut(&8).unwrap().state = ReservationState::Released;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&split.domain_checkpoint_bytes()),
        Err(RestoreError::InvalidState)
    );
    let mut dues = Simulation::restore_domain_checkpoint(&decode(SCHED_0)).unwrap();
    dues.tasks.get_mut(&7).unwrap().due_tick = 4;
    assert_eq!(
        Simulation::restore_domain_checkpoint(&dues.domain_checkpoint_bytes()),
        Err(RestoreError::InvalidState)
    );
    let mut sched_cursor = body_of(&decode(SCHED_0)).to_vec();
    let digest_at = sched_cursor
        .windows(32)
        .position(|window| window == digest_of(F2_H0))
        .unwrap();
    sched_cursor[digest_at - 8..digest_at].fill(0);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&sched_cursor)),
        Err(RestoreError::InvalidState)
    );
    let mut missing = Simulation::restore_domain_checkpoint(&decode(SCHED_0)).unwrap();
    missing.cargo.clear();
    assert_eq!(
        Simulation::restore_domain_checkpoint(&missing.domain_checkpoint_bytes()),
        Err(RestoreError::InvalidState)
    );

    for vector in [
        T01_SAVE,
        T01_FINAL,
        PROBE_SAVE,
        PROBE_FINAL,
        MINT_SAVE,
        MINT_FINAL,
        SCHED_0,
        SCHED_1,
        SCHED_2,
        SCHED_3,
        SCHED_4,
        WITNESS_1,
        WITNESS_2,
        DISABLED_MINT,
    ] {
        let bytes = decode(vector);
        let restored = Simulation::restore_domain_checkpoint(&bytes).unwrap();
        assert_eq!(restored.domain_checkpoint_bytes(), bytes);
    }
}

#[test]
fn c02_t01g_bounded_counts() {
    let blob = decode(SCHED_0);
    let body = body_of(&blob);
    assert_eq!(&body[82..90], &2u64.to_le_bytes());
    let mut actors = body.to_vec();
    actors[82..90].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&actors)),
        Err(RestoreError::Corrupt)
    );

    let mut pending = body.to_vec();
    assert_eq!(&pending[103..111], &0u64.to_le_bytes());
    pending[103..111].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&pending)),
        Err(RestoreError::Corrupt)
    );

    let locations = body
        .windows(32)
        .position(|window| {
            window[..8] == 3u64.to_le_bytes()
                && window[8..16] == 1u64.to_le_bytes()
                && window[16..24] == 2u64.to_le_bytes()
                && window[24..32] == 3u64.to_le_bytes()
        })
        .unwrap();
    let cargo_at = locations + 32;
    assert_eq!(&body[cargo_at..cargo_at + 8], &2u64.to_le_bytes());
    let mut cargo = body.to_vec();
    cargo[cargo_at..cargo_at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&cargo)),
        Err(RestoreError::Corrupt)
    );
    let remaining = body.len() - (cargo_at + 8);
    let mut overflow = body.to_vec();
    overflow[cargo_at..cargo_at + 8].copy_from_slice(&(remaining as u64 + 1).to_le_bytes());
    assert_eq!(
        Simulation::restore_domain_checkpoint(&reseal(&overflow)),
        Err(RestoreError::Corrupt)
    );
    assert!(Simulation::restore_domain_checkpoint(&blob).is_ok());
}

#[test]
fn c02_t01e_file_round_trip() {
    let mut sim = Simulation::new(FixtureConfig {
        seed: 100,
        actor_enabled: [true, true],
    });
    sim.step(
        0,
        &[claim_at(2, ActorId::TWO, 0), claim_at(2, ActorId::ONE, 0)],
    )
    .unwrap();
    sim.step(1, &[claim_at(3, ActorId::TWO, 1)]).unwrap();
    let directory = std::env::temp_dir().join(format!("sealed-c02-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("tick2.bin");
    sim.write_domain_checkpoint(&path).unwrap();
    let file = std::fs::read(&path).unwrap();
    assert_eq!(file, sim.domain_checkpoint_bytes());
    let mut loaded = Simulation::read_domain_checkpoint(&path).unwrap();
    loaded.step(2, &[]).unwrap();
    assert_blob(&loaded, T01_FINAL);

    let short = directory.join("short.bin");
    std::fs::write(&short, &file[..10]).unwrap();
    match Simulation::read_domain_checkpoint(&short) {
        Err(CheckpointReadError::Restore(RestoreError::Truncated)) => {}
        other => panic!("unexpected {other:?}"),
    }
    match Simulation::read_domain_checkpoint(directory.join("missing.bin")) {
        Err(CheckpointReadError::Io(_)) => {}
        other => panic!("unexpected {other:?}"),
    }
    let before = sim.domain_checkpoint_bytes();
    assert!(sim.write_domain_checkpoint(&directory).is_err());
    assert_eq!(sim.domain_checkpoint_bytes(), before);
    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn c02_t02a_primary_schedule() {
    let mut sim = scheduler();
    assert_blob(&sim, SCHED_0);
    assert_eq!(
        sim.processing_mode(ActorId::ONE),
        Some(ProcessingMode::Active)
    );
    assert_eq!(sim.cargo_owner(4), Some(1));
    assert_eq!(sim.cargo_owner(5), Some(1));
    assert_eq!(sim.reservation_state(8), Some(ReservationState::Active));
    assert_eq!(sim.reservation_state(9), Some(ReservationState::Active));
    assert!(!sim.task_view(6).unwrap().completed);
    assert!(!sim.task_view(7).unwrap().completed);
    assert_eq!(sim.next_record_id(), 10);
    assert_eq!(sim.event_digest(), digest_of(F2_H0));

    assert!(sim.drive(0, &primary_batch()).unwrap().events().is_empty());
    assert_eq!(sim.tick(), 1);
    assert_blob(&sim, SCHED_1);
    assert!(sim.drive(1, &[]).unwrap().events().is_empty());
    assert_eq!(sim.tick(), 2);
    assert_eq!(
        sim.processing_mode(ActorId::ONE),
        Some(ProcessingMode::Scheduled)
    );
    assert_eq!(
        sim.processing_mode(ActorId::TWO),
        Some(ProcessingMode::Scheduled)
    );
    assert_eq!(sim.cargo_owner(4), Some(1));
    assert_eq!(sim.pending_commands().count(), 2);
    assert_blob(&sim, SCHED_2);
    let saved = sim.domain_checkpoint_bytes();
    let completed = sim.drive(2, &[]).unwrap();
    assert_eq!(completed.events().len(), 2);
    assert_eq!(completed.events()[0].canonical_bytes(), decode(TASK_1));
    assert_eq!(completed.events()[1].canonical_bytes(), decode(TASK_2));
    assert_eq!(
        chain(digest_of(F2_H0), &decode(TASK_1)),
        digest_of(TASK_1_DIGEST)
    );
    assert_eq!(sim.event_digest(), digest_of(TASK_2_DIGEST));
    match completed.events()[0] {
        DomainEvent::Generated(event) => {
            assert_eq!(event.actor_id(), ActorId::ONE);
            assert_eq!(event.tick(), 3);
            assert!(matches!(
                event.outcome(),
                GeneratedOutcome::TaskCompleted { task_id: 6, .. }
            ));
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(sim.cargo_owner(4), Some(2));
    assert_eq!(sim.cargo_owner(5), Some(3));
    assert_eq!(sim.reservation_state(8), Some(ReservationState::Released));
    assert_eq!(
        sim.task_view(6).unwrap().mode_at_completion,
        Some(ProcessingMode::Scheduled)
    );
    assert_eq!(sim.cargo_ids().count(), 2);
    assert_blob(&sim, SCHED_3);
    assert!(sim.drive(3, &[]).unwrap().events().is_empty());
    assert_eq!(
        sim.processing_mode(ActorId::ONE),
        Some(ProcessingMode::Active)
    );
    assert_eq!(
        sim.processing_mode(ActorId::TWO),
        Some(ProcessingMode::Active)
    );
    assert_blob(&sim, SCHED_4);
    let digest = sim.event_digest();
    let cursor = sim.next_event_id();
    let owners = (sim.cargo_owner(4), sim.cargo_owner(5));
    assert!(sim.drive(4, &[]).unwrap().events().is_empty());
    assert_eq!(sim.tick(), 5);
    assert_eq!(sim.event_digest(), digest);
    assert_eq!(sim.next_event_id(), cursor);
    assert_eq!((sim.cargo_owner(4), sim.cargo_owner(5)), owners);
    assert_eq!(sim.next_actor_id(), 3);
    assert_eq!(sim.next_record_id(), 10);
    assert_eq!(sim.slot_owner(), None);
    assert_eq!(sim.next_ordinal(1, ActorId::ONE), Some(0));

    let bad = claim_at(6, ActorId::ONE, 0);
    let before = sim.domain_checkpoint_bytes();
    assert_eq!(
        sim.drive(5, &[bad]),
        Err(StepError::Command {
            key: bad.key(),
            reason: InvalidCommand::InvalidPayload,
        })
    );
    assert_eq!(sim.domain_checkpoint_bytes(), before);

    let mut resumed = Simulation::restore_domain_checkpoint(&saved).unwrap();
    let before_step = resumed.domain_checkpoint_bytes();
    assert_eq!(resumed.step(2, &[]), Err(StepError::UseDrive));
    assert_eq!(resumed.domain_checkpoint_bytes(), before_step);
    let suffix = resumed.drive(2, &[]).unwrap();
    assert_eq!(suffix.events()[0].canonical_bytes(), decode(TASK_1));
    assert!(resumed.drive(3, &[]).unwrap().events().is_empty());
    assert_blob(&resumed, SCHED_4);
}

#[test]
fn c02_t02b_mode_before_commands_and_tasks() {
    let mut sim = Simulation::scheduler(100, 2, [true, true]).unwrap();
    let mint_command = mint(2, ActorId::ONE, 0);
    let mode_one = set_mode(2, ActorId::ONE, 1, ProcessingMode::Scheduled);
    let mode_two = set_mode(2, ActorId::TWO, 0, ProcessingMode::Scheduled);
    assert!(mint_command.key() < mode_one.key());
    assert!(
        sim.drive(0, &[mode_two, mint_command, mode_one])
            .unwrap()
            .events()
            .is_empty()
    );
    assert_blob(&sim, WITNESS_1);
    assert_eq!(
        sim.processing_mode(ActorId::ONE),
        Some(ProcessingMode::Active)
    );
    let output = sim.drive(1, &[]).unwrap();
    assert_blob(&sim, WITNESS_2);
    assert_eq!(output.events().len(), 3);
    assert_eq!(output.events()[0].canonical_bytes(), decode(WITNESS_MINT));
    assert_eq!(output.events()[1].canonical_bytes(), decode(WITNESS_TASK_1));
    assert_eq!(output.events()[2].canonical_bytes(), decode(WITNESS_TASK_2));
    assert_eq!(sim.event_digest(), digest_of(WITNESS_FINAL_DIGEST));
    match output.events()[0] {
        DomainEvent::SchedulerCommand(event) => assert_eq!(
            event.outcome(),
            SchedulerCommandOutcome::RecordMinted {
                marker_id: 10,
                mode_at_mint: ProcessingMode::Scheduled,
            }
        ),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        sim.task_view(6).unwrap().mode_at_completion,
        Some(ProcessingMode::Scheduled)
    );
    assert_eq!(
        sim.task_view(7).unwrap().mode_at_completion,
        Some(ProcessingMode::Scheduled)
    );
    assert_eq!(sim.marker_mode(10), Some(ProcessingMode::Scheduled));
    assert_eq!(
        sim.processing_mode(ActorId::TWO),
        Some(ProcessingMode::Scheduled)
    );
    assert_eq!(sim.cargo_owner(4), Some(2));
    assert_eq!(sim.cargo_owner(5), Some(3));
    assert_eq!(sim.reservation_state(8), Some(ReservationState::Released));
    assert_eq!(sim.reservation_state(9), Some(ReservationState::Released));
    assert_eq!(sim.next_record_id(), 11);
    assert_eq!(sim.next_event_id(), 4);
    assert_eq!(
        chain(digest_of(F2_H0), &decode(WITNESS_MINT)),
        digest_of(WITNESS_MINT_DIGEST)
    );
}

#[test]
fn c02_t02c_disabled_mint() {
    let mut sim = Simulation::scheduler(100, 3, [false, true]).unwrap();
    let output = sim.drive(0, &[mint(1, ActorId::ONE, 0)]).unwrap();
    assert_eq!(output.events()[0].canonical_bytes(), decode(DISABLED_EVENT));
    match output.events()[0] {
        DomainEvent::SchedulerCommand(event) => assert_eq!(
            event.outcome(),
            SchedulerCommandOutcome::Rejected(SchedulerCommandRejection::DisabledActor)
        ),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(sim.event_digest(), digest_of(DISABLED_DIGEST));
    assert_blob(&sim, DISABLED_MINT);
    assert_eq!(sim.next_record_id(), 10);
    assert_eq!(sim.marker_ids().count(), 0);
    assert_eq!(sim.cargo_owner(4), Some(1));
    assert!(!sim.task_view(6).unwrap().completed);
}

#[test]
fn c02_t02d_grouped_overdue_and_overflow() {
    let mut immediate = scheduler();
    let mut grouped = scheduler();
    let batch = primary_batch();
    let batches: Vec<&[Command]> = vec![&batch, &[], &[], &[]];
    let mut immediate_log = Vec::new();
    let mut snapshots = Vec::new();
    for (boundary, commands) in batches.iter().enumerate() {
        immediate_log.extend(canonical(
            &immediate.drive(boundary as u64, commands).unwrap(),
        ));
        snapshots.push(immediate.domain_checkpoint_bytes());
    }
    let mut grouped_log = Vec::new();
    for (boundary, commands) in batches.iter().enumerate() {
        grouped_log.extend(canonical(
            &grouped.drive(boundary as u64, commands).unwrap(),
        ));
        assert_eq!(grouped.domain_checkpoint_bytes(), snapshots[boundary]);
    }
    assert_eq!(grouped_log, immediate_log);
    assert_eq!(snapshots[1], decode(SCHED_2));
    assert_eq!(immediate.domain_checkpoint_bytes(), decode(SCHED_4));

    let mut overdue = scheduler();
    overdue.tasks.get_mut(&6).unwrap().due_tick = 0;
    let before = overdue.domain_checkpoint_bytes();
    assert_eq!(overdue.drive(0, &[]), Err(StepError::InvalidState));
    assert_eq!(overdue.domain_checkpoint_bytes(), before);
    assert_eq!(
        Simulation::restore_domain_checkpoint(&before),
        Err(RestoreError::InvalidState)
    );

    let mut overflow = scheduler();
    overflow.next_record_id = u64::MAX;
    let before = overflow.domain_checkpoint_bytes();
    assert_eq!(
        overflow.drive(0, &[mint(1, ActorId::ONE, 0)]),
        Err(StepError::CapacityExceeded)
    );
    assert_eq!(overflow.domain_checkpoint_bytes(), before);
    assert_eq!(overflow.next_record_id(), u64::MAX);
    let unchanged = overflow.domain_checkpoint_bytes();
    assert_eq!(overflow.step(0, &[]), Err(StepError::UseDrive));
    assert_eq!(overflow.domain_checkpoint_bytes(), unchanged);
}

fn chain(previous: [u8; 32], event: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"SH:C02:EVENT:v1");
    hash.update(previous);
    hash.update(event);
    hash.finalize().into()
}

fn canonical(output: &DriveOutput) -> Vec<u8> {
    output
        .events()
        .iter()
        .flat_map(|event| event.canonical_bytes())
        .collect()
}
