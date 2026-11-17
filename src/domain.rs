use std::collections::BTreeSet;

use crate::{
    ActorId, CommandKey, FixtureConfig, Payload, ProcessingMode, Reservation, ReservationState,
    RestoreError, Simulation, Task, encoding,
};

const TAG: &[u8] = b"SH:C02:STATE";

pub(super) fn domain_checkpoint_bytes(sim: &Simulation) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(TAG);
    body.extend(1u16.to_le_bytes());
    body.extend(sim.fixture_id.to_le_bytes());
    body.extend(content_hash(sim.fixture_id));
    body.extend(sim.tick.to_le_bytes());
    body.extend(sim.config.seed.to_le_bytes());
    body.extend(sim.next_actor_id.to_le_bytes());
    body.extend(sim.next_record_id.to_le_bytes());

    body.extend(2u64.to_le_bytes());
    for (actor, (enabled, mode)) in [ActorId::ONE, ActorId::TWO]
        .into_iter()
        .zip(sim.config.actor_enabled.into_iter().zip(sim.modes))
    {
        body.extend(actor.get().to_le_bytes());
        body.push(u8::from(enabled));
        body.push(mode_byte(mode));
    }

    match sim.slot_owner {
        Some(owner) => {
            body.push(1);
            body.extend(owner.get().to_le_bytes());
        }
        None => body.push(0),
    }

    body.extend((sim.pending.len() as u64).to_le_bytes());
    for (&key, &payload) in &sim.pending {
        encode_command(&mut body, key, payload);
    }

    body.extend(4u64.to_le_bytes());
    for (stream_id, ordinals) in (1u8..=2).zip(sim.ordinals) {
        for (actor, ordinal) in [ActorId::ONE, ActorId::TWO].into_iter().zip(ordinals) {
            body.push(stream_id);
            body.extend(actor.get().to_le_bytes());
            body.extend(ordinal.to_le_bytes());
        }
    }

    body.extend(sim.next_event_id.to_le_bytes());
    body.extend(sim.event_digest);

    body.extend((sim.locations.len() as u64).to_le_bytes());
    for id in &sim.locations {
        body.extend(id.to_le_bytes());
    }

    body.extend((sim.cargo.len() as u64).to_le_bytes());
    for (&id, &owner) in &sim.cargo {
        body.extend(id.to_le_bytes());
        body.extend(owner.to_le_bytes());
    }

    body.extend((sim.reservations.len() as u64).to_le_bytes());
    for (&id, reservation) in &sim.reservations {
        body.extend(id.to_le_bytes());
        body.extend(reservation.cargo_id.to_le_bytes());
        body.extend(reservation.actor_id.get().to_le_bytes());
        body.extend(reservation.task_id.to_le_bytes());
        body.push(match reservation.state {
            ReservationState::Active => 1,
            ReservationState::Released => 2,
        });
    }

    body.extend((sim.tasks.len() as u64).to_le_bytes());
    for (&id, task) in &sim.tasks {
        body.extend(id.to_le_bytes());
        body.extend(task.actor_id.get().to_le_bytes());
        body.extend(task.due_tick.to_le_bytes());
        body.extend(task.cargo_id.to_le_bytes());
        body.extend(task.reservation_id.to_le_bytes());
        body.extend(task.destination_id.to_le_bytes());
        body.push(u8::from(task.completed) + 1);
        body.push(task.mode_at_completion);
    }

    body.extend((sim.markers.len() as u64).to_le_bytes());
    for (&id, &mode) in &sim.markers {
        body.extend(id.to_le_bytes());
        body.push(mode_byte(mode));
    }

    let trailer = encoding::sha256(&body);
    body.extend(trailer);
    body
}

