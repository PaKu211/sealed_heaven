use sha2::{Digest, Sha256};

use crate::{ActorId, CommandKey, Event, FIXTURE_ID, Outcome, Payload, Rejection, Simulation};

pub(super) fn checkpoint_bytes(sim: &Simulation) -> Vec<u8> {
    let mut bytes = b"SH:C01:STATE".to_vec();
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(FIXTURE_ID.to_le_bytes());
    bytes.extend(Simulation::fixture_content_hash());
    bytes.extend(sim.tick.to_le_bytes());
    bytes.extend(sim.config.seed.to_le_bytes());

    let actors = [ActorId::ONE, ActorId::TWO];
    bytes.extend((actors.len() as u64).to_le_bytes());
    for (actor, enabled) in actors.into_iter().zip(sim.config.actor_enabled) {
        bytes.extend(actor.get().to_le_bytes());
        bytes.push(u8::from(enabled));
    }

    bytes.push(u8::from(sim.slot_owner.is_some()));
    if let Some(owner) = sim.slot_owner {
        bytes.extend(owner.get().to_le_bytes());
    }

    bytes.extend((sim.pending.len() as u64).to_le_bytes());
    for (&key, &payload) in &sim.pending {
        encode_key(&mut bytes, key);
        match payload {
            Payload::ClaimSlot => bytes.push(1),
            Payload::Probe {
                stream_id,
                draw_count,
            } => bytes.extend([2, stream_id, draw_count]),
            Payload::SetMode(_) | Payload::MintRecord => {
                panic!("checkpoint_bytes is the C0.1 observation")
            }
            Payload::SetEnabled(_)
            | Payload::ReserveStock { .. }
            | Payload::ReserveBundle { .. }
            | Payload::CancelReservation(_)
            | Payload::CompleteTask(_)
            | Payload::BeginTask { .. } => {
                panic!("checkpoint_bytes is the C0.1 observation")
            }
        }
    }

    bytes.extend(4u64.to_le_bytes());
    for (stream_id, ordinals) in (1u8..=2).zip(sim.ordinals) {
        for (actor, ordinal) in actors.into_iter().zip(ordinals) {
            bytes.push(stream_id);
            bytes.extend(actor.get().to_le_bytes());
            bytes.extend(ordinal.to_le_bytes());
        }
    }

    bytes.extend(sim.next_event_id.to_le_bytes());
    bytes.extend(sim.event_digest);
    bytes
}

pub(super) fn event_bytes(event: Event) -> Vec<u8> {
    let mut bytes = event.id.to_le_bytes().to_vec();
    encode_key(&mut bytes, event.key);
    match event.outcome {
        Outcome::Claimed => bytes.push(1),
        Outcome::Rejected(Rejection::DisabledActor) => bytes.extend([2, 1]),
        Outcome::Rejected(Rejection::OccupiedSlot) => bytes.extend([2, 2]),
        Outcome::Probed {
            stream_id,
            first_word,
            second_word,
        } => {
            bytes.extend([3, stream_id, 1 + u8::from(second_word.is_some())]);
            bytes.extend(first_word.to_le_bytes());
            if let Some(word) = second_word {
                bytes.extend(word.to_le_bytes());
            }
        }
    }
    bytes
}

fn encode_key(bytes: &mut Vec<u8>, key: CommandKey) {
    bytes.extend(key.target_tick.to_le_bytes());
    bytes.extend(key.actor_id.get().to_le_bytes());
    bytes.extend(key.producer_id.to_le_bytes());
    bytes.extend(key.sequence.to_le_bytes());
}

pub(super) fn probe_seed(seed: u64, stream_id: u8, actor: ActorId, ordinal: u64) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"SH:C01:RNG:v1");
    hash.update(seed.to_le_bytes());
    hash.update([stream_id]);
    hash.update(actor.get().to_le_bytes());
    hash.update(ordinal.to_le_bytes());
    hash.finalize().into()
}

pub(super) fn extend_event_digest(previous: [u8; 32], event: Event) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"SH:C01:EVENT:v1");
    hash.update(previous);
    hash.update(event_bytes(event));
    hash.finalize().into()
}

pub(super) fn extend_scheduler_digest(previous: [u8; 32], event: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"SH:C02:EVENT:v1");
    hash.update(previous);
    hash.update(event);
    hash.finalize().into()
}

pub(super) fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
