//! Revision-nine crisis response. A guard has to see a shortage, personally
//! reach the Hall steward, and then the steward has to acquire and physically
//! carry a real herb back to Qinghe. No global faction telepathy or free stock.

use super::navigation::in_reach;
use super::{ActorId, Goal, Npc, Position, Region, Role, World};

const FIRST_REQUEST_TICK: u64 = 48;
const REQUEST_COOLDOWN: u64 = 24;
const MISSION_TIMEOUT: u64 = 240;
const CRITICAL_STOCK: u32 = 1;
const EVENT_CAP: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReliefPhase {
    SeekingSponsor,
    Procuring,
    Delivering,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReliefCase {
    pub opened: u64,
    pub messenger: ActorId,
    pub sponsor: ActorId,
    pub phase: ReliefPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReliefEventKind {
    Observed,
    Reported,
    Purchased,
    Redirected,
    Delivered,
    Withdrawn,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReliefEvent {
    pub at: u64,
    pub actor: ActorId,
    pub position: Position,
    pub kind: ReliefEventKind,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Relief {
    pub active: Option<ReliefCase>,
    pub requests: u32,
    pub deliveries: u32,
    pub failures: u32,
    pub last_closed: u64,
    pub history: Vec<ReliefEvent>,
}

impl Relief {
    pub(super) fn new() -> Self {
        Self::default()
    }
}

impl World {
    pub fn relief(&self) -> Option<&Relief> {
        self.relief.as_ref()
    }

    /// Observations are local. The guard must physically see the shrine;
    /// he does not know that a distant courier may already be on the road.
    pub(super) fn advance_relief(&mut self) {
        let Some(relief) = self.relief.as_ref() else {
            return;
        };
        if let Some(case) = relief.active {
            let messenger = self.npcs.get(&case.messenger);
            let sponsor = self.npcs.get(&case.sponsor);
            if !messenger.is_some_and(Npc::alive) || !sponsor.is_some_and(Npc::alive) {
                let actor = if !sponsor.is_some_and(Npc::alive) {
                    case.sponsor
                } else {
                    case.messenger
                };
                let site = self.npcs[&actor].position;
                self.close_relief(ReliefEventKind::Failed, actor, site);
            } else if self.companion(case.sponsor).is_some() {
                // A promised travelling companion cannot silently abandon
                // the player for an institutional delivery.
                self.close_relief(
                    ReliefEventKind::Withdrawn,
                    case.sponsor,
                    self.npcs[&case.sponsor].position,
                );
            } else if case.phase != ReliefPhase::Delivering
                && self.supply.shrine > CRITICAL_STOCK
                && (self.faction_agency.is_none()
                    || messenger
                        .is_some_and(|npc| self.perceives(npc.position, self.scenario.shrine)))
            {
                self.close_relief(
                    ReliefEventKind::Withdrawn,
                    case.messenger,
                    self.npcs[&case.messenger].position,
                );
            } else if self.tick.saturating_sub(case.opened) >= MISSION_TIMEOUT {
                self.close_relief(
                    ReliefEventKind::Failed,
                    case.sponsor,
                    self.npcs[&case.sponsor].position,
                );
            }
            return;
        }
        if self.tick < FIRST_REQUEST_TICK
            || self.tick.saturating_sub(relief.last_closed) < REQUEST_COOLDOWN
            || self.supply.shrine > CRITICAL_STOCK
        {
            return;
        }
        let guard = self.npcs.values().find(|npc| {
            npc.alive()
                && npc.definition.role == Role::Watch
                && self.perceives(npc.position, self.scenario.shrine)
        });
        let sponsor = self.npcs.values().find(|npc| {
            npc.alive()
                && npc.definition.role == Role::HallSteward
                && self.companion(npc.id()).is_none()
        });
        let (Some(guard), Some(sponsor)) = (guard, sponsor) else {
            return;
        };
        let (messenger, site, sponsor) = (guard.id(), guard.position, sponsor.id());
        let relief = self.relief.as_mut().expect("agency rules");
        relief.requests = relief.requests.saturating_add(1);
        relief.active = Some(ReliefCase {
            opened: self.tick,
            messenger,
            sponsor,
            phase: ReliefPhase::SeekingSponsor,
        });
        self.record_relief(ReliefEventKind::Observed, messenger, site);
    }

    pub(super) fn relief_goal(&self, npc: &Npc) -> Option<Goal> {
        let case = self.relief()?.active?;
        match case.phase {
            ReliefPhase::SeekingSponsor if npc.id() == case.messenger => Some(Goal::RequestRelief),
            ReliefPhase::Procuring if npc.id() == case.sponsor => Some(if npc.herbs > 0 {
                Goal::DeliverRelief
            } else {
                Goal::BuyRelief
            }),
            ReliefPhase::Delivering if npc.id() == case.sponsor => Some(Goal::DeliverRelief),
            _ => None,
        }
    }

    pub(super) fn work_relief(&mut self, id: ActorId, goal: Goal) {
        let Some(case) = self.relief().and_then(|relief| relief.active) else {
            return;
        };
        let position = self.npcs[&id].position;
        match goal {
            Goal::RequestRelief if case.messenger == id => {
                let sponsor_position = self.npcs[&case.sponsor].position;
                let sponsor_home = self.npcs[&case.sponsor].definition.home;
                if in_reach(position, sponsor_position) {
                    if self.social.relationship(case.sponsor, id).refuses_aid() {
                        self.close_relief(ReliefEventKind::Failed, case.sponsor, sponsor_position);
                        return;
                    }
                    self.relief
                        .as_mut()
                        .expect("agency rules")
                        .active
                        .as_mut()
                        .expect("active case")
                        .phase = ReliefPhase::Procuring;
                    self.record_relief(ReliefEventKind::Reported, id, position);
                } else {
                    // The guard knows the steward's workplace, not his location
                    // through walls. He waits there if the steward is away.
                    self.travel(id, sponsor_home, 1);
                }
            }
            Goal::BuyRelief if case.sponsor == id && case.phase == ReliefPhase::Procuring => {
                if self.npcs[&id].herbs > 0 {
                    self.begin_relief_delivery(id, ReliefEventKind::Redirected, position);
                    return;
                }
                let market = Region::sites().market;
                if !in_reach(position, market) {
                    self.travel(id, market, 1);
                    return;
                }
                if self.is_night() {
                    return;
                }
                let quote = self.market_quote().expect("regional market");
                if self.region().is_none_or(|region| region.market_herbs == 0)
                    || self
                        .politics()
                        .is_none_or(|politics| politics.hall_coins < quote.buy)
                {
                    return;
                }
                let region = self.region.as_mut().expect("agency region");
                region.market_herbs -= 1;
                region.market_coins += quote.buy;
                let politics = self.politics.as_mut().expect("agency politics");
                politics.hall_coins -= quote.buy;
                politics.hall_purchases += 1;
                self.npcs.get_mut(&id).expect("relief steward").herbs += 1;
                self.begin_relief_delivery(id, ReliefEventKind::Purchased, position);
            }
            Goal::DeliverRelief if case.sponsor == id => {
                if case.phase == ReliefPhase::Procuring && self.npcs[&id].herbs > 0 {
                    self.begin_relief_delivery(id, ReliefEventKind::Redirected, position);
                }
                if self.npcs[&id].herbs == 0 {
                    self.relief
                        .as_mut()
                        .expect("agency rules")
                        .active
                        .as_mut()
                        .expect("active case")
                        .phase = ReliefPhase::Procuring;
                    return;
                }
                if position != self.scenario.shrine {
                    self.travel(id, self.scenario.shrine, 0);
                    return;
                }
                self.npcs.get_mut(&id).expect("relief steward").herbs -= 1;
                self.supply.shrine += 1;
                self.supply.npc_deliveries += 1;
                let rivalry = self.rivalry.as_mut().expect("sect pressure");
                rivalry.pressure = rivalry.pressure.saturating_sub(1);
                self.close_relief(ReliefEventKind::Delivered, id, position);
            }
            _ => {}
        }
    }

    fn begin_relief_delivery(&mut self, actor: ActorId, event: ReliefEventKind, at: Position) {
        self.relief
            .as_mut()
            .expect("agency rules")
            .active
            .as_mut()
            .expect("active case")
            .phase = ReliefPhase::Delivering;
        self.record_relief(event, actor, at);
    }

    fn close_relief(&mut self, kind: ReliefEventKind, actor: ActorId, at: Position) {
        let relief = self.relief.as_mut().expect("agency rules");
        relief.active = None;
        relief.last_closed = self.tick;
        if kind == ReliefEventKind::Delivered {
            relief.deliveries = relief.deliveries.saturating_add(1);
        } else if kind == ReliefEventKind::Failed {
            relief.failures = relief.failures.saturating_add(1);
        }
        self.record_relief(kind, actor, at);
        if kind == ReliefEventKind::Delivered {
            self.credit_hall_relief(actor, at);
        }
    }

    fn record_relief(&mut self, kind: ReliefEventKind, actor: ActorId, at: Position) {
        let witnessed = self.player_sees(at);
        let relief = self.relief.as_mut().expect("agency rules");
        if relief.history.len() == EVENT_CAP {
            relief.history.remove(0);
        }
        relief.history.push(ReliefEvent {
            at: self.tick,
            actor,
            position: at,
            kind,
            witnessed,
        });
    }
}
