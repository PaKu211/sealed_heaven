//! Local mentorship, bodily practice and actor-owned martial techniques.
//! A teacher must be present; an NPC student may only approach one it sees.

use std::collections::BTreeMap;

use super::navigation::in_reach;
use super::{ActorId, Allegiance, Goal, Npc, Outcome, PLAYER, Rejection, Role, Scenario, World};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Form {
    ReedPalm,
    StoneFist,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MartialSkill {
    pub form: Option<Form>,
    pub teacher: Option<ActorId>,
    pub lessons: u8,
    pub strain: u8,
    /// One prepared parry, spent against an actual incoming blow.
    pub parry: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lesson {
    pub id: u32,
    pub at: u64,
    pub student: ActorId,
    pub teacher: ActorId,
    pub form: Form,
    pub learned: bool,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Martial {
    pub skills: BTreeMap<ActorId, MartialSkill>,
    pub lessons: Vec<Lesson>,
}

fn teaching_form(npc: &Npc) -> Option<Form> {
    super::content::teaching_form(npc.definition.role)
}

impl Martial {
    pub(super) fn new(scenario: &Scenario) -> Self {
        let mut skills = BTreeMap::new();
        skills.insert(PLAYER, MartialSkill::default());
        for definition in scenario.npcs {
            let state = MartialSkill {
                form: super::content::teaching_form(definition.role),
                ..MartialSkill::default()
            };
            skills.insert(definition.id, state);
        }
        Self {
            skills,
            lessons: Vec::new(),
        }
    }
}

impl World {
    pub fn martial(&self) -> Option<&Martial> {
        self.martial.as_ref()
    }

    fn teaching_available(&self, teacher: &Npc) -> bool {
        teacher.alive()
            && !self.is_night()
            && teacher.hp > teacher.definition.hp / 2
            && teacher.fatigue < 32
            && !matches!(teacher.activity.goal, Goal::Retaliate | Goal::Retreat)
    }

    fn teacher_form(&self, npc: &Npc) -> Option<Form> {
        teaching_form(npc).or_else(|| {
            (self.sect_hierarchy().is_some()
                && self.is_hall_teacher(npc)
                && self
                    .martial()
                    .is_some_and(|martial| martial.skills[&npc.id()].form == Some(Form::ReedPalm)))
            .then_some(Form::ReedPalm)
        })
    }

    pub(super) fn drill(&mut self, teacher: ActorId) -> Result<Outcome, Rejection> {
        let npc = self
            .npcs
            .get(&teacher)
            .filter(|npc| npc.alive())
            .ok_or(Rejection::UnknownActor)?;
        let form = self.teacher_form(npc).ok_or(Rejection::Refused)?;
        self.require_reach(npc.position)?;
        if !self.teaching_available(npc) || self.social.relationship(teacher, PLAYER).refuses_aid()
        {
            return Err(Rejection::Refused);
        }
        let allegiance = self.politics.as_ref().map(|politics| politics.allegiance);
        if (form == Form::ReedPalm && allegiance == Some(Allegiance::Stonebank))
            || (form == Form::StoneFist && self.region.as_ref().unwrap().enrolled)
        {
            return Err(Rejection::OathConflict);
        }
        let skill = self.martial().unwrap().skills[&PLAYER];
        if skill.form.is_some() {
            return Err(Rejection::FormAlreadyKnown);
        }
        if skill.strain >= 3 {
            return Err(Rejection::Overtrained);
        }
        let waived = form == Form::ReedPalm && self.hall_tuition_waived();
        if !waived {
            let tuition = super::content::martial_profile(form).tuition;
            if self.region.as_ref().unwrap().coins < tuition {
                return Err(Rejection::NotEnoughCoins);
            }
            match form {
                Form::ReedPalm => {
                    self.politics.as_mut().unwrap().hall_coins += tuition;
                }
                Form::StoneFist => {
                    self.rivalry.as_mut().unwrap().lodge_coins += tuition;
                }
            }
            self.region.as_mut().unwrap().coins -= tuition;
        }
        self.social.meet(teacher, PLAYER);
        let learned = self.commit_lesson(PLAYER, teacher, form);
        Ok(Outcome::MartialDrilled { teacher, learned })
    }

    fn commit_lesson(&mut self, student: ActorId, teacher: ActorId, form: Form) -> bool {
        let skill = self
            .martial
            .as_mut()
            .unwrap()
            .skills
            .get_mut(&student)
            .unwrap();
        if skill.teacher != Some(teacher) {
            skill.lessons = 0;
            skill.teacher = Some(teacher);
        }
        skill.lessons += 1;
        skill.strain += 1;
        let learned = skill.lessons >= super::content::martial_profile(form).lessons;
        if learned {
            skill.form = Some(form);
        }
        let site = self.npcs[&teacher].position;
        let witnessed = self.player.alive()
            && self.perceives(self.player.position, site)
            && (student == PLAYER
                || self.perceives(self.player.position, self.npcs[&student].position));
        let memory = self.martial.as_mut().unwrap();
        memory.lessons.push(Lesson {
            id: u32::try_from(memory.lessons.len()).expect("bounded lesson journal"),
            at: self.tick,
            student,
            teacher,
            form,
            learned,
            witnessed,
        });
        learned
    }

    pub(super) fn martial_goal(&self, npc: &Npc) -> Option<Goal> {
        let state = self.martial()?;
        if !npc.alive() {
            return None;
        }
        if state.skills[&npc.id()].strain >= 3 {
            return Some(Goal::MartialRest);
        }
        if npc.definition.role != Role::HallDeputy || state.skills[&npc.id()].form.is_some() {
            return None;
        }
        self.npcs
            .values()
            .filter(|teacher| {
                teacher.id() != npc.id()
                    && self.teaching_available(teacher)
                    && self.teacher_form(teacher) == Some(Form::ReedPalm)
                    && self.perceives(npc.position, teacher.position)
            })
            .min_by_key(|teacher| teacher.id())
            .map(|teacher| Goal::MartialPractice(teacher.id()))
    }

    pub(super) fn work_martial(&mut self, student: ActorId, teacher: ActorId) {
        let Some(teacher_npc) = self.npcs.get(&teacher).copied().filter(|npc| npc.alive()) else {
            return;
        };
        let Some(form) = self.teacher_form(&teacher_npc) else {
            return;
        };
        if !self.perceives(self.npcs[&student].position, teacher_npc.position)
            || !self.teaching_available(&teacher_npc)
        {
            return;
        }
        if !in_reach(self.npcs[&student].position, teacher_npc.position) {
            self.travel(student, teacher_npc.position, 1);
            return;
        }
        if self.martial().unwrap().skills[&student].form.is_none()
            && self.martial().unwrap().skills[&student].strain < 3
        {
            self.commit_lesson(student, teacher, form);
        }
    }

    pub(super) fn technique_strike(&mut self, target: ActorId) -> Result<Outcome, Rejection> {
        let site = self
            .npcs
            .get(&target)
            .filter(|npc| npc.alive())
            .ok_or(Rejection::UnknownActor)?
            .position;
        self.require_reach(site)?;
        let skill = self.martial().unwrap().skills[&PLAYER];
        let form = skill.form.ok_or(Rejection::FormRequired)?;
        if skill.strain >= 3 {
            return Err(Rejection::Overtrained);
        }
        if self.meridians().is_some() {
            self.use_martial_qi(PLAYER)?;
        }
        let profile = super::content::martial_profile(form);
        let (damage, exertion, parry) = (profile.damage, profile.exertion, profile.parry);
        let skill = self
            .martial
            .as_mut()
            .unwrap()
            .skills
            .get_mut(&PLAYER)
            .unwrap();
        skill.strain += exertion;
        skill.parry = parry;
        self.damage(PLAYER, target, self.wounded_strike(PLAYER, damage), false);
        Ok(Outcome::TechniqueStruck { target, form })
    }

    pub(super) fn npc_martial_damage(&mut self, actor: ActorId) -> u16 {
        let Some(skill) = self
            .martial()
            .and_then(|martial| martial.skills.get(&actor))
            .copied()
        else {
            return super::STRIKE;
        };
        if skill.strain >= 3 {
            return super::STRIKE;
        }
        let Some(form) = skill.form else {
            return super::STRIKE;
        };
        if self.meridians().is_some() && self.use_martial_qi(actor).is_err() {
            return super::STRIKE;
        }
        let state = self
            .martial
            .as_mut()
            .unwrap()
            .skills
            .get_mut(&actor)
            .unwrap();
        let profile = super::content::martial_profile(form);
        state.strain += profile.exertion;
        // Preserve an existing parry for the NPC Stone Fist action.
        if profile.parry != 0 {
            state.parry = profile.parry;
        }
        profile.damage
    }

    pub(super) fn martial_block(&mut self, target: ActorId) -> u16 {
        let Some(state) = self
            .martial
            .as_mut()
            .and_then(|martial| martial.skills.get_mut(&target))
        else {
            return 0;
        };
        if state.parry == 0 {
            return 0;
        }
        state.parry -= 1;
        1
    }

    pub(super) fn recover_martial(&mut self, actor: ActorId) {
        if let Some(skill) = self
            .martial
            .as_mut()
            .and_then(|martial| martial.skills.get_mut(&actor))
        {
            skill.strain = skill.strain.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Direction, Intent};

    #[test]
    fn public_mentorship_npc_autonomy_and_distinct_player_forms() {
        let mut world = World::martial_mentorship(0);
        let mut passive = world.clone();
        for _ in 0..25 {
            assert_eq!(
                world
                    .step(world.tick(), Intent::Move(Direction::East))
                    .unwrap(),
                Outcome::Moved
            );
            passive.step(passive.tick(), Intent::Wait).unwrap();
        }
        let envoy = ActorId::new(35);
        let deputy = ActorId::new(34);
        assert_eq!(
            world.martial().unwrap().skills[&deputy].form,
            Some(Form::ReedPalm)
        );
        assert_eq!(
            passive.martial().unwrap().skills[&deputy].form,
            Some(Form::ReedPalm)
        );
        let initial_purse = world.region().unwrap().coins;
        let initial_treasury = world.rivalry().unwrap().lodge_coins;
        assert_eq!(
            world.step(world.tick(), Intent::Drill(envoy)).unwrap(),
            Outcome::MartialDrilled {
                teacher: envoy,
                learned: false
            }
        );
        for action in [
            Intent::Move(Direction::East),
            Intent::Move(Direction::South),
        ] {
            world.step(world.tick(), action).unwrap();
        }
        assert_eq!(
            world.step(world.tick(), Intent::Drill(envoy)).unwrap(),
            Outcome::MartialDrilled {
                teacher: envoy,
                learned: true
            }
        );
        assert_eq!(
            world.martial().unwrap().skills[&PLAYER].form,
            Some(Form::StoneFist)
        );
        assert_eq!(world.region().unwrap().coins, initial_purse - 2);
        assert_eq!(world.rivalry().unwrap().lodge_coins, initial_treasury + 2);
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
        let mut ordinary = world.clone();
        assert_eq!(
            world
                .step(world.tick(), Intent::TechniqueStrike(envoy))
                .unwrap(),
            Outcome::TechniqueStruck {
                target: envoy,
                form: Form::StoneFist
            }
        );
        assert_eq!(
            ordinary
                .step(ordinary.tick(), Intent::Attack(envoy))
                .unwrap(),
            Outcome::Struck(envoy)
        );
        assert_eq!(ordinary.npcs[&envoy].hp, world.npcs[&envoy].hp + 1);
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
        assert_eq!(passive.martial().unwrap().skills[&PLAYER].form, None);
        let old = World::sect_cultivation(0);
        assert!(old.martial().is_none());
        assert_eq!(World::restore(&old.checkpoint()).unwrap(), old);
    }

    #[test]
    fn reed_palm_parries_the_next_blow_and_must_be_taught_in_person() {
        let mut world = World::martial_mentorship(0);
        let mentor = ActorId::new(33);
        assert_eq!(
            world.step(world.tick(), Intent::Drill(mentor)).unwrap(),
            Outcome::Rejected(Rejection::OutOfReach)
        );
        let prior_coins = world.region().unwrap().coins;
        assert_eq!(
            world
                .step(world.tick(), Intent::TechniqueStrike(mentor))
                .unwrap(),
            Outcome::Rejected(Rejection::OutOfReach)
        );
        assert_eq!(world.region().unwrap().coins, prior_coins);
        world = World::martial_mentorship(0);
        for _ in 0..27 {
            world
                .step(world.tick(), Intent::Move(Direction::East))
                .unwrap();
        }
        for action in [
            Intent::Move(Direction::South),
            Intent::Move(Direction::East),
        ] {
            world.step(world.tick(), action).unwrap();
        }
        assert_eq!(
            world.step(world.tick(), Intent::Drill(mentor)).unwrap(),
            Outcome::MartialDrilled {
                teacher: mentor,
                learned: false
            }
        );
        world
            .step(world.tick(), Intent::Move(Direction::West))
            .unwrap();
        assert_eq!(
            world.step(world.tick(), Intent::Drill(mentor)).unwrap(),
            Outcome::MartialDrilled {
                teacher: mentor,
                learned: true
            }
        );
        assert_eq!(
            world.martial().unwrap().skills[&PLAYER].form,
            Some(Form::ReedPalm)
        );
        assert_eq!(world.region().unwrap().coins, prior_coins - 2);
        let mut ordinary = world.clone();
        let hp = world.player.hp;
        assert_eq!(
            world
                .step(world.tick(), Intent::TechniqueStrike(mentor))
                .unwrap(),
            Outcome::TechniqueStruck {
                target: mentor,
                form: Form::ReedPalm
            }
        );
        assert_eq!(
            ordinary
                .step(ordinary.tick(), Intent::Attack(mentor))
                .unwrap(),
            Outcome::Struck(mentor)
        );
        assert!(world.player.hp >= ordinary.player.hp);
        assert!(world.player.hp <= hp);
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
    }

    #[test]
    fn trained_npc_uses_its_own_parry_and_a_dead_teacher_stops_new_lessons() {
        let deputy = ActorId::new(34);
        let mentor = ActorId::new(33);
        let mut world = World::martial_mentorship(0);
        for _ in 0..3 {
            world.step(world.tick(), Intent::Wait).unwrap();
        }
        assert_eq!(
            world.martial().unwrap().skills[&deputy].form,
            Some(Form::ReedPalm)
        );
        world.player.position = world.npcs[&deputy].position;
        world.damage(PLAYER, deputy, 1, false);
        world.step(world.tick(), Intent::Wait).unwrap();
        assert_eq!(world.martial().unwrap().skills[&deputy].parry, 1);
        let hp = world.npcs[&deputy].hp;
        world.damage(PLAYER, deputy, 2, false);
        assert_eq!(world.npcs[&deputy].hp, hp - 1);
        assert_eq!(world.martial().unwrap().skills[&deputy].parry, 0);
        let mut without_teacher = World::martial_mentorship(0);
        without_teacher.npcs.get_mut(&mentor).unwrap().hp = 0;
        for _ in 0..8 {
            without_teacher
                .step(without_teacher.tick(), Intent::Wait)
                .unwrap();
        }
        assert_eq!(
            without_teacher.martial().unwrap().skills[&deputy].form,
            None
        );
        assert!(without_teacher.martial().unwrap().lessons.is_empty());
    }

    #[test]
    fn multiple_seeds_preserve_martial_replay_and_finite_goods_over_longer_runs() {
        for seed in 0..4 {
            let mut world = World::martial_mentorship(seed);
            for turn in 0..180 {
                world.step(world.tick(), Intent::Wait).unwrap();
                assert_eq!(world.contested().unwrap().lots.len(), 4);
                assert!(
                    world
                        .martial()
                        .unwrap()
                        .lessons
                        .iter()
                        .enumerate()
                        .all(|(index, event)| event.id == index as u32)
                );
                if matches!(turn, 0 | 30 | 90 | 179) {
                    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
                }
            }
        }
    }

    #[test]
    fn loss_of_a_teacher_interrupts_a_partly_paid_apprenticeship() {
        let mut world = World::martial_mentorship(0);
        let mentor = ActorId::new(33);
        world.player.position = world.npcs[&mentor].position;
        world.step(world.tick(), Intent::Drill(mentor)).unwrap();
        assert_eq!(world.martial().unwrap().skills[&PLAYER].lessons, 1);
        let coins = world.region().unwrap().coins;
        let hall = world.politics().unwrap().hall_coins;
        world.damage(PLAYER, mentor, world.npcs[&mentor].hp, false);
        assert_eq!(
            world.step(world.tick(), Intent::Drill(mentor)).unwrap(),
            Outcome::Rejected(Rejection::UnknownActor)
        );
        assert_eq!(world.martial().unwrap().skills[&PLAYER].form, None);
        assert_eq!(world.region().unwrap().coins, coins);
        assert_eq!(world.politics().unwrap().hall_coins, hall);
    }
}
