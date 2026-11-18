//! C05 player agency, composed with (not substituted for) the frozen C03 NPC kernel.
//!
//! A drive first advances the single embedded NPC simulation, then evaluates at most
//! one new player action against that committed snapshot. NPCs retain their original
//! collision-free rules: a later NPC arrival may overlap the player, who can leave.
//! C05 herbs have their own finite, typed ledger; they are never C03 stocks.

use std::collections::BTreeMap;

use crate::{RestoreError, Simulation, StepError, encoding::sha256};

mod codec;
#[cfg(test)]
mod tests;

pub const PLAYER_ID: u32 = 21;
pub const WIDTH: u32 = 12;
pub const HEIGHT: u32 = 9;
pub const MAX_COMMANDS: usize = 4096;
pub const MAX_BATCH: usize = 4096;
const MAX_NPC_BYTES: usize = 16 * 1024 * 1024;
const DEMO: &[u8] = include_bytes!("../apps/sealed_playable/assets/demo.c03");
const START: Position = Position { x: 0, y: 4 };
const PATCH: Position = Position { x: 9, y: 2 };
const SHRINE: Position = Position { x: 3, y: 5 };

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    Npc(u32),
    HerbPatch,
    Shrine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Intent {
    Move(Direction),
    Talk(u32),
    Gather,
    Deliver,
}

/// Numeric field order is canonical precedence; only actor 21 / producer 1 is admitted.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Key {
    pub target_tick: u64,
    pub actor_id: u32,
    pub producer_id: u32,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Command {
    pub key: Key,
    pub intent: Intent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    OutOfBounds,
    Terrain,
    Occupied,
    OutOfRange,
    UnknownTarget,
    RequestNotAccepted,
    Empty,
    AlreadyCompleted,
    Overflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Moved,
    Requested,
    Gathered,
    Delivered,
    Rejected(Rejection),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    pub id: u64,
    pub tick: u64,
    pub key: Key,
    pub outcome: Outcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    WrongBoundary { expected: u64, received: u64 },
    InvalidKey(Key),
    ConflictingIdentity(Key),
    Capacity,
    Restore(RestoreError),
    Npc(StepError),
}

