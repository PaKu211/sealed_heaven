//! Bounded evaluation of a threshold energy queue.
//! It is not a second simulation tick and it does not drive `sealed_sim`.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Actor {
    pub entity_id: u32,
    pub speed: u32,
    pub energy: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnergyScheduler {
    pub threshold: i64,
    actors: Vec<Actor>,
}

impl EnergyScheduler {
    pub fn new(threshold: i64) -> Self {
        Self {
            threshold,
            actors: Vec::new(),
        }
    }

    pub fn insert(&mut self, actor: Actor) {
        let index = self
            .actors
            .binary_search_by_key(&actor.entity_id, |item| item.entity_id)
            .unwrap_or_else(|index| index);
        self.actors.insert(index, actor);
    }

    pub fn actors(&self) -> &[Actor] {
        &self.actors
    }

    /// Advances every positive-speed actor by the same deterministic delta.
    /// Speed zero is incapacitated and is never used as a divisor.
    pub fn advance_to_ready(&mut self) -> Option<u32> {
        if !self.actors.iter().any(|actor| actor.speed > 0) {
            return None;
        }
        if let Some(id) = self.ready() {
            return Some(id);
        }
        let min_ticks = self
            .actors
            .iter()
            .filter(|actor| actor.speed > 0)
            .map(|actor| {
                let needed = (self.threshold - actor.energy).max(1);
                let speed = i64::from(actor.speed);
                (needed + speed - 1) / speed
            })
            .min()
            .unwrap_or(1);
        for actor in &mut self.actors {
            if actor.speed > 0 {
                actor.energy += i64::from(actor.speed) * min_ticks;
            }
        }
        self.ready()
    }

    pub fn consume_action(&mut self, entity_id: u32, cost: i64) {
        if let Some(actor) = self
            .actors
            .iter_mut()
            .find(|actor| actor.entity_id == entity_id)
        {
            actor.energy -= cost;
        }
    }

    fn ready(&self) -> Option<u32> {
        self.actors
            .iter()
            .filter(|actor| actor.speed > 0 && actor.energy >= self.threshold)
            .max_by_key(|actor| (actor.energy, actor.speed, -(i64::from(actor.entity_id))))
            .map(|actor| actor.entity_id)
    }
}
