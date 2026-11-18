//! One hauler moves actor-owned spirit herbs onto the shrine site stock.
//!
//! Quantities exist only in the C0.3 checkpoint. The desk records demand and which
//! kernel task it has already seen. It does not keep a second balance, and the
//! personal stock is not a room storehouse.

use sealed_sim::{ActorId, Command, CommandKey, Payload, Simulation};
use sha2::{Digest, Sha256};

use crate::view::{StockView, View};

pub const HERB: u64 = 7;
pub const CARRIER_STOCK: u64 = 1;
pub const SHRINE_STOCK: u64 = 2;
pub const HAULER: u32 = 1;
pub const SCENARIO_DEMAND: u64 = 5;
pub const SCENARIO_HERBS: u64 = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Requested,
    Assigned,
    InProgress,
    Completed,
    Rejected,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Seen {
    Watching,
    Assigned,
    Duplicate,
    Satisfied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Desk {
    demand: u64,
    open_task: Option<u64>,
    dispatches: u64,
    duplicates: u64,
}

impl Desk {
    fn new(demand: u64) -> Self {
        Self {
            demand,
            open_task: None,
            dispatches: 0,
            duplicates: 0,
        }
    }

    fn observe(&mut self, task_id: u64, shrine_quantity: u64) -> Seen {
        if shrine_quantity >= self.demand && task_id == 0 {
            self.open_task = None;
            return Seen::Satisfied;
        }
        if task_id == 0 {
            return Seen::Watching;
        }
        if self.open_task == Some(task_id) {
            self.duplicates += 1;
            return Seen::Duplicate;
        }
        self.open_task = Some(task_id);
        self.dispatches += 1;
        Seen::Assigned
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    pub demand: u64,
    pub carrier: u64,
    pub shrine: u64,
    pub total: u64,
    pub worker: u32,
    pub task_id: u64,
    pub goal: Option<u8>,
    pub stage: Option<u8>,
    pub deliveries: u64,
    pub duplicate_observations: u64,
    pub phase: Phase,
}

#[derive(Clone, Debug)]
pub struct Supply {
    sim: Simulation,
    desk: Desk,
    held: bool,
    phase: Phase,
}

impl Supply {
    pub fn shrine() -> Self {
        Self::open(SCENARIO_HERBS, SCENARIO_DEMAND)
    }

    pub fn open(carrier: u64, demand: u64) -> Self {
        let bytes = checkpoint(carrier);
        let sim = Simulation::restore_domain_checkpoint(&bytes)
            .expect("the shrine supply checkpoint must restore");
        Self {
            sim,
            desk: Desk::new(demand),
            held: false,
            phase: Phase::Requested,
        }
    }

    pub fn demand(&self) -> u64 {
        self.desk.demand
    }

    /// A second request does not raise demand and does not mint herbs.
    pub fn request(&mut self, _amount: u64) -> bool {
        false
    }

    pub fn status(&self) -> Result<Status, String> {
        let view = View::from_sim(&self.sim)?;
        let carrier = quantity(&view.stocks, CARRIER_STOCK)?;
        let shrine = quantity(&view.stocks, SHRINE_STOCK)?;
        let total = view
            .stocks
            .iter()
            .filter(|stock| stock.definition == HERB)
            .map(|stock| stock.quantity)
            .sum();
        let actor = view
            .actors
            .iter()
            .find(|actor| actor.id == HAULER)
            .ok_or("supply hauler missing")?;
        let task = view.tasks.iter().find(|task| task.id == actor.task);
        let kernel_carrier = self
            .sim
            .resource_stock_quantity(CARRIER_STOCK)
            .ok_or("carrier stock missing")?;
        let kernel_shrine = self
            .sim
            .resource_stock_quantity(SHRINE_STOCK)
            .ok_or("shrine stock missing")?;
        if kernel_carrier != carrier || kernel_shrine != shrine {
            return Err("checkpoint projection disagrees with the kernel stock".into());
        }
        Ok(Status {
            demand: self.desk.demand,
            carrier,
            shrine,
            total,
            worker: actor.id,
            task_id: actor.task,
            goal: task.map(|task| task.goal),
            stage: task.map(|task| task.stage),
            deliveries: self.desk.dispatches,
            duplicate_observations: self.desk.duplicates,
            phase: self.phase,
        })
    }

    pub fn checkpoint(&self) -> Vec<u8> {
        self.sim.domain_checkpoint_bytes()
    }

    pub fn restore(bytes: &[u8], demand: u64) -> Result<Self, String> {
        let (sim, view) = View::validated(bytes)?;
        let total = view
            .stocks
            .iter()
            .filter(|stock| stock.definition == HERB)
            .map(|stock| stock.quantity)
            .sum();
        let target = sim.domain_checkpoint_bytes();
        let mut supply = Self::open(total, demand);
        if supply.checkpoint() == target {
            return Ok(supply);
        }
        for _ in 0..80 {
            supply.step()?;
            if supply.checkpoint() == target {
                return Ok(supply);
            }
        }
        Err("saved supply is not a replay of the actor-owned stack".into())
    }

    /// Advances the kernel until demand is met and the hauler is clear, then stops.
    /// Stopping is required: the kernel would otherwise keep hauling the rest of
    /// the actor-owned stack, because demand is not a kernel quota.
    pub fn step(&mut self) -> Result<(), String> {
        if self.held {
            return Ok(());
        }
        let status = self.status()?;
        let seen = self.desk.observe(status.task_id, status.shrine);
        if seen == Seen::Satisfied {
            self.held = true;
            self.phase = Phase::Completed;
            return Ok(());
        }
        self.phase = if status.task_id == 0 {
            Phase::Requested
        } else if matches!(seen, Seen::Assigned) {
            Phase::Assigned
        } else {
            Phase::InProgress
        };
        let tick = self.sim.tick();
        self.sim
            .drive(tick, &[])
            .map_err(|error| format!("supply tick {tick}: {error:?}"))?;
        Ok(())
    }

    pub fn cancel_open(&mut self) -> Result<(), String> {
        let tick = self.sim.tick();
        self.sim
            .drive(
                tick,
                &[Command::new(
                    CommandKey {
                        target_tick: tick + 1,
                        actor_id: ActorId::new(HAULER),
                        producer_id: 1,
                        sequence: tick,
                    },
                    Payload::CancelReservation(0),
                )],
            )
            .map_err(|error| format!("cancel: {error:?}"))?;
        let status = self.status()?;
        self.phase = if status.task_id == 0 {
            Phase::Cancelled
        } else {
            Phase::InProgress
        };
        if status.shrine >= self.desk.demand && status.task_id == 0 {
            self.held = true;
            self.phase = Phase::Completed;
        }
        Ok(())
    }

    pub fn reject_unknown(&mut self) -> Result<(), String> {
        let before = self.checkpoint();
        let tick = self.sim.tick();
        let rejected = self.sim.drive(
            tick,
            &[Command::new(
                CommandKey {
                    target_tick: tick + 1,
                    actor_id: ActorId::new(99),
                    producer_id: 1,
                    sequence: 0,
                },
                Payload::CancelReservation(1),
            )],
        );
        if rejected.is_ok() || self.checkpoint() != before {
            return Err("unknown worker mutated the checkpoint".into());
        }
        self.phase = Phase::Rejected;
        Ok(())
    }

    pub fn run_until_met(&mut self) -> Result<Status, String> {
        for _ in 0..80 {
            let status = self.status()?;
            if status.shrine >= self.desk.demand && status.task_id == 0 && self.held {
                return self.status();
            }
            self.step()?;
        }
        Err("demand was not met".into())
    }
}

fn quantity(stocks: &[StockView], id: u64) -> Result<u64, String> {
    stocks
        .iter()
        .find(|stock| stock.id == id)
        .map(|stock| stock.quantity)
        .ok_or_else(|| format!("stock {id} missing"))
}

fn checkpoint(carrier: u64) -> Vec<u8> {
    let scenario = frozen_scenario();
    let mut out = b"SH:C03:STATE".to_vec();
    out.extend(1u16.to_le_bytes());
    out.extend(3u32.to_le_bytes());
    out.extend(sha256(&scenario));
    u64s(&mut out, &[0, 100, 2, 1, 1, 3]);
    out.extend(scenario);
    u64s(&mut out, &[1]);
    out.extend(1u32.to_le_bytes());
    out.extend([1, 1]);
    u64s(&mut out, &[1]);
    for channel in 0..5 {
        out.extend((if channel == 3 { 1000u32 } else { 0 }).to_le_bytes());
    }
    u64s(&mut out, &[0; 6]);
    u64s(&mut out, &[2, 0]);
    out.push(1);
    u64s(&mut out, &[1]);
    out.extend(0u32.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    u64s(&mut out, &[0, 0]);
    u64s(&mut out, &[3]);
    for (id, grid) in [(1u64, 1u8), (2, 0), (9, 0)] {
        u64s(&mut out, &[id]);
        out.push(grid);
    }
    u64s(&mut out, &[2]);
    for (id, kind) in [(20u64, 2u8), (30, 4)] {
        u64s(&mut out, &[id]);
        out.push(kind);
    }
    u64s(&mut out, &[2, 1, 2, 1, 1, 2, 1, 2, 1]);
    u64s(&mut out, &[1]);
    out.extend(3u32.to_le_bytes());
    out.extend(3u32.to_le_bytes());
    out.extend([1, 1, 1, 1, 0, 1, 1, 1, 1]);
    u64s(&mut out, &[2]);
    stock(&mut out, CARRIER_STOCK, HERB, 1, carrier, 1, 1);
    stock(&mut out, SHRINE_STOCK, HERB, 1, 0, 3, 2);
    u64s(&mut out, &[0, 0, 0, 0, 0, 0, 1]);
    out.extend(sha256(b"SH:C03:EVENT-INIT:v1"));
    let checksum = sha256(&out);
    out.extend(checksum);
    out
}

fn frozen_scenario() -> Vec<u8> {
    let mut scenario = Vec::new();
    u64s(&mut scenario, &[4]);
    for row in 0..5 {
        for col in 0..5 {
            scenario.extend((if row == col { 1000i64 } else { 0 }).to_le_bytes());
        }
    }
    for _ in 0..5 {
        scenario.extend(i64::MIN.to_le_bytes());
    }
    u64s(&mut scenario, &[1, 1, 1, 1, 2, 1]);
    scenario.push(3);
    u64s(&mut scenario, &[2]);
    scenario
}

fn stock(
    out: &mut Vec<u8>,
    id: u64,
    definition: u64,
    class: u8,
    quantity: u64,
    kind: u8,
    owner: u64,
) {
    u64s(out, &[id, definition]);
    out.push(class);
    u64s(out, &[quantity]);
    out.push(kind);
    u64s(out, &[owner]);
}

fn u64s(out: &mut Vec<u8>, values: &[u64]) {
    for value in values {
        out.extend(value.to_le_bytes());
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
