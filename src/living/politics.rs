//! Revision-seven political economy: the Hall runs a competing, physically
//! delivered herb operation. Public allegiance is a choice, not a global
//! knowledge shortcut for private acts of violence.

use super::navigation::in_reach;
use super::{ActorId, Duty, Goal, Outcome, PLAYER, Position, Region, Rejection, Role, World};

const HALL_OPENING_COINS: u32 = 18;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Allegiance {
    Independent,
    Stonebank,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoliticalEventKind {
    HallPurchase,
    HallSupply,
    LodgeOath,
    LodgeAid,
    LodgeRenunciation,
    HallPatronage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PoliticalEvent {
    pub at: u64,
    pub actor: ActorId,
    pub position: Position,
    pub kind: PoliticalEventKind,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Politics {
    pub allegiance: Allegiance,
    pub hall_coins: u32,
    pub hall_purchases: u32,
    pub hall_supplies: u32,
    pub lodge_oaths: u32,
    pub lodge_aid: u32,
    pub hall_favor: i8,
    pub lodge_favor: i8,
    pub history: Vec<PoliticalEvent>,
}

impl Politics {
    pub(super) fn new() -> Self {
        Self {
            allegiance: Allegiance::Independent,
            hall_coins: HALL_OPENING_COINS,
            hall_purchases: 0,
            hall_supplies: 0,
            lodge_oaths: 0,
            lodge_aid: 0,
            hall_favor: 0,
            lodge_favor: 0,
            history: Vec::new(),
        }
    }
}

impl World {
    pub fn politics(&self) -> Option<&Politics> {
        self.politics.as_ref()
    }

    pub(super) fn hall_can_buy(&self) -> bool {
        let reserve = if self.faction_agency_state().is_some() || self.contracts().is_some() {
            4
        } else {
            0
        };
        self.politics().is_some_and(|politics| {
            !self.is_night()
                && self.rivalry().is_some_and(|rivalry| rivalry.pressure > 0)
                && self.region().is_some_and(|region| region.market_herbs > 0)
                && self
                    .market_quote()
                    .is_some_and(|quote| politics.hall_coins >= quote.buy + reserve)
        })
    }

    pub(super) fn hall_steward_work(&mut self, id: ActorId, goal: Goal) {
        let sites = Region::sites();
        match goal {
            Goal::BuyForHall => {
                if !in_reach(self.npcs[&id].position, sites.market) {
                    self.travel(id, sites.market, 1);
                    return;
                }
                if !self.hall_can_buy() {
                    return;
                }
                let price = self.market_quote().expect("regional market").buy;
                let region = self.region.as_mut().expect("regional market");
                region.market_herbs -= 1;
                region.market_coins += price;
                let politics = self.politics.as_mut().expect("political rules");
                politics.hall_coins -= price;
                politics.hall_purchases += 1;
                self.npcs.get_mut(&id).expect("hall steward").herbs += 1;
                self.record_political_event(PoliticalEventKind::HallPurchase, id, sites.market);
            }
            Goal::SupplyHall => {
                if !in_reach(self.npcs[&id].position, sites.hall) {
                    self.travel(id, sites.hall, 1);
                    return;
                }
                if self.npcs[&id].herbs == 0 {
                    return;
                }
                self.npcs.get_mut(&id).expect("hall steward").herbs -= 1;
                self.region.as_mut().expect("regional rules").hall_offerings += 1;
                let rival = self.rivalry.as_mut().expect("sect campaign");
                rival.pressure = rival.pressure.saturating_sub(1);
                self.politics
                    .as_mut()
                    .expect("political rules")
                    .hall_supplies += 1;
                self.note_steward_hall_supply(id);
                self.record_political_event(PoliticalEventKind::HallSupply, id, sites.hall);
            }
            _ => unreachable!("only Hall economy goals"),
        }
    }

    pub(super) fn record_political_event(
        &mut self,
        kind: PoliticalEventKind,
        actor: ActorId,
        position: Position,
    ) {
        let witnessed = self.player_sees(position);
        self.politics
            .as_mut()
            .expect("political rules")
            .history
            .push(PoliticalEvent {
                at: self.tick,
                actor,
                position,
                kind,
                witnessed,
            });
    }

    fn envoy_for_oath(&self) -> Result<(ActorId, Position), Rejection> {
        let envoy = self
            .npcs
            .values()
            .find(|npc| npc.definition.role == Role::RivalEnvoy && npc.alive())
            .ok_or(Rejection::UnknownActor)?;
        self.require_reach(envoy.position)?;
        if self.social.relationship(envoy.id(), PLAYER).refuses_aid() {
            return Err(Rejection::Refused);
        }
        Ok((envoy.id(), envoy.position))
    }

    pub(super) fn pledge_lodge(&mut self) -> Result<Outcome, Rejection> {
        let (envoy, position) = self.envoy_for_oath()?;
        if self.politics().expect("political rules").allegiance == Allegiance::Stonebank {
            return Err(Rejection::OathConflict);
        }
        if self.social.relationship(envoy, PLAYER).familiarity == 0 {
            return Err(Rejection::IntroductionRequired);
        }
        if self
            .school()
            .is_some_and(|school| matches!(school.duty, Duty::Carrying | Duty::Delivered))
        {
            return Err(Rejection::BoundToDuty);
        }
        if self.player.herbs == 0 {
            return Err(Rejection::NoCargo);
        }
        self.player.herbs -= 1;
        self.npcs.get_mut(&envoy).expect("envoy").herbs += 1;
        let politics = self.politics.as_mut().expect("political rules");
        politics.allegiance = Allegiance::Stonebank;
        politics.lodge_oaths += 1;
        politics.lodge_favor = politics.lodge_favor.saturating_add(2).min(8);
        politics.hall_favor = politics.hall_favor.saturating_sub(2).max(-8);
        self.region.as_mut().expect("regional rules").enrolled = false;
        self.school.as_mut().expect("school rules").duty = Duty::Unassigned;
        self.remove_hierarchy_player();
        self.social.aided(PLAYER, envoy);
        self.record_political_event(PoliticalEventKind::LodgeOath, PLAYER, position);
        Ok(Outcome::LodgePledged)
    }

    pub(super) fn aid_lodge(&mut self) -> Result<Outcome, Rejection> {
        if self.politics().expect("political rules").allegiance != Allegiance::Stonebank {
            return Err(Rejection::OathConflict);
        }
        let (envoy, position) = self.envoy_for_oath()?;
        if self.player.herbs == 0 {
            return Err(Rejection::NoCargo);
        }
        self.player.herbs -= 1;
        self.npcs.get_mut(&envoy).expect("envoy").herbs += 1;
        let rival = self.rivalry.as_mut().expect("sect campaign");
        rival.pressure = rival.pressure.saturating_add(1).min(8);
        let politics = self.politics.as_mut().expect("political rules");
        politics.lodge_aid += 1;
        politics.lodge_favor = politics.lodge_favor.saturating_add(1).min(8);
        politics.hall_favor = politics.hall_favor.saturating_sub(1).max(-8);
        self.social.aided(PLAYER, envoy);
        self.record_political_event(PoliticalEventKind::LodgeAid, PLAYER, position);
        Ok(Outcome::LodgeAided)
    }

    pub(super) fn renounce_lodge(&mut self) -> Result<Outcome, Rejection> {
        self.require_reach(Region::sites().hall)?;
        if self.politics().expect("political rules").allegiance != Allegiance::Stonebank {
            return Err(Rejection::OathConflict);
        }
        let politics = self.politics.as_mut().expect("political rules");
        politics.allegiance = Allegiance::Independent;
        politics.lodge_favor = politics.lodge_favor.saturating_sub(3).max(-8);
        self.record_political_event(
            PoliticalEventKind::LodgeRenunciation,
            PLAYER,
            Region::sites().hall,
        );
        Ok(Outcome::LodgeRenounced)
    }
}
