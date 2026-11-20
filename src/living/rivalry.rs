//! A bounded, material-backed dispute between Reed Hall and the Willowford lodge.
//! The envoy uses the real market stock and physically reaches the hall. There is
//! no faction-wide knowledge of events that its members have not observed.

use super::navigation::in_reach;
use super::{ActorId, Goal, Outcome, PLAYER, Position, Region, Rejection, Role, World};

const OPENING_COINS: u32 = 16;
const CONTEST_THRESHOLD: u8 = 4;
const MAX_PRESSURE: u8 = 8;
const ACCORD_TURNS: u64 = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectEventKind {
    Purchased,
    Petitioned,
    Accord,
    Patronage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SectEvent {
    pub at: u64,
    pub kind: SectEventKind,
    pub actor: ActorId,
    pub position: Position,
    /// A personal observation, not a global announcement.
    pub witnessed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rivalry {
    pub pressure: u8,
    pub lodge_coins: u32,
    pub market_purchases: u32,
    pub petitions: u32,
    pub consumed_herbs: u32,
    pub accord_offerings: u32,
    pub hall_patronage: u32,
    pub peace_until: u64,
    pub history: Vec<SectEvent>,
}

impl Rivalry {
    pub(super) fn new() -> Self {
        Self {
            pressure: 0,
            lodge_coins: OPENING_COINS,
            market_purchases: 0,
            petitions: 0,
            consumed_herbs: 0,
            accord_offerings: 0,
            hall_patronage: 0,
            peace_until: 0,
            history: Vec::new(),
        }
    }

    pub fn at_peace(&self, tick: u64) -> bool {
        tick < self.peace_until
    }

    pub fn contested(&self, tick: u64) -> bool {
        self.pressure >= CONTEST_THRESHOLD && !self.at_peace(tick)
    }
}

impl World {
    pub fn rivalry(&self) -> Option<&Rivalry> {
        self.rivalry.as_ref()
    }

    pub(super) fn rival_can_buy(&self) -> bool {
        // Revision-ten institutions preserve four coins for observed crises.
        let reserve = if self.faction_agency_state().is_some() || self.contracts().is_some() {
            4
        } else {
            0
        };
        self.rivalry().is_some_and(|rival| {
            !rival.at_peace(self.tick)
                && !self.is_night()
                && self.region().is_some_and(|region| region.market_herbs > 0)
                && self
                    .market_quote()
                    .is_some_and(|quote| rival.lodge_coins >= quote.buy + reserve)
        })
    }

    fn record_sect_event(&mut self, kind: SectEventKind, actor: ActorId, position: Position) {
        let witnessed = self.player_sees(position);
        self.rivalry
            .as_mut()
            .expect("sect campaign")
            .history
            .push(SectEvent {
                at: self.tick,
                kind,
                actor,
                position,
                witnessed,
            });
    }

    pub(super) fn rival_work(&mut self, id: ActorId, goal: Goal) {
        let sites = Region::sites();
        match goal {
            Goal::BuyForLodge => {
                if !in_reach(self.npcs[&id].position, sites.market) {
                    self.travel(id, sites.market, 1);
                    return;
                }
                if !self.rival_can_buy() {
                    return;
                }
                let price = self.market_quote().expect("regional market").buy;
                let region = self.region.as_mut().expect("regional market");
                region.market_herbs -= 1;
                region.market_coins += price;
                let lodge = self.rivalry.as_mut().expect("sect campaign");
                lodge.lodge_coins -= price;
                lodge.market_purchases += 1;
                self.npcs.get_mut(&id).expect("envoy").herbs += 1;
                self.record_sect_event(SectEventKind::Purchased, id, sites.market);
            }
            Goal::PetitionHall => {
                if !in_reach(self.npcs[&id].position, sites.hall) {
                    self.travel(id, sites.hall, 1);
                    return;
                }
                if self
                    .rivalry()
                    .is_some_and(|rival| rival.at_peace(self.tick))
                {
                    return;
                }
                let teacher_present = self.npcs.values().any(|npc| {
                    npc.alive() && npc.position == sites.hall && self.is_hall_teacher(npc)
                });
                if !teacher_present || self.npcs[&id].herbs == 0 {
                    return;
                }
                self.npcs.get_mut(&id).expect("envoy").herbs -= 1;
                let lodge = self.rivalry.as_mut().expect("sect campaign");
                lodge.petitions += 1;
                lodge.consumed_herbs += 1;
                lodge.pressure = lodge.pressure.saturating_add(2).min(MAX_PRESSURE);
                self.record_sect_event(SectEventKind::Petitioned, id, sites.hall);
            }
            _ => unreachable!("only rival agenda goals"),
        }
    }

    pub(super) fn accord_with_lodge(&mut self) -> Result<Outcome, Rejection> {
        let envoy = self
            .npcs
            .values()
            .find(|npc| npc.definition.role == Role::RivalEnvoy && npc.alive())
            .ok_or(Rejection::UnknownActor)?;
        let id = envoy.id();
        let position = envoy.position;
        self.require_reach(position)?;
        if self.social.relationship(id, PLAYER).refuses_aid() {
            return Err(Rejection::Refused);
        }
        if self.player.herbs == 0 {
            return Err(Rejection::NoCargo);
        }
        self.player.herbs -= 1;
        let lodge = self.rivalry.as_mut().expect("sect campaign");
        lodge.accord_offerings += 1;
        lodge.pressure = lodge.pressure.saturating_sub(3);
        lodge.peace_until = self.tick + ACCORD_TURNS;
        self.record_sect_event(SectEventKind::Accord, PLAYER, position);
        Ok(Outcome::LodgeAccord {
            until: self.tick + ACCORD_TURNS,
        })
    }

    pub(super) fn support_reed_hall(&mut self) -> Result<Outcome, Rejection> {
        self.require_reach(Region::sites().hall)?;
        if !self.region().is_some_and(|region| region.enrolled) {
            return Err(Rejection::MembershipRequired);
        }
        if self.player.herbs == 0 {
            return Err(Rejection::NoCargo);
        }
        self.player.herbs -= 1;
        self.region
            .as_mut()
            .expect("regional campaign")
            .hall_offerings += 1;
        let lodge = self.rivalry.as_mut().expect("sect campaign");
        lodge.hall_patronage += 1;
        lodge.pressure = lodge.pressure.saturating_sub(2);
        self.record_sect_event(SectEventKind::Patronage, PLAYER, Region::sites().hall);
        if let Some(politics) = self.politics.as_mut() {
            politics.hall_favor = politics.hall_favor.saturating_add(1).min(8);
            politics.lodge_favor = politics.lodge_favor.saturating_sub(1).max(-8);
            self.record_political_event(
                super::PoliticalEventKind::HallPatronage,
                PLAYER,
                Region::sites().hall,
            );
        }
        Ok(Outcome::HallSupported)
    }
}
