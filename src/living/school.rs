//! Reed Hall's charter, the player's duty, and a single taught technique.

use super::{
    ActorId, BREATH_MASTERY, IncidentId, Npc, Outcome, PLAYER, Rejection, Role, Scenario, World,
};

pub const MAX_QI: u8 = super::content::MAX_QI;
pub const FORM_QI_COST: u8 = super::content::FORM_QI_COST;
use super::content::FORM_HERBS;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HallRank {
    Apprentice,
    Disciple,
    SeniorDisciple,
    Elder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Duty {
    Unassigned,
    Carrying,
    Delivered,
    Reported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Succession {
    pub at: u64,
    pub former: ActorId,
    pub successor: Option<ActorId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct School {
    pub leader: Option<ActorId>,
    pub succession: Option<Succession>,
    pub duty: Duty,
    pub history: Vec<Succession>,
    pub(super) vacancy_incident: Option<IncidentId>,
    pub form_learned: bool,
    pub qi: u8,
}

impl School {
    pub(super) fn new(scenario: &Scenario) -> Self {
        Self {
            leader: scenario
                .npcs
                .iter()
                .filter(|npc| npc.role == Role::Mentor)
                .map(|npc| npc.id)
                .min(),
            succession: None,
            history: Vec::new(),
            vacancy_incident: None,
            duty: Duty::Unassigned,
            form_learned: false,
            qi: 0,
        }
    }
}

impl World {
    pub(super) fn is_hall_teacher(&self, npc: &Npc) -> bool {
        match self.school() {
            Some(school) => school.leader == Some(npc.id()),
            None => npc.definition.role == Role::Mentor,
        }
    }

    pub(super) fn update_succession(&mut self) {
        if self.company.is_some() {
            self.update_known_succession();
            return;
        }
        let Some(former) = self.school().and_then(|school| school.leader) else {
            return;
        };
        if self.npcs[&former].alive() {
            return;
        }
        // The charter appoints a living deputy. Both candidates are permanent
        // actors, so succession changes responsibilities without moving anyone.
        let successor = self
            .npcs
            .values()
            .find(|npc| npc.alive() && npc.definition.role == Role::HallDeputy)
            .map(Npc::id);
        let school = self.school.as_mut().expect("school exists");
        school.leader = successor;
        school.succession = Some(Succession {
            at: self.tick,
            former,
            successor,
        });
    }

    fn update_known_succession(&mut self) {
        let school = self.school().expect("regional school");
        if let Some(leader) = school.leader {
            if self.npcs[&leader].alive() {
                return;
            }
            let cause = self
                .social
                .incidents()
                .iter()
                .rev()
                .find(|event| event.fatal && event.target == leader)
                .map(|event| event.id);
            self.change_office(leader, None);
            self.school
                .as_mut()
                .expect("regional school")
                .vacancy_incident = cause;
        }
        let school = self.school().expect("regional school");
        let Some(vacancy) = school.succession else {
            return;
        };
        let former = vacancy.former;
        let successor = self
            .npcs
            .values()
            .find(|npc| {
                npc.alive()
                    && npc.definition.role == Role::HallDeputy
                    && (self.perceives(npc.position, self.npcs[&former].position)
                        || school
                            .vacancy_incident
                            .is_some_and(|cause| self.social.knowledge(npc.id(), cause).is_some()))
            })
            .map(Npc::id);
        if successor.is_some() {
            self.change_office(former, successor);
            self.school
                .as_mut()
                .expect("regional school")
                .vacancy_incident = None;
        }
    }

    fn change_office(&mut self, former: ActorId, successor: Option<ActorId>) {
        let school = self.school.as_mut().expect("regional school");
        let event = Succession {
            at: self.tick,
            former,
            successor,
        };
        school.leader = successor;
        school.succession = Some(event);
        school.history.push(event);
    }

    pub(super) fn hall_duty(&mut self) -> Result<Outcome, Rejection> {
        if self.hall_rank().is_none() {
            return Err(Rejection::MembershipRequired);
        }
        let duty = self.school().expect("school rules").duty;
        if duty == Duty::Carrying {
            self.require_reach(self.scenario.shrine)?;
            self.supply.shrine += 1;
            self.supply.player_deliveries += 1;
            self.note_governance_contribution(super::Need::ShrineMedicine);
            self.school.as_mut().expect("school rules").duty = Duty::Delivered;
            self.note_player_duty_delivered();
            return Ok(Outcome::DutyDelivered);
        }
        let mentor = self.hall_mentor()?;
        match duty {
            Duty::Unassigned => {
                let region = self.region.as_mut().expect("regional school");
                if region.hall_offerings == 0 {
                    return Err(Rejection::Empty);
                }
                region.hall_offerings -= 1;
                self.school.as_mut().expect("school rules").duty = Duty::Carrying;
                self.note_player_duty_accepted();
                Ok(Outcome::DutyAccepted)
            }
            Duty::Delivered => {
                self.school.as_mut().expect("school rules").duty = Duty::Reported;
                self.social.aided(PLAYER, mentor);
                self.complete_player_hierarchy_duty();
                Ok(Outcome::DutyReported)
            }
            Duty::Reported if self.sect_hierarchy().is_some() => self.accept_repeat_player_duty(),
            Duty::Reported => Err(Rejection::DutyComplete),
            Duty::Carrying => unreachable!("cargo is handled at the shrine"),
        }
    }

    pub(super) fn study_form(&mut self) -> Result<Outcome, Rejection> {
        self.hall_mentor()?;
        if !matches!(
            self.hall_rank(),
            Some(HallRank::Disciple | HallRank::SeniorDisciple | HallRank::Elder)
        ) {
            return Err(Rejection::RankRequired);
        }
        let school = self.school.as_mut().expect("school rules");
        if school.form_learned {
            return Err(Rejection::FormAlreadyKnown);
        }
        if self.breath < BREATH_MASTERY {
            return Err(Rejection::PracticeRequired);
        }
        if self.player.herbs < FORM_HERBS {
            return Err(Rejection::NoCargo);
        }
        self.player.herbs -= FORM_HERBS;
        school.form_learned = true;
        Ok(Outcome::FormLearned)
    }

    pub(super) fn center_qi(&mut self) -> Result<Outcome, Rejection> {
        let school = self.school.as_mut().expect("school rules");
        if !school.form_learned {
            return Err(Rejection::FormRequired);
        }
        if school.qi == MAX_QI {
            return Err(Rejection::QiFull);
        }
        school.qi += 1;
        Ok(Outcome::QiCentered { total: school.qi })
    }

    pub(super) fn spend_form_qi(&mut self) -> Result<(), Rejection> {
        let school = self.school.as_mut().expect("school rules");
        if !school.form_learned {
            return Err(Rejection::FormRequired);
        }
        if school.qi < FORM_QI_COST {
            return Err(Rejection::InsufficientQi);
        }
        school.qi -= FORM_QI_COST;
        Ok(())
    }
}
