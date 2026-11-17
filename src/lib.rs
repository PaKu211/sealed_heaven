//! The headless Sealed Heaven C0.2 simulation fixture.
//!
//! ```
//! use sealed_sim::{ActorId, Command, CommandKey, FixtureConfig, Outcome, Payload, Simulation};
//!
//! let mut sim = Simulation::new(FixtureConfig { seed: 100, actor_enabled: [true, true] });
//! let claim = Command::new(CommandKey {
//!     target_tick: 1,
//!     actor_id: ActorId::ONE,
//!     producer_id: 1,
//!     sequence: 0,
//! }, Payload::ClaimSlot);
//! let events = sim.step(0, &[claim]).unwrap();
//! assert_eq!(events[0].outcome(), Outcome::Claimed);
//! assert_eq!(sim.slot_owner(), Some(ActorId::ONE));
//! ```

mod domain;
mod encoding;

use std::collections::{BTreeMap, BTreeSet};

use rand_chacha::{
    ChaCha12Rng,
    rand_core::{Rng, SeedableRng},
};

pub const FIXTURE_ID: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ActorId(u32);

impl ActorId {
    pub const ONE: Self = Self(1);
    pub const TWO: Self = Self(2);

    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    fn index(self) -> Option<usize> {
        match self {
            Self::ONE => Some(0),
            Self::TWO => Some(1),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixtureConfig {
    pub seed: u64,
    /// Flags for actor 1 and actor 2, respectively.
    pub actor_enabled: [bool; 2],
}

/// Field order is the fixture's numeric precedence and identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CommandKey {
    pub target_tick: u64,
    pub actor_id: ActorId,
    pub producer_id: u32,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessingMode {
    Active = 1,
    Scheduled = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Payload {
    ClaimSlot,
    /// The fixture accepts streams 1 and 2 and one or two raw words.
    Probe {
        stream_id: u8,
        draw_count: u8,
    },
    SetMode(ProcessingMode),
    MintRecord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureError {
    InvalidDueTick,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Command {
    key: CommandKey,
    payload: Payload,
}

impl Command {
    pub const fn new(key: CommandKey, payload: Payload) -> Self {
        Self { key, payload }
    }

    pub const fn key(self) -> CommandKey {
        self.key
    }

    pub const fn payload(self) -> Payload {
        self.payload
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidCommand {
    LateTarget,
    UnknownActor,
    InvalidProducer,
    InvalidPayload,
    ConflictingIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepError {
    WrongBoundary {
        expected: u64,
        received: u64,
    },
    Command {
        key: CommandKey,
        reason: InvalidCommand,
    },
    CapacityExceeded,
    InvalidState,
    /// `step` was called on fixture 2. The simulation is unchanged.
    UseDrive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreError {
    Truncated,
    UnsupportedFormat,
    UnsupportedRevision { found: u16 },
    ChecksumMismatch,
    Corrupt,
    UnknownFixture { found: u32 },
    ContentMismatch,
    InvalidState,
}

#[derive(Debug)]
pub enum CheckpointReadError {
    Io(std::io::Error),
    Restore(RestoreError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    DisabledActor,
    OccupiedSlot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Claimed,
    Rejected(Rejection),
    Probed {
        stream_id: u8,
        first_word: u32,
        second_word: Option<u32>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    id: u64,
    key: CommandKey,
    outcome: Outcome,
}

impl Event {
    pub const fn id(self) -> u64 {
        self.id
    }

    /// Its target tick is the tick at which the event occurred.
    pub const fn key(self) -> CommandKey {
        self.key
    }

    pub const fn outcome(self) -> Outcome {
        self.outcome
    }

    pub fn canonical_bytes(self) -> Vec<u8> {
        encoding::event_bytes(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedulerCommandEvent {
    id: u64,
    key: CommandKey,
    outcome: SchedulerCommandOutcome,
}

impl SchedulerCommandEvent {
    pub const fn id(self) -> u64 {
        self.id
    }

    pub const fn key(self) -> CommandKey {
        self.key
    }

    pub const fn outcome(self) -> SchedulerCommandOutcome {
        self.outcome
    }

    pub fn canonical_bytes(self) -> Vec<u8> {
        let mut bytes = self.id.to_le_bytes().to_vec();
        bytes.extend(self.key.target_tick.to_le_bytes());
        bytes.extend(self.key.actor_id.get().to_le_bytes());
        match self.outcome {
            SchedulerCommandOutcome::RecordMinted {
                marker_id,
                mode_at_mint,
            } => {
                bytes.push(2);
                bytes.extend(marker_id.to_le_bytes());
                bytes.push(match mode_at_mint {
                    ProcessingMode::Active => 1,
                    ProcessingMode::Scheduled => 2,
                });
            }
            SchedulerCommandOutcome::Rejected(SchedulerCommandRejection::DisabledActor) => {
                bytes.extend([3, 1]);
            }
        }
        bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerCommandOutcome {
    RecordMinted {
        marker_id: u64,
        mode_at_mint: ProcessingMode,
    },
    Rejected(SchedulerCommandRejection),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerCommandRejection {
    DisabledActor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedEvent {
    id: u64,
    tick: u64,
    actor_id: ActorId,
    outcome: GeneratedOutcome,
}

impl GeneratedEvent {
    pub const fn id(self) -> u64 {
        self.id
    }

    pub const fn tick(self) -> u64 {
        self.tick
    }

    pub const fn actor_id(self) -> ActorId {
        self.actor_id
    }

    pub const fn outcome(self) -> GeneratedOutcome {
        self.outcome
    }

    pub fn canonical_bytes(self) -> Vec<u8> {
        let GeneratedOutcome::TaskCompleted {
            task_id,
            reservation_id,
            cargo_id,
            origin_location_id,
            destination_location_id,
        } = self.outcome;
        let mut bytes = self.id.to_le_bytes().to_vec();
        bytes.extend(self.tick.to_le_bytes());
        bytes.extend(self.actor_id.get().to_le_bytes());
        bytes.push(1);
        for value in [
            task_id,
            reservation_id,
            cargo_id,
            origin_location_id,
            destination_location_id,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedOutcome {
    TaskCompleted {
        task_id: u64,
        reservation_id: u64,
        cargo_id: u64,
        origin_location_id: u64,
        destination_location_id: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainEvent {
    Command(Event),
    SchedulerCommand(SchedulerCommandEvent),
    Generated(GeneratedEvent),
}

impl DomainEvent {
    pub const fn id(self) -> u64 {
        match self {
            Self::Command(event) => event.id(),
            Self::SchedulerCommand(event) => event.id(),
            Self::Generated(event) => event.id(),
        }
    }

    pub fn canonical_bytes(self) -> Vec<u8> {
        match self {
            Self::Command(event) => event.canonical_bytes(),
            Self::SchedulerCommand(event) => event.canonical_bytes(),
            Self::Generated(event) => event.canonical_bytes(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriveOutput {
    events: Vec<DomainEvent>,
}

impl DriveOutput {
    pub fn events(&self) -> &[DomainEvent] {
        &self.events
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReservationState {
    Active,
    Released,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskView {
    pub id: u64,
    pub actor_id: ActorId,
    pub due_tick: u64,
    pub cargo_id: u64,
    pub reservation_id: u64,
    pub destination_location_id: u64,
    pub completed: bool,
    pub mode_at_completion: Option<ProcessingMode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Reservation {
    pub cargo_id: u64,
    pub actor_id: ActorId,
    pub task_id: u64,
    pub state: ReservationState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Task {
    pub actor_id: ActorId,
    pub due_tick: u64,
    pub cargo_id: u64,
    pub reservation_id: u64,
    pub destination_id: u64,
    pub completed: bool,
    pub mode_at_completion: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Simulation {
    fixture_id: u32,
    config: FixtureConfig,
    modes: [ProcessingMode; 2],
    tick: u64,
    pub(crate) slot_owner: Option<ActorId>,
    pub(crate) pending: BTreeMap<CommandKey, Payload>,
    pub(crate) ordinals: [[u64; 2]; 2],
    next_event_id: u64,
    event_digest: [u8; 32],
    next_actor_id: u64,
    pub(crate) next_record_id: u64,
    locations: BTreeSet<u64>,
    pub(crate) cargo: BTreeMap<u64, u64>,
    pub(crate) reservations: BTreeMap<u64, Reservation>,
    pub(crate) tasks: BTreeMap<u64, Task>,
    markers: BTreeMap<u64, ProcessingMode>,
}

impl Simulation {
    pub fn new(config: FixtureConfig) -> Self {
        Self::blank(1, config, encoding::sha256(b"SH:C01:EVENT-INIT:v1"), 1)
    }

    pub fn scheduler(
        seed: u64,
        task_due_tick: u64,
        actor_enabled: [bool; 2],
    ) -> Result<Self, FixtureError> {
        if task_due_tick == 0 {
            return Err(FixtureError::InvalidDueTick);
        }
        let mut sim = Self::blank(
            2,
            FixtureConfig {
                seed,
                actor_enabled,
            },
            encoding::sha256(b"SH:C02:EVENT-INIT:v1"),
            1,
        );
        sim.locations.extend([1, 2, 3]);
        sim.cargo.insert(4, 1);
        sim.cargo.insert(5, 1);
        sim.tasks.insert(
            6,
            Task {
                actor_id: ActorId::ONE,
                due_tick: task_due_tick,
                cargo_id: 4,
                reservation_id: 8,
                destination_id: 2,
                completed: false,
                mode_at_completion: 0,
            },
        );
        sim.tasks.insert(
            7,
            Task {
                actor_id: ActorId::TWO,
                due_tick: task_due_tick,
                cargo_id: 5,
                reservation_id: 9,
                destination_id: 3,
                completed: false,
                mode_at_completion: 0,
            },
        );
        sim.reservations.insert(
            8,
            Reservation {
                cargo_id: 4,
                actor_id: ActorId::ONE,
                task_id: 6,
                state: ReservationState::Active,
            },
        );
        sim.reservations.insert(
            9,
            Reservation {
                cargo_id: 5,
                actor_id: ActorId::TWO,
                task_id: 7,
                state: ReservationState::Active,
            },
        );
        sim.next_record_id = 10;
        Ok(sim)
    }

    fn blank(
        fixture_id: u32,
        config: FixtureConfig,
        event_digest: [u8; 32],
        next_record_id: u64,
    ) -> Self {
        Self {
            fixture_id,
            config,
            modes: [ProcessingMode::Active; 2],
            tick: 0,
            slot_owner: None,
            pending: BTreeMap::new(),
            ordinals: [[0; 2]; 2],
            next_event_id: 1,
            event_digest,
            next_actor_id: 3,
            next_record_id,
            locations: BTreeSet::new(),
            cargo: BTreeMap::new(),
            reservations: BTreeMap::new(),
            tasks: BTreeMap::new(),
            markers: BTreeMap::new(),
        }
    }

    pub fn fixture_content_hash() -> [u8; 32] {
        encoding::sha256(b"SH:C01:FIXTURE:v1\n")
    }

    pub const fn config(&self) -> FixtureConfig {
        self.config
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub fn actor_enabled(&self, actor: ActorId) -> Option<bool> {
        Some(self.config.actor_enabled[actor.index()?])
    }

    pub const fn slot_owner(&self) -> Option<ActorId> {
        self.slot_owner
    }

    pub fn pending_commands(&self) -> impl ExactSizeIterator<Item = Command> + '_ {
        self.pending
            .iter()
            .map(|(&key, &payload)| Command::new(key, payload))
    }

    pub fn next_ordinal(&self, stream_id: u8, actor: ActorId) -> Option<u64> {
        Some(
            *self
                .ordinals
                .get(usize::from(stream_id.checked_sub(1)?))?
                .get(actor.index()?)?,
        )
    }

    pub const fn next_event_id(&self) -> u64 {
        self.next_event_id
    }

    pub const fn event_digest(&self) -> [u8; 32] {
        self.event_digest
    }

    pub fn checkpoint_bytes(&self) -> Vec<u8> {
        assert!(
            self.fixture_id == FIXTURE_ID
                && self
                    .pending
                    .values()
                    .all(|payload| matches!(payload, Payload::ClaimSlot | Payload::Probe { .. })),
            "checkpoint_bytes is the C0.1 observation"
        );
        encoding::checkpoint_bytes(self)
    }

    pub fn checkpoint_sha256(&self) -> [u8; 32] {
        encoding::sha256(&self.checkpoint_bytes())
    }

    pub fn domain_checkpoint_bytes(&self) -> Vec<u8> {
        domain::domain_checkpoint_bytes(self)
    }

    pub fn domain_checkpoint_sha256(&self) -> [u8; 32] {
        encoding::sha256(&self.domain_checkpoint_bytes())
    }

    pub fn restore_domain_checkpoint(bytes: &[u8]) -> Result<Self, RestoreError> {
        domain::restore(bytes)
    }

    pub fn write_domain_checkpoint(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> std::io::Result<()> {
        std::fs::write(path, self.domain_checkpoint_bytes())
    }

    pub fn read_domain_checkpoint(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, CheckpointReadError> {
        match std::fs::read(path) {
            Ok(bytes) => {
                Self::restore_domain_checkpoint(&bytes).map_err(CheckpointReadError::Restore)
            }
            Err(error) => Err(CheckpointReadError::Io(error)),
        }
    }

    pub fn processing_mode(&self, actor: ActorId) -> Option<ProcessingMode> {
        self.modes.get(actor.index()?).copied()
    }

    pub const fn next_actor_id(&self) -> u64 {
        self.next_actor_id
    }

    pub const fn next_record_id(&self) -> u64 {
        self.next_record_id
    }

    pub fn location_ids(&self) -> impl ExactSizeIterator<Item = u64> + '_ {
        self.locations.iter().copied()
    }

    pub fn cargo_ids(&self) -> impl ExactSizeIterator<Item = u64> + '_ {
        self.cargo.keys().copied()
    }

    pub fn cargo_owner(&self, id: u64) -> Option<u64> {
        self.cargo.get(&id).copied()
    }

    pub fn reservation_state(&self, id: u64) -> Option<ReservationState> {
        self.reservations
            .get(&id)
            .map(|reservation| reservation.state)
    }

    pub fn task_view(&self, id: u64) -> Option<TaskView> {
        self.tasks.get(&id).map(|task| TaskView {
            id,
            actor_id: task.actor_id,
            due_tick: task.due_tick,
            cargo_id: task.cargo_id,
            reservation_id: task.reservation_id,
            destination_location_id: task.destination_id,
            completed: task.completed,
            mode_at_completion: match task.mode_at_completion {
                1 => Some(ProcessingMode::Active),
                2 => Some(ProcessingMode::Scheduled),
                _ => None,
            },
        })
    }

    pub fn marker_ids(&self) -> impl ExactSizeIterator<Item = u64> + '_ {
        self.markers.keys().copied()
    }

    pub fn marker_mode(&self, id: u64) -> Option<ProcessingMode> {
        self.markers.get(&id).copied()
    }

    /// Seals one immutable submission batch and advances one logical tick.
    /// An error leaves the entire previous checkpoint available for retry.
    /// Fixture 2 callers use [`Self::drive`].
    pub fn step(
        &mut self,
        submission_tick: u64,
        batch: &[Command],
    ) -> Result<Vec<Event>, StepError> {
        if self.fixture_id != FIXTURE_ID {
            return Err(StepError::UseDrive);
        }
        Ok(self
            .drive(submission_tick, batch)?
            .events
            .into_iter()
            .map(|event| match event {
                DomainEvent::Command(event) => event,
                DomainEvent::SchedulerCommand(_) | DomainEvent::Generated(_) => {
                    unreachable!("fixture 1 publishes command events only")
                }
            })
            .collect())
    }

    /// Drives one boundary for either fixture.
    ///
    /// ```
    /// use sealed_sim::{
    ///     ActorId, Command, CommandKey, DomainEvent, Event, FixtureConfig, GeneratedOutcome, Outcome,
    ///     Payload, SchedulerCommandOutcome, Simulation, StepError,
    /// };
    ///
    /// let mut sim = Simulation::new(FixtureConfig { seed: 100, actor_enabled: [true, true] });
    /// let claim = Command::new(
    ///     CommandKey {
    ///         target_tick: 1,
    ///         actor_id: ActorId::ONE,
    ///         producer_id: 1,
    ///         sequence: 0,
    ///     },
    ///     Payload::ClaimSlot,
    /// );
    /// let events: Vec<Event> = sim.step(0, &[claim]).unwrap();
    /// assert_eq!(events[0].id(), 1);
    /// assert_eq!(events[0].key(), claim.key());
    /// assert_eq!(events[0].outcome(), Outcome::Claimed);
    /// let claim_bytes = events[0].canonical_bytes();
    ///
    /// let mut replay = Simulation::new(FixtureConfig { seed: 100, actor_enabled: [true, true] });
    /// let driven = replay.drive(0, &[claim]).unwrap();
    /// match driven.events()[0] {
    ///     DomainEvent::Command(event) => {
    ///         assert_eq!(event.id(), 1);
    ///         assert_eq!(event.key(), claim.key());
    ///         assert_eq!(event.outcome(), Outcome::Claimed);
    ///         assert_eq!(event.canonical_bytes(), claim_bytes);
    ///     }
    ///     DomainEvent::SchedulerCommand(_) | DomainEvent::Generated(_) => panic!("fixture 1 is command-only"),
    /// }
    ///
    /// let mut scheduled = Simulation::scheduler(100, 3, [true, true]).unwrap();
    /// assert_eq!(scheduled.step(0, &[]), Err(StepError::UseDrive));
    /// assert_eq!(scheduled.tick(), 0);
    /// let quiet = scheduled.drive(0, &[]).unwrap();
    /// assert!(quiet.events().is_empty());
    /// ```
    pub fn drive(
        &mut self,
        submission_tick: u64,
        batch: &[Command],
    ) -> Result<DriveOutput, StepError> {
        if submission_tick != self.tick {
            return Err(StepError::WrongBoundary {
                expected: self.tick,
                received: submission_tick,
            });
        }

        let mut ordered = batch.to_vec();
        ordered.sort_unstable_by_key(|command| command.key);
        for command in &ordered {
            self.preflight(*command)?;
        }

        // Stage the small fixture so even a late overflow can discard the whole step.
        let mut staged = self.clone();
        for Command { key, payload } in ordered {
            if *staged.pending.entry(key).or_insert(payload) != payload {
                return Err(StepError::Command {
                    key,
                    reason: InvalidCommand::ConflictingIdentity,
                });
            }
        }

        staged.tick = checked_increment(staged.tick)?;
        staged.apply_modes();
        let mut events = Vec::new();
        staged.apply_commands(&mut events)?;
        staged.apply_work(&mut events)?;
        *self = staged;
        Ok(DriveOutput { events })
    }

    fn preflight(&self, command: Command) -> Result<(), StepError> {
        let key = command.key;
        let reason = if key.target_tick <= self.tick {
            InvalidCommand::LateTarget
        } else if key.actor_id.index().is_none() {
            InvalidCommand::UnknownActor
        } else if key.producer_id == 0 {
            InvalidCommand::InvalidProducer
        } else if !self.payload_allowed(command.payload) {
            InvalidCommand::InvalidPayload
        } else {
            return Ok(());
        };
        Err(StepError::Command { key, reason })
    }

    fn payload_allowed(&self, payload: Payload) -> bool {
        match (self.fixture_id, payload) {
            (1, Payload::ClaimSlot) => true,
            (
                1,
                Payload::Probe {
                    stream_id,
                    draw_count,
                },
            ) => matches!(stream_id, 1 | 2) && matches!(draw_count, 1 | 2),
            (2, Payload::SetMode(_)) | (2, Payload::MintRecord) => true,
            _ => false,
        }
    }

    fn apply_modes(&mut self) {
        let keys: Vec<CommandKey> = self
            .pending
            .iter()
            .filter_map(|(&key, payload)| {
                (key.target_tick == self.tick && matches!(payload, Payload::SetMode(_)))
                    .then_some(key)
            })
            .collect();
        for key in keys {
            let Some(Payload::SetMode(mode)) = self.pending.remove(&key) else {
                continue;
            };
            if let Some(index) = key.actor_id.index() {
                self.modes[index] = mode;
            }
        }
    }

    fn apply_commands(&mut self, events: &mut Vec<DomainEvent>) -> Result<(), StepError> {
        while let Some((&key, _)) = self.pending.first_key_value() {
            if key.target_tick != self.tick {
                break;
            }
            let payload = self.pending.pop_first().expect("key exists").1;
            let event = match payload {
                Payload::ClaimSlot | Payload::Probe { .. } => {
                    DomainEvent::Command(self.commit_command(key, payload)?)
                }
                Payload::MintRecord => DomainEvent::SchedulerCommand(self.commit_mint(key)?),
                Payload::SetMode(_) => unreachable!("the mode phase removes due mode commands"),
            };
            events.push(event);
        }
        Ok(())
    }

    fn apply_work(&mut self, events: &mut Vec<DomainEvent>) -> Result<(), StepError> {
        if self
            .tasks
            .values()
            .any(|task| !task.completed && task.due_tick < self.tick)
        {
            return Err(StepError::InvalidState);
        }
        let mut due: Vec<u64> = self
            .tasks
            .iter()
            .filter(|(_, task)| !task.completed && task.due_tick == self.tick)
            .map(|(id, _)| *id)
            .collect();
        due.sort_by_key(|id| {
            let task = self.tasks[id];
            (task.due_tick, task.actor_id, *id)
        });
        for id in due {
            events.push(DomainEvent::Generated(self.complete_task(id)?));
        }
        Ok(())
    }

    fn commit_command(&mut self, key: CommandKey, payload: Payload) -> Result<Event, StepError> {
        let id = self.next_event_id;
        self.next_event_id = checked_increment(id)?;
        let actor_index = key.actor_id.index().expect("actor validated in preflight");
        let outcome = if !self.config.actor_enabled[actor_index] {
            Outcome::Rejected(Rejection::DisabledActor)
        } else {
            match payload {
                Payload::ClaimSlot if self.slot_owner.is_some() => {
                    Outcome::Rejected(Rejection::OccupiedSlot)
                }
                Payload::ClaimSlot => {
                    self.slot_owner = Some(key.actor_id);
                    Outcome::Claimed
                }
                Payload::Probe {
                    stream_id,
                    draw_count,
                } => {
                    let ordinal = &mut self.ordinals[usize::from(stream_id - 1)][actor_index];
                    let key_bytes =
                        encoding::probe_seed(self.config.seed, stream_id, key.actor_id, *ordinal);
                    *ordinal = checked_increment(*ordinal)?;
                    let mut rng = ChaCha12Rng::from_seed(key_bytes);
                    let first_word = rng.next_u32();
                    let second_word = (draw_count == 2).then(|| rng.next_u32());
                    Outcome::Probed {
                        stream_id,
                        first_word,
                        second_word,
                    }
                }
                Payload::SetMode(_) | Payload::MintRecord => {
                    unreachable!("fixture 1 rejects scheduler payloads")
                }
            }
        };
        let event = Event { id, key, outcome };
        self.event_digest = encoding::extend_event_digest(self.event_digest, event);
        Ok(event)
    }

    fn commit_mint(&mut self, key: CommandKey) -> Result<SchedulerCommandEvent, StepError> {
        let id = self.next_event_id;
        self.next_event_id = checked_increment(id)?;
        let actor_index = key.actor_id.index().expect("actor validated in preflight");
        let outcome = if !self.config.actor_enabled[actor_index] {
            SchedulerCommandOutcome::Rejected(SchedulerCommandRejection::DisabledActor)
        } else {
            let marker_id = self.next_record_id;
            self.next_record_id = checked_increment(marker_id)?;
            let mode_at_mint = self.modes[actor_index];
            self.markers.insert(marker_id, mode_at_mint);
            SchedulerCommandOutcome::RecordMinted {
                marker_id,
                mode_at_mint,
            }
        };
        let event = SchedulerCommandEvent { id, key, outcome };
        self.event_digest =
            encoding::extend_scheduler_digest(self.event_digest, &event.canonical_bytes());
        Ok(event)
    }

    fn complete_task(&mut self, id: u64) -> Result<GeneratedEvent, StepError> {
        let task = self.tasks[&id];
        let reservation = self
            .reservations
            .get(&task.reservation_id)
            .copied()
            .ok_or(StepError::InvalidState)?;
        let origin = self
            .cargo
            .get(&task.cargo_id)
            .copied()
            .ok_or(StepError::InvalidState)?;
        if reservation.state != ReservationState::Active
            || reservation.cargo_id != task.cargo_id
            || reservation.task_id != id
            || origin == task.destination_id
        {
            return Err(StepError::InvalidState);
        }
        let actor_index = task.actor_id.index().ok_or(StepError::InvalidState)?;
        let mode = self.modes[actor_index];
        let event_id = self.next_event_id;
        self.next_event_id = checked_increment(event_id)?;
        self.cargo.insert(task.cargo_id, task.destination_id);
        self.reservations
            .get_mut(&task.reservation_id)
            .expect("reservation")
            .state = ReservationState::Released;
        let task = self.tasks.get_mut(&id).expect("task");
        task.completed = true;
        task.mode_at_completion = match mode {
            ProcessingMode::Active => 1,
            ProcessingMode::Scheduled => 2,
        };
        let event = GeneratedEvent {
            id: event_id,
            tick: self.tick,
            actor_id: task.actor_id,
            outcome: GeneratedOutcome::TaskCompleted {
                task_id: id,
                reservation_id: task.reservation_id,
                cargo_id: task.cargo_id,
                origin_location_id: origin,
                destination_location_id: task.destination_id,
            },
        };
        self.event_digest =
            encoding::extend_scheduler_digest(self.event_digest, &event.canonical_bytes());
        Ok(event)
    }
}

fn checked_increment(value: u64) -> Result<u64, StepError> {
    value.checked_add(1).ok_or(StepError::CapacityExceeded)
}

#[cfg(test)]
mod tests;
