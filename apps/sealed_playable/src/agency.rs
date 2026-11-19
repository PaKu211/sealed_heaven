//! C0.5 host adapter. This module stamps inputs; it contains no gameplay rules.
pub mod persistence;

use crate::locale::Locale;
use crate::supply::Supply;
use bevy::prelude::*;
use sealed_sim::player5::{Command, Event, Intent, Key, PLAYER_ID, Target, World};
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

pub const TICK_NS: u128 = crate::TICK_NS;
pub const MAX_INPUTS: usize = 32;

#[derive(Clone, Debug)]
pub enum Action {
    Intent(Intent),
    Select(Target),
    Pause,
    Resume,
    Step,
    Save,
    Load,
    Reset,
    Language(Locale),
}

#[derive(Resource, Default)]
pub struct Input {
    pub elapsed_ns: u128,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub tick: u64,
    pub checkpoint: Vec<u8>,
    pub events: Vec<Event>,
}

#[derive(Resource)]
pub struct Session {
    world: World,
    pub paused: bool,
    pub locale: Locale,
    pub selected: Target,
    pub status: String,
    pub diagnostic: String,
    pub save_path: PathBuf,
    pub debt_ns: u128,
    queue: VecDeque<Intent>,
    pub recent: VecDeque<Event>,
    pub admitted: VecDeque<Command>,
    /// Test/replay input already stamped at boundaries, never inferred from FPS.
    pub schedule: BTreeMap<u64, Vec<Command>>,
    pub capture: bool,
    pub evidence: Vec<Evidence>,
    /// Wall-time observations only; never inputs to domain rules/checkpoints.
    pub tick_samples_ns: VecDeque<u128>,
    supply: Supply,
    conflict_scenario: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            world: World::settlement(),
            paused: true,
            locale: Locale::English,
            selected: Target::Npc(1),
            status: "c05.ready".into(),
            diagnostic: String::new(),
            save_path: persistence::default_path(),
            debt_ns: 0,
            queue: VecDeque::new(),
            recent: VecDeque::new(),
            admitted: VecDeque::new(),
            schedule: BTreeMap::new(),
            capture: false,
            evidence: Vec::new(),
            tick_samples_ns: VecDeque::new(),
            supply: Supply::shrine(),
            conflict_scenario: false,
        }
    }
}

impl Session {
    pub fn conflict() -> Self {
        Self {
            world: World::conflict(),
            conflict_scenario: true,
            ..Self::default()
        }
    }
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn supply(&self) -> &Supply {
        &self.supply
    }

    fn supply_path(&self) -> PathBuf {
        self.save_path.with_file_name("shrine-supply.c03")
    }

    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    fn fail(&mut self, error: String) {
        self.paused = true;
        self.debt_ns = 0;
        self.status = "status.error".into();
        self.diagnostic = error;
    }

    fn advance(&mut self) -> bool {
        let tick = self.world.tick();
        let mut batch = self.schedule.get(&tick).cloned().unwrap_or_default();
        let queued = self.queue.front().copied();
        if let Some(intent) = queued {
            batch.push(Command {
                key: Key {
                    target_tick: match tick.checked_add(1) {
                        Some(value) => value,
                        None => {
                            self.fail("clock.overflow".into());
                            return false;
                        }
                    },
                    actor_id: PLAYER_ID,
                    producer_id: 1,
                    sequence: self.world.next_sequence(),
                },
                intent,
            });
        }
        let started = std::time::Instant::now();
        let result = self.world.drive(tick, &batch);
        self.tick_samples_ns.push_back(started.elapsed().as_nanos());
        self.tick_samples_ns.truncate_front(600);
        match result {
            Err(error) => {
                self.fail(format!("{error:?}"));
                false
            }
            Ok(events) => {
                if queued.is_some() {
                    self.queue.pop_front();
                }
                if !events.is_empty() {
                    self.status = "c05.resolved".into();
                    self.diagnostic.clear();
                }
                self.admitted.extend(batch);
                self.recent.extend(events.iter().cloned());
                self.admitted.truncate_front(256);
                self.recent.truncate_front(32);
                if self.capture {
                    self.evidence.push(Evidence {
                        tick: self.world.tick(),
                        checkpoint: self.world.checkpoint(),
                        events,
                    });
                }
                if let Err(error) = self.supply.step() {
                    self.diagnostic = error;
                }
                true
            }
        }
    }