pub(super) fn restore(bytes: &[u8]) -> Result<Simulation, RestoreError> {
    if bytes.len() < 14 {
        return Err(RestoreError::Truncated);
    }
    if &bytes[..12] != TAG {
        return Err(RestoreError::UnsupportedFormat);
    }
    let revision = u16::from_le_bytes([bytes[12], bytes[13]]);
    if revision != 1 {
        return Err(RestoreError::UnsupportedRevision { found: revision });
    }
    if bytes.len() < 46 {
        return Err(RestoreError::ChecksumMismatch);
    }
    let (body, trailer) = bytes.split_at(bytes.len() - 32);
    if encoding::sha256(body) != *trailer {
        return Err(RestoreError::ChecksumMismatch);
    }

    let raw = parse_body(body)?;
    if raw.fixture_id != 1 && raw.fixture_id != 2 {
        return Err(RestoreError::UnknownFixture {
            found: raw.fixture_id,
        });
    }
    if raw.content_hash != content_hash(raw.fixture_id) {
        return Err(RestoreError::ContentMismatch);
    }
    validate(&raw)?;
    Ok(raw.into_simulation())
}

type RawTask = (u64, u32, u64, u64, u64, u64, u8, u8);

struct Raw {
    fixture_id: u32,
    content_hash: [u8; 32],
    tick: u64,
    seed: u64,
    next_actor_id: u64,
    next_record_id: u64,
    actors: Vec<(u32, bool, u8)>,
    slot_owner: Option<u32>,
    pending: Vec<(CommandKey, Payload)>,
    ordinals: Vec<(u8, u32, u64)>,
    next_event_id: u64,
    digest: [u8; 32],
    locations: Vec<u64>,
    cargo: Vec<(u64, u64)>,
    reservations: Vec<(u64, u64, u32, u64, u8)>,
    tasks: Vec<RawTask>,
    markers: Vec<(u64, u8)>,
}

impl Raw {
    fn into_simulation(self) -> Simulation {
        let mut modes = [ProcessingMode::Active; 2];
        let mut enabled = [false; 2];
        for (id, flag, mode) in self.actors {
            let index = usize::try_from(id - 1).expect("validated actor");
            enabled[index] = flag;
            modes[index] = mode_from_byte(mode).expect("validated mode");
        }
        let ordinals = [
            [self.ordinals[0].2, self.ordinals[1].2],
            [self.ordinals[2].2, self.ordinals[3].2],
        ];
        Simulation {
            fixture_id: self.fixture_id,
            config: FixtureConfig {
                seed: self.seed,
                actor_enabled: enabled,
            },
            modes,
            tick: self.tick,
            slot_owner: self.slot_owner.map(ActorId::new),
            pending: self.pending.into_iter().collect(),
            ordinals,
            next_event_id: self.next_event_id,
            event_digest: self.digest,
            next_actor_id: self.next_actor_id,
            next_record_id: self.next_record_id,
            locations: self.locations.into_iter().collect(),
            cargo: self.cargo.into_iter().collect(),
            reservations: self
                .reservations
                .into_iter()
                .map(|(id, cargo_id, actor_id, task_id, state)| {
                    (
                        id,
                        Reservation {
                            cargo_id,
                            actor_id: ActorId::new(actor_id),
                            task_id,
                            state: match state {
                                1 => ReservationState::Active,
                                2 => ReservationState::Released,
                                _ => unreachable!("validated reservation state"),
                            },
                        },
                    )
                })
                .collect(),
            tasks: self
                .tasks
                .into_iter()
                .map(
                    |(
                        id,
                        actor_id,
                        due_tick,
                        cargo_id,
                        reservation_id,
                        destination_id,
                        state,
                        mode_at_completion,
                    )| {
                        (
                            id,
                            Task {
                                actor_id: ActorId::new(actor_id),
                                due_tick,
                                cargo_id,
                                reservation_id,
                                destination_id,
                                completed: state == 2,
                                mode_at_completion,
                            },
                        )
                    },
                )
                .collect(),
            markers: self
                .markers
                .into_iter()
                .map(|(id, mode)| (id, mode_from_byte(mode).expect("validated marker mode")))
                .collect(),
            world3: None,
        }
    }
}

