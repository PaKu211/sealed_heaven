//! Room demand recorded beside the canonical ledger. Stock quantities live only
//! in the restored C0.3 checkpoint.

use sealed_sim::{ActorId, Command, CommandKey, Payload, Simulation};

use crate::kernel::{self, read_ledger};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Desk {
    pub demand: u64,
    pub open_task: Option<u64>,
    pub dispatches: u64,
    pub duplicates: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dispatch {
    Watching,
    Assigned,
    Duplicate,
    Satisfied,
}

impl Desk {
    pub fn new(demand: u64) -> Self {
        Self {
            demand,
            open_task: None,
            dispatches: 0,
            duplicates: 0,
        }
    }

    /// Records the kernel's current task. It does not mint a second reservation.
    pub fn observe(&mut self, task_id: u64, shrine_quantity: u64) -> Dispatch {
        if shrine_quantity >= self.demand && task_id == 0 {
            self.open_task = None;
            return Dispatch::Satisfied;
        }
        if task_id == 0 {
            return Dispatch::Watching;
        }
        if self.open_task == Some(task_id) {
            self.duplicates += 1;
            return Dispatch::Duplicate;
        }
        self.open_task = Some(task_id);
        self.dispatches += 1;
        Dispatch::Assigned
    }
}

#[derive(Clone, Debug)]
pub struct TransferReport {
    pub ticks: u64,
    pub shrine_quantity: u64,
    pub conserved: u64,
    pub dispatches: u64,
    pub duplicates: u64,
    pub checkpoint_equal: bool,
    pub rejected_drive_preserved: bool,
}

pub fn shrine_checkpoint() -> Vec<u8> {
    let scenario = kernel::frozen_scenario();
    let mut out = b"SH:C03:STATE".to_vec();
    out.extend(1u16.to_le_bytes());
    out.extend(3u32.to_le_bytes());
    out.extend(crate::sha256(&scenario));
    kernel::u64s(&mut out, &[0, 100, 2, 1, 1, 3]);
    out.extend(scenario);
    kernel::u64s(&mut out, &[1]);
    out.extend(1u32.to_le_bytes());
    out.extend([1, 1]);
    kernel::u64s(&mut out, &[1]);
    for channel in 0..5 {
        out.extend((if channel == 3 { 1000u32 } else { 0 }).to_le_bytes());
    }
    kernel::u64s(&mut out, &[0; 6]);
    kernel::u64s(&mut out, &[2, 0]);
    out.push(1);
    kernel::u64s(&mut out, &[1]);
    out.extend(0u32.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    kernel::u64s(&mut out, &[0, 0]);
    kernel::u64s(&mut out, &[3]);
    for (id, grid) in [(1u64, 1u8), (2, 0), (9, 0)] {
        kernel::u64s(&mut out, &[id]);
        out.push(grid);
    }
    kernel::u64s(&mut out, &[2]);
    for (id, kind) in [(20u64, 2u8), (30, 4)] {
        kernel::u64s(&mut out, &[id]);
        out.push(kind);
    }
    kernel::u64s(&mut out, &[2, 1, 2, 1, 1, 2, 1, 2, 1]);
    kernel::u64s(&mut out, &[1]);
    out.extend(3u32.to_le_bytes());
    out.extend(3u32.to_le_bytes());
    out.extend([1, 1, 1, 1, 0, 1, 1, 1, 1]);
    kernel::u64s(&mut out, &[2]);
    kernel::stock(&mut out, 1, 7, 1, 12, 1, 1);
    kernel::stock(&mut out, 2, 7, 1, 0, 3, 2);
    kernel::u64s(&mut out, &[0, 0, 0, 0, 0, 0, 1]);
    out.extend(crate::sha256(b"SH:C03:EVENT-INIT:v1"));
    let checksum = crate::sha256(&out);
    out.extend(checksum);
    out
}

pub fn run_transfer() -> Result<TransferReport, String> {
    let initial = shrine_checkpoint();
    let mut sim = Simulation::restore_domain_checkpoint(&initial)
        .map_err(|error| format!("shrine restore: {error:?}"))?;
    if sim.domain_checkpoint_bytes() != initial {
        return Err("shrine checkpoint is not canonical".into());
    }
    let preserved = sim.domain_checkpoint_bytes();
    let rejected = sim.drive(
        0,
        &[Command::new(
            CommandKey {
                target_tick: 1,
                actor_id: ActorId::new(99),
                producer_id: 1,
                sequence: 0,
            },
            Payload::CancelReservation(1),
        )],
    );
    let rejected_drive_preserved = rejected.is_err() && sim.domain_checkpoint_bytes() == preserved;
    let mut desk = Desk::new(5);
    let conserved = herb_total(&sim)?;
    let mut shrine_quantity;
    let mut ticks = 0u64;
    for tick in 0..80 {
        let view = read_ledger(&sim.domain_checkpoint_bytes())?;
        let task_id = view.actors.first().ok_or("missing hauler")?.task_id;
        shrine_quantity = shrine_total(&sim)?;
        let total = herb_total(&sim)?;
        if total != conserved {
            return Err(format!("herb total changed from {conserved} to {total}"));
        }
        let first = desk.observe(task_id, shrine_quantity);
        if task_id != 0 {
            let second = desk.observe(task_id, shrine_quantity);
            if second != Dispatch::Duplicate {
                return Err(format!(
                    "same task was dispatched twice as {second:?} after {first:?}"
                ));
            }
        }
        if shrine_quantity >= desk.demand && task_id == 0 {
            ticks = tick;
            break;
        }
        sim.drive(tick, &[])
            .map_err(|error| format!("logistics tick {tick}: {error:?}"))?;
        ticks = tick + 1;
    }
    shrine_quantity = shrine_total(&sim)?;
    if shrine_quantity < 5 {
        return Err(format!(
            "shrine received {shrine_quantity} herbs within {ticks} ticks"
        ));
    }
    let mut replay = Simulation::restore_domain_checkpoint(&initial)
        .map_err(|error| format!("replay restore: {error:?}"))?;
    for tick in 0..ticks {
        replay
            .drive(tick, &[])
            .map_err(|error| format!("replay tick {tick}: {error:?}"))?;
    }
    Ok(TransferReport {
        ticks,
        shrine_quantity,
        conserved,
        dispatches: desk.dispatches,
        duplicates: desk.duplicates,
        checkpoint_equal: replay.domain_checkpoint_bytes() == sim.domain_checkpoint_bytes(),
        rejected_drive_preserved,
    })
}

fn herb_total(sim: &Simulation) -> Result<u64, String> {
    Ok(read_ledger(&sim.domain_checkpoint_bytes())?
        .stocks
        .iter()
        .filter(|stock| stock.definition_id == 7)
        .map(|stock| stock.quantity)
        .sum())
}

fn shrine_total(sim: &Simulation) -> Result<u64, String> {
    Ok(read_ledger(&sim.domain_checkpoint_bytes())?
        .stocks
        .iter()
        .filter(|stock| stock.definition_id == 7 && stock.owner_kind == 3 && stock.owner_id == 2)
        .map(|stock| stock.quantity)
        .sum())
}
