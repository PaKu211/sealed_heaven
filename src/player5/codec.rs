//! C05 rev1, little endian, no padding: magic[12], revision:u16, content[32],
//! tick:u64, player_id:u32, x/y:u32, three (owner:u8,id:u32,quantity:u64),
//! requested/completed:u8, sequence/event cursors:u64, event digest[32],
//! registry_count:u32, sorted records (key[24], intent[5], outcome[2]),
//! C03_length:u32, exact C03 checkpoint bytes, SHA256 of all preceding bytes.
//! Owner tags: source=1/id1, player=2/id21, designated shrine=3/id1.

use super::*;

const MAGIC: &[u8; 12] = b"SH:C05:STATE";
const RECORD_SIZE: usize = 31;
const MAX_BYTES: usize = MAX_NPC_BYTES + MAX_COMMANDS * RECORD_SIZE + 256;

fn content_hash() -> [u8; 32] {
    let mut content = b"SH:C05:CONTENT:v1;12x9;player21@0,4;wall6:1..6/gate4;patch9,2;shrine3,5;healer1;herbs3;rangeManhattan<=1;NPC1=3,3/2=9,5/9=10,7;grid=3+x,3+y;NPCfirst;oneAction;producer1;registry4096;batch4096;owners=source1,player21,shrine1;ledgerSeparate;repeatTalk;deliverOnce;sequenceHighwater;eventChain:v1".to_vec();
    content.extend(b";completeNPCAndPlayerReplay;maxTicks:u64le=");
    content.extend(MAX_TICKS.to_le_bytes());
    content.extend(sha256(DEMO));
    sha256(&content)
}

pub(super) fn encode(world: &World) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(content_hash());
    bytes.extend(world.tick().to_le_bytes());
    bytes.extend(PLAYER_ID.to_le_bytes());
    bytes.extend(world.position.x.to_le_bytes());
    bytes.extend(world.position.y.to_le_bytes());
    for holding in world.herbs {
        let (tag, id): (u8, u32) = match holding.owner {
            HerbOwner::Source => (1, 1),
            HerbOwner::Player(id) => (2, id),
            HerbOwner::DesignatedShrine => (3, 1),
        };
        bytes.push(tag);
        bytes.extend(id.to_le_bytes());
        bytes.extend(holding.quantity.to_le_bytes());
    }
    bytes.extend([u8::from(world.requested), u8::from(world.completed)]);
    bytes.extend(world.next_sequence.to_le_bytes());
    bytes.extend(world.next_event_id.to_le_bytes());
    bytes.extend(world.digest);
    bytes.extend((world.registry.len() as u32).to_le_bytes());
    for (&key, record) in &world.registry {
        put_key(&mut bytes, key);
        put_intent(&mut bytes, record.intent);
        put_outcome(&mut bytes, record.outcome);
    }
    let npcs = world.npcs.domain_checkpoint_bytes();
    bytes.extend((npcs.len() as u32).to_le_bytes());
    bytes.extend(npcs);
    bytes.extend(sha256(&bytes));
    bytes
}

pub(super) fn decode(bytes: &[u8]) -> Result<World, Error> {
    decode_inner(bytes).map_err(Error::Restore)
}

fn decode_inner(bytes: &[u8]) -> Result<World, RestoreError> {
    if bytes.len() > MAX_BYTES {
        return Err(RestoreError::Corrupt);
    }
    let mut r = Reader { bytes, at: 0 };
    if r.take(12)? != MAGIC {
        return Err(RestoreError::UnsupportedFormat);
    }
    let revision = r.u16()?;
    if revision != 1 {
        return Err(RestoreError::UnsupportedRevision { found: revision });
    }
    if r.take(32)? != content_hash() {
        return Err(RestoreError::ContentMismatch);
    }
    let tick = r.u64()?;
    // Reject hostile u64 clocks before parsing the NPC candidate or doing any
    // replay. Drive enforces the same horizon, so valid saves remain loadable.
    if tick > MAX_TICKS {
        return Err(RestoreError::InvalidState);
    }
    if r.u32()? != PLAYER_ID {
        return Err(RestoreError::InvalidState);
    }
    let position = Position {
        x: r.u32()?,
        y: r.u32()?,
    };
    let mut herbs = World::settlement().herbs;
    for holding in &mut herbs {
        let tag = r.u8()?;
        let id = r.u32()?;
        let owner = match (tag, id) {
            (1, 1) => HerbOwner::Source,
            (2, PLAYER_ID) => HerbOwner::Player(PLAYER_ID),
            (3, 1) => HerbOwner::DesignatedShrine,
            _ => return Err(RestoreError::InvalidState),
        };
        if owner != holding.owner {
            return Err(RestoreError::InvalidState);
        }
        holding.quantity = r.u64()?;
    }
    let requested = r.boolean()?;
    let completed = r.boolean()?;
    let next_sequence = r.u64()?;
    let next_event_id = r.u64()?;
    let digest = r.array()?;
    let count = r.u32()? as usize;
    if count > MAX_COMMANDS || count > r.remaining() / RECORD_SIZE {
        return Err(RestoreError::Corrupt);
    }
    let mut registry = BTreeMap::new();
    let mut previous = None;
    for _ in 0..count {
        let key = r.key()?;
        if previous.is_some_and(|old| old >= key) {
            return Err(RestoreError::InvalidState);
        }
        previous = Some(key);
        let record = Record {
            intent: r.intent()?,
            outcome: r.outcome()?,
        };
        registry.insert(key, record);
    }
    let npc_length = r.u32()? as usize;
    if npc_length > MAX_NPC_BYTES {
        return Err(RestoreError::Corrupt);
    }
    let npc_bytes = r.take(npc_length)?;
    let checksum_at = r.at;
    let checksum: [u8; 32] = r.array()?;
    if r.remaining() != 0 {
        return Err(RestoreError::Corrupt);
    }
    if sha256(&bytes[..checksum_at]) != checksum {
        return Err(RestoreError::ChecksumMismatch);
    }
    if npc_bytes.get(..12) != Some(b"SH:C03:STATE") {
        return Err(RestoreError::UnsupportedFormat);
    }
    let npcs = Simulation::restore_domain_checkpoint(npc_bytes)?;
    if npcs.tick() != tick || npcs.domain_checkpoint_bytes() != npc_bytes {
        return Err(RestoreError::InvalidState);
    }
    let candidate = World {
        npcs,
        position,
        herbs,
        requested,
        completed,
        registry,
        next_sequence,
        next_event_id,
        digest,
    };
    validate(&candidate)?;
    Ok(candidate)
}