fn parse_body(body: &[u8]) -> Result<Raw, RestoreError> {
    let mut reader = Reader { data: body, at: 14 };
    let fixture_id = reader.u32()?;
    let content_hash = reader.array32()?;
    let tick = reader.u64()?;
    let seed = reader.u64()?;
    let next_actor_id = reader.u64()?;
    let next_record_id = reader.u64()?;

    let actor_count = reader.count(6)?;
    let mut actors = Vec::with_capacity(actor_count);
    for _ in 0..actor_count {
        let id = reader.u32()?;
        let enabled = match reader.u8()? {
            0 => false,
            1 => true,
            _ => return Err(RestoreError::Corrupt),
        };
        let mode = reader.u8()?;
        if !matches!(mode, 1 | 2) {
            return Err(RestoreError::Corrupt);
        }
        actors.push((id, enabled, mode));
    }

    let slot_owner = match reader.u8()? {
        0 => None,
        1 => Some(reader.u32()?),
        _ => return Err(RestoreError::Corrupt),
    };

    let pending_count = reader.count(25)?;
    let mut pending = Vec::with_capacity(pending_count);
    for _ in 0..pending_count {
        let key = CommandKey {
            target_tick: reader.u64()?,
            actor_id: ActorId::new(reader.u32()?),
            producer_id: reader.u32()?,
            sequence: reader.u64()?,
        };
        let payload = match reader.u8()? {
            1 => Payload::ClaimSlot,
            2 => Payload::Probe {
                stream_id: reader.u8()?,
                draw_count: reader.u8()?,
            },
            3 => {
                let mode = reader.u8()?;
                Payload::SetMode(mode_from_byte(mode).ok_or(RestoreError::Corrupt)?)
            }
            4 => Payload::MintRecord,
            _ => return Err(RestoreError::Corrupt),
        };
        pending.push((key, payload));
    }

    let ordinal_count = reader.count(13)?;
    let mut ordinals = Vec::with_capacity(ordinal_count);
    for _ in 0..ordinal_count {
        ordinals.push((reader.u8()?, reader.u32()?, reader.u64()?));
    }

    let next_event_id = reader.u64()?;
    let digest = reader.array32()?;

    let location_count = reader.count(8)?;
    let mut locations = Vec::with_capacity(location_count);
    for _ in 0..location_count {
        locations.push(reader.u64()?);
    }

    let cargo_count = reader.count(16)?;
    let mut cargo = Vec::with_capacity(cargo_count);
    for _ in 0..cargo_count {
        cargo.push((reader.u64()?, reader.u64()?));
    }

    let reservation_count = reader.count(29)?;
    let mut reservations = Vec::with_capacity(reservation_count);
    for _ in 0..reservation_count {
        let id = reader.u64()?;
        let cargo_id = reader.u64()?;
        let actor_id = reader.u32()?;
        let task_id = reader.u64()?;
        let state = reader.u8()?;
        if !matches!(state, 1 | 2) {
            return Err(RestoreError::Corrupt);
        }
        reservations.push((id, cargo_id, actor_id, task_id, state));
    }

    let task_count = reader.count(46)?;
    let mut tasks = Vec::with_capacity(task_count);
    for _ in 0..task_count {
        let id = reader.u64()?;
        let actor_id = reader.u32()?;
        let due_tick = reader.u64()?;
        let cargo_id = reader.u64()?;
        let reservation_id = reader.u64()?;
        let destination_id = reader.u64()?;
        let state = reader.u8()?;
        let mode_at_completion = reader.u8()?;
        if !matches!(state, 1 | 2) || !matches!(mode_at_completion, 0..=2) {
            return Err(RestoreError::Corrupt);
        }
        tasks.push((
            id,
            actor_id,
            due_tick,
            cargo_id,
            reservation_id,
            destination_id,
            state,
            mode_at_completion,
        ));
    }

    let marker_count = reader.count(9)?;
    let mut markers = Vec::with_capacity(marker_count);
    for _ in 0..marker_count {
        let id = reader.u64()?;
        let mode = reader.u8()?;
        if !matches!(mode, 1 | 2) {
            return Err(RestoreError::Corrupt);
        }
        markers.push((id, mode));
    }

    if reader.rest() != 0 {
        return Err(RestoreError::Corrupt);
    }
    Ok(Raw {
        fixture_id,
        content_hash,
        tick,
        seed,
        next_actor_id,
        next_record_id,
        actors,
        slot_owner,
        pending,
        ordinals,
        next_event_id,
        digest,
        locations,
        cargo,
        reservations,
        tasks,
        markers,
    })
}

