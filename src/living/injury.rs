//! Wounds outlast hit points and make escape, retaliation and treatment matter.
//! Physical care draws from the same finite shrine stock as ordinary medicine.

use std::collections::BTreeMap;

use super::navigation::in_reach;
use super::{ActorId, Goal, Outcome, PLAYER, Position, Rejection, Role, Scenario, World};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Wounds {
    pub arms: u8,
    pub legs: u8,
}

impl Wounds {
    pub fn severe(self) -> bool {
        self.arms >= 2 || self.legs >= 2
    }

    pub fn injured(self) -> bool {
        self.arms > 0 || self.legs > 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WoundKind {
    Arms,
    Legs,
    Treated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WoundEvent {
    pub id: u32,
    pub at: u64,
    pub actor: ActorId,
    pub kind: WoundKind,
    pub site: Position,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Injuries {
    pub people: BTreeMap<ActorId, Wounds>,
    pub events: Vec<WoundEvent>,
}

impl Injuries {
    pub(super) fn new(scenario: &Scenario) -> Self {
        let people = std::iter::once((PLAYER, Wounds::default()))
            .chain(scenario.npcs.iter().map(|npc| (npc.id, Wounds::default())))
            .collect();
        Self {
            people,
            events: Vec::new(),
        }
    }
}

impl World {
    pub fn injuries(&self) -> Option<&Injuries> {
        self.injuries.as_ref()
    }

    pub(super) fn hobbled(&self, actor: ActorId) -> bool {
        self.injuries()
            .is_some_and(|state| state.people[&actor].legs >= 2)
            && self.tick % 2 == 1
    }

    pub(super) fn wounded_strike(&self, actor: ActorId, power: u16) -> u16 {
        if self
            .injuries()
            .is_some_and(|state| state.people[&actor].arms >= 2)
        {
            power.saturating_sub(1)
        } else {
            power
        }
    }

    pub(super) fn severe_wounds(&self, actor: ActorId) -> bool {
        self.injuries()
            .is_some_and(|state| state.people[&actor].severe())
    }

    pub(super) fn record_wound(&mut self, actor: ActorId, harm: u16, remaining: u16, power: u16) {
        let limit = if actor == PLAYER {
            super::PLAYER_HP
        } else {
            self.npcs[&actor].definition.hp
        };
        if self.injuries.is_none() || harm < 2 || remaining == 0 || remaining > limit / 2 {
            return;
        }
        let wounds = self
            .injuries
            .as_mut()
            .unwrap()
            .people
            .get_mut(&actor)
            .unwrap();
        let kind = if power >= 3 {
            if wounds.arms == 2 {
                return;
            }
            wounds.arms = wounds.arms.saturating_add(1).min(2);
            WoundKind::Arms
        } else {
            if wounds.legs == 2 {
                return;
            }
            wounds.legs = wounds.legs.saturating_add(1).min(2);
            WoundKind::Legs
        };
        self.log_wound(actor, kind);
    }

    pub(super) fn treat_wounds(&mut self) -> Result<Outcome, Rejection> {
        self.require_reach(self.scenario.clinic)?;
        if !self.injuries().unwrap().people[&PLAYER].injured() {
            return Err(Rejection::Uninjured);
        }
        let medic = self.npcs.values().any(|npc| {
            npc.alive()
                && npc.definition.role == Role::Medic
                && npc.position == self.scenario.clinic
                && !self.social.relationship(npc.id(), PLAYER).refuses_aid()
        });
        if !medic {
            return Err(Rejection::HealerAway);
        }
        if self.supply.shrine == 0 {
            return Err(Rejection::Empty);
        }
        self.supply.shrine -= 1;
        self.supply.consumed += 1;
        self.player.hp = (self.player.hp + 1).min(super::PLAYER_HP);
        self.reduce_wound(PLAYER);
        Ok(Outcome::WoundTreated)
    }

    pub(super) fn wound_goal(&self, actor: ActorId) -> Option<Goal> {
        let wounds = self.injuries()?.people[&actor];
        if !wounds.injured() || !self.npcs[&actor].alive() || self.supply.shrine == 0 {
            return None;
        }
        self.npcs
            .values()
            .any(|npc| {
                npc.alive()
                    && npc.definition.role == Role::Medic
                    && !self.social.relationship(npc.id(), actor).refuses_aid()
            })
            .then_some(Goal::SeekWoundCare)
    }

    pub(super) fn work_wound_care(&mut self, actor: ActorId) {
        let site = self.scenario.clinic;
        if !in_reach(self.npcs[&actor].position, site) {
            self.travel(actor, site, 1);
            return;
        }
        if self.supply.shrine == 0
            || !self.npcs.values().any(|npc| {
                npc.alive()
                    && npc.definition.role == Role::Medic
                    && npc.position == site
                    && !self.social.relationship(npc.id(), actor).refuses_aid()
            })
        {
            return;
        }
        self.supply.shrine -= 1;
        self.supply.consumed += 1;
        let hp = self.npcs[&actor].hp;
        self.npcs.get_mut(&actor).unwrap().hp = (hp + 1).min(self.npcs[&actor].definition.hp);
        self.reduce_wound(actor);
    }

    fn reduce_wound(&mut self, actor: ActorId) {
        let wound = self
            .injuries
            .as_mut()
            .unwrap()
            .people
            .get_mut(&actor)
            .unwrap();
        if wound.arms >= wound.legs && wound.arms > 0 {
            wound.arms -= 1;
        } else if wound.legs > 0 {
            wound.legs -= 1;
        }
        self.log_wound(actor, WoundKind::Treated);
    }

    fn log_wound(&mut self, actor: ActorId, kind: WoundKind) {
        let site = if actor == PLAYER {
            self.player.position
        } else {
            self.npcs[&actor].position
        };
        let witnessed = self.player.alive() && self.perceives(self.player.position, site);
        let state = self.injuries.as_mut().unwrap();
        state.events.push(WoundEvent {
            id: u32::try_from(state.events.len()).expect("bounded wound log"),
            at: self.tick,
            actor,
            kind,
            site,
            witnessed,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Direction, Intent};

    fn turn(world: &mut World, intent: Intent) -> Outcome {
        world.step(world.tick(), intent).unwrap()
    }

    #[test]
    fn guarded_damage_does_not_invent_wounds_and_neither_do_legacy_rules() {
        let mut world = World::persistent_injury(0);
        let mentor = ActorId::new(33);
        world.player.position = world.npcs[&mentor].position;
        world.damage(mentor, PLAYER, 1, false);
        assert_eq!(world.injuries().unwrap().people[&PLAYER], Wounds::default());
        world.player.hp = 4;
        world.contested.as_mut().unwrap().wards = 1;
        world.damage(mentor, PLAYER, 2, false);
        assert_eq!(world.player.hp, 3);
        assert_eq!(world.injuries().unwrap().people[&PLAYER], Wounds::default());
        world.damage(mentor, PLAYER, 2, false);
        assert_eq!(world.injuries().unwrap().people[&PLAYER].legs, 1);
        assert_eq!(world.social().incidents().last().unwrap().damage, 2);
        let old = World::meridian_cultivation(0);
        assert!(old.injuries().is_none());
        assert_eq!(World::restore(&old.checkpoint()).unwrap(), old);
    }

    #[test]
    fn distant_injuries_record_the_site_without_forging_player_knowledge() {
        let mut world = World::persistent_injury(0);
        let patient = ActorId::new(35);
        let site = world.npcs[&patient].position;
        world.npcs.get_mut(&patient).unwrap().hp = 3;
        world.damage(ActorId::new(33), patient, 2, false);
        let receipt = world.injuries().unwrap().events[0];
        assert_eq!(
            (receipt.actor, receipt.site, receipt.kind),
            (patient, site, WoundKind::Legs)
        );
        assert!(!receipt.witnessed);
        assert!(
            world
                .injuries()
                .unwrap()
                .events
                .iter()
                .all(|event| !event.witnessed)
        );
    }

    #[test]
    fn severe_legs_slow_player_and_npc_and_care_consumes_one_real_medicine() {
        let mut world = World::persistent_injury(0);
        let patient = ActorId::new(34);
        // Fixture: two nonfatal bouts separated by HP care, with actual hits.
        for _ in 0..2 {
            world.npcs.get_mut(&patient).unwrap().hp = 3;
            world.damage(PLAYER, patient, 2, false);
        }
        assert_eq!(world.injuries().unwrap().people[&patient].legs, 2);
        assert!(world.severe_wounds(patient));
        let npc_start = world.npcs[&patient].position;
        world.tick = 1;
        world.travel(patient, Position { x: 27, y: 8 }, 0);
        assert_eq!(world.npcs[&patient].position, npc_start);
        world.tick = 2;
        world.travel(patient, Position { x: 27, y: 8 }, 0);
        assert_ne!(world.npcs[&patient].position, npc_start);
        world.tick = 0;
        world
            .injuries
            .as_mut()
            .unwrap()
            .people
            .get_mut(&PLAYER)
            .unwrap()
            .legs = 2;
        let before = world.player.position;
        assert_eq!(
            turn(&mut world, Intent::Move(Direction::East)),
            Outcome::Rejected(Rejection::Hobbled)
        );
        assert_eq!(world.player.position, before);
        assert_eq!(
            turn(&mut world, Intent::Move(Direction::East)),
            Outcome::Moved
        );
        world.supply.shrine = 2;
        world.player.position = world.scenario.clinic;
        world.npcs.get_mut(&patient).unwrap().position = world.scenario.clinic;
        let medic = world
            .npcs
            .values()
            .find(|npc| npc.definition.role == Role::Medic)
            .unwrap()
            .id();
        world.npcs.get_mut(&medic).unwrap().position = world.scenario.clinic;
        assert_eq!(world.wound_goal(patient), Some(Goal::SeekWoundCare));
        let stock = world.supply.shrine;
        let consumed = world.supply.consumed;
        world.work_wound_care(patient);
        assert_eq!(world.supply.shrine, stock - 1);
        assert_eq!(world.supply.consumed, consumed + 1);
        assert_eq!(world.injuries().unwrap().people[&patient].legs, 1);
        assert_eq!(turn(&mut world, Intent::TreatWounds), Outcome::WoundTreated);
        assert_eq!(world.supply.shrine, stock - 2);
        assert_eq!(world.supply.consumed, consumed + 2);
        assert_eq!(world.injuries().unwrap().people[&PLAYER].legs, 1);
        assert_eq!(
            turn(&mut world, Intent::TreatWounds),
            Outcome::Rejected(Rejection::Empty)
        );
    }

    #[test]
    fn severe_arms_reduce_real_strikes_and_make_npcs_seek_safety() {
        let mut world = World::persistent_injury(0);
        let victim = ActorId::new(35);
        world.player.position = world.npcs[&victim].position;
        world.player.hp = 5;
        world.damage(victim, PLAYER, 3, false);
        world.player.hp = 5;
        world.damage(victim, PLAYER, 3, false);
        assert_eq!(world.injuries().unwrap().people[&PLAYER].arms, 2);
        world.player.hp = 6;
        world.npcs.get_mut(&victim).unwrap().hp = 3;
        assert_eq!(
            turn(&mut world, Intent::Attack(victim)),
            Outcome::Struck(victim)
        );
        assert_eq!(world.npcs[&victim].hp, 2);
        world.npcs.get_mut(&victim).unwrap().hp = 5;
        world.damage(PLAYER, victim, 3, false);
        world.npcs.get_mut(&victim).unwrap().hp = 5;
        world.damage(PLAYER, victim, 3, false);
        assert_eq!(world.injuries().unwrap().people[&victim].arms, 2);
        world.supply.shrine = 2;
        assert_eq!(world.choose_goal(&world.npcs[&victim]), Goal::Retreat);
    }

    #[test]
    fn wounded_npc_autonomously_walks_to_a_staffed_clinic_and_pays_for_care() {
        let mut world = World::persistent_injury(0);
        let patient = ActorId::new(34);
        let medic = ActorId::new(1);
        world.supply.shrine = 3;
        world.npcs.get_mut(&medic).unwrap().position = world.scenario.clinic;
        world.npcs.get_mut(&patient).unwrap().position = Position { x: 0, y: 2 };
        for _ in 0..2 {
            world.npcs.get_mut(&patient).unwrap().hp = 3;
            world.damage(PLAYER, patient, 2, false);
        }
        assert_eq!(world.injuries().unwrap().people[&patient].legs, 2);
        let consumed = world.supply.consumed;
        for _ in 0..20 {
            turn(&mut world, Intent::Wait);
            if world.injuries().unwrap().people[&patient].legs == 0 {
                break;
            }
        }
        assert_eq!(world.injuries().unwrap().people[&patient].legs, 0);
        assert!(world.supply.consumed >= consumed + 2);
        assert_eq!(
            world
                .injuries()
                .unwrap()
                .events
                .iter()
                .filter(|event| event.actor == patient && event.kind == WoundKind::Treated)
                .count(),
            2
        );
    }

    #[test]
    fn unmodified_seeded_campaigns_replay_and_preserve_wound_receipts() {
        for seed in 0..4 {
            let mut world = World::persistent_injury(seed);
            for index in 0..180 {
                let intent = if index < 27 {
                    Intent::Move(Direction::East)
                } else {
                    Intent::Wait
                };
                turn(&mut world, intent);
                if matches!(index, 0 | 27 | 90 | 179) {
                    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
                }
            }
            assert!(
                world
                    .injuries()
                    .unwrap()
                    .events
                    .iter()
                    .enumerate()
                    .all(|(index, event)| event.id == index as u32)
            );
        }
    }

    #[test]
    fn public_road_assault_creates_witnessed_persistent_wound_and_replays() {
        let mut world = World::persistent_injury(0);
        let target = ActorId::new(1);
        for _ in 0..70 {
            let from = world.player.position;
            let site = world.npcs[&target].position;
            let intent = if in_reach(from, site) && world.npcs[&target].hp > 2 {
                Intent::Attack(target)
            } else if world.injuries().unwrap().people[&target].injured() {
                break;
            } else {
                let next =
                    super::super::navigation::next_step(world.scenario, from, site, 1, |cell| {
                        world.walkable(cell) && !world.occupied(cell)
                    });
                match next {
                    Some(cell) if cell.x > from.x => Intent::Move(Direction::East),
                    Some(cell) if cell.x < from.x => Intent::Move(Direction::West),
                    Some(cell) if cell.y > from.y => Intent::Move(Direction::South),
                    Some(_) => Intent::Move(Direction::North),
                    None => Intent::Wait,
                }
            };
            turn(&mut world, intent);
        }
        assert_eq!(world.injuries().unwrap().people[&target].legs, 1);
        assert!(
            world
                .injuries()
                .unwrap()
                .events
                .iter()
                .any(|event| event.actor == target
                    && event.kind == WoundKind::Legs
                    && event.witnessed)
        );
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
    }
}