    pub fn act(&mut self, action: Action) {
        match action {
            Action::Intent(intent) => {
                if self.queue.len() == MAX_INPUTS {
                    self.status = "c05.queue_full".into();
                } else {
                    self.queue.push_back(intent);
                    self.status = "c05.queued".into();
                }
            }
            Action::Select(target) => self.selected = target,
            Action::Language(locale) => self.locale = locale,
            Action::Pause => {
                self.paused = true;
                self.debt_ns = 0;
            }
            Action::Resume => {
                self.paused = false;
                self.debt_ns = 0;
            }
            Action::Step if self.paused => {
                self.advance();
            }
            Action::Step => {}
            Action::Save => {
                // The village file is a C0.5 world. The hauler file is a C0.3
                // ledger checkpoint and must not pass through World::restore.
                match persistence::save(&self.save_path, &self.world.checkpoint()).and_then(|_| {
                    crate::persistence::save(&self.supply_path(), &self.supply.checkpoint())
                }) {
                    Ok(()) => {
                        self.status = "status.saved".into();
                        self.diagnostic.clear();
                    }
                    Err(error) => self.fail(error),
                }
            }
            Action::Load => {
                match persistence::load(&self.save_path)
                    .and_then(|bytes| World::restore(&bytes).map_err(|error| format!("{error:?}")))
                    .and_then(|world| {
                        let bytes = crate::persistence::load(&self.supply_path())?;
                        let supply = Supply::restore(&bytes, self.supply.demand())?;
                        Ok((world, supply))
                    }) {
                    Ok((world, supply)) => {
                        self.conflict_scenario = world.combat().is_some();
                        self.world = world;
                        self.supply = supply;
                        self.clear_transient();
                        self.status = "status.loaded".into();
                    }
                    Err(error) => self.fail(error),
                }
            }
            Action::Reset => {
                self.world = if self.conflict_scenario {
                    World::conflict()
                } else {
                    World::settlement()
                };
                self.supply = Supply::shrine();
                self.clear_transient();
                self.status = "status.reset".into();
            }
        }
    }

    fn clear_transient(&mut self) {
        self.paused = true;
        self.debt_ns = 0;
        self.queue.clear();
        self.schedule.clear();
        self.recent.clear();
        self.admitted.clear();
        self.evidence.clear();
        self.diagnostic.clear();
    }
}

trait TruncateFront {
    fn truncate_front(&mut self, limit: usize);
}
impl<T> TruncateFront for VecDeque<T> {
    fn truncate_front(&mut self, limit: usize) {
        while self.len() > limit {
            self.pop_front();
        }
    }
}

pub struct AgencyPlugin;
impl Plugin for AgencyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Session>()
            .init_resource::<Input>()
            .add_systems(Update, drive_frame);
    }
}

pub fn drive_frame(mut session: ResMut<Session>, mut input: ResMut<Input>) {
    let actions = std::mem::take(&mut input.actions);
    let mut transport = false;
    let mut steps = 0u32;
    let mut admitted = Vec::new();
    // Admit every non-step action before any step. A same-frame Step therefore
    // cannot advance past an intent that arrived later in the raw action list.
    // The same intent is admitted once per frame even when another action sits
    // between the two copies.
    for action in actions {
        match action {
            Action::Step => steps = steps.saturating_add(1),
            Action::Intent(intent) => {
                if admitted.contains(&intent) {
                    continue;
                }
                admitted.push(intent);
                session.act(Action::Intent(intent));
            }
            other => {
                transport |= matches!(
                    other,
                    Action::Pause | Action::Resume | Action::Load | Action::Reset
                );
                session.act(other);
            }
        }
    }
    if steps > 0 && session.paused {
        transport = true;
        session.act(Action::Step);
    }
    let elapsed = std::mem::take(&mut input.elapsed_ns);
    if !session.paused && !transport {
        let Some(debt) = session.debt_ns.checked_add(elapsed) else {
            session.fail("clock.overflow".into());
            return;
        };
        session.debt_ns = debt;
        for _ in 0..8 {
            if session.debt_ns < TICK_NS || !session.advance() {
                break;
            }
            session.debt_ns -= TICK_NS;
        }
    }
}
