//! Bevy host adapter; all gameplay is delegated to the frozen sealed_sim crate.
pub mod agency;
pub mod ascii;
pub mod clock;
pub mod locale;
pub mod persistence;
pub mod supply;
pub mod view;

use bevy::prelude::*;
use locale::Locale;
use sealed_sim::{Command, ResourceEvent, Simulation};
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
};
use view::View;

pub const DEMO: &[u8] = include_bytes!("../assets/demo.c03");
pub const TICK_NS: u128 = 500_000_000;

#[derive(Clone, Debug)]
pub enum Action {
    Pause,
    Resume,
    Step,
    Reset,
    Save,
    Load,
    Select(u32),
    Language(Locale),
}

/// Explicit frame input; tests and the native input system use the same plugin.
#[derive(Resource, Default)]
pub struct FrameInput {
    pub elapsed_ns: u128,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickEvidence {
    pub tick: u64,
    pub checkpoint: Vec<u8>,
    pub digest: [u8; 32],
    pub next_event: u64,
    pub events: Vec<Vec<u8>>,
}

#[derive(Resource)]
pub struct Session {
    sim: Simulation,
    pub view: View,
    pub paused: bool,
    pub selected: u32,
    pub locale: Locale,
    pub status: String,
    pub diagnostic: String,
    pub recent: VecDeque<ResourceEvent>,
    pub save_path: PathBuf,
    pub debt_ns: u128,
    /// Explicit command schedule indexed by submission boundary. No FPS inputs.
    pub schedule: BTreeMap<u64, Vec<Command>>,
    /// Optional per-tick evidence, enabled by finite tests (not runtime copies).
    pub capture: bool,
    pub evidence: Vec<TickEvidence>,
}

impl Default for Session {
    fn default() -> Self {
        let (sim, view) = View::validated(DEMO).expect("authored C03 checkpoint must validate");
        Self {
            sim,
            view,
            paused: true,
            selected: 1,
            locale: Locale::English,
            status: "status.ready".into(),
            diagnostic: String::new(),
            recent: VecDeque::new(),
            save_path: persistence::default_path(),
            debt_ns: 0,
            schedule: BTreeMap::new(),
            capture: false,
            evidence: Vec::new(),
        }
    }
}

impl Session {
    pub fn tick(&self) -> u64 {
        self.sim.tick()
    }
    pub fn checkpoint(&self) -> Vec<u8> {
        self.sim.domain_checkpoint_bytes()
    }
    pub fn hash(&self) -> String {
        hex(&self.sim.domain_checkpoint_sha256())
    }
    pub fn digest(&self) -> String {
        hex(&self.sim.event_digest())
    }
    pub fn next_event(&self) -> u64 {
        self.sim.next_event_id()
    }

    fn fail(&mut self, error: String) {
        self.paused = true;
        self.debt_ns = 0;
        self.status = "status.error".into();
        self.diagnostic = error;
    }

    fn advance(&mut self) -> bool {
        let tick = self.tick();
        let batch = self.schedule.get(&tick).cloned().unwrap_or_default();
        // Clone preserves both kernel and view publication if projection fails.
        let mut candidate = self.sim.clone();
        match candidate.drive(tick, &batch) {
            Err(error) => {
                self.fail(format!("{error:?}"));
                false
            }
            Ok(output) => {
                let view = match View::from_sim(&candidate) {
                    Ok(view) => view,
                    Err(error) => {
                        self.fail(error);
                        return false;
                    }
                };
                if self.capture {
                    self.evidence.push(TickEvidence {
                        tick: candidate.tick(),
                        checkpoint: candidate.domain_checkpoint_bytes(),
                        digest: candidate.event_digest(),
                        next_event: candidate.next_event_id(),
                        events: output
                            .resource_events()
                            .iter()
                            .map(ResourceEvent::canonical_bytes)
                            .collect(),
                    });
                }
                self.sim = candidate;
                self.view = view;
                self.recent.extend(output.resource_events().iter().cloned());
                while self.recent.len() > 128 {
                    self.recent.pop_front();
                }
                true
            }
        }
    }

    pub fn act(&mut self, action: Action) {
        match action {
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
            Action::Select(id) => {
                if self.view.actors.iter().any(|a| a.id == id) {
                    self.selected = id;
                }
            }
            Action::Language(locale) => self.locale = locale,
            Action::Reset => match View::validated(DEMO) {
                Ok((sim, view)) => {
                    self.sim = sim;
                    self.view = view;
                    self.paused = true;
                    self.debt_ns = 0;
                    self.recent.clear();
                    self.evidence.clear();
                    self.schedule.clear();
                    self.selected = 1;
                    self.status = "status.reset".into();
                    self.diagnostic.clear();
                }
                Err(error) => self.fail(error),
            },
            Action::Save => match persistence::save(&self.save_path, &self.checkpoint()) {
                Ok(()) => {
                    self.status = "status.saved".into();
                    self.diagnostic.clear();
                }
                Err(error) => self.fail(error),
            },
            Action::Load => {
                match persistence::load(&self.save_path).and_then(|bytes| View::validated(&bytes)) {
                    Ok((sim, view)) => {
                        self.sim = sim;
                        self.view = view;
                        self.paused = true;
                        self.debt_ns = 0;
                        self.recent.clear();
                        self.evidence.clear();
                        if !self.view.actors.iter().any(|a| a.id == self.selected) {
                            self.selected = self.view.actors.first().map_or(0, |a| a.id);
                        }
                        self.status = "status.loaded".into();
                        self.diagnostic.clear();
                    }
                    Err(error) => self.fail(error),
                }
            }
        }
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub struct SimulationPlugin;
impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Session>()
            .init_resource::<FrameInput>()
            .add_systems(Update, drive_frame);
    }
}

/// Same entry point for native Bevy and deterministic frame-partition tests.
pub fn drive_frame(mut session: ResMut<Session>, mut input: ResMut<FrameInput>) {
    // Any transport action starts a fresh interval; do not count elapsed time
    // from the frame preceding a resume/load/reset or double-count a step.
    let mut transport = false;
    for action in std::mem::take(&mut input.actions) {
        transport |= matches!(
            action,
            Action::Pause | Action::Resume | Action::Reset | Action::Load
        ) || matches!(action, Action::Step) && session.paused;
        session.act(action);
    }
    let elapsed = std::mem::take(&mut input.elapsed_ns);
    if !session.paused && !transport {
        match session.debt_ns.checked_add(elapsed) {
            Some(debt) => session.debt_ns = debt,
            None => {
                session.fail("clock.overflow".into());
                return;
            }
        }
        for _ in 0..8 {
            if session.debt_ns < TICK_NS {
                break;
            }
            if !session.advance() {
                break;
            }
            session.debt_ns -= TICK_NS;
        }
    }
}