/// Owner identities are specific to the C05 herb ledger, not C03 owner tags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HerbOwner {
    Source,
    Player(u32),
    DesignatedShrine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Holding {
    owner: HerbOwner,
    quantity: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Record {
    intent: Intent,
    outcome: Outcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    npcs: Simulation,
    position: Position,
    herbs: [Holding; 3],
    requested: bool,
    completed: bool,
    registry: BTreeMap<Key, Record>,
    next_sequence: u64,
    next_event_id: u64,
    digest: [u8; 32],
}

impl World {
    pub fn settlement() -> Self {
        Self {
            npcs: Simulation::restore_domain_checkpoint(DEMO)
                .expect("the authored C03 demo must remain valid"),
            position: START,
            herbs: [
                Holding {
                    owner: HerbOwner::Source,
                    quantity: 3,
                },
                Holding {
                    owner: HerbOwner::Player(PLAYER_ID),
                    quantity: 0,
                },
                Holding {
                    owner: HerbOwner::DesignatedShrine,
                    quantity: 0,
                },
            ],
            requested: false,
            completed: false,
            registry: BTreeMap::new(),
            next_sequence: 0,
            next_event_id: 1,
            digest: initial_digest(),
        }
    }

    pub fn tick(&self) -> u64 {
        self.npcs.tick()
    }
    pub fn player_position(&self) -> Position {
        self.position
    }
    pub fn player_herbs(&self) -> u64 {
        self.herbs[1].quantity
    }
    pub fn source_herbs(&self) -> u64 {
        self.herbs[0].quantity
    }
    pub fn delivered_herbs(&self) -> u64 {
        self.herbs[2].quantity
    }
    pub fn requested(&self) -> bool {
        self.requested
    }
    pub fn completed(&self) -> bool {
        self.completed
    }
    pub fn npcs(&self) -> &Simulation {
        &self.npcs
    }
    pub fn event_digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Enabled actors only. Traversing actors have no committed map cell.
    pub fn npc_position(&self, id: u32) -> Option<Position> {
        let actor = self.npcs.world3.as_ref()?.actors.get(&id)?;
        if actor.enabled != 1 {
            return None;
        }
        match (actor.locus, actor.site) {
            (1, 1) => Some(Position { x: 3, y: 3 }),
            (1, 2) => Some(Position { x: 9, y: 5 }),
            (1, 9) => Some(Position { x: 10, y: 7 }),
            (2, 1) => Some(Position {
                x: 3u32.checked_add(actor.x)?,
                y: 3u32.checked_add(actor.y)?,
            }),
            _ => None,
        }
    }

    /// Immutable terrain only; use `occupied` separately for current NPC occupancy.
    /// The two immovable interactable cells are not walkable terrain.
    pub fn is_walkable(&self, position: Position) -> bool {
        walkable(position)
    }

    /// Includes immovable interactables and enabled NPCs, but not the player itself.
    pub fn occupied(&self, position: Position) -> bool {
        position == PATCH
            || position == SHRINE
            || (1..=20).any(|id| self.npc_position(id) == Some(position))
    }

    pub fn target_position(&self, target: Target) -> Option<Position> {
        match target {
            Target::Npc(id) => self.npc_position(id),
            Target::HerbPatch => Some(PATCH),
            Target::Shrine => Some(SHRINE),
        }
    }

    pub fn in_range(&self, target: Target) -> bool {
        self.target_position(target)
            .is_some_and(|p| adjacent(self.position, p))
    }

    pub fn checkpoint(&self) -> Vec<u8> {
        codec::encode(self)
    }
    pub fn restore(bytes: &[u8]) -> Result<Self, Error> {
        codec::decode(bytes)
    }
    pub fn hash(&self) -> [u8; 32] {
        sha256(&self.checkpoint())
    }

    /// Exact registered retries publish no event but still advance NPCs one tick.
    /// New commands must target `boundary + 1` and have sequence >= the durable
    /// highwater. At most one new unique command is allowed (duplicates deduplicate).
    /// Every fatal admission or NPC failure preserves the entire previous checkpoint.
    pub fn drive(&mut self, boundary: u64, batch: &[Command]) -> Result<Vec<Event>, Error> {
        if boundary != self.tick() {
            return Err(Error::WrongBoundary {
                expected: self.tick(),
                received: boundary,
            });
        }
        let tick = boundary.checked_add(1).ok_or(Error::Capacity)?;
        if batch.len() > MAX_BATCH {
            return Err(Error::Capacity);
        }
        let mut ordered = batch.to_vec();
        ordered.sort_unstable_by_key(|command| command.key);
        for &Command { key, .. } in &ordered {
            if key.actor_id != PLAYER_ID || key.producer_id != 1 {
                return Err(Error::InvalidKey(key));
            }
        }
        for pair in ordered.windows(2) {
            if pair[0].key == pair[1].key && pair[0].intent != pair[1].intent {
                return Err(Error::ConflictingIdentity(pair[0].key));
            }
        }
        ordered.dedup_by_key(|command| command.key);
        let mut fresh = None;
        for &Command { key, intent } in &ordered {
            if let Some(record) = self.registry.get(&key) {
                if record.intent != intent {
                    return Err(Error::ConflictingIdentity(key));
                }
                continue;
            }
            if key.target_tick != tick || key.sequence < self.next_sequence {
                return Err(Error::InvalidKey(key));
            }
            key.sequence.checked_add(1).ok_or(Error::Capacity)?;
            if fresh.replace(Command { key, intent }).is_some() {
                return Err(Error::Capacity);
            }
        }
        if fresh.is_some() {
            if self.registry.len() >= MAX_COMMANDS {
                return Err(Error::Capacity);
            }
            self.next_event_id.checked_add(1).ok_or(Error::Capacity)?;
        }
        let mut staged = self.clone();
        staged.npcs.drive(boundary, &[]).map_err(Error::Npc)?;
        let mut events = Vec::new();
        if let Some(Command { key, intent }) = fresh {
            let outcome = staged.apply(intent);
            let event = Event {
                id: staged.next_event_id,
                tick,
                key,
                outcome,
            };
            staged.next_event_id += 1;
            staged.next_sequence = key.sequence + 1;
            staged.digest = extend_digest(staged.digest, event);
            staged.registry.insert(key, Record { intent, outcome });
            events.push(event);
        }
        if staged.npcs.domain_checkpoint_bytes().len() > MAX_NPC_BYTES {
            return Err(Error::Capacity);
        }
        *self = staged;
        Ok(events)
    }

    fn apply(&mut self, intent: Intent) -> Outcome {
        let result = match intent {
            Intent::Move(direction) => self.move_player(direction),
            Intent::Talk(id) => {
                if id != 1 {
                    Err(Rejection::UnknownTarget)
                } else if !self.in_range(Target::Npc(id)) {
                    Err(Rejection::OutOfRange)
                } else {
                    self.requested = true;
                    Ok(Outcome::Requested)
                }
            }
            Intent::Gather => {
                if !self.in_range(Target::HerbPatch) {
                    Err(Rejection::OutOfRange)
                } else if !self.requested {
                    Err(Rejection::RequestNotAccepted)
                } else if self.source_herbs() == 0 {
                    Err(Rejection::Empty)
                } else {
                    self.transfer(0, 1).map(|()| Outcome::Gathered)
                }
            }
            Intent::Deliver => {
                if !self.in_range(Target::Shrine) {
                    Err(Rejection::OutOfRange)
                } else if !self.requested {
                    Err(Rejection::RequestNotAccepted)
                } else if self.completed {
                    Err(Rejection::AlreadyCompleted)
                } else if self.player_herbs() == 0 {
                    Err(Rejection::Empty)
                } else {
                    self.transfer(1, 2).map(|()| {
                        self.completed = true;
                        Outcome::Delivered
                    })
                }
            }
        };
        result.unwrap_or_else(Outcome::Rejected)
    }

    fn move_player(&mut self, direction: Direction) -> Result<Outcome, Rejection> {
        let destination = moved(self.position, direction).ok_or(Rejection::OutOfBounds)?;
        if destination.x >= WIDTH || destination.y >= HEIGHT {
            return Err(Rejection::OutOfBounds);
        }
        if !walkable(destination) {
            return Err(Rejection::Terrain);
        }
        if self.occupied(destination) {
            return Err(Rejection::Occupied);
        }
        self.position = destination;
        Ok(Outcome::Moved)
    }

    fn transfer(&mut self, from: usize, to: usize) -> Result<(), Rejection> {
        let debit = self.herbs[from]
            .quantity
            .checked_sub(1)
            .ok_or(Rejection::Empty)?;
        let credit = self.herbs[to]
            .quantity
            .checked_add(1)
            .ok_or(Rejection::Overflow)?;
        self.herbs[from].quantity = debit;
        self.herbs[to].quantity = credit;
        Ok(())
    }
}

fn walkable(p: Position) -> bool {
    p.x < WIDTH
        && p.y < HEIGHT
        && p != PATCH
        && p != SHRINE
        && !(p.x == 6 && (1..=6).contains(&p.y) && p.y != 4)
}

fn adjacent(a: Position, b: Position) -> bool {
    u64::from(a.x.abs_diff(b.x)) + u64::from(a.y.abs_diff(b.y)) <= 1
}

fn moved(p: Position, d: Direction) -> Option<Position> {
    Some(match d {
        Direction::North => Position {
            x: p.x,
            y: p.y.checked_sub(1)?,
        },
        Direction::East => Position {
            x: p.x.checked_add(1)?,
            y: p.y,
        },
        Direction::South => Position {
            x: p.x,
            y: p.y.checked_add(1)?,
        },
        Direction::West => Position {
            x: p.x.checked_sub(1)?,
            y: p.y,
        },
    })
}

fn initial_digest() -> [u8; 32] {
    sha256(b"SH:C05:EVENT-INIT:v1")
}

fn extend_digest(previous: [u8; 32], event: Event) -> [u8; 32] {
    let mut bytes = b"SH:C05:EVENT:v1".to_vec();
    bytes.extend(previous);
    bytes.extend(event.id.to_le_bytes());
    bytes.extend(event.tick.to_le_bytes());
    codec::put_key(&mut bytes, event.key);
    codec::put_outcome(&mut bytes, event.outcome);
    sha256(&bytes)
}
