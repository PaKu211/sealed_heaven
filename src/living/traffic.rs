//! Revision-eight bounded, deterministic traffic courtesy.
//! Occupancy remains exclusive. A willing neighbor may exchange one tile, but
//! a character may not be moved twice in the same canonical turn.

use std::collections::{BTreeMap, BTreeSet};

use super::navigation::{distance, next_step};
use super::{ActorId, Goal, PLAYER, Position, Region, Rejection, Role, World};

const YIELD_AFTER_STALLS: u8 = 2;
const HISTORY_LIMIT: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrafficEventKind {
    PlayerPass,
    NpcPass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrafficEvent {
    pub at: u64,
    pub mover: ActorId,
    pub other: ActorId,
    pub from: Position,
    pub to: Position,
    pub kind: TrafficEventKind,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Traffic {
    pub stalls: BTreeMap<ActorId, u8>,
    pub swaps: u32,
    pub approach_steps: u32,
    pub history: Vec<TrafficEvent>,
    /// Only a turn-local reservation; empty at every committed boundary.
    committed: BTreeSet<ActorId>,
}

impl Traffic {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn begin_tick(&mut self) {
        self.committed.clear();
    }

    pub(super) fn finish_tick(&mut self) {
        self.committed.clear();
    }

    pub(super) fn was_committed(&self, id: ActorId) -> bool {
        self.committed.contains(&id)
    }

    pub(super) fn mark_committed(&mut self, id: ActorId) {
        self.committed.insert(id);
    }

    fn clear_stall(&mut self, id: ActorId) {
        self.stalls.remove(&id);
    }

    fn stall(&mut self, id: ActorId) -> u8 {
        let count = self.stalls.entry(id).or_default();
        *count = count.saturating_add(1).min(YIELD_AFTER_STALLS);
        *count
    }
}

impl World {
    pub fn traffic(&self) -> Option<&Traffic> {
        self.traffic.as_ref()
    }

    /// Player-initiated swaps are an explicit movement command. Unfriendly,
    /// dead, or actively treating/teaching NPCs cannot be pushed aside.
    pub(super) fn move_player_with_courtesy(
        &mut self,
        destination: Position,
    ) -> Result<(), Rejection> {
        if let Some(other) = self
            .npcs
            .values()
            .find(|npc| npc.alive() && npc.position == destination)
            .map(|npc| npc.id())
        {
            if !self.can_yield(other, PLAYER) {
                return Err(Rejection::Occupied);
            }
            let old_player = self.player.position;
            self.npcs.get_mut(&other).expect("adjacent NPC").position = old_player;
            self.player.position = destination;
            self.traffic
                .as_mut()
                .expect("traffic rules")
                .mark_committed(other);
            self.record_traffic_event(
                PLAYER,
                other,
                old_player,
                destination,
                TrafficEventKind::PlayerPass,
            );
            return Ok(());
        }
        if self.occupied(destination) {
            return Err(Rejection::Occupied);
        }
        self.player.position = destination;
        Ok(())
    }

    /// Existing NPC path planners still seek the shortest dynamic route.
    /// When all such routes are blocked, approach the static choke instead of
    /// freezing at a distant tile, and request a deterministic adjacent swap.
    pub(super) fn travel_with_courtesy(&mut self, id: ActorId, destination: Position, reach: u32) {
        let position = self.npcs[&id].position;
        if distance(position, destination) <= reach {
            self.traffic
                .as_mut()
                .expect("traffic rules")
                .clear_stall(id);
            return;
        }
        if let Some(next) = next_step(self.scenario, position, destination, reach, |cell| {
            self.walkable(cell) && !self.occupied(cell)
        }) {
            self.npcs.get_mut(&id).expect("moving NPC").position = next;
            self.traffic
                .as_mut()
                .expect("traffic rules")
                .clear_stall(id);
            return;
        }

        let Some(ideal) = next_step(self.scenario, position, destination, reach, |cell| {
            self.walkable(cell)
        }) else {
            self.traffic
                .as_mut()
                .expect("traffic rules")
                .clear_stall(id);
            return;
        };
        if !self.occupied(ideal) {
            self.npcs.get_mut(&id).expect("approaching NPC").position = ideal;
            let traffic = self.traffic.as_mut().expect("traffic rules");
            traffic.clear_stall(id);
            traffic.approach_steps = traffic.approach_steps.saturating_add(1);
            return;
        }
        let blocker = self
            .npcs
            .values()
            .find(|npc| npc.alive() && npc.position == ideal)
            .map(|npc| npc.id());
        let stalls = self.traffic.as_mut().expect("traffic rules").stall(id);
        if let Some(blocker) = blocker
            && stalls >= YIELD_AFTER_STALLS
            && self.can_yield(blocker, id)
            && !self
                .traffic()
                .expect("traffic rules")
                .was_committed(blocker)
        {
            self.npcs.get_mut(&id).expect("moving NPC").position = ideal;
            self.npcs.get_mut(&blocker).expect("yielding NPC").position = position;
            let traffic = self.traffic.as_mut().expect("traffic rules");
            traffic.mark_committed(id);
            traffic.mark_committed(blocker);
            traffic.clear_stall(id);
            traffic.clear_stall(blocker);
            self.record_traffic_event(id, blocker, position, ideal, TrafficEventKind::NpcPass);
        }
    }

    fn can_yield(&self, blocker: ActorId, mover: ActorId) -> bool {
        let npc = &self.npcs[&blocker];
        if !npc.alive() || self.social.relationship(blocker, mover).refuses_aid() {
            return false;
        }
        if matches!(
            npc.activity.goal,
            Goal::Treat(_) | Goal::FieldCare | Goal::Retaliate | Goal::Retreat
        ) {
            return false;
        }
        if npc.definition.role == Role::Medic && npc.position == self.scenario.clinic {
            return false;
        }
        if self.is_hall_teacher(npc) && npc.position == Region::sites().hall {
            return false;
        }
        true
    }

    fn record_traffic_event(
        &mut self,
        mover: ActorId,
        other: ActorId,
        from: Position,
        to: Position,
        kind: TrafficEventKind,
    ) {
        let witnessed = mover == PLAYER || self.player_sees(from) || self.player_sees(to);
        let traffic = self.traffic.as_mut().expect("traffic rules");
        traffic.swaps = traffic.swaps.saturating_add(1);
        if traffic.history.len() == HISTORY_LIMIT {
            traffic.history.remove(0);
        }
        traffic.history.push(TrafficEvent {
            at: self.tick,
            mover,
            other,
            from,
            to,
            kind,
            witnessed,
        });
    }
}