fn validate(raw: &Raw) -> Result<(), RestoreError> {
    if raw.next_event_id == 0 {
        return Err(RestoreError::InvalidState);
    }
    if raw.next_actor_id != 3 {
        return Err(RestoreError::InvalidState);
    }
    let actor_ids: Vec<u32> = raw.actors.iter().map(|(id, _, _)| *id).collect();
    if actor_ids != [1, 2] {
        return Err(RestoreError::InvalidState);
    }
    if !strictly_increasing(raw.actors.iter().map(|(id, _, _)| *id))
        || !strictly_increasing(raw.pending.iter().map(|(key, _)| *key))
        || !strictly_increasing(
            raw.ordinals
                .iter()
                .map(|(stream, actor, _)| (*stream, *actor)),
        )
        || !strictly_increasing(raw.locations.iter().copied())
        || !strictly_increasing(raw.cargo.iter().map(|(id, _)| *id))
        || !strictly_increasing(raw.reservations.iter().map(|(id, _, _, _, _)| *id))
        || !strictly_increasing(raw.tasks.iter().map(|(id, _, _, _, _, _, _, _)| *id))
        || !strictly_increasing(raw.markers.iter().map(|(id, _)| *id))
    {
        return Err(RestoreError::InvalidState);
    }
    if raw.pending.iter().any(|(key, _)| {
        key.target_tick <= raw.tick || !matches!(key.actor_id.get(), 1 | 2) || key.producer_id == 0
    }) {
        return Err(RestoreError::InvalidState);
    }
    let ordinal_pairs: Vec<(u8, u32)> = raw
        .ordinals
        .iter()
        .map(|(stream, actor, _)| (*stream, *actor))
        .collect();
    if ordinal_pairs != [(1, 1), (1, 2), (2, 1), (2, 2)] {
        return Err(RestoreError::InvalidState);
    }
    let mut issued = BTreeSet::new();
    for id in raw
        .locations
        .iter()
        .copied()
        .chain(raw.cargo.iter().map(|(id, _)| *id))
        .chain(raw.reservations.iter().map(|(id, _, _, _, _)| *id))
        .chain(raw.tasks.iter().map(|(id, _, _, _, _, _, _, _)| *id))
        .chain(raw.markers.iter().map(|(id, _)| *id))
    {
        if id == 0 || !issued.insert(id) || id >= raw.next_record_id {
            return Err(RestoreError::InvalidState);
        }
    }

    if raw.fixture_id == 1 {
        validate_fixture_one(raw)
    } else {
        validate_fixture_two(raw, &issued)
    }
}

