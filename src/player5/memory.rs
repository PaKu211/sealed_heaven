//! One fixture-scoped causal memory: witness 2 reports a player-caused foe death
//! after two successful canonical ticks; healer 1 becomes wary exactly once.
//! No renderer or wall clock owns the incident, deadline or attitude.

use super::*;

pub const WITNESS_ID: u32 = 2;
pub const WITNESS_RADIUS: u32 = 4;
pub const REPORT_DELAY: u64 = 2;

pub(super) fn witnessed_at(witness: Option<Position>, death: Position) -> bool {
    witness.is_some_and(|position| manhattan(position, death) <= WITNESS_RADIUS)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Incident {
    pub death_event_id: u64,
    pub tick: u64,
    pub witness_id: u32,
    pub report_tick: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryKind {
    WitnessedDeath,
    ReportDelivered,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryEvent {
    pub id: u64,
    pub tick: u64,
    pub death_event_id: u64,
    pub kind: MemoryKind,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalMemory {
    pub incident: Option<Incident>,
    pub reported: bool,
    /// The C0.9 fixture has only two possible states: neutral (0), wary (-1).
    pub healer_standing: i8,
    /// Reconstructed by exact replay and not stored redundantly in the checkpoint.
    pub log: Vec<MemoryEvent>,
}

impl World {
    /// Called after NPC, player and hostile phases, including on empty world ticks.
    pub(super) fn advance_memory(&mut self) -> Result<(), Error> {
        if self.memory.is_none() {
            return Ok(());
        }
        let tick = self.tick();
        if self.memory.as_ref().unwrap().incident.is_none() {
            let player_death_event = self
                .combat
                .as_ref()
                .and_then(|combat| combat.log.last())
                .copied()
                .filter(|event| {
                    event.tick == tick
                        && event.source == PLAYER_ID
                        && event.target == FOE_ID
                        && event.kind == CombatKind::Death
                });
            if let Some(death) = player_death_event {
                let witnessed = self
                    .combat
                    .as_ref()
                    .is_some_and(|combat| witnessed_at(self.npc_position(WITNESS_ID), combat.foe));
                if witnessed {
                    self.memory.as_mut().unwrap().incident = Some(Incident {
                        death_event_id: death.id,
                        tick,
                        witness_id: WITNESS_ID,
                        report_tick: tick + REPORT_DELAY,
                    });
                    self.emit_memory(MemoryKind::WitnessedDeath)?;
                }
            }
        }
        let due = self.memory.as_ref().unwrap();
        if !due.reported
            && due
                .incident
                .is_some_and(|incident| tick >= incident.report_tick)
        {
            let memory = self.memory.as_mut().unwrap();
            memory.reported = true;
            memory.healer_standing = -1;
            self.emit_memory(MemoryKind::ReportDelivered)?;
        }
        Ok(())
    }

    fn emit_memory(&mut self, kind: MemoryKind) -> Result<(), Error> {
        let id = self.next_event_id;
        self.next_event_id = id.checked_add(1).ok_or(Error::Capacity)?;
        let incident = self.memory.as_ref().unwrap().incident.unwrap();
        let event = MemoryEvent {
            id,
            tick: self.tick(),
            death_event_id: incident.death_event_id,
            kind,
        };
        let mut bytes = b"SH:C09:MEMORY-EVENT:v1".to_vec();
        bytes.extend(self.digest);
        bytes.extend(event.id.to_le_bytes());
        bytes.extend(event.tick.to_le_bytes());
        bytes.extend(event.death_event_id.to_le_bytes());
        bytes.push(match kind {
            MemoryKind::WitnessedDeath => 1,
            MemoryKind::ReportDelivered => 2,
        });
        self.digest = sha256(&bytes);
        self.memory.as_mut().unwrap().log.push(event);
        Ok(())
    }
}