/// Replay the complete bounded history from the authored initial state. Only the
/// unchanged NPC kernel supplies historical positions; `apply` checks the exact
/// same live eligibility predicates for both successful and rejected actions.
/// No recorded outcome, final NPC state, cursor, or digest is trusted or repaired.
fn validate(world: &World) -> Result<(), RestoreError> {
    let invalid = RestoreError::InvalidState;
    if world.tick() > MAX_TICKS {
        return Err(invalid);
    }
    let mut replay = World::settlement();
    let mut previous_tick = 0;
    for (&key, record) in &world.registry {
        if key.actor_id != PLAYER_ID
            || key.producer_id != 1
            || key.target_tick <= previous_tick
            || key.target_tick > world.tick()
            || key.sequence < replay.next_sequence
        {
            return Err(invalid);
        }
        previous_tick = key.target_tick;
        let next_sequence = key.sequence.checked_add(1).ok_or(invalid)?;
        advance_npcs(&mut replay, key.target_tick)?;
        let outcome = replay.apply(record.intent);
        if outcome != record.outcome {
            return Err(invalid);
        }
        let event = Event {
            id: replay.next_event_id,
            tick: key.target_tick,
            key,
            outcome,
        };
        replay.next_event_id = replay.next_event_id.checked_add(1).ok_or(invalid)?;
        replay.next_sequence = next_sequence;
        replay.digest = extend_digest(replay.digest, event);
        replay.registry.insert(
            key,
            Record {
                intent: record.intent,
                outcome,
            },
        );
    }
    // Also replay empty/retry-only boundaries after the final recorded command.
    advance_npcs(&mut replay, world.tick())?;
    if !same_npc_history_except_exhausted_event_cursor(
        &world.npcs.domain_checkpoint_bytes(),
        &replay.npcs.domain_checkpoint_bytes(),
    ) || world.registry != replay.registry
        || world.position != replay.position
        || world.herbs != replay.herbs
        || world.requested != replay.requested
        || world.completed != replay.completed
        || world.next_sequence != replay.next_sequence
        || world.next_event_id != replay.next_event_id
        || world.digest != replay.digest
        || !walkable(world.position)
        || world
            .source_herbs()
            .checked_add(world.player_herbs())
            .and_then(|n| n.checked_add(world.delivered_herbs()))
            != Some(3)
    {
        return Err(invalid);
    }
    Ok(())
}

/// The frozen C03 event cursor is a durable checked counter whose exhaustion is
/// a supported atomic-drive failure state. All other NPC bytes, including its
/// event digest, must match exact replay; permit only this cursor to differ.
fn same_npc_history_except_exhausted_event_cursor(saved: &[u8], replayed: &[u8]) -> bool {
    const TRAILER: usize = 32;
    const DIGEST: usize = 32;
    const EVENT_CURSOR: usize = 8;
    let Some(cursor_start) = saved.len().checked_sub(TRAILER + DIGEST + EVENT_CURSOR) else {
        return false;
    };
    if saved.len() != replayed.len() || cursor_start + EVENT_CURSOR > saved.len() - TRAILER {
        return false;
    }
    let Some(cursor_bytes) = saved.get(cursor_start..cursor_start + EVENT_CURSOR) else {
        return false;
    };
    if u64::from_le_bytes(cursor_bytes.try_into().expect("fixed-width event cursor")) != u64::MAX {
        return saved == replayed;
    }
    let digest_start = cursor_start + EVENT_CURSOR;
    let digest_end = saved.len() - TRAILER;
    saved[..cursor_start] == replayed[..cursor_start]
        && saved[digest_start..digest_end] == replayed[digest_start..digest_end]
}