fn validate_fixture_one(raw: &Raw) -> Result<(), RestoreError> {
    if raw.actors.iter().any(|(_, _, mode)| *mode != 1) {
        return Err(RestoreError::InvalidState);
    }
    if !raw.locations.is_empty()
        || !raw.cargo.is_empty()
        || !raw.reservations.is_empty()
        || !raw.tasks.is_empty()
        || !raw.markers.is_empty()
        || raw.next_record_id != 1
    {
        return Err(RestoreError::InvalidState);
    }
    if raw
        .pending
        .iter()
        .any(|(_, payload)| matches!(payload, Payload::SetMode(_) | Payload::MintRecord))
    {
        return Err(RestoreError::InvalidState);
    }
    if raw.pending.iter().any(|(_, payload)| {
        matches!(
            payload,
            Payload::Probe {
                stream_id,
                draw_count
            } if !matches!(stream_id, 1 | 2) || !matches!(draw_count, 1 | 2)
        )
    }) {
        return Err(RestoreError::InvalidState);
    }
    if let Some(owner) = raw.slot_owner {
        let enabled = raw
            .actors
            .iter()
            .find(|(id, _, _)| *id == owner)
            .is_some_and(|(_, enabled, _)| *enabled);
        if !matches!(owner, 1 | 2) || !enabled {
            return Err(RestoreError::InvalidState);
        }
    }
    Ok(())
}

fn validate_fixture_two(raw: &Raw, issued: &BTreeSet<u64>) -> Result<(), RestoreError> {
    if raw.slot_owner.is_some() {
        return Err(RestoreError::InvalidState);
    }
    if raw.ordinals.iter().any(|(_, _, ordinal)| *ordinal != 0) {
        return Err(RestoreError::InvalidState);
    }
    if raw
        .pending
        .iter()
        .any(|(_, payload)| matches!(payload, Payload::ClaimSlot | Payload::Probe { .. }))
    {
        return Err(RestoreError::InvalidState);
    }
    let expected_ids: BTreeSet<u64> = (1..raw.next_record_id).collect();
    if raw.next_record_id < 10 || issued != &expected_ids {
        return Err(RestoreError::InvalidState);
    }
    let marker_ids: Vec<u64> = raw.markers.iter().map(|(id, _)| *id).collect();
    let expected_markers: Vec<u64> = (10..raw.next_record_id).collect();
    if raw.locations != [1, 2, 3]
        || raw.cargo.iter().map(|(id, _)| *id).collect::<Vec<_>>() != [4, 5]
        || raw
            .tasks
            .iter()
            .map(|(id, _, _, _, _, _, _, _)| *id)
            .collect::<Vec<_>>()
            != [6, 7]
        || raw
            .reservations
            .iter()
            .map(|(id, _, _, _, _)| *id)
            .collect::<Vec<_>>()
            != [8, 9]
        || marker_ids != expected_markers
    {
        return Err(RestoreError::InvalidState);
    }

    let task6 = task_by_id(raw, 6);
    let task7 = task_by_id(raw, 7);
    let reservation8 = reservation_by_id(raw, 8);
    let reservation9 = reservation_by_id(raw, 9);
    if task6.1 != 1
        || task6.3 != 4
        || task6.4 != 8
        || task6.5 != 2
        || task7.1 != 2
        || task7.3 != 5
        || task7.4 != 9
        || task7.5 != 3
        || reservation8.1 != 4
        || reservation8.2 != 1
        || reservation8.3 != 6
        || reservation9.1 != 5
        || reservation9.2 != 2
        || reservation9.3 != 7
    {
        return Err(RestoreError::InvalidState);
    }
    if task6.2 != task7.2 {
        return Err(RestoreError::InvalidState);
    }
    if task6.6 != task7.6 {
        return Err(RestoreError::InvalidState);
    }
    for task in [task6, task7] {
        let owner = raw
            .cargo
            .iter()
            .find(|(id, _)| *id == task.3)
            .map(|(_, owner)| *owner);
        let reservation_state = raw
            .reservations
            .iter()
            .find(|(id, _, _, _, _)| *id == task.4)
            .map(|(_, _, _, _, state)| *state);
        if task.6 == 1 {
            if task.7 != 0 || task.2 <= raw.tick || reservation_state != Some(1) || owner != Some(1)
            {
                return Err(RestoreError::InvalidState);
            }
        } else if !matches!(task.7, 1 | 2)
            || task.2 > raw.tick
            || reservation_state != Some(2)
            || owner != Some(task.5)
        {
            return Err(RestoreError::InvalidState);
        }
    }
    Ok(())
}

