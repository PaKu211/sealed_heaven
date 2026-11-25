//! Qi is carried by a body, not an XP meter. Physical herbs and a staffed
//! sacred site open the cycle; exertion can obstruct channels until treated.

use std::collections::BTreeMap;

use super::navigation::in_reach;
use super::{ActorId, Goal, Outcome, PLAYER, Position, Region, Rejection, Role, Scenario, World};

const MAX_QI: u8 = 3;
const OPEN_CHANNELS: u8 = 2;
const BLOCKED: u8 = 2;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MeridianState {
    pub qi: u8,
    pub channels: u8,
    pub blockage: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeridianEventKind {
    Drawn,
    Opened,
    Obstructed,
    Treated,
    Spent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeridianEvent {
    pub id: u32,
    pub at: u64,
    pub actor: ActorId,
    pub kind: MeridianEventKind,
    pub site: Position,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Meridians {
    pub people: BTreeMap<ActorId, MeridianState>,
    pub events: Vec<MeridianEvent>,
}

impl Meridians {
    pub(super) fn new(scenario: &Scenario) -> Self {
        let mut people = BTreeMap::new();
        people.insert(PLAYER, MeridianState::default());
        for npc in scenario.npcs {
            let mut state = MeridianState::default();
            if matches!(npc.role, Role::Mentor | Role::RivalEnvoy) {
                state.qi = 2;
                state.channels = OPEN_CHANNELS;
            }
            people.insert(npc.id, state);
        }
        Self {
            people,
            events: Vec::new(),
        }
    }
}

impl World {
    pub fn meridians(&self) -> Option<&Meridians> {
        self.meridians.as_ref()
    }

    fn sacred_site(&self, site: Position) -> bool {
        in_reach(site, self.scenario.shrine) || in_reach(site, Region::sites().hall)
    }

    pub(super) fn draw_qi(&mut self) -> Result<Outcome, Rejection> {
        if !self.sacred_site(self.player.position) {
            return Err(Rejection::OutOfReach);
        }
        let state = self.meridians().unwrap().people[&PLAYER];
        if state.blockage >= BLOCKED {
            return Err(Rejection::MeridiansBlocked);
        }
        if state.qi == MAX_QI {
            return Err(Rejection::QiFull);
        }
        if self.player.herbs == 0 {
            return Err(Rejection::NoCargo);
        }
        self.player.herbs -= 1;
        self.supply.consumed += 1;
        let state = self
            .meridians
            .as_mut()
            .unwrap()
            .people
            .get_mut(&PLAYER)
            .unwrap();
        state.qi += 1;
        let qi = state.qi;
        self.record_meridian(PLAYER, MeridianEventKind::Drawn);
        Ok(Outcome::QiDrawn(qi))
    }

    pub(super) fn circulate_qi(&mut self) -> Result<Outcome, Rejection> {
        let state = self.meridians().unwrap().people[&PLAYER];
        if state.blockage >= BLOCKED {
            return Err(Rejection::MeridiansBlocked);
        }
        if state.channels >= OPEN_CHANNELS {
            return Err(Rejection::PracticeComplete);
        }
        if state.qi == 0 {
            return Err(Rejection::InsufficientQi);
        }
        let strained = self.martial().unwrap().skills[&PLAYER].strain >= 2
            || self.player.hp <= super::PLAYER_HP / 2;
        let state = self
            .meridians
            .as_mut()
            .unwrap()
            .people
            .get_mut(&PLAYER)
            .unwrap();
        state.qi -= 1;
        if strained {
            state.blockage += 1;
        } else {
            state.channels = state.channels.saturating_add(1).min(OPEN_CHANNELS);
        }
        let result = *state;
        self.record_meridian(
            PLAYER,
            if strained {
                MeridianEventKind::Obstructed
            } else {
                MeridianEventKind::Opened
            },
        );
        Ok(Outcome::QiCirculated {
            channels: result.channels,
            blockage: result.blockage,
        })
    }

    pub(super) fn mend_meridians(&mut self) -> Result<Outcome, Rejection> {
        self.require_reach(self.scenario.clinic)?;
        let state = self.meridians().unwrap().people[&PLAYER];
        if state.blockage == 0 {
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
        self.meridians
            .as_mut()
            .unwrap()
            .people
            .get_mut(&PLAYER)
            .unwrap()
            .blockage = 0;
        self.record_meridian(PLAYER, MeridianEventKind::Treated);
        Ok(Outcome::MeridiansMended)
    }

    pub(super) fn use_martial_qi(&mut self, actor: ActorId) -> Result<(), Rejection> {
        let state = self.meridians().unwrap().people[&actor];
        if state.blockage >= BLOCKED {
            return Err(Rejection::MeridiansBlocked);
        }
        if state.channels < OPEN_CHANNELS {
            return Err(Rejection::PracticeRequired);
        }
        if state.qi == 0 {
            return Err(Rejection::InsufficientQi);
        }
        self.meridians
            .as_mut()
            .unwrap()
            .people
            .get_mut(&actor)
            .unwrap()
            .qi -= 1;
        self.record_meridian(actor, MeridianEventKind::Spent);
        Ok(())
    }

    pub(super) fn npc_meridian_goal(&self, actor: ActorId) -> Option<Goal> {
        let martial = self.martial()?;
        if martial.skills.get(&actor)?.form.is_none() || !self.npcs[&actor].alive() {
            return None;
        }
        let meridian = self.meridians()?.people[&actor];
        if meridian.blockage >= BLOCKED {
            return Some(Goal::MeridianCare);
        }
        if meridian.qi > 0 && meridian.channels < OPEN_CHANNELS && martial.skills[&actor].strain < 2
        {
            return Some(Goal::MeridianCirculate);
        }
        if self.npcs[&actor].definition.role != Role::HallDeputy
            || meridian.qi >= MAX_QI
            || self.region()?.hall_offerings == 0
        {
            return None;
        }
        Some(Goal::MeridianDraw)
    }

    pub(super) fn work_meridian(&mut self, actor: ActorId, goal: Goal) {
        match goal {
            Goal::MeridianDraw => {
                let site = Region::sites().hall;
                if !in_reach(self.npcs[&actor].position, site) {
                    self.travel(actor, site, 1);
                    return;
                }
                if self.region.as_ref().unwrap().hall_offerings == 0 {
                    return;
                }
                if self.meridians().unwrap().people[&actor].qi >= MAX_QI {
                    return;
                }
                self.region.as_mut().unwrap().hall_offerings -= 1;
                self.supply.consumed += 1;
                self.meridians
                    .as_mut()
                    .unwrap()
                    .people
                    .get_mut(&actor)
                    .unwrap()
                    .qi += 1;
                self.record_meridian(actor, MeridianEventKind::Drawn);
            }
            Goal::MeridianCirculate => {
                let person = self.meridians().unwrap().people[&actor];
                if person.qi == 0 || person.blockage >= BLOCKED {
                    return;
                }
                let strained = self.martial().unwrap().skills[&actor].strain >= 2
                    || self.npcs[&actor].hp <= self.npcs[&actor].definition.hp / 2;
                let state = self
                    .meridians
                    .as_mut()
                    .unwrap()
                    .people
                    .get_mut(&actor)
                    .unwrap();
                state.qi -= 1;
                if strained {
                    state.blockage += 1;
                } else {
                    state.channels = state.channels.saturating_add(1).min(OPEN_CHANNELS);
                }
                self.record_meridian(
                    actor,
                    if strained {
                        MeridianEventKind::Obstructed
                    } else {
                        MeridianEventKind::Opened
                    },
                );
            }
            Goal::MeridianCare => {
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
                self.meridians
                    .as_mut()
                    .unwrap()
                    .people
                    .get_mut(&actor)
                    .unwrap()
                    .blockage = 0;
                self.record_meridian(actor, MeridianEventKind::Treated);
            }
            _ => {}
        }
    }

    fn record_meridian(&mut self, actor: ActorId, kind: MeridianEventKind) {
        let site = if actor == PLAYER {
            self.player.position
        } else {
            self.npcs[&actor].position
        };
        let witnessed = self.player.alive() && self.perceives(self.player.position, site);
        let state = self.meridians.as_mut().unwrap();
        state.events.push(MeridianEvent {
            id: u32::try_from(state.events.len()).expect("bounded meridian log"),
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

    fn material_total(world: &World) -> u32 {
        let region = world.region().unwrap();
        world.player.herbs
            + world.supply.source
            + world.supply.shrine
            + world.supply.consumed
            + world
                .npcs
                .values()
                .map(|npc| npc.herbs + u32::from(npc.packed_herbs))
                .sum::<u32>()
            + region.market_herbs
            + region.garden_herbs
            + region.hall_offerings
            + (region.remedies + region.remedies_used) * 2
            + world.rivalry().unwrap().consumed_herbs
            + world.company().unwrap().herbs_used
            + u32::from(world.breath)
            + world.school().unwrap().form_learned as u32 * 2
            + u32::from(world.school().unwrap().duty == super::super::Duty::Carrying)
    }

    fn walk_to(world: &mut World, destination: Position) {
        for _ in 0..150 {
            let from = world.player.position;
            if in_reach(from, destination) {
                return;
            }
            let next =
                super::super::navigation::next_step(world.scenario, from, destination, 1, |site| {
                    world.walkable(site) && !world.occupied(site)
                });
            let action = match next {
                Some(site) if site.x > from.x => Intent::Move(Direction::East),
                Some(site) if site.x < from.x => Intent::Move(Direction::West),
                Some(site) if site.y > from.y => Intent::Move(Direction::South),
                Some(_) => Intent::Move(Direction::North),
                None => Intent::Wait,
            };
            world.step(world.tick(), action).unwrap();
        }
        panic!("no public path to {destination:?}");
    }

    #[test]
    fn public_gather_meditation_meridian_risk_and_real_clinic_cure() {
        let mut world = World::meridian_cultivation(0);
        let initial = material_total(&world);
        let patch = world.scenario.patch;
        let shrine = world.scenario.shrine;
        walk_to(&mut world, patch);
        for _ in 0..4 {
            assert_eq!(
                world.step(world.tick(), Intent::Gather).unwrap(),
                Outcome::Gathered
            );
        }
        assert_eq!(world.player.herbs, 4);
        walk_to(&mut world, shrine);
        let mut clean = world.clone();
        assert_eq!(
            world.step(world.tick(), Intent::DrawQi).unwrap(),
            Outcome::QiDrawn(1)
        );
        assert_eq!(
            world.step(world.tick(), Intent::CirculateQi).unwrap(),
            Outcome::QiCirculated {
                channels: 1,
                blockage: 0
            }
        );
        assert_eq!(
            world.step(world.tick(), Intent::DrawQi).unwrap(),
            Outcome::QiDrawn(1)
        );
        assert_eq!(
            world.step(world.tick(), Intent::CirculateQi).unwrap(),
            Outcome::QiCirculated {
                channels: 2,
                blockage: 0
            }
        );
        assert_eq!(
            world.step(world.tick(), Intent::DrawQi).unwrap(),
            Outcome::QiDrawn(1)
        );
        assert_eq!(world.meridians().unwrap().people[&PLAYER].channels, 2);
        assert_eq!(world.player.herbs, 1);
        assert_eq!(
            material_total(&world),
            initial + world.supply.regrown + world.region().unwrap().garden_regrown
        );
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
        // Two physical training lessons leave the body too strained for safe
        // circulation. A separate public route from the same checkpoint does
        // not have to accept that risk.
        clean
            .martial
            .as_mut()
            .unwrap()
            .skills
            .get_mut(&PLAYER)
            .unwrap()
            .strain = 2;
        assert_eq!(
            clean.step(clean.tick(), Intent::DrawQi).unwrap(),
            Outcome::QiDrawn(1)
        );
        assert_eq!(
            clean.step(clean.tick(), Intent::CirculateQi).unwrap(),
            Outcome::QiCirculated {
                channels: 0,
                blockage: 1
            }
        );
        assert_eq!(
            clean.step(clean.tick(), Intent::DrawQi).unwrap(),
            Outcome::QiDrawn(1)
        );
        assert_eq!(
            clean.step(clean.tick(), Intent::CirculateQi).unwrap(),
            Outcome::QiCirculated {
                channels: 0,
                blockage: 2
            }
        );
        assert_eq!(
            clean.step(clean.tick(), Intent::DrawQi).unwrap(),
            Outcome::Rejected(Rejection::MeridiansBlocked)
        );
        let clinic = clean.scenario.clinic;
        walk_to(&mut clean, clinic);
        assert!(clean.supply.shrine > 0);
        for _ in 0..500 {
            if clean.npcs.values().any(|npc| {
                npc.alive() && npc.definition.role == Role::Medic && npc.position == clinic
            }) {
                break;
            }
            clean.step(clean.tick(), Intent::Wait).unwrap();
        }
        let before = clean.supply.shrine;
        assert_eq!(
            clean.step(clean.tick(), Intent::MendMeridians).unwrap(),
            Outcome::MeridiansMended
        );
        assert_eq!(clean.supply.shrine, before - 1);
        assert_eq!(clean.meridians().unwrap().people[&PLAYER].blockage, 0);
        // Fixture-added strain, not position or resources, is intentionally
        // excluded from replay evidence; the unaffected public route above
        // and ASCII acceptance prove checkpoint integrity.
    }

    #[test]
    fn autonomous_disciple_cultivates_from_hall_offerings_and_replays_across_seeds() {
        for seed in 0..4 {
            let mut world = World::meridian_cultivation(seed);
            let initial = material_total(&world);
            for _ in 0..180 {
                world.step(world.tick(), Intent::Wait).unwrap();
                assert_eq!(
                    material_total(&world),
                    initial + world.supply.regrown + world.region().unwrap().garden_regrown
                );
            }
            assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
            assert!(
                world
                    .meridians()
                    .unwrap()
                    .events
                    .iter()
                    .enumerate()
                    .all(|(n, event)| event.id == n as u32)
            );
            if seed == 0 {
                assert_eq!(
                    world.meridians().unwrap().people[&ActorId::new(34)].channels,
                    2
                );
                assert!(
                    world
                        .meridians()
                        .unwrap()
                        .events
                        .iter()
                        .any(|event| event.actor == ActorId::new(34)
                            && event.kind == MeridianEventKind::Opened)
                );
            }
        }
    }

    #[test]
    fn learned_technique_requires_open_channels_and_spends_qi_only_on_a_valid_strike() {
        let mut world = World::meridian_cultivation(0);
        let target = ActorId::new(35);
        world.player.position = world.npcs[&target].position;
        world
            .martial
            .as_mut()
            .unwrap()
            .skills
            .get_mut(&PLAYER)
            .unwrap()
            .form = Some(super::super::Form::StoneFist);
        assert_eq!(
            world
                .step(world.tick(), Intent::TechniqueStrike(target))
                .unwrap(),
            Outcome::Rejected(Rejection::PracticeRequired)
        );
        world
            .meridians
            .as_mut()
            .unwrap()
            .people
            .get_mut(&PLAYER)
            .unwrap()
            .channels = 2;
        assert_eq!(
            world
                .step(world.tick(), Intent::TechniqueStrike(target))
                .unwrap(),
            Outcome::Rejected(Rejection::InsufficientQi)
        );
        world
            .meridians
            .as_mut()
            .unwrap()
            .people
            .get_mut(&PLAYER)
            .unwrap()
            .qi = 1;
        assert_eq!(
            world
                .step(world.tick(), Intent::TechniqueStrike(target))
                .unwrap(),
            Outcome::TechniqueStruck {
                target,
                form: super::super::Form::StoneFist
            }
        );
        assert_eq!(world.meridians().unwrap().people[&PLAYER].qi, 0);
        assert_eq!(
            world
                .step(world.tick(), Intent::TechniqueStrike(target))
                .unwrap(),
            Outcome::Rejected(Rejection::InsufficientQi)
        );
        let previous = World::martial_mentorship(0);
        assert!(previous.meridians().is_none());
        assert_eq!(World::restore(&previous.checkpoint()).unwrap(), previous);
    }

    #[test]
    fn earlier_relic_wards_still_block_real_damage_in_meridian_rules() {
        let mut world = World::meridian_cultivation(0);
        world.player.position = Position { x: 8, y: 4 };
        world.claim_lot(0).unwrap();
        world.attune_lot(0).unwrap();
        let hp = world.player.hp;
        world.damage(ActorId::new(35), PLAYER, 2, false);
        assert_eq!(world.player.hp, hp - 1);
        assert_eq!(world.contested().unwrap().wards, 1);
        assert_eq!(world.social().incidents().last().unwrap().damage, 1);
    }
}
