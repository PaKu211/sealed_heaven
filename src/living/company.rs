//! One voluntary travelling partnership. All actors retain their jobs, bodies and inventory.
use super::navigation::in_reach;
use super::{ActorId, Goal, Npc, Outcome, PLAYER, PLAYER_HP, Position, Rejection, Role, World};

const SEARCH_TURNS: u64 = 12;
const PACK_CAPACITY: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Invitation {
    OpenRoad,
    SchoolMember,
    LodgeMember,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Journey {
    pub actor: ActorId,
    pub last_seen: Position,
    pub seen_at: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JourneyEvent {
    Joined,
    Dismissed,
    LostContact,
    Distrust,
    Office,
    Fallen,
    PlayerFallen,
    FieldCare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompanyEvent {
    pub at: u64,
    pub actor: ActorId,
    pub place: Position,
    pub kind: JourneyEvent,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Company {
    pub journey: Option<Journey>,
    pub history: Vec<CompanyEvent>,
    pub herbs_used: u32,
}

impl World {
    pub fn invitation(&self, id: ActorId) -> Option<Invitation> {
        self.company()?;
        self.scenario
            .recruits
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map(|(_, rule)| *rule)
    }

    pub(super) fn companion(&self, id: ActorId) -> Option<Journey> {
        self.company()?
            .journey
            .filter(|journey| journey.actor == id)
    }

    fn approach_actor(&self, id: ActorId) -> Result<&Npc, Rejection> {
        let npc = self
            .npc(id)
            .filter(|npc| npc.alive())
            .ok_or(Rejection::UnknownActor)?;
        self.require_reach(npc.position)?;
        Ok(npc)
    }

    pub(super) fn invite(&mut self, id: ActorId) -> Result<Outcome, Rejection> {
        let npc = self.approach_actor(id)?;
        let relation = self.social.relationship(id, PLAYER);
        if relation.refuses_aid() {
            return Err(Rejection::Refused);
        }
        let policy = self.invitation(id).ok_or(Rejection::BoundToDuty)?;
        if self.is_hall_teacher(npc) {
            return Err(Rejection::BoundToDuty);
        }
        if policy == Invitation::SchoolMember && self.hall_rank().is_none() {
            return Err(Rejection::MembershipRequired);
        }
        if policy == Invitation::LodgeMember
            && !self
                .politics()
                .is_some_and(|politics| politics.allegiance == super::Allegiance::Stonebank)
        {
            return Err(Rejection::OathConflict);
        }
        if policy == Invitation::LodgeMember
            && self
                .politics()
                .is_some_and(|politics| politics.lodge_favor < 3)
        {
            return Err(Rejection::FactionTrustRequired);
        }
        if npc.definition.role == Role::HallSteward
            && self
                .politics()
                .is_some_and(|politics| politics.hall_favor < 1)
        {
            return Err(Rejection::FactionTrustRequired);
        }
        if relation.familiarity == 0 {
            return Err(Rejection::IntroductionRequired);
        }
        // A pledged physical delivery/report is not compatible with joining a
        // travelling party. Reject without changing duty, inventory or company.
        // Keep revision 25's valid historical command outcomes unchanged.
        if self.rules.has_duty_coherence() && self.sect_duty_active(id) {
            return Err(Rejection::BoundToDuty);
        }
        let company = self.company.as_mut().expect("company campaign");
        if company.journey.is_some() {
            return Err(Rejection::CompanyFull);
        }
        company.journey = Some(Journey {
            actor: id,
            seen_at: self.tick,
            last_seen: self.player.position,
        });
        self.record_company(id, JourneyEvent::Joined);
        Ok(Outcome::JoinedCompany(id))
    }

    pub(super) fn provision(&mut self, id: ActorId) -> Result<Outcome, Rejection> {
        let npc = self.approach_actor(id)?;
        if self.companion(id).is_none() {
            return Err(Rejection::NotCompanion);
        }
        if self.social.relationship(id, PLAYER).refuses_aid() {
            return Err(Rejection::Refused);
        }
        if npc.packed_herbs >= PACK_CAPACITY {
            return Err(Rejection::PackFull);
        }
        if self.player.herbs == 0 {
            return Err(Rejection::NoCargo);
        }
        self.player.herbs -= 1;
        self.npcs.get_mut(&id).expect("adjacent actor").packed_herbs += 1;
        Ok(Outcome::Provisioned(id))
    }

    pub(super) fn part(&mut self, id: ActorId) -> Result<Outcome, Rejection> {
        self.approach_actor(id)?;
        if self.companion(id).is_none() {
            return Err(Rejection::NotCompanion);
        }
        self.end_journey(id, JourneyEvent::Dismissed);
        Ok(Outcome::Parted(id))
    }

    fn record_company(&mut self, actor: ActorId, kind: JourneyEvent) {
        let place = self.npcs[&actor].position;
        let witnessed = self.player_sees(place);
        self.company
            .as_mut()
            .expect("company campaign")
            .history
            .push(CompanyEvent {
                at: self.tick,
                actor,
                place,
                kind,
                witnessed,
            });
    }

    fn end_journey(&mut self, actor: ActorId, kind: JourneyEvent) {
        self.company.as_mut().expect("company campaign").journey = None;
        self.record_company(actor, kind);
    }

    pub(super) fn update_journey(&mut self) {
        let Some(journey) = self.company().and_then(|company| company.journey) else {
            return;
        };
        let npc = &self.npcs[&journey.actor];
        let reason = if !npc.alive() {
            Some(JourneyEvent::Fallen)
        } else if !self.player.alive() && self.perceives(npc.position, self.player.position) {
            Some(JourneyEvent::PlayerFallen)
        } else if self.is_hall_teacher(npc) {
            Some(JourneyEvent::Office)
        } else if (self.invitation(npc.id()) == Some(Invitation::SchoolMember)
            && self.hall_rank().is_none())
            || (self.invitation(npc.id()) == Some(Invitation::LodgeMember)
                && !self
                    .politics()
                    .is_some_and(|politics| politics.allegiance == super::Allegiance::Stonebank))
            || self.social.relationship(npc.id(), PLAYER).refuses_aid()
        {
            Some(JourneyEvent::Distrust)
        } else if self.player.alive() && self.perceives(npc.position, self.player.position) {
            self.company.as_mut().expect("company campaign").journey = Some(Journey {
                seen_at: self.tick,
                last_seen: self.player.position,
                ..journey
            });
            None
        } else if self.tick - journey.seen_at >= SEARCH_TURNS {
            Some(JourneyEvent::LostContact)
        } else {
            None
        };
        if let Some(reason) = reason {
            self.end_journey(journey.actor, reason);
        }
    }

    pub(super) fn observe_player_death(&mut self) {
        if !self.player.alive()
            && let Some(journey) =
                self.company()
                    .and_then(|company| company.journey)
                    .filter(|journey| {
                        self.perceives(self.npcs[&journey.actor].position, self.player.position)
                    })
        {
            self.end_journey(journey.actor, JourneyEvent::PlayerFallen);
        }
    }

    pub(super) fn company_goal(&self, npc: &Npc) -> Option<Goal> {
        let journey = self.companion(npc.id())?;
        if npc.packed_herbs > 0
            && self.player.hp < PLAYER_HP
            && self.player.alive()
            && in_reach(npc.position, self.player.position)
        {
            Some(Goal::FieldCare)
        } else {
            Some(Goal::Follow(journey.last_seen))
        }
    }

    pub(super) fn field_care(&mut self, id: ActorId) {
        // Recheck after earlier actors resolved combat this tick.
        if self.company_goal(&self.npcs[&id]) != Some(Goal::FieldCare) {
            return;
        }
        self.npcs.get_mut(&id).expect("companion").packed_herbs -= 1;
        self.player.hp = (self.player.hp + 2).min(PLAYER_HP);
        self.company.as_mut().expect("company campaign").herbs_used += 1;
        self.social.aided(id, PLAYER);
        self.record_company(id, JourneyEvent::FieldCare);
    }
}