fn task_by_id(raw: &Raw, id: u64) -> RawTask {
    raw.tasks
        .iter()
        .find(|(task_id, _, _, _, _, _, _, _)| *task_id == id)
        .copied()
        .expect("task partition")
}

fn reservation_by_id(raw: &Raw, id: u64) -> (u64, u64, u32, u64, u8) {
    raw.reservations
        .iter()
        .find(|(reservation_id, _, _, _, _)| *reservation_id == id)
        .copied()
        .expect("reservation partition")
}

fn strictly_increasing<T: PartialOrd>(items: impl IntoIterator<Item = T>) -> bool {
    let mut previous = None;
    for item in items {
        if previous.is_some_and(|prior| item <= prior) {
            return false;
        }
        previous = Some(item);
    }
    true
}

fn content_hash(fixture_id: u32) -> [u8; 32] {
    match fixture_id {
        1 => Simulation::fixture_content_hash(),
        2 => encoding::sha256(b"SH:C02:FIXTURE:v1\n"),
        _ => [0; 32],
    }
}

fn encode_command(bytes: &mut Vec<u8>, key: CommandKey, payload: Payload) {
    bytes.extend(key.target_tick.to_le_bytes());
    bytes.extend(key.actor_id.get().to_le_bytes());
    bytes.extend(key.producer_id.to_le_bytes());
    bytes.extend(key.sequence.to_le_bytes());
    match payload {
        Payload::ClaimSlot => bytes.push(1),
        Payload::Probe {
            stream_id,
            draw_count,
        } => bytes.extend([2, stream_id, draw_count]),
        Payload::SetMode(mode) => bytes.extend([3, mode_byte(mode)]),
        Payload::MintRecord => bytes.push(4),
        Payload::SetEnabled(_)
        | Payload::ReserveStock { .. }
        | Payload::ReserveBundle { .. }
        | Payload::CancelReservation(_)
        | Payload::CompleteTask(_)
        | Payload::BeginTask { .. } => {
            unreachable!("fixture 3 payloads are not C0.2 commands")
        }
    }
}

fn mode_byte(mode: ProcessingMode) -> u8 {
    match mode {
        ProcessingMode::Active => 1,
        ProcessingMode::Scheduled => 2,
    }
}

fn mode_from_byte(byte: u8) -> Option<ProcessingMode> {
    match byte {
        1 => Some(ProcessingMode::Active),
        2 => Some(ProcessingMode::Scheduled),
        _ => None,
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn rest(&self) -> usize {
        self.data.len() - self.at
    }

    fn take(&mut self, size: usize) -> Result<&'a [u8], RestoreError> {
        let end = self.at.checked_add(size).ok_or(RestoreError::Corrupt)?;
        if end > self.data.len() {
            return Err(RestoreError::Corrupt);
        }
        let slice = &self.data[self.at..end];
        self.at = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, RestoreError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, RestoreError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes(bytes.try_into().expect("length")))
    }

    fn u64(&mut self) -> Result<u64, RestoreError> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes(bytes.try_into().expect("length")))
    }

    fn array32(&mut self) -> Result<[u8; 32], RestoreError> {
        Ok(self.take(32)?.try_into().expect("length"))
    }

    fn count(&mut self, minimum: u64) -> Result<usize, RestoreError> {
        let count = self.u64()?;
        let bytes = count.checked_mul(minimum).ok_or(RestoreError::Corrupt)?;
        if bytes > u64::try_from(self.rest()).map_err(|_| RestoreError::Corrupt)? {
            return Err(RestoreError::Corrupt);
        }
        usize::try_from(count).map_err(|_| RestoreError::Corrupt)
    }
}
