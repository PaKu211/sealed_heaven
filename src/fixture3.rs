//! Fixture 3 resource core. Fixture 1 and fixture 2 never enter this module.

#![allow(clippy::collapsible_if)]

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ActorId, Command, CommandKey, FixtureConfig, InvalidCommand, Leg, Payload, ProcessingMode,
    ResourceEvent, RestoreError, Simulation, StepError, encoding,
};

const TAG: &[u8] = b"SH:C03:STATE";

mod validation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PathStep {
    Edge(u64),
    Grid(u8),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Actor {
    pub enabled: u8,
    pub mode: u8,
    pub next_wake: u64,
    pub channels: [u32; 5],
    pub roles: [i64; 5],
    pub relationship: i64,
    pub home: u64,
    pub task_id: u64,
    pub locus: u8,
    pub site: u64,
    pub x: u32,
    pub y: u32,
    pub edge: u64,
    pub traversed: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Stock {
    pub definition_id: u64,
    pub class: u8,
    pub quantity: u64,
    pub owner_kind: u8,
    pub owner_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Reservation {
    pub stock_id: u64,
    pub claimant: u32,
    pub quantity: u64,
    pub acquired_tick: u64,
    pub expiry_tick: u64,
    pub task_id: u64,
    pub state: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Completion {
    pub key: CommandKey,
    pub submitted: u64,
    pub resolved: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RegistryRecord {
    pub submitted: u64,
    pub resolved: u64,
    pub role: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Task {
    pub actor_id: u32,
    pub goal: u8,
    pub recipe_id: u64,
    pub stage: u8,
    pub interruption: u8,
    pub target_locus: u8,
    pub target_site: u64,
    pub target_x: u32,
    pub target_y: u32,
    pub progress: u64,
    pub created: u64,
    pub due: u64,
    pub unconsumed: Vec<Completion>,
    pub path_index: u64,
    pub steps: Vec<PathStep>,
    pub reservations: Vec<u64>,
    pub legs: Vec<Leg>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Transaction {
    pub task_id: u64,
    pub source: u8,
    pub key: CommandKey,
    pub applied: u8,
    pub legs: Vec<Leg>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct World {
    pub tick: u64,
    pub seed: u64,
    pub wake_interval: u64,
    pub next_actor_id: u64,
    pub next_record_id: u64,
    pub next_transaction_id: u64,
    pub next_stock_id: u64,
    pub next_event_id: u64,
    pub digest: [u8; 32],
    pub actors: BTreeMap<u32, Actor>,
    pub sites: BTreeMap<u64, u8>,
    pub holders: BTreeMap<u64, u8>,
    pub edges: Vec<(u64, u64, u64, u64)>,
    pub grid_site: u64,
    pub grid_w: u32,
    pub grid_h: u32,
    pub grid: Vec<u8>,
    pub stocks: BTreeMap<u64, Stock>,
    pub reservations: BTreeMap<u64, Reservation>,
    pub tasks: BTreeMap<u64, Task>,
    pub transactions: BTreeMap<u64, Transaction>,
    pub registry: BTreeMap<CommandKey, RegistryRecord>,
    pub pending: BTreeMap<CommandKey, Payload>,
    // Codec-only trace records: actor ID is the map key, then the 320-byte tail.
    // S1 does not produce decisions or evaluate needs.
    pub traces: BTreeMap<u32, Vec<u8>>,
    pub cancels: BTreeSet<u64>,
}

impl World {
    pub(crate) fn standard(seed: u64) -> Self {
        let mut sites = BTreeMap::new();
        sites.insert(1, 1);
        sites.insert(2, 0);
        sites.insert(9, 0);
        let mut holders = BTreeMap::new();
        holders.insert(20, 2);
        holders.insert(30, 4);
        let mut grid = vec![1; 9];
        grid[4] = 0;
        Self {
            tick: 0,
            seed,
            wake_interval: 4,
            next_actor_id: 1,
            next_record_id: 1,
            next_transaction_id: 1,
            next_stock_id: 1,
            next_event_id: 1,
            digest: encoding::sha256(b"SH:C03:EVENT-INIT:v1"),
            actors: BTreeMap::new(),
            sites,
            holders,
            edges: vec![(1, 2, 1, 1), (2, 1, 2, 1)],
            grid_site: 1,
            grid_w: 3,
            grid_h: 3,
            grid,
            stocks: BTreeMap::new(),
            reservations: BTreeMap::new(),
            tasks: BTreeMap::new(),
            transactions: BTreeMap::new(),
            registry: BTreeMap::new(),
            pending: BTreeMap::new(),
            traces: BTreeMap::new(),
            cancels: BTreeSet::new(),
        }
    }

    pub(crate) fn insert_actor(&mut self, id: u32, task_id: u64) {
        self.actors.insert(
            id,
            Actor {
                enabled: 1,
                mode: 1,
                next_wake: 100,
                channels: [0; 5],
                roles: [0; 5],
                relationship: 0,
                home: 1,
                task_id,
                locus: 1,
                site: 1,
                x: 0,
                y: 0,
                edge: 0,
                traversed: 0,
            },
        );
        self.next_actor_id = self.next_actor_id.max(u64::from(id) + 1);
    }

    pub(crate) fn insert_scripted_task(&mut self, id: u64, actor_id: u32) {
        self.tasks.insert(id, scripted_task(actor_id));
        self.next_record_id = self.next_record_id.max(id + 1);
    }
}

fn scripted_task(actor_id: u32) -> Task {
    Task {
        actor_id,
        goal: 0,
        recipe_id: 0,
        stage: 1,
        interruption: 0,
        target_locus: 1,
        target_site: 1,
        target_x: 0,
        target_y: 0,
        progress: 0,
        created: 0,
        due: 0,
        unconsumed: Vec::new(),
        path_index: 0,
        steps: Vec::new(),
        reservations: Vec::new(),
        legs: Vec::new(),
    }
}

pub(crate) fn two_claimants() -> World {
    let mut world = World::standard(100);
    world.insert_actor(1, 1);
    world.insert_actor(2, 2);
    world.insert_scripted_task(1, 1);
    world.insert_scripted_task(2, 2);
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
    world
}

pub(crate) fn into_simulation(world: World) -> Simulation {
    let enabled = [
        world.actors.get(&1).is_some_and(|actor| actor.enabled == 1),
        world.actors.get(&2).is_some_and(|actor| actor.enabled == 1),
    ];
    Simulation {
        fixture_id: 3,
        config: FixtureConfig {
            seed: world.seed,
            actor_enabled: enabled,
        },
        modes: [ProcessingMode::Active; 2],
        tick: world.tick,
        slot_owner: None,
        pending: BTreeMap::new(),
        ordinals: [[0; 2]; 2],
        next_event_id: world.next_event_id,
        event_digest: world.digest,
        next_actor_id: world.next_actor_id,
        next_record_id: world.next_record_id,
        locations: BTreeSet::new(),
        cargo: BTreeMap::new(),
        reservations: BTreeMap::new(),
        tasks: BTreeMap::new(),
        markers: BTreeMap::new(),
        world3: Some(world),
    }
}

pub(crate) fn drive(
    sim: &mut Simulation,
    submission_tick: u64,
    batch: &[Command],
) -> Result<Vec<ResourceEvent>, StepError> {
    let world = sim
        .world3
        .as_ref()
        .expect("fixture 3 drive requires a resource world");
    if submission_tick != world.tick {
        return Err(StepError::WrongBoundary {
            expected: world.tick,
            received: submission_tick,
        });
    }
    let mut staged = world.clone();
    let mut ordered = batch.to_vec();
    ordered.sort_by_key(|command| command.key());
    // §5 rows 2-4 are batch-wide and precede every per-command replay/late check.
    if let Some(command) = ordered.iter().find(|c| c.key().producer_id != 1) {
        return Err(command_error(*command, InvalidCommand::InvalidProducer));
    }
    if let Some(command) = ordered
        .iter()
        .find(|c| !world.actors.contains_key(&c.key().actor_id.get()))
    {
        return Err(command_error(*command, InvalidCommand::UnknownActor));
    }
    for (index, command) in ordered.iter().enumerate() {
        if world.pending.contains_key(&command.key())
            || ordered
                .get(index + 1)
                .is_some_and(|next| next.key() == command.key())
        {
            return Err(command_error(*command, InvalidCommand::ConflictingIdentity));
        }
    }
    let mut scheduled_begins: BTreeSet<CommandKey> = world
        .pending
        .iter()
        .filter_map(|(&key, payload)| matches!(payload, Payload::BeginTask { .. }).then_some(key))
        .collect();
    for command in &ordered {
        match classify(&staged, *command, &scheduled_begins)? {
            Admit::Noop => {}
            Admit::Store => {
                if matches!(command.payload(), Payload::BeginTask { .. }) {
                    scheduled_begins.insert(command.key());
                }
                if staged
                    .pending
                    .insert(command.key(), command.payload())
                    .is_some()
                {
                    return Err(command_error(*command, InvalidCommand::ConflictingIdentity));
                }
            }
        }
    }
    staged.tick = staged
        .tick
        .checked_add(1)
        .ok_or(StepError::CapacityExceeded)?;
    let mut events = Vec::new();
    apply_due(&mut staged, &mut events)?;
    invalidate(&mut staged, &mut events)?;
    work(&mut staged, &mut events)?;
    sim.tick = staged.tick;
    sim.next_event_id = staged.next_event_id;
    sim.event_digest = staged.digest;
    sim.next_actor_id = staged.next_actor_id;
    sim.next_record_id = staged.next_record_id;
    sim.world3 = Some(staged);
    Ok(events)
}

enum Admit {
    Noop,
    Store,
}

fn command_error(command: Command, reason: InvalidCommand) -> StepError {
    StepError::Command {
        key: command.key(),
        reason,
    }
}

fn classify(
    world: &World,
    command: Command,
    scheduled_begins: &BTreeSet<CommandKey>,
) -> Result<Admit, StepError> {
    let key = command.key();
    let payload = command.payload();
    if key.producer_id != 1 {
        return Err(command_error(command, InvalidCommand::InvalidProducer));
    }
    if !world.actors.contains_key(&key.actor_id.get()) {
        return Err(command_error(command, InvalidCommand::UnknownActor));
    }
    if world.pending.contains_key(&key) {
        return Err(command_error(command, InvalidCommand::ConflictingIdentity));
    }
    if let Some(record) = world.registry.get(&key) {
        return match payload {
            Payload::CompleteTask(id) if id == record.submitted => Ok(Admit::Noop),
            Payload::CompleteTask(_) => {
                Err(command_error(command, InvalidCommand::ConflictingIdentity))
            }
            _ => Err(command_error(command, InvalidCommand::ConflictingIdentity)),
        };
    }
    if let Payload::CancelReservation(id) = payload {
        if id != 0 {
            if let Some(reservation) = world.reservations.get(&id) {
                if reservation.claimant != key.actor_id.get() {
                    return Err(command_error(command, InvalidCommand::InvalidPayload));
                }
                let terminal = world
                    .tasks
                    .get(&reservation.task_id)
                    .is_some_and(|task| task.stage >= 6);
                if terminal {
                    return Ok(Admit::Noop);
                }
            }
        }
    }
    if key.target_tick <= world.tick {
        return Err(command_error(command, InvalidCommand::LateTarget));
    }
    if !payload_shape_ok(world, command) {
        return Err(command_error(command, InvalidCommand::InvalidPayload));
    }
    if let Payload::CompleteTask(0) = payload {
        if !scheduled_begins.iter().any(|begin| {
            *begin < key && begin.target_tick == key.target_tick && begin.actor_id == key.actor_id
        }) {
            return Err(command_error(command, InvalidCommand::InvalidPayload));
        }
    }
    if let Payload::CompleteTask(id) = payload {
        if id != 0 {
            let actor = world.actors.get(&key.actor_id.get()).expect("checked");
            let current = world.tasks.get(&actor.task_id);
            let names_current = actor.task_id == id && current.is_some_and(|task| task.stage < 6);
            if !names_current {
                return Err(command_error(command, InvalidCommand::InvalidPayload));
            }
        }
    }
    if let Payload::CancelReservation(id) = payload {
        if id != 0 {
            match world.reservations.get(&id) {
                None => return Ok(Admit::Noop),
                Some(reservation) => {
                    let actor = world.actors.get(&key.actor_id.get()).expect("checked");
                    if reservation.task_id != actor.task_id {
                        return Err(command_error(command, InvalidCommand::InvalidPayload));
                    }
                }
            }
        } else {
            let actor = world.actors.get(&key.actor_id.get()).expect("checked");
            if actor.task_id == 0 {
                return Ok(Admit::Noop);
            }
        }
    }
    if let Payload::ReserveStock { task_id, .. } | Payload::ReserveBundle { task_id, .. } = payload
    {
        let actor = world.actors.get(&key.actor_id.get()).expect("checked");
        if task_id == 0
            || actor.task_id != task_id
            || !world.tasks.get(&task_id).is_some_and(|t| t.stage < 6)
        {
            return Err(command_error(command, InvalidCommand::InvalidPayload));
        }
    }
    if let Payload::BeginTask { .. } = payload {
        let actor = world.actors.get(&key.actor_id.get()).expect("checked");
        if actor.task_id != 0
            || scheduled_begins
                .iter()
                .any(|begin| begin.target_tick == key.target_tick && begin.actor_id == key.actor_id)
        {
            return Err(StepError::InvalidState);
        }
    }
    Ok(Admit::Store)
}

fn payload_shape_ok(world: &World, command: Command) -> bool {
    let actor = world
        .actors
        .get(&command.key().actor_id.get())
        .expect("checked");
    match command.payload() {
        Payload::SetEnabled(bit) => bit <= 1,
        Payload::ReserveStock {
            quantity,
            expiry_tick,
            ..
        } => quantity >= 1 && expiry_tick > command.key().target_tick,
        Payload::ReserveBundle {
            expiry_tick,
            pair_count,
            stocks,
            quantities,
            ..
        } => {
            let count = usize::from(pair_count);
            (1..=4).contains(&count)
                && expiry_tick > command.key().target_tick
                && stocks[..count].windows(2).all(|pair| pair[0] < pair[1])
                && quantities[..count].iter().all(|quantity| *quantity >= 1)
        }
        Payload::CancelReservation(_) => true,
        Payload::CompleteTask(_) => true,
        Payload::BeginTask {
            goal,
            recipe_id,
            stage,
            leg_count,
            legs,
            target_locus,
            target_site,
            target_x,
            target_y,
            ..
        } => {
            goal <= 5
                && matches!(recipe_id, 0 | 1)
                && matches!(stage, 1 | 5)
                && leg_count <= 4
                && actor.locus != 3
                && legs[..usize::from(leg_count)].iter().all(leg_shape_ok)
                && validation::target_valid(world, target_locus, target_site, target_x, target_y)
        }
        Payload::SetMode(_) => true,
        Payload::ClaimSlot | Payload::Probe { .. } | Payload::MintRecord => false,
    }
}

fn apply_due(world: &mut World, events: &mut Vec<ResourceEvent>) -> Result<(), StepError> {
    let mode_keys: Vec<CommandKey> = world
        .pending
        .iter()
        .filter_map(|(&key, payload)| {
            (key.target_tick == world.tick && matches!(payload, Payload::SetMode(_))).then_some(key)
        })
        .collect();
    for key in mode_keys {
        let Some(Payload::SetMode(mode)) = world.pending.remove(&key) else {
            continue;
        };
        if let Some(actor) = world.actors.get_mut(&key.actor_id.get()) {
            actor.mode = match mode {
                ProcessingMode::Active => 1,
                ProcessingMode::Scheduled => 2,
            };
        }
    }
    let due: Vec<CommandKey> = world
        .pending
        .keys()
        .copied()
        .filter(|key| key.target_tick == world.tick)
        .collect();
    let mut created_this_phase = BTreeMap::new();
    for key in due {
        let Some(payload) = world.pending.remove(&key) else {
            continue;
        };
        let old_task = world
            .actors
            .get(&key.actor_id.get())
            .ok_or(StepError::InvalidState)?
            .task_id;
        if matches!(payload, Payload::CompleteTask(0))
            && created_this_phase.get(&key.actor_id.get()) != Some(&old_task)
        {
            return Err(command_error(
                Command::new(key, payload),
                InvalidCommand::InvalidPayload,
            ));
        }
        apply_command(world, key, payload, events)?;
        if matches!(payload, Payload::BeginTask { .. }) {
            let task = world.actors[&key.actor_id.get()].task_id;
            if task != 0 && task != old_task {
                created_this_phase.insert(key.actor_id.get(), task);
            }
        }
    }
    Ok(())
}

fn leg_shape_ok(leg: &Leg) -> bool {
    matches!(leg.op, 1..=4)
        && matches!(leg.class, 1 | 2)
        && leg.definition_id != 0
        && leg.quantity != 0
}

fn apply_command(
    world: &mut World,
    key: CommandKey,
    payload: Payload,
    events: &mut Vec<ResourceEvent>,
) -> Result<(), StepError> {
    let command = Command::new(key, payload);
    if !payload_shape_ok(world, command) {
        return Err(command_error(command, InvalidCommand::InvalidPayload));
    }
    let actor = world
        .actors
        .get(&key.actor_id.get())
        .ok_or(StepError::InvalidState)?;
    match payload {
        Payload::BeginTask { .. } if actor.task_id != 0 => return Err(StepError::InvalidState),
        Payload::CompleteTask(id)
            if actor.task_id == 0
                || (id != 0 && id != actor.task_id)
                || !world
                    .tasks
                    .get(&actor.task_id)
                    .is_some_and(|t| t.stage < 6 && t.actor_id == key.actor_id.get()) =>
        {
            return Err(command_error(command, InvalidCommand::InvalidPayload));
        }
        Payload::ReserveStock { task_id, .. } | Payload::ReserveBundle { task_id, .. }
            if task_id == 0
                || task_id != actor.task_id
                || !world
                    .tasks
                    .get(&task_id)
                    .is_some_and(|t| t.stage < 6 && t.actor_id == key.actor_id.get()) =>
        {
            return Err(command_error(command, InvalidCommand::InvalidPayload));
        }
        _ => {}
    }
    match payload {
        Payload::SetEnabled(bit) => {
            if let Some(actor) = world.actors.get_mut(&key.actor_id.get()) {
                if actor.enabled == 0 && bit == 1 && actor.next_wake == world.tick {
                    actor.next_wake = world
                        .tick
                        .checked_add(world.wake_interval)
                        .ok_or(StepError::CapacityExceeded)?;
                }
                actor.enabled = bit;
            }
        }
        Payload::BeginTask {
            goal,
            stage,
            recipe_id,
            target_locus,
            target_site,
            target_x,
            target_y,
            due_tick,
            leg_count,
            legs,
        } => {
            let actor = world.actors.get(&key.actor_id.get()).expect("checked");
            if actor.enabled == 0 {
                emit(
                    world,
                    key.actor_id.get(),
                    2,
                    pack(&[u64_bytes(0), u64_bytes(0), vec![1]]),
                    events,
                )?;
                return Ok(());
            }
            let id = world.next_record_id;
            world.next_record_id = id.checked_add(1).ok_or(StepError::CapacityExceeded)?;
            let mut task = scripted_task(key.actor_id.get());
            task.goal = goal;
            task.recipe_id = recipe_id;
            task.stage = stage;
            task.target_locus = target_locus;
            task.target_site = target_site;
            task.target_x = target_x;
            task.target_y = target_y;
            task.created = world.tick;
            task.due = due_tick;
            task.legs = legs[..usize::from(leg_count)].to_vec();
            world.tasks.insert(id, task);
            world
                .actors
                .get_mut(&key.actor_id.get())
                .expect("actor")
                .task_id = id;
            emit(
                world,
                key.actor_id.get(),
                7,
                pack(&[u64_bytes(id), vec![stage]]),
                events,
            )?;
        }
        Payload::ReserveStock {
            stock_id,
            quantity,
            expiry_tick,
            task_id,
        } => reserve(
            world,
            key.actor_id.get(),
            task_id,
            expiry_tick,
            &[(stock_id, quantity)],
            events,
        )?,
        Payload::ReserveBundle {
            task_id,
            expiry_tick,
            pair_count,
            stocks,
            quantities,
        } => {
            let pairs: Vec<(u64, u64)> = (0..usize::from(pair_count))
                .map(|index| (stocks[index], quantities[index]))
                .collect();
            reserve(
                world,
                key.actor_id.get(),
                task_id,
                expiry_tick,
                &pairs,
                events,
            )?;
        }
        Payload::CancelReservation(id) => {
            let task_id = if id == 0 {
                world
                    .actors
                    .get(&key.actor_id.get())
                    .expect("actor")
                    .task_id
            } else {
                world
                    .reservations
                    .get(&id)
                    .map(|reservation| reservation.task_id)
                    .unwrap_or(0)
            };
            if task_id != 0 {
                world.cancels.insert(task_id);
            }
        }
        Payload::CompleteTask(submitted) => {
            let actor = world.actors.get(&key.actor_id.get()).expect("actor");
            let resolved = if submitted == 0 {
                actor.task_id
            } else {
                submitted
            };
            if resolved == 0 {
                return Err(StepError::InvalidState);
            }
            if let Some(task) = world.tasks.get_mut(&resolved) {
                task.unconsumed.push(Completion {
                    key,
                    submitted,
                    resolved,
                });
                task.unconsumed.sort_by_key(|record| record.key);
            }
        }
        Payload::SetMode(_) | Payload::ClaimSlot | Payload::Probe { .. } | Payload::MintRecord => {}
    }
    Ok(())
}

fn reserve(
    world: &mut World,
    actor_id: u32,
    task_id: u64,
    expiry_tick: u64,
    pairs: &[(u64, u64)],
    events: &mut Vec<ResourceEvent>,
) -> Result<(), StepError> {
    let mut requested: BTreeMap<u64, u64> = BTreeMap::new();
    for &(stock_id, quantity) in pairs {
        let sum = requested.entry(stock_id).or_insert(0);
        *sum = sum
            .checked_add(quantity)
            .ok_or(StepError::CapacityExceeded)?;
    }
    // Preflight every numeric aggregate before emitting even a rejection. A
    // later overflow must roll back the entire boundary, not partly fill a bundle.
    for (&stock_id, &quantity) in &requested {
        if let Some(stock) = world.stocks.get(&stock_id) {
            if quantity > stock.quantity {
                return Err(StepError::InvalidState);
            }
            held(world, stock_id, None)?
                .checked_add(quantity)
                .ok_or(StepError::CapacityExceeded)?;
        }
    }
    for (&stock_id, &quantity) in &requested {
        let Some(stock) = world.stocks.get(&stock_id) else {
            emit(
                world,
                actor_id,
                2,
                pack(&[u64_bytes(stock_id), u64_bytes(quantity), vec![6]]),
                events,
            )?;
            return Ok(());
        };
        if quantity > stock.quantity {
            return Err(StepError::InvalidState);
        }
        let total = held(world, stock_id, None)?;
        let available = stock
            .quantity
            .checked_sub(total)
            .ok_or(StepError::InvalidState)?;
        if quantity > available {
            emit(
                world,
                actor_id,
                2,
                pack(&[u64_bytes(stock_id), u64_bytes(quantity), vec![4]]),
                events,
            )?;
            return Ok(());
        }
    }
    let mut ids = Vec::new();
    for &(stock_id, quantity) in pairs {
        let id = world.next_record_id;
        world.next_record_id = id.checked_add(1).ok_or(StepError::CapacityExceeded)?;
        world.reservations.insert(
            id,
            Reservation {
                stock_id,
                claimant: actor_id,
                quantity,
                acquired_tick: world.tick,
                expiry_tick,
                task_id,
                state: 1,
            },
        );
        ids.push((id, stock_id, quantity));
    }
    let travel = world
        .tasks
        .get(&task_id)
        .ok_or(StepError::InvalidState)?
        .steps
        .iter()
        .try_fold(0u64, |sum, step| {
            let cost = match step {
                PathStep::Grid(_) => 1,
                PathStep::Edge(id) => world
                    .edges
                    .iter()
                    .find(|edge| edge.2 == *id)
                    .map(|edge| edge.3)
                    .ok_or(StepError::InvalidState)?,
            };
            sum.checked_add(cost).ok_or(StepError::CapacityExceeded)
        })?;
    let task = world
        .tasks
        .get_mut(&task_id)
        .ok_or(StepError::InvalidState)?;
    let changed = task.stage == 1;
    if changed {
        task.stage = 2;
    }
    for &(id, _, _) in &ids {
        task.reservations.push(id);
    }
    task.reservations.sort_unstable();
    task.due = world
        .tick
        .checked_add(travel)
        .and_then(|sum| sum.checked_add(2))
        .ok_or(StepError::CapacityExceeded)?;
    for (id, stock_id, quantity) in ids {
        emit(
            world,
            actor_id,
            1,
            pack(&[u64_bytes(id), u64_bytes(stock_id), u64_bytes(quantity)]),
            events,
        )?;
    }
    if changed {
        emit(
            world,
            actor_id,
            7,
            pack(&[u64_bytes(task_id), vec![2]]),
            events,
        )?;
    }
    Ok(())
}

fn held(world: &World, stock_id: u64, exclude_task: Option<u64>) -> Result<u64, StepError> {
    world
        .reservations
        .values()
        .filter(|reservation| {
            reservation.state == 1
                && reservation.stock_id == stock_id
                && Some(reservation.task_id) != exclude_task
        })
        .map(|reservation| reservation.quantity)
        .try_fold(0u64, |sum, quantity| {
            sum.checked_add(quantity).ok_or(StepError::CapacityExceeded)
        })
}

fn invalidate(world: &mut World, events: &mut Vec<ResourceEvent>) -> Result<(), StepError> {
    let mut task_ids: Vec<u64> = world.tasks.keys().copied().collect();
    task_ids.sort_by_key(|id| (world.tasks[id].actor_id, *id));
    for task_id in task_ids {
        let Some(task) = world.tasks.get(&task_id) else {
            continue;
        };
        if task.stage >= 6 {
            continue;
        }
        let actor = world.actors.get(&task.actor_id).expect("task actor");
        let reason = if actor.enabled == 0 {
            Some(1)
        } else if world.cancels.contains(&task_id) {
            Some(2)
        } else if task.reservations.iter().any(|id| {
            world.reservations.get(id).is_some_and(|reservation| {
                reservation.state == 1 && reservation.expiry_tick <= world.tick
            })
        }) {
            Some(3)
        } else {
            None
        };
        let Some(reason) = reason else {
            continue;
        };
        let actor_id = task.actor_id;
        let reservation_ids = task.reservations.clone();
        let unconsumed = task.unconsumed.clone();
        for id in reservation_ids {
            if let Some(reservation) = world.reservations.get_mut(&id) {
                if reservation.state == 1 {
                    reservation.state = 2;
                    emit(
                        world,
                        actor_id,
                        3,
                        pack(&[u64_bytes(id), vec![reason]]),
                        events,
                    )?;
                }
            }
        }
        let stage = if reason == 2 { 7 } else { 8 };
        if let Some(task) = world.tasks.get_mut(&task_id) {
            task.stage = stage;
            task.interruption = reason;
            task.unconsumed.clear();
        }
        for record in unconsumed {
            world.registry.insert(
                record.key,
                RegistryRecord {
                    submitted: record.submitted,
                    resolved: record.resolved,
                    role: 2,
                },
            );
        }
        emit(
            world,
            actor_id,
            7,
            pack(&[u64_bytes(task_id), vec![stage]]),
            events,
        )?;
        emit(
            world,
            actor_id,
            5,
            pack(&[u64_bytes(task_id), vec![reason]]),
            events,
        )?;
        if let Some(actor) = world.actors.get_mut(&actor_id) {
            if actor.task_id == task_id {
                actor.task_id = 0;
            }
        }
    }
    world.cancels.clear();
    Ok(())
}

fn work(world: &mut World, events: &mut Vec<ResourceEvent>) -> Result<(), StepError> {
    let ids: Vec<u64> = world
        .actors
        .values()
        .map(|actor| actor.task_id)
        .filter(|id| *id != 0)
        .collect();
    for task_id in ids {
        step_task(world, task_id, events)?;
    }
    Ok(())
}

fn step_task(
    world: &mut World,
    task_id: u64,
    events: &mut Vec<ResourceEvent>,
) -> Result<(), StepError> {
    let task = world.tasks.get(&task_id).ok_or(StepError::InvalidState)?;
    let actor = world.actors.get(&task.actor_id).expect("actor");
    if task.created == world.tick || task.stage == 1 || task.stage >= 6 {
        return Ok(());
    }
    if task.due < world.tick {
        return Err(StepError::InvalidState);
    }
    let at_target = actor.locus == task.target_locus
        && actor.site == task.target_site
        && actor.x == task.target_x
        && actor.y == task.target_y;
    match task.stage {
        2 if task.steps.is_empty() && at_target => advance(world, task_id, 4, events),
        2 if !task.steps.is_empty() => advance(world, task_id, 3, events),
        2 => Err(StepError::InvalidState),
        4 if task.goal == 0 && task.unconsumed.is_empty() => Ok(()),
        4 => advance(world, task_id, 5, events),
        5 if !task.unconsumed.is_empty() || task.due == world.tick => {
            complete(world, task_id, events)
        }
        5 => Ok(()),
        3 => Err(StepError::InvalidState),
        _ => Err(StepError::InvalidState),
    }
}

fn advance(
    world: &mut World,
    task_id: u64,
    stage: u8,
    events: &mut Vec<ResourceEvent>,
) -> Result<(), StepError> {
    let task = world.tasks.get_mut(&task_id).expect("task");
    let actor_id = task.actor_id;
    task.stage = stage;
    task.progress = task
        .progress
        .checked_add(1)
        .ok_or(StepError::CapacityExceeded)?;
    emit(
        world,
        actor_id,
        7,
        pack(&[u64_bytes(task_id), vec![stage]]),
        events,
    )
}

fn complete(
    world: &mut World,
    task_id: u64,
    events: &mut Vec<ResourceEvent>,
) -> Result<(), StepError> {
    let task = world.tasks.get(&task_id).expect("task").clone();
    if task.recipe_id != 0 && !recipe_matches(&task) {
        return Err(StepError::InvalidLeg);
    }
    check_reservation_totals(world, &task)?;
    let mut stocks = world.stocks.clone();
    let mut next_stock = world.next_stock_id;
    let mut applied = Vec::new();
    for leg in &task.legs {
        apply_leg(world, task_id, leg, &mut stocks, &mut next_stock)?;
        let mut stored = *leg;
        if stored.stock_id == 0 {
            stored.stock_id = next_stock - 1;
        }
        applied.push(stored);
    }
    world.stocks = stocks;
    world.next_stock_id = next_stock;
    let transaction_id = world.next_transaction_id;
    world.next_transaction_id = transaction_id
        .checked_add(1)
        .ok_or(StepError::CapacityExceeded)?;
    let (source, key) = if task.unconsumed.is_empty() {
        (
            2,
            CommandKey {
                target_tick: world.tick,
                actor_id: ActorId::new(task.actor_id),
                producer_id: 0,
                sequence: 0,
            },
        )
    } else {
        (1, task.unconsumed[0].key)
    };
    world.transactions.insert(
        transaction_id,
        Transaction {
            task_id,
            source,
            key,
            applied: 1,
            legs: applied,
        },
    );
    if source == 1 {
        for (index, record) in task.unconsumed.iter().enumerate() {
            world.registry.insert(
                record.key,
                RegistryRecord {
                    submitted: record.submitted,
                    resolved: record.resolved,
                    role: if index == 0 { 1 } else { 2 },
                },
            );
        }
    }
    emit(
        world,
        task.actor_id,
        4,
        pack(&[u64_bytes(transaction_id), u64_bytes(task_id)]),
        events,
    )?;
    let mut reservation_ids = task.reservations.clone();
    reservation_ids.sort_unstable();
    for id in reservation_ids {
        if let Some(reservation) = world.reservations.get_mut(&id) {
            if reservation.state == 1 {
                reservation.state = 2;
                emit(
                    world,
                    task.actor_id,
                    3,
                    pack(&[u64_bytes(id), vec![8]]),
                    events,
                )?;
            }
        }
    }
    if let Some(stored) = world.tasks.get_mut(&task_id) {
        stored.stage = 6;
        stored.interruption = 0;
        stored.unconsumed.clear();
        stored.progress = stored
            .progress
            .checked_add(1)
            .ok_or(StepError::CapacityExceeded)?;
    }
    if let Some(actor) = world.actors.get_mut(&task.actor_id) {
        if actor.task_id == task_id {
            actor.task_id = 0;
        }
    }
    emit(
        world,
        task.actor_id,
        7,
        pack(&[u64_bytes(task_id), vec![6]]),
        events,
    )
}

fn recipe_matches(task: &Task) -> bool {
    if task.recipe_id != 1 {
        return false;
    }
    let input = task
        .legs
        .iter()
        .any(|leg| matches!(leg.op, 1 | 3) && leg.definition_id == 1 && leg.quantity == 1);
    let output = task.legs.iter().any(|leg| {
        matches!(leg.op, 2 | 4)
            && leg.definition_id == 2
            && leg.quantity == 1
            && leg.owner_kind_after == 3
            && leg.owner_id_after == 2
    });
    input && output
}

fn check_reservation_totals(world: &World, task: &Task) -> Result<(), StepError> {
    let mut planned: BTreeMap<u64, u64> = BTreeMap::new();
    for leg in &task.legs {
        if matches!(leg.op, 1 | 3) {
            let sum = planned.entry(leg.stock_id).or_insert(0);
            *sum = sum
                .checked_add(leg.quantity)
                .ok_or(StepError::CapacityExceeded)?;
        }
    }
    let mut reserved: BTreeMap<u64, u64> = BTreeMap::new();
    for id in &task.reservations {
        let Some(reservation) = world.reservations.get(id) else {
            continue;
        };
        if reservation.state != 1 {
            continue;
        }
        let Some(stock) = world.stocks.get(&reservation.stock_id) else {
            continue;
        };
        if stock.class == 1 {
            let sum = reserved.entry(reservation.stock_id).or_insert(0);
            *sum = sum
                .checked_add(reservation.quantity)
                .ok_or(StepError::CapacityExceeded)?;
        }
    }
    for (stock_id, quantity) in reserved {
        if planned.get(&stock_id).copied().unwrap_or(0) != quantity {
            return Err(StepError::InvalidLeg);
        }
    }
    Ok(())
}

fn apply_leg(
    world: &World,
    task_id: u64,
    leg: &Leg,
    stocks: &mut BTreeMap<u64, Stock>,
    next_stock: &mut u64,
) -> Result<(), StepError> {
    if leg.quantity == 0 || !matches!(leg.class, 1 | 2) || leg.definition_id == 0 {
        return Err(StepError::InvalidLeg);
    }
    match leg.op {
        1 | 3 => {
            let stock = stocks.get_mut(&leg.stock_id).ok_or(StepError::InvalidLeg)?;
            if stock.definition_id != leg.definition_id || stock.class != leg.class {
                return Err(StepError::InvalidLeg);
            }
            if leg.owner_kind_after != 0 || leg.owner_id_after != 0 {
                return Err(StepError::InvalidLeg);
            }
            if stock.class == 2 {
                return Err(StepError::InvalidLeg);
            }
            let others = held(world, leg.stock_id, Some(task_id))?;
            let available = stock
                .quantity
                .checked_sub(others)
                .ok_or(StepError::InvalidLeg)?;
            if leg.quantity > available {
                return Err(StepError::InvalidLeg);
            }
            stock.quantity = stock
                .quantity
                .checked_sub(leg.quantity)
                .ok_or(StepError::InvalidLeg)?;
        }
        2 | 4 => {
            if leg.stock_id == 0 {
                validate_owner(world, leg)?;
                let id = *next_stock;
                *next_stock = id.checked_add(1).ok_or(StepError::CapacityExceeded)?;
                stocks.insert(
                    id,
                    Stock {
                        definition_id: leg.definition_id,
                        class: leg.class,
                        quantity: leg.quantity,
                        owner_kind: leg.owner_kind_after,
                        owner_id: leg.owner_id_after,
                    },
                );
            } else {
                let stock = stocks.get_mut(&leg.stock_id).ok_or(StepError::InvalidLeg)?;
                if stock.definition_id != leg.definition_id
                    || stock.class != leg.class
                    || stock.owner_kind != leg.owner_kind_after
                    || stock.owner_id != leg.owner_id_after
                {
                    return Err(StepError::InvalidLeg);
                }
                if stock.class == 2 {
                    return Err(StepError::InvalidLeg);
                }
                stock.quantity = stock
                    .quantity
                    .checked_add(leg.quantity)
                    .ok_or(StepError::InvalidLeg)?;
            }
        }
        _ => return Err(StepError::InvalidLeg),
    }
    Ok(())
}

fn validate_owner(world: &World, leg: &Leg) -> Result<(), StepError> {
    if validation::owner_exists(world, leg.owner_kind_after, leg.owner_id_after) {
        Ok(())
    } else {
        Err(StepError::InvalidLeg)
    }
}

fn emit(
    world: &mut World,
    actor_id: u32,
    kind: u8,
    payload: Vec<u8>,
    events: &mut Vec<ResourceEvent>,
) -> Result<(), StepError> {
    let id = world.next_event_id;
    world.next_event_id = id.checked_add(1).ok_or(StepError::CapacityExceeded)?;
    let event = ResourceEvent {
        id,
        tick: world.tick,
        actor_id,
        kind,
        payload,
    };
    let canonical = event.canonical_bytes();
    world.digest =
        encoding::sha256(&[b"SH:C03:EVENT:v1".as_slice(), &world.digest, &canonical].concat());
    events.push(event);
    Ok(())
}

fn pack(parts: &[Vec<u8>]) -> Vec<u8> {
    parts.iter().flat_map(|part| part.clone()).collect()
}

fn u64_bytes(value: u64) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}

pub(crate) fn checkpoint_bytes(sim: &Simulation) -> Vec<u8> {
    let world = sim.world3.as_ref().expect("fixture 3 checkpoint");
    let mut body = Vec::new();
    body.extend(TAG);
    body.extend(1u16.to_le_bytes());
    body.extend(3u32.to_le_bytes());
    let scenario = scenario_block(world.wake_interval);
    body.extend(encoding::sha256(&scenario));
    body.extend(world.tick.to_le_bytes());
    body.extend(world.seed.to_le_bytes());
    body.extend(world.next_actor_id.to_le_bytes());
    body.extend(world.next_record_id.to_le_bytes());
    body.extend(world.next_transaction_id.to_le_bytes());
    body.extend(world.next_stock_id.to_le_bytes());
    body.extend(scenario);
    push_u64(&mut body, world.actors.len() as u64);
    for (&id, actor) in &world.actors {
        body.extend(id.to_le_bytes());
        body.push(actor.enabled);
        body.push(actor.mode);
        push_u64(&mut body, actor.next_wake);
        for channel in actor.channels {
            body.extend(channel.to_le_bytes());
        }
        for role in actor.roles {
            body.extend(role.to_le_bytes());
        }
        body.extend(actor.relationship.to_le_bytes());
        push_u64(&mut body, actor.home);
        push_u64(&mut body, actor.task_id);
        body.push(actor.locus);
        push_u64(&mut body, actor.site);
        body.extend(actor.x.to_le_bytes());
        body.extend(actor.y.to_le_bytes());
        push_u64(&mut body, actor.edge);
        push_u64(&mut body, actor.traversed);
    }
    push_u64(&mut body, world.sites.len() as u64);
    for (&id, &has_grid) in &world.sites {
        push_u64(&mut body, id);
        body.push(has_grid);
    }
    push_u64(&mut body, world.holders.len() as u64);
    for (&id, &kind) in &world.holders {
        push_u64(&mut body, id);
        body.push(kind);
    }
    push_u64(&mut body, world.edges.len() as u64);
    for &(from, to, id, cost) in &world.edges {
        push_u64(&mut body, from);
        push_u64(&mut body, to);
        push_u64(&mut body, id);
        push_u64(&mut body, cost);
    }
    push_u64(&mut body, world.grid_site);
    body.extend(world.grid_w.to_le_bytes());
    body.extend(world.grid_h.to_le_bytes());
    body.extend(&world.grid);
    push_u64(&mut body, world.stocks.len() as u64);
    for (&id, stock) in &world.stocks {
        push_u64(&mut body, id);
        push_u64(&mut body, stock.definition_id);
        body.push(stock.class);
        push_u64(&mut body, stock.quantity);
        body.push(stock.owner_kind);
        push_u64(&mut body, stock.owner_id);
    }
    push_u64(&mut body, world.reservations.len() as u64);
    for (&id, reservation) in &world.reservations {
        push_u64(&mut body, id);
        push_u64(&mut body, reservation.stock_id);
        body.extend(reservation.claimant.to_le_bytes());
        push_u64(&mut body, reservation.quantity);
        push_u64(&mut body, reservation.acquired_tick);
        push_u64(&mut body, reservation.expiry_tick);
        push_u64(&mut body, reservation.task_id);
        body.push(reservation.state);
    }
    push_u64(&mut body, world.tasks.len() as u64);
    for (&id, task) in &world.tasks {
        encode_task(&mut body, id, task);
    }
    push_u64(&mut body, world.transactions.len() as u64);
    for (&id, transaction) in &world.transactions {
        push_u64(&mut body, id);
        push_u64(&mut body, transaction.task_id);
        body.push(transaction.source);
        push_u64(&mut body, transaction.key.target_tick);
        body.extend(transaction.key.actor_id.get().to_le_bytes());
        body.extend(transaction.key.producer_id.to_le_bytes());
        push_u64(&mut body, transaction.key.sequence);
        body.push(transaction.applied);
        push_u64(&mut body, transaction.legs.len() as u64);
        for leg in &transaction.legs {
            encode_leg(&mut body, leg);
        }
    }
    push_u64(&mut body, world.registry.len() as u64);
    for (key, record) in &world.registry {
        encode_key(&mut body, *key);
        push_u64(&mut body, record.submitted);
        push_u64(&mut body, record.resolved);
        body.push(record.role);
    }
    push_u64(&mut body, world.pending.len() as u64);
    for (&key, &payload) in &world.pending {
        encode_key(&mut body, key);
        encode_resource_payload(&mut body, payload);
    }
    push_u64(&mut body, world.traces.len() as u64);
    for (&actor, tail) in &world.traces {
        body.extend(actor.to_le_bytes());
        body.extend(tail);
    }
    push_u64(&mut body, world.next_event_id);
    body.extend(world.digest);
    let trailer = encoding::sha256(&body);
    body.extend(trailer);
    body
}

fn encode_task(body: &mut Vec<u8>, id: u64, task: &Task) {
    push_u64(body, id);
    body.extend(task.actor_id.to_le_bytes());
    body.push(task.goal);
    push_u64(body, task.recipe_id);
    body.push(task.stage);
    body.push(task.interruption);
    body.push(task.target_locus);
    push_u64(body, task.target_site);
    body.extend(task.target_x.to_le_bytes());
    body.extend(task.target_y.to_le_bytes());
    push_u64(body, task.progress);
    push_u64(body, task.created);
    push_u64(body, task.due);
    push_u64(body, task.unconsumed.len() as u64);
    for record in &task.unconsumed {
        encode_key(body, record.key);
        push_u64(body, record.submitted);
        push_u64(body, record.resolved);
    }
    push_u64(body, task.path_index);
    push_u64(body, task.steps.len() as u64);
    for step in &task.steps {
        match step {
            PathStep::Edge(id) => {
                body.push(1);
                push_u64(body, *id);
            }
            PathStep::Grid(direction) => body.extend([2, *direction]),
        }
    }
    push_u64(body, task.reservations.len() as u64);
    for id in &task.reservations {
        push_u64(body, *id);
    }
    push_u64(body, task.legs.len() as u64);
    for leg in &task.legs {
        encode_leg(body, leg);
    }
}

fn encode_leg(body: &mut Vec<u8>, leg: &Leg) {
    body.push(leg.op);
    push_u64(body, leg.stock_id);
    push_u64(body, leg.definition_id);
    body.push(leg.class);
    push_u64(body, leg.quantity);
    body.push(leg.owner_kind_after);
    push_u64(body, leg.owner_id_after);
}

fn encode_key(body: &mut Vec<u8>, key: CommandKey) {
    push_u64(body, key.target_tick);
    body.extend(key.actor_id.get().to_le_bytes());
    body.extend(key.producer_id.to_le_bytes());
    push_u64(body, key.sequence);
}

fn encode_resource_payload(body: &mut Vec<u8>, payload: Payload) {
    match payload {
        Payload::SetMode(mode) => body.extend([
            1,
            match mode {
                ProcessingMode::Active => 1,
                ProcessingMode::Scheduled => 2,
            },
        ]),
        Payload::CancelReservation(id) => {
            body.push(2);
            push_u64(body, id);
        }
        Payload::SetEnabled(bit) => body.extend([3, bit]),
        Payload::ReserveStock {
            stock_id,
            quantity,
            expiry_tick,
            task_id,
        } => {
            body.push(4);
            push_u64(body, stock_id);
            push_u64(body, quantity);
            push_u64(body, expiry_tick);
            push_u64(body, task_id);
        }
        Payload::ReserveBundle {
            task_id,
            expiry_tick,
            pair_count,
            stocks,
            quantities,
        } => {
            body.push(5);
            push_u64(body, task_id);
            push_u64(body, expiry_tick);
            push_u64(body, u64::from(pair_count));
            for index in 0..usize::from(pair_count) {
                push_u64(body, stocks[index]);
                push_u64(body, quantities[index]);
            }
        }
        Payload::CompleteTask(id) => {
            body.push(6);
            push_u64(body, id);
        }
        Payload::BeginTask {
            goal,
            stage,
            recipe_id,
            target_locus,
            target_site,
            target_x,
            target_y,
            due_tick,
            leg_count,
            legs,
        } => {
            body.push(7);
            body.push(goal);
            body.push(stage);
            push_u64(body, recipe_id);
            body.push(target_locus);
            push_u64(body, target_site);
            body.extend(target_x.to_le_bytes());
            body.extend(target_y.to_le_bytes());
            push_u64(body, due_tick);
            push_u64(body, u64::from(leg_count));
            for leg in legs.iter().take(usize::from(leg_count)) {
                encode_leg(body, leg);
            }
        }
        Payload::ClaimSlot | Payload::Probe { .. } | Payload::MintRecord => {
            body.push(0);
        }
    }
}

fn scenario_block(wake: u64) -> Vec<u8> {
    let mut body = Vec::new();
    push_u64(&mut body, wake);
    for row in [
        [1000i64, 0, 0, 0, 0],
        [0, 1000, 0, 0, 0],
        [0, 0, 1000, 0, 0],
        [0, 0, 0, 1000, 0],
        [0, 0, 0, 0, 1000],
    ] {
        for weight in row {
            body.extend(weight.to_le_bytes());
        }
    }
    for _ in 0..5 {
        body.extend(i64::MIN.to_le_bytes());
    }
    push_u64(&mut body, 1);
    push_u64(&mut body, 1);
    push_u64(&mut body, 1);
    push_u64(&mut body, 1);
    push_u64(&mut body, 2);
    push_u64(&mut body, 1);
    body.push(3);
    push_u64(&mut body, 2);
    body
}

fn push_u64(body: &mut Vec<u8>, value: u64) {
    body.extend(value.to_le_bytes());
}

pub(crate) fn restore(bytes: &[u8]) -> Result<Simulation, RestoreError> {
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
        return Err(RestoreError::Truncated);
    }
    let (body, trailer) = bytes.split_at(bytes.len() - 32);
    if encoding::sha256(body) != *trailer {
        return Err(RestoreError::ChecksumMismatch);
    }
    let world = decode_body(body)?;
    Ok(into_simulation(world))
}

fn decode_body(bytes: &[u8]) -> Result<World, RestoreError> {
    let mut reader = Reader {
        data: bytes,
        at: 14,
        semantic_valid: true,
    };
    let fixture_id = reader.u32()?;
    let content = reader.array32()?;
    let tick = reader.u64()?;
    let seed = reader.u64()?;
    let next_actor_id = reader.u64()?;
    let next_record_id = reader.u64()?;
    let next_transaction_id = reader.u64()?;
    let next_stock_id = reader.u64()?;
    let scenario_start = reader.at;
    let wake_interval = reader.u64()?;
    reader.skip(5 * 5 * 8)?;
    reader.skip(5 * 8)?;
    let recipe_count = reader.count(49)?;
    reader.skip(recipe_count.checked_mul(49).ok_or(RestoreError::Corrupt)?)?;
    let scenario = bytes
        .get(scenario_start..reader.at)
        .ok_or(RestoreError::Truncated)?;
    let actor_count = reader.count(131)?;
    let mut actors = BTreeMap::new();
    let mut previous = None;
    for _ in 0..actor_count {
        let id = reader.u32()?;
        reader.check_order(&mut previous, id);
        let actor = Actor {
            enabled: reader.u8()?,
            mode: reader.u8()?,
            next_wake: reader.u64()?,
            channels: [
                reader.u32()?,
                reader.u32()?,
                reader.u32()?,
                reader.u32()?,
                reader.u32()?,
            ],
            roles: [
                reader.i64()?,
                reader.i64()?,
                reader.i64()?,
                reader.i64()?,
                reader.i64()?,
            ],
            relationship: reader.i64()?,
            home: reader.u64()?,
            task_id: reader.u64()?,
            locus: reader.u8()?,
            site: reader.u64()?,
            x: reader.u32()?,
            y: reader.u32()?,
            edge: reader.u64()?,
            traversed: reader.u64()?,
        };
        actors.insert(id, actor);
    }
    let site_count = reader.count(9)?;
    let mut sites = BTreeMap::new();
    let mut previous = None;
    for _ in 0..site_count {
        let id = reader.u64()?;
        reader.check_order(&mut previous, id);
        sites.insert(id, reader.u8()?);
    }
    let holder_count = reader.count(9)?;
    let mut holders = BTreeMap::new();
    let mut previous = None;
    for _ in 0..holder_count {
        let id = reader.u64()?;
        reader.check_order(&mut previous, id);
        holders.insert(id, reader.u8()?);
    }
    let edge_count = reader.count(32)?;
    let mut edges = Vec::new();
    let mut previous = None;
    for _ in 0..edge_count {
        let edge = (reader.u64()?, reader.u64()?, reader.u64()?, reader.u64()?);
        reader.check_order(&mut previous, (edge.0, edge.1, edge.2));
        edges.push(edge);
    }
    let grid_site = reader.u64()?;
    let grid_w = reader.u32()?;
    let grid_h = reader.u32()?;
    if grid_w > 4096
        || grid_h > 4096
        || (grid_w == 0) != (grid_h == 0)
        || (grid_site == 0) != (grid_w == 0 && grid_h == 0)
    {
        return Err(RestoreError::Corrupt);
    }
    let cells = usize::try_from(grid_w.checked_mul(grid_h).ok_or(RestoreError::Corrupt)?)
        .map_err(|_| RestoreError::Corrupt)?;
    if cells > reader.remaining() {
        return Err(RestoreError::Corrupt);
    }
    let grid = reader.bytes(cells)?.to_vec();
    if grid.iter().any(|byte| *byte > 1) {
        return Err(RestoreError::Corrupt);
    }
    let stock_count = reader.count(34)?;
    let mut stocks = BTreeMap::new();
    let mut previous = None;
    for _ in 0..stock_count {
        let id = reader.u64()?;
        reader.check_order(&mut previous, id);
        stocks.insert(
            id,
            Stock {
                definition_id: reader.u64()?,
                class: reader.u8()?,
                quantity: reader.u64()?,
                owner_kind: reader.u8()?,
                owner_id: reader.u64()?,
            },
        );
    }
    let reservation_count = reader.count(53)?;
    let mut reservations = BTreeMap::new();
    let mut previous = None;
    for _ in 0..reservation_count {
        let id = reader.u64()?;
        reader.check_order(&mut previous, id);
        reservations.insert(
            id,
            Reservation {
                stock_id: reader.u64()?,
                claimant: reader.u32()?,
                quantity: reader.u64()?,
                acquired_tick: reader.u64()?,
                expiry_tick: reader.u64()?,
                task_id: reader.u64()?,
                state: reader.u8()?,
            },
        );
    }
    let task_count = reader.count(104)?;
    let mut tasks = BTreeMap::new();
    let mut previous = None;
    for _ in 0..task_count {
        let id = reader.u64()?;
        reader.check_order(&mut previous, id);
        let actor_id = reader.u32()?;
        let goal = reader.u8()?;
        let recipe_id = reader.u64()?;
        let stage = reader.u8()?;
        let interruption = reader.u8()?;
        let target_locus = reader.u8()?;
        let target_site = reader.u64()?;
        let target_x = reader.u32()?;
        let target_y = reader.u32()?;
        let progress = reader.u64()?;
        let created = reader.u64()?;
        let due = reader.u64()?;
        let unconsumed_count = reader.count(40)?;
        let mut unconsumed = Vec::new();
        for _ in 0..unconsumed_count {
            unconsumed.push(Completion {
                key: reader.key()?,
                submitted: reader.u64()?,
                resolved: reader.u64()?,
            });
        }
        let path_index = reader.u64()?;
        let step_count = reader.count(2)?;
        let mut steps = Vec::new();
        for _ in 0..step_count {
            steps.push(match reader.u8()? {
                1 => PathStep::Edge(reader.u64()?),
                2 => PathStep::Grid(reader.u8()?),
                _ => return Err(RestoreError::Corrupt),
            });
        }
        let reservation_count = reader.count(8)?;
        let mut task_reservations = Vec::new();
        for _ in 0..reservation_count {
            task_reservations.push(reader.u64()?);
        }
        let leg_count = reader.count(35)?;
        let mut legs = Vec::new();
        for _ in 0..leg_count {
            legs.push(reader.leg()?);
        }
        tasks.insert(
            id,
            Task {
                actor_id,
                goal,
                recipe_id,
                stage,
                interruption,
                target_locus,
                target_site,
                target_x,
                target_y,
                progress,
                created,
                due,
                unconsumed,
                path_index,
                steps,
                reservations: task_reservations,
                legs,
            },
        );
    }
    let transaction_count = reader.count(50)?;
    let mut transactions = BTreeMap::new();
    let mut previous = None;
    for _ in 0..transaction_count {
        let id = reader.u64()?;
        reader.check_order(&mut previous, id);
        let task_id = reader.u64()?;
        let source = reader.u8()?;
        let key = reader.key()?;
        let applied = reader.u8()?;
        let leg_count = reader.count(35)?;
        let mut legs = Vec::new();
        for _ in 0..leg_count {
            legs.push(reader.leg()?);
        }
        transactions.insert(
            id,
            Transaction {
                task_id,
                source,
                key,
                applied,
                legs,
            },
        );
    }
    let tables_sorted = reader.semantic_valid;
    reader.semantic_valid = true;
    let registry_count = reader.count(41)?;
    let mut registry = BTreeMap::new();
    let mut previous = None;
    for _ in 0..registry_count {
        let key = reader.key()?;
        reader.check_order(&mut previous, key);
        registry.insert(
            key,
            RegistryRecord {
                submitted: reader.u64()?,
                resolved: reader.u64()?,
                role: reader.u8()?,
            },
        );
    }
    let pending_count = reader.count(26)?;
    let mut pending = BTreeMap::new();
    let mut previous = None;
    for _ in 0..pending_count {
        let key = reader.key()?;
        reader.check_order(&mut previous, key);
        pending.insert(key, reader.payload()?);
    }
    let keys_sorted = reader.semantic_valid;
    reader.semantic_valid = true;
    let trace_count = reader.count(324)?;
    let mut traces = BTreeMap::new();
    let mut previous = None;
    for _ in 0..trace_count {
        let actor = reader.u32()?;
        reader.check_order(&mut previous, actor);
        traces.insert(actor, reader.bytes(320)?.to_vec());
    }
    let next_event_id = reader.u64()?;
    let digest = reader.array32()?;
    if reader.at != bytes.len() {
        return Err(RestoreError::Corrupt);
    }
    if fixture_id != 3 {
        return Err(RestoreError::UnknownFixture { found: fixture_id });
    }
    if encoding::sha256(scenario) != content
        || !matches!(wake_interval, 4 | 100)
        || scenario != scenario_block(wake_interval)
    {
        return Err(RestoreError::ContentMismatch);
    }
    let traces_sorted = reader.semantic_valid;
    let world = World {
        tick,
        seed,
        wake_interval,
        next_actor_id,
        next_record_id,
        next_transaction_id,
        next_stock_id,
        next_event_id,
        digest,
        actors,
        sites,
        holders,
        edges,
        grid_site,
        grid_w,
        grid_h,
        grid,
        stocks,
        reservations,
        tasks,
        transactions,
        registry,
        pending,
        traces,
        cancels: BTreeSet::new(),
    };
    validation::validate(&world, tables_sorted, keys_sorted, traces_sorted)?;
    Ok(world)
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
    semantic_valid: bool,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.data.len() - self.at
    }

    fn count(&mut self, minimum: u64) -> Result<usize, RestoreError> {
        let count = self.u64()?;
        let size = count.checked_mul(minimum).ok_or(RestoreError::Corrupt)?;
        if count == u64::MAX || size > self.remaining() as u64 {
            return Err(RestoreError::Corrupt);
        }
        usize::try_from(count).map_err(|_| RestoreError::Corrupt)
    }

    fn check_order<T: Copy + Ord>(&mut self, previous: &mut Option<T>, value: T) {
        self.semantic_valid &= previous.is_none_or(|old| old < value);
        *previous = Some(value);
    }
    fn take(&mut self, len: usize) -> Result<&'a [u8], RestoreError> {
        let end = self.at.checked_add(len).ok_or(RestoreError::Truncated)?;
        let bytes = self.data.get(self.at..end).ok_or(RestoreError::Truncated)?;
        self.at = end;
        Ok(bytes)
    }

    fn u8(&mut self) -> Result<u8, RestoreError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, RestoreError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, RestoreError> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64, RestoreError> {
        Ok(self.u64()? as i64)
    }

    fn array32(&mut self) -> Result<[u8; 32], RestoreError> {
        let bytes = self.take(32)?;
        Ok(bytes.try_into().unwrap())
    }

    fn bytes(&mut self, len: usize) -> Result<&'a [u8], RestoreError> {
        self.take(len)
    }

    fn skip(&mut self, len: usize) -> Result<(), RestoreError> {
        self.take(len).map(|_| ())
    }

    fn key(&mut self) -> Result<CommandKey, RestoreError> {
        Ok(CommandKey {
            target_tick: self.u64()?,
            actor_id: ActorId::new(self.u32()?),
            producer_id: self.u32()?,
            sequence: self.u64()?,
        })
    }

    fn leg(&mut self) -> Result<Leg, RestoreError> {
        Ok(Leg {
            op: self.u8()?,
            stock_id: self.u64()?,
            definition_id: self.u64()?,
            class: self.u8()?,
            quantity: self.u64()?,
            owner_kind_after: self.u8()?,
            owner_id_after: self.u64()?,
        })
    }

    fn payload(&mut self) -> Result<Payload, RestoreError> {
        match self.u8()? {
            1 => Ok(Payload::SetMode(match self.u8()? {
                1 => ProcessingMode::Active,
                2 => ProcessingMode::Scheduled,
                _ => return Err(RestoreError::Corrupt),
            })),
            2 => Ok(Payload::CancelReservation(self.u64()?)),
            3 => Ok(Payload::SetEnabled(self.u8()?)),
            4 => Ok(Payload::ReserveStock {
                stock_id: self.u64()?,
                quantity: self.u64()?,
                expiry_tick: self.u64()?,
                task_id: self.u64()?,
            }),
            5 => {
                let task_id = self.u64()?;
                let expiry_tick = self.u64()?;
                let pair_count = self.count(16)?;
                if pair_count > 4 {
                    return Err(RestoreError::Corrupt);
                }
                let mut stocks = [0; 4];
                let mut quantities = [0; 4];
                for index in 0..pair_count {
                    stocks[index] = self.u64()?;
                    quantities[index] = self.u64()?;
                }
                Ok(Payload::ReserveBundle {
                    task_id,
                    expiry_tick,
                    pair_count: u8::try_from(pair_count).unwrap(),
                    stocks,
                    quantities,
                })
            }
            6 => Ok(Payload::CompleteTask(self.u64()?)),
            7 => {
                let goal = self.u8()?;
                let stage = self.u8()?;
                let recipe_id = self.u64()?;
                let target_locus = self.u8()?;
                let target_site = self.u64()?;
                let target_x = self.u32()?;
                let target_y = self.u32()?;
                let due_tick = self.u64()?;
                let leg_count = self.count(35)?;
                if leg_count > 4 {
                    return Err(RestoreError::Corrupt);
                }
                let mut legs = [Leg {
                    op: 0,
                    stock_id: 0,
                    definition_id: 0,
                    class: 0,
                    quantity: 0,
                    owner_kind_after: 0,
                    owner_id_after: 0,
                }; 4];
                for leg in legs.iter_mut().take(leg_count) {
                    *leg = self.leg()?;
                }
                Ok(Payload::BeginTask {
                    goal,
                    stage,
                    recipe_id,
                    target_locus,
                    target_site,
                    target_x,
                    target_y,
                    due_tick,
                    leg_count: u8::try_from(leg_count).unwrap(),
                    legs,
                })
            }
            _ => Err(RestoreError::Corrupt),
        }
    }
}
