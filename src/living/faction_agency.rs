//! Revision-ten autonomous faction response. Stonebank sends its own envoy to
//! inspect Qinghe. Only a local observation can launch an aid journey; the
//! envoy pays for a real herb (or redirects one it already carries), travels
//! back and delivers it. Reed Hall's independently initiated mission can
//! compete for the same finite market stock on the same canonical tick.

use super::navigation::in_reach;
use super::{ActorId, Goal, Npc, Position, Region, Role, World};

const FIRST_SURVEY: u64 = 24;
const SURVEY_INTERVAL: u64 = 112;
const CASE_TIMEOUT: u64 = 320;
const CRITICAL_STOCK: u32 = 1;
const MAX_EVENTS: usize = 128;
const MAX_PRESSURE: u8 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactionPhase {
    Survey,
    Procuring,
    Delivering,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FactionCase {
    pub opened: u64,
    pub envoy: ActorId,
    pub phase: FactionPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactionEventKind {
    SurveyClear,
    ShortageSeen,
    Purchased,
    Redirected,
    Delivered,
    HallDelivered,
    Withdrawn,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FactionEvent {
    pub at: u64,
    pub actor: ActorId,
    pub position: Position,
    pub kind: FactionEventKind,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct FactionAgency {
    pub active: Option<FactionCase>,
    pub surveys: u32,
    pub lodge_deliveries: u32,
    pub hall_deliveries: u32,
    /// A locally witnessed delivery changes Qinghe's sympathy; positive
    /// favors Reed Hall, negative favors Stonebank. It is not player knowledge.
    pub public_standing: i8,
    pub last_closed: u64,
    pub history: Vec<FactionEvent>,
}

impl FactionAgency {
    pub(super) fn new() -> Self {
        Self::default()
    }
}

impl World {
    pub fn faction_agency_state(&self) -> Option<&FactionAgency> {
        self.faction_agency.as_ref()
    }

    pub(super) fn advance_faction_agency(&mut self) {
        let Some(agency) = &self.faction_agency else {
            return;
        };
        if let Some(case) = agency.active {
            let (alive, position) = self
                .npcs
                .get(&case.envoy)
                .map_or((false, self.scenario.shrine), |npc| {
                    (npc.alive(), npc.position)
                });
            if !alive {
                self.close_faction_case(FactionEventKind::Failed, case.envoy, position);
            } else if self.companion(case.envoy).is_some() {
                // An existing company promise takes precedence over a patrol.
                self.close_faction_case(FactionEventKind::Withdrawn, case.envoy, position);
            } else if self.tick.saturating_sub(case.opened) >= CASE_TIMEOUT {
                self.close_faction_case(FactionEventKind::Failed, case.envoy, position);
            }
            return;
        }
        if self.tick < FIRST_SURVEY
            || (agency.surveys > 0
                && self.tick.saturating_sub(agency.last_closed) < SURVEY_INTERVAL)
        {
            return;
        }
        let Some(envoy) = self.npcs.values().find(|npc| {
            npc.definition.role == Role::RivalEnvoy
                && npc.alive()
                && self.companion(npc.id()).is_none()
        }) else {
            return;
        };
        let envoy = envoy.id();
        let agency = self.faction_agency.as_mut().expect("revision-ten agency");
        agency.active = Some(FactionCase {
            opened: self.tick,
            envoy,
            phase: FactionPhase::Survey,
        });
        agency.surveys = agency.surveys.saturating_add(1);
    }

    pub(super) fn faction_goal(&self, npc: &Npc) -> Option<Goal> {
        let case = self.faction_agency_state()?.active?;
        if npc.id() != case.envoy {
            return None;
        }
        Some(match case.phase {
            FactionPhase::Survey => Goal::SurveyQinghe,
            FactionPhase::Procuring if npc.herbs == 0 => Goal::BuyLodgeAid,
            FactionPhase::Procuring | FactionPhase::Delivering => Goal::DeliverLodgeAid,
        })
    }

    pub(super) fn work_faction_agency(&mut self, id: ActorId, goal: Goal) {
        let Some(case) = self.faction_agency_state().and_then(|agency| agency.active) else {
            return;
        };
        if case.envoy != id {
            return;
        }
        let position = self.npcs[&id].position;
        match goal {
            Goal::SurveyQinghe if case.phase == FactionPhase::Survey => {
                if !self.perceives(position, self.scenario.shrine) {
                    self.travel(id, self.scenario.shrine, 1);
                    return;
                }
                if self.supply.shrine > CRITICAL_STOCK {
                    self.close_faction_case(FactionEventKind::SurveyClear, id, position);
                } else {
                    self.faction_agency
                        .as_mut()
                        .expect("agency")
                        .active
                        .as_mut()
                        .expect("case")
                        .phase = FactionPhase::Procuring;
                    self.record_faction_event(FactionEventKind::ShortageSeen, id, position);
                }
            }
            Goal::BuyLodgeAid if case.phase == FactionPhase::Procuring => {
                if self.npcs[&id].herbs > 0 {
                    self.begin_faction_delivery(id, FactionEventKind::Redirected, position);
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
                let price = self.market_quote().expect("regional market").buy;
                if self.region().is_none_or(|region| region.market_herbs == 0)
                    || self.rivalry().is_none_or(|rival| rival.lodge_coins < price)
                {
                    return;
                }
                let region = self.region.as_mut().expect("regional market");
                region.market_herbs -= 1;
                region.market_coins += price;
                let rival = self.rivalry.as_mut().expect("lodge campaign");
                rival.lodge_coins -= price;
                rival.market_purchases += 1;
                self.npcs.get_mut(&id).expect("lodge envoy").herbs += 1;
                self.begin_faction_delivery(id, FactionEventKind::Purchased, position);
            }
            Goal::DeliverLodgeAid
                if matches!(
                    case.phase,
                    FactionPhase::Procuring | FactionPhase::Delivering
                ) =>
            {
                if case.phase == FactionPhase::Procuring && self.npcs[&id].herbs > 0 {
                    self.begin_faction_delivery(id, FactionEventKind::Redirected, position);
                }
                if self.npcs[&id].herbs == 0 {
                    self.faction_agency
                        .as_mut()
                        .expect("agency")
                        .active
                        .as_mut()
                        .expect("case")
                        .phase = FactionPhase::Procuring;
                    return;
                }
                if position != self.scenario.shrine {
                    self.travel(id, self.scenario.shrine, 0);
                    return;
                }
                self.npcs.get_mut(&id).expect("lodge envoy").herbs -= 1;
                self.supply.shrine += 1;
                self.supply.npc_deliveries += 1;
                let pressure = self.rivalry().expect("sect rivalry").pressure;
                self.rivalry.as_mut().expect("sect rivalry").pressure =
                    pressure.saturating_add(1).min(MAX_PRESSURE);
                self.faction_agency
                    .as_mut()
                    .expect("agency")
                    .lodge_deliveries += 1;
                self.credit_local_aid(id, false);
                self.close_faction_case(FactionEventKind::Delivered, id, position);
            }
            _ => {}
        }
    }

    fn begin_faction_delivery(&mut self, actor: ActorId, kind: FactionEventKind, at: Position) {
        self.faction_agency
            .as_mut()
            .expect("agency")
            .active
            .as_mut()
            .expect("case")
            .phase = FactionPhase::Delivering;
        self.record_faction_event(kind, actor, at);
    }

    fn close_faction_case(&mut self, kind: FactionEventKind, actor: ActorId, at: Position) {
        let agency = self.faction_agency.as_mut().expect("agency");
        agency.active = None;
        agency.last_closed = self.tick;
        self.record_faction_event(kind, actor, at);
    }

    /// C1.8's Hall mission is still autonomous. This hook adds a distinct
    /// consequence only in new campaigns; revision-nine digests remain exact.
    pub(super) fn credit_hall_relief(&mut self, actor: ActorId, at: Position) {
        if self.faction_agency.is_some() {
            self.faction_agency
                .as_mut()
                .expect("agency")
                .hall_deliveries += 1;
            self.credit_local_aid(actor, true);
            self.record_faction_event(FactionEventKind::HallDelivered, actor, at);
        }
    }

    fn credit_local_aid(&mut self, actor: ActorId, hall: bool) {
        // Local trust, not a world-wide faction reputation broadcast.
        let witnesses: Vec<_> = self
            .npcs
            .values()
            .filter(|npc| {
                npc.alive()
                    && npc.definition.role == Role::Watch
                    && self.perceives(npc.position, self.scenario.shrine)
            })
            .map(Npc::id)
            .collect();
        if witnesses.is_empty() {
            return;
        }
        let agency = self.faction_agency.as_mut().expect("agency");
        agency.public_standing = if hall {
            agency.public_standing.saturating_add(1).min(8)
        } else {
            agency.public_standing.saturating_sub(1).max(-8)
        };
        for witness in witnesses {
            self.social.aided(actor, witness);
        }
    }

    fn record_faction_event(&mut self, kind: FactionEventKind, actor: ActorId, at: Position) {
        let witnessed = self.player_sees(at);
        let agency = self.faction_agency.as_mut().expect("agency");
        if agency.history.len() == MAX_EVENTS {
            agency.history.remove(0);
        }
        agency.history.push(FactionEvent {
            at: self.tick,
            actor,
            position: at,
            kind,
            witnessed,
        });
    }
}