/// Call the frozen NPC drive directly: no recursive restore/World::drive and no
/// per-tick checkpoint serialization. Across the entire candidate this loop can
/// execute at most MAX_TICKS times, not MAX_TICKS times per command.
fn advance_npcs(replay: &mut World, target_tick: u64) -> Result<(), RestoreError> {
    while replay.tick() < target_tick {
        replay
            .npcs
            .drive(replay.tick(), &[])
            .map_err(|_| RestoreError::InvalidState)?;
    }
    Ok(())
}

pub(super) fn put_key(bytes: &mut Vec<u8>, key: Key) {
    bytes.extend(key.target_tick.to_le_bytes());
    bytes.extend(key.actor_id.to_le_bytes());
    bytes.extend(key.producer_id.to_le_bytes());
    bytes.extend(key.sequence.to_le_bytes());
}

fn put_intent(bytes: &mut Vec<u8>, intent: Intent) {
    let (tag, argument): (u8, u32) = match intent {
        Intent::Move(Direction::North) => (1, 1),
        Intent::Move(Direction::East) => (1, 2),
        Intent::Move(Direction::South) => (1, 3),
        Intent::Move(Direction::West) => (1, 4),
        Intent::Talk(id) => (2, id),
        Intent::Gather => (3, 0),
        Intent::Deliver => (4, 0),
    };
    bytes.push(tag);
    bytes.extend(argument.to_le_bytes());
}

pub(super) fn put_outcome(bytes: &mut Vec<u8>, outcome: Outcome) {
    let (tag, reason) = match outcome {
        Outcome::Moved => (1, 0),
        Outcome::Requested => (2, 0),
        Outcome::Gathered => (3, 0),
        Outcome::Delivered => (4, 0),
        Outcome::Rejected(reason) => (
            5,
            match reason {
                Rejection::OutOfBounds => 1,
                Rejection::Terrain => 2,
                Rejection::Occupied => 3,
                Rejection::OutOfRange => 4,
                Rejection::UnknownTarget => 5,
                Rejection::RequestNotAccepted => 6,
                Rejection::Empty => 7,
                Rejection::AlreadyCompleted => 8,
                Rejection::Overflow => 9,
            },
        ),
    };
    bytes.extend([tag, reason]);
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], RestoreError> {
        let end = self.at.checked_add(n).ok_or(RestoreError::Corrupt)?;
        let result = self
            .bytes
            .get(self.at..end)
            .ok_or(RestoreError::Truncated)?;
        self.at = end;
        Ok(result)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], RestoreError> {
        self.take(N)?
            .try_into()
            .map_err(|_| RestoreError::Truncated)
    }
    fn u8(&mut self) -> Result<u8, RestoreError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, RestoreError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, RestoreError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, RestoreError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn boolean(&mut self) -> Result<bool, RestoreError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(RestoreError::Corrupt),
        }
    }
    fn key(&mut self) -> Result<Key, RestoreError> {
        Ok(Key {
            target_tick: self.u64()?,
            actor_id: self.u32()?,
            producer_id: self.u32()?,
            sequence: self.u64()?,
        })
    }
    fn intent(&mut self) -> Result<Intent, RestoreError> {
        let tag = self.u8()?;
        let argument = self.u32()?;
        match (tag, argument) {
            (1, 1) => Ok(Intent::Move(Direction::North)),
            (1, 2) => Ok(Intent::Move(Direction::East)),
            (1, 3) => Ok(Intent::Move(Direction::South)),
            (1, 4) => Ok(Intent::Move(Direction::West)),
            (2, id) => Ok(Intent::Talk(id)),
            (3, 0) => Ok(Intent::Gather),
            (4, 0) => Ok(Intent::Deliver),
            _ => Err(RestoreError::Corrupt),
        }
    }
    fn outcome(&mut self) -> Result<Outcome, RestoreError> {
        let tag = self.u8()?;
        let reason = self.u8()?;
        match (tag, reason) {
            (1, 0) => Ok(Outcome::Moved),
            (2, 0) => Ok(Outcome::Requested),
            (3, 0) => Ok(Outcome::Gathered),
            (4, 0) => Ok(Outcome::Delivered),
            (5, n) => Ok(Outcome::Rejected(match n {
                1 => Rejection::OutOfBounds,
                2 => Rejection::Terrain,
                3 => Rejection::Occupied,
                4 => Rejection::OutOfRange,
                5 => Rejection::UnknownTarget,
                6 => Rejection::RequestNotAccepted,
                7 => Rejection::Empty,
                8 => Rejection::AlreadyCompleted,
                9 => Rejection::Overflow,
                _ => return Err(RestoreError::Corrupt),
            })),
            _ => Err(RestoreError::Corrupt),
        }
    }
}
