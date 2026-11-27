use std::cmp::Reverse;

use super::character_agency::{Ambition, Decision, DecisionReason, ambition_reason, decision_for};
use super::navigation::{distance, in_reach, moved, next_step};
use super::{
    ActorId, BREATH_MASTERY, GUARD_BLOCK, IncidentId, KnowledgeSource, PLAYER, Position, Region,
    World,
};

const THREAT_MEMORY: u64 = 12;
const INVESTIGATE_WINDOW: u64 = 24;
const PURSUIT_LEASH: u32 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Medic,
    Watch,
    Supplier,
    Traveler,
    Grower,
    Courier,
    Mentor,
    HallDeputy,
    RivalEnvoy,
    HallSteward,
    FrontierCarrier,
    FrontierGrower,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NpcDefinition {
    pub id: ActorId,
    pub name: &'static str,
    pub glyph: char,
    pub role: Role,
    pub spawn: Position,
    pub home: Position,
    pub route: &'static [Position],
    pub hp: u16,
    /// Fatigue at which the actor normally breaks from duty to rest.
    pub rest_threshold: u8,
    pub responds_to_attack: bool,
    pub shares_news: bool,
    pub report_to: Option<ActorId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Goal {
    Idle,
    Rest,
    Patrol,
    TendClinic,
    CollectHerbs,
    DeliverHerbs,
    SeekCare,
    Treat(ActorId),
    Investigate(IncidentId),
    InvestigateBody(ActorId),
    RecoverBody(ActorId),
    ReturnRecoveredCargo,
    ClaimLot(u32),
    DeliverLot(u32),
    MartialPractice(ActorId),
    MartialRest,
    MeridianDraw,
    MeridianCirculate,
    MeridianCare,
    SeekWoundCare,
    GatherForCare,
    SellForCare,
    Report {
        recipient: ActorId,
        incident: IncidentId,
    },
    Retreat,
    Retaliate,
    StockGarden,
    StockMarket,
    CollectFreight,
    DeliverFreight,
    TeachAtHall,
    Follow(Position),
    FieldCare,
    BuyForLodge,
    PetitionHall,
    BuyForHall,
    SupplyHall,
    RequestRelief,
    BuyRelief,
    DeliverRelief,
    CampRest,
    SurveyQinghe,
    BuyLodgeAid,
    DeliverLodgeAid,
    NegotiateContract(super::Need),
    ProcureContract(super::Need),
    FulfillContract(super::Need),
    AuditContract(super::Need),
    DeliverTestimony {
        recipient: ActorId,
        event: super::EventId,
    },
    InvestigateDispute(u32),
    SectDuty,
    GatherFrontier,
    StockFrontierDepot,
    CollectFrontierFreight,
    DeliverFrontierFreight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Activity {
    pub goal: Goal,
    /// The tick when the current activity began. Preemption starts a new activity.
    pub since: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Npc {
    pub definition: &'static NpcDefinition,
    pub position: Position,
    pub hp: u16,
    pub fatigue: u8,
    pub herbs: u32,
    /// Personal medical supplies, retained when employment or company changes.
    pub packed_herbs: u8,
    pub activity: Activity,
    pub(super) waypoint: usize,
    pub(super) last_investigated: Option<IncidentId>,
}

impl Npc {
    pub(super) fn new(definition: &'static NpcDefinition) -> Self {
        Self {
            definition,
            position: definition.spawn,
            hp: definition.hp,
            fatigue: 0,
            herbs: 0,
            packed_herbs: 0,
            activity: Activity {
                goal: Goal::Idle,
                since: 0,
            },
            waypoint: 0,
            last_investigated: None,
        }
    }

    pub fn id(&self) -> ActorId {
        self.definition.id
    }

    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}

impl World {
    pub(super) fn advance_npcs(&mut self, guarding: bool) {
        // All actors plan before anyone moves, claims stock, or attacks. Commit
        // in ID order and revalidate any effects that compete for shared state.
        let plans: Vec<_> = self
            .npcs
            .values()
            .filter(|npc| npc.alive())
            .map(|npc| (npc.id(), self.choose_decision(npc)))
            .collect();
        for (id, decision) in plans {
            if self
                .traffic()
                .is_some_and(|traffic| traffic.was_committed(id))
            {
                continue;
            }
            self.commit_decision(id, decision);
            self.execute_goal(id, decision.goal, guarding);
            if self.character_agency().is_some()
                && matches!(decision.goal, Goal::Treat(_) | Goal::FieldCare)
            {
                self.traffic
                    .as_mut()
                    .expect("agency inherits traffic")
                    .mark_care_committed(id);
            }
            // A committed NPC may have traded, treated or reported without
            // walking. It still cannot be displaced by a later actor this turn.
            if let Some(traffic) = self.traffic.as_mut() {
                traffic.mark_committed(id);
            }
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn choose_goal(&self, npc: &Npc) -> Goal {
        self.choose_decision(npc).goal
    }

    pub(super) fn choose_decision(&self, npc: &Npc) -> Decision {
        let id = npc.id();
        let threat = self.player.alive()
            && self.perceives(npc.position, self.player.position)
            && self
                .social
                .recent(id, self.tick, THREAT_MEMORY)
                .any(|(incident, receipt)| {
                    incident.target == id
                        && incident.source == PLAYER
                        && receipt.source == KnowledgeSource::Observed
                });
        let retaliation = threat
            && npc.definition.responds_to_attack
            && npc.hp > 1
            && !self.severe_wounds(id)
            && distance(npc.definition.home, self.player.position) <= PURSUIT_LEASH;
        let care = self.supply.shrine > 0
            && npc.hp < npc.definition.hp
            && self.npcs.values().any(|medic| {
                medic.id() != id
                    && medic.definition.role == Role::Medic
                    && (medic.alive()
                        || (self.original_rules() && !self.perceives(npc.position, medic.position)))
                    && !self.social.relationship(id, medic.id()).refuses_aid()
                    && (self.original_rules()
                        || !self.social.relationship(medic.id(), id).refuses_aid())
            })
            && (self.original_rules() || self.can_approach(npc.position, self.scenario.clinic));
        let patient = (npc.definition.role == Role::Medic && self.supply.shrine > 0)
            .then(|| {
                self.npcs
                    .values()
                    .filter(|patient| self.can_treat(npc, patient))
                    .max_by_key(|patient| {
                        (patient.definition.hp - patient.hp, Reverse(patient.id()))
                    })
                    .map(|patient| Goal::Treat(patient.id()))
            })
            .flatten();
        let news = self.report_goal(npc);
        let testimony = self.testimony_goal(npc);
        let dispute_goal = self.dispute_goal(id);
        let disturbance = (npc.definition.role == Role::Watch)
            .then(|| {
                self.social
                    .recent(id, self.tick, INVESTIGATE_WINDOW)
                    .find(|(incident, _)| {
                        !incident.retaliation
                            && incident.source != id
                            && incident.target != id
                            && npc.last_investigated.is_none_or(|last| incident.id > last)
                    })
                    .map(|(incident, _)| Goal::Investigate(incident.id))
            })
            .flatten();
        let body_lead = (self.is_player_reporting() && npc.definition.role == Role::Watch)
            .then(|| {
                self.social
                    .discoveries
                    .values()
                    .filter(|discovery| {
                        discovery.observer == id
                            && matches!(discovery.source, KnowledgeSource::Reported { .. })
                    })
                    .filter(|discovery| {
                        self.npcs
                            .get(&discovery.body)
                            .is_some_and(|body| !body.alive())
                    })
                    .min_by_key(|discovery| (discovery.at, discovery.body))
                    .map(|discovery| Goal::InvestigateBody(discovery.body))
            })
            .flatten();
        let recovery = (self.is_physical_recovery()
            && npc.definition.role == Role::Watch
            && self.companion(id).is_none())
        .then(|| {
            if self.social.recovery_cargo.get(&id).copied().unwrap_or(0) > 0 {
                return Some(Goal::ReturnRecoveredCargo);
            }
            if self.supply.shrine >= self.scenario.shrine_target {
                return None;
            }
            self.social
                .discoveries
                .values()
                .filter(|receipt| {
                    receipt.observer == id && receipt.source == KnowledgeSource::Observed
                })
                .filter(|receipt| {
                    self.npcs
                        .get(&receipt.body)
                        .is_some_and(|body| !body.alive() && body.herbs > 0)
                })
                .min_by_key(|receipt| (receipt.at, receipt.body))
                .map(|receipt| Goal::RecoverBody(receipt.body))
        })
        .flatten();
        let lot_goal = self.lot_goal(id);
        let martial_goal = self.martial_goal(npc);
        let meridian_goal = self.npc_meridian_goal(id);
        let wound_goal = self.wound_goal(id);
        let sect_duty = self.sect_duty_goal(id);
        let ambition = self
            .character_agency()
            .map(|agency| agency.people[&id].ambition);
        let suspended = self
            .character_agency()
            .and_then(|agency| agency.people[&id].suspended);
        let martial_priority = if ambition == Some(Ambition::MartialMastery) {
            68
        } else {
            64
        };
        let cultivation_priority = if ambition == Some(Ambition::Cultivation) {
            69
        } else {
            65
        };
        let wound_reason = if matches!(wound_goal, Some(Goal::GatherForCare | Goal::SellForCare)) {
            DecisionReason::TreatmentFunds
        } else {
            DecisionReason::PhysicalRecovery
        };
        let teaches = self.is_hall_teacher(npc);
        let off_duty = !self.original_rules()
            && self.is_night()
            && (teaches
                || matches!(
                    npc.definition.role,
                    Role::Medic | Role::Supplier | Role::Grower
                ));
        let resting = off_duty
            || npc.fatigue >= npc.definition.rest_threshold
            || (npc.activity.goal == Goal::Rest && npc.fatigue > 0);
        let duty = match npc.definition.role {
            Role::FrontierCarrier | Role::FrontierGrower => self.frontier_duty(npc),
            Role::Medic => Goal::TendClinic,
            Role::Supplier if npc.herbs > 0 => Goal::DeliverHerbs,
            Role::Supplier
                if self.supply.shrine < self.scenario.shrine_target
                    && self.supply.source > 0
                    && (self.company.is_none() || self.west_shortage() > 0) =>
            {
                Goal::CollectHerbs
            }
            Role::Grower if npc.herbs > 0 => Goal::StockMarket,
            Role::Grower if self.needs_east_harvest() => Goal::StockGarden,
            Role::Courier if npc.herbs > 0 => Goal::DeliverFreight,
            Role::Courier if self.needs_courier_supply() => Goal::CollectFreight,
            Role::RivalEnvoy
                if self
                    .rivalry()
                    .is_some_and(|rival| rival.at_peace(self.tick)) =>
            {
                Goal::Patrol
            }
            Role::RivalEnvoy if npc.herbs > 0 => Goal::PetitionHall,
            Role::RivalEnvoy if self.rival_can_buy() => Goal::BuyForLodge,
            Role::HallSteward if npc.herbs > 0 => Goal::SupplyHall,
            Role::HallSteward if self.hall_can_buy() => Goal::BuyForHall,
            _ if teaches => Goal::TeachAtHall,
            _ if !npc.definition.route.is_empty() => Goal::Patrol,
            _ => Goal::Idle,
        };
        let expedition = self.company_goal(npc);
        let expedition_priority = if expedition == Some(Goal::FieldCare) {
            79
        } else {
            40
        };
        let faction_goal = self.faction_goal(npc);
        let contract_goal = self.contract_goal(npc);
        let audit_goal = self.audit_goal(id);
        let sect_duty_in_progress = self.sect_duty_active(id) && sect_duty.is_some();
        let mission_active = self.relief_goal(npc).is_some()
            || faction_goal.is_some()
            || contract_goal.is_some()
            || audit_goal.is_some()
            || dispute_goal.is_some()
            || sect_duty_in_progress;
        let plans = [
            (
                100,
                retaliation.then_some(Goal::Retaliate),
                DecisionReason::ImmediateDanger,
            ),
            (
                100,
                (threat && !retaliation).then_some(Goal::Retreat),
                DecisionReason::ImmediateDanger,
            ),
            (
                99,
                (recovery.is_some() && npc.fatigue >= 36).then_some(Goal::CampRest),
                DecisionReason::PhysicalRecovery,
            ),
            (98, recovery, DecisionReason::InstitutionalObligation),
            (
                87,
                (npc.definition.role == Role::FrontierCarrier && npc.fatigue >= 24)
                    .then_some(Goal::CampRest),
                DecisionReason::Fatigue,
            ),
            (97, lot_goal, DecisionReason::KnownOpportunity),
            (
                96,
                (self.character_agency().is_some() && wound_goal.is_some() && npc.fatigue >= 36)
                    .then_some(Goal::CampRest),
                DecisionReason::PhysicalRecovery,
            ),
            (
                if self.severe_wounds(id) { 99 } else { 90 },
                wound_goal,
                wound_reason,
            ),
            (
                martial_priority,
                martial_goal,
                ambition_reason(npc, martial_goal.unwrap_or(duty)),
            ),
            (
                if meridian_goal == Some(Goal::MeridianCare) {
                    89
                } else {
                    cultivation_priority
                },
                meridian_goal,
                if meridian_goal == Some(Goal::MeridianCare) {
                    DecisionReason::PhysicalRecovery
                } else {
                    DecisionReason::Cultivation
                },
            ),
            (
                96,
                (mission_active && npc.fatigue >= 36).then_some(Goal::CampRest),
                DecisionReason::PhysicalRecovery,
            ),
            (
                95,
                self.relief_goal(npc),
                DecisionReason::InstitutionalObligation,
            ),
            (94, faction_goal, DecisionReason::InstitutionalObligation),
            (95, contract_goal, DecisionReason::InstitutionalObligation),
            (91, audit_goal, DecisionReason::InstitutionalObligation),
            (88, dispute_goal, DecisionReason::InstitutionalObligation),
            (
                86,
                (care && npc.hp <= npc.definition.hp / 2).then_some(Goal::SeekCare),
                DecisionReason::PhysicalRecovery,
            ),
            (80, patient, DecisionReason::MedicalDuty),
            (
                expedition_priority,
                expedition,
                if expedition == Some(Goal::FieldCare) {
                    DecisionReason::MedicalDuty
                } else {
                    DecisionReason::KnownOpportunity
                },
            ),
            (76, testimony, DecisionReason::InstitutionalObligation),
            (74, news, DecisionReason::KnownOpportunity),
            (70, body_lead, DecisionReason::InstitutionalObligation),
            (68, disturbance, DecisionReason::InstitutionalObligation),
            (
                if sect_duty_in_progress { 90 } else { 67 },
                sect_duty,
                DecisionReason::InstitutionalObligation,
            ),
            (
                54 + u16::from(npc.fatigue),
                resting.then_some(Goal::Rest),
                DecisionReason::Fatigue,
            ),
            (
                52,
                care.then_some(Goal::SeekCare),
                DecisionReason::PhysicalRecovery,
            ),
            (20, Some(duty), ambition_reason(npc, duty)),
        ];
        let (score, _, goal, reason) = plans
            .into_iter()
            .enumerate()
            .filter_map(|(rank, (score, goal, reason))| {
                goal.map(|goal| {
                    let score = if suspended == Some(goal) {
                        score.saturating_add(3)
                    } else {
                        score
                    };
                    (score, Reverse(rank), goal, reason)
                })
            })
            .max_by_key(|&(score, rank, _, _)| (score, rank))
            .expect("duty is always available");
        decision_for(goal, reason, score)
    }

    fn report_goal(&self, npc: &Npc) -> Option<Goal> {
        if !npc.definition.shares_news {
            return None;
        }
        let preferred = npc.definition.report_to;
        let proposal = |peer: &Npc| {
            let recipient = peer.id();
            let incident = self.reportable(npc.id(), recipient)?;
            if self.original_rules() {
                return (peer.alive() || !self.perceives(npc.position, peer.position)).then_some(
                    Goal::Report {
                        recipient,
                        incident,
                    },
                );
            }
            (peer.alive()
                && self.can_approach(npc.position, self.meeting_place(npc.position, peer)))
            .then_some(Goal::Report {
                recipient,
                incident,
            })
        };
        let primary = preferred
            .and_then(|id| self.npcs.get(&id))
            .and_then(&proposal);
        if primary.is_some() || self.original_rules() || preferred.is_none() {
            return primary;
        }
        // `reportable` checks familiarity, trust, receipt and hop rules. A bystander
        // outside that contact network can never be selected by this fallback.
        self.npcs
            .values()
            .filter(|peer| Some(peer.id()) != preferred && peer.alive())
            .find_map(proposal)
    }

    pub(super) fn meeting_place(&self, observer: Position, peer: &Npc) -> Position {
        if self.perceives(observer, peer.position) {
            peer.position
        } else if peer.definition.role == Role::Medic {
            self.scenario.clinic
        } else {
            peer.definition
                .route
                .first()
                .copied()
                .unwrap_or(peer.definition.home)
        }
    }

    pub(super) fn can_approach(&self, from: Position, destination: Position) -> bool {
        in_reach(from, destination)
            || next_step(self.scenario, from, destination, 1, |cell| {
                self.walkable(cell) && !self.occupied(cell)
            })
            .is_some()
    }

    fn can_treat(&self, medic: &Npc, patient: &Npc) -> bool {
        medic.id() != patient.id()
            && patient.alive()
            && patient.hp < patient.definition.hp
            && in_reach(self.scenario.clinic, patient.position)
            && !self
                .social
                .relationship(medic.id(), patient.id())
                .refuses_aid()
    }

    fn execute_goal(&mut self, id: ActorId, goal: Goal, guarding: bool) {
        let npc = self.npcs[&id];
        let activity = &mut self.npcs.get_mut(&id).expect("planned actor").activity;
        if activity.goal != goal {
            *activity = Activity {
                goal,
                since: self.tick,
            };
        }
        if goal == Goal::Rest
            && (npc.position == npc.definition.home || self.companion(id).is_some())
        {
            self.npcs.get_mut(&id).expect("planned actor").fatigue = npc.fatigue.saturating_sub(4);
            self.recover_martial(id);
            return;
        }
        let fatigue = npc.fatigue.saturating_add(1).min(40);
        self.npcs.get_mut(&id).expect("planned actor").fatigue = fatigue;
        let pos = npc.position;
        let scenario = self.scenario;

        match goal {
            Goal::Idle => {}
            Goal::Rest => self.travel(id, npc.definition.home, 0),
            Goal::CampRest => {
                self.npcs.get_mut(&id).expect("camping actor").fatigue =
                    npc.fatigue.saturating_sub(7);
                self.recover_martial(id);
            }
            Goal::MartialRest => {
                self.recover_martial(id);
                self.npcs.get_mut(&id).expect("resting student").fatigue =
                    npc.fatigue.saturating_sub(4);
            }
            Goal::MartialPractice(teacher) => self.work_martial(id, teacher),
            Goal::MeridianDraw | Goal::MeridianCirculate | Goal::MeridianCare => {
                self.work_meridian(id, goal)
            }
            Goal::SeekWoundCare => self.work_wound_care(id),
            Goal::GatherForCare | Goal::SellForCare => self.work_for_care(id, goal),
            Goal::SectDuty => self.work_sect_duty(id),
            Goal::GatherFrontier
            | Goal::StockFrontierDepot
            | Goal::CollectFrontierFreight
            | Goal::DeliverFrontierFreight => self.work_frontier(id, goal),
            Goal::Retreat => self.retreat(id),
            Goal::TendClinic => self.travel(id, scenario.clinic, 0),
            Goal::SeekCare => self.travel(id, scenario.clinic, 1),
            Goal::Patrol => {
                let route = npc.definition.route;
                let destination = route[npc.waypoint];
                if pos == destination {
                    self.npcs.get_mut(&id).expect("planned actor").waypoint =
                        (npc.waypoint + 1) % route.len();
                } else {
                    self.travel(id, destination, 0);
                }
            }
            Goal::CollectHerbs if pos == scenario.patch => {
                // Commit-time stock and quota check for competing suppliers.
                let shortage = if self.company.is_some() {
                    self.west_shortage()
                } else {
                    let incoming: u32 = self
                        .npcs
                        .values()
                        .filter(|actor| actor.alive())
                        .map(|actor| actor.herbs)
                        .sum();
                    scenario
                        .shrine_target
                        .saturating_sub(self.supply.shrine + incoming)
                };
                let quantity = self.supply.source.min(2).min(shortage);
                self.supply.source -= quantity;
                self.npcs.get_mut(&id).expect("planned actor").herbs += quantity;
            }
            Goal::CollectHerbs => self.travel(id, scenario.patch, 0),
            Goal::DeliverHerbs if pos == scenario.shrine => {
                self.supply.shrine += npc.herbs;
                self.supply.npc_deliveries += npc.herbs;
                self.npcs.get_mut(&id).expect("planned actor").herbs = 0;
            }
            Goal::DeliverHerbs => self.travel(id, scenario.shrine, 0),
            Goal::Treat(patient) if pos == scenario.clinic => {
                if self.supply.shrine > 0 && self.can_treat(&npc, &self.npcs[&patient]) {
                    self.treat(patient);
                    self.social.aided(id, patient);
                }
            }
            Goal::Treat(_) => self.travel(id, scenario.clinic, 0),
            Goal::Investigate(incident) => {
                let site = self
                    .social
                    .incident(incident)
                    .expect("known incident")
                    .position;
                if in_reach(pos, site) {
                    self.npcs
                        .get_mut(&id)
                        .expect("planned actor")
                        .last_investigated = Some(incident);
                } else {
                    self.travel(id, site, 1);
                }
            }
            Goal::InvestigateBody(body) => {
                // A corpse stays at the site described by the reporting
                // witness. Walking there costs real turns; only sight on
                // arrival upgrades a secondhand report to direct discovery.
                let site = self.npcs[&body].position;
                if !self.perceives(pos, site) {
                    self.travel(id, site, 1);
                }
            }
            Goal::RecoverBody(body) => self.recover_corpse_cargo(id, body),
            Goal::ReturnRecoveredCargo => self.deliver_recovered_cargo(id),
            Goal::ClaimLot(_) | Goal::DeliverLot(_) => self.work_lot(id, goal),
            Goal::Report { recipient, .. } => {
                let peer = &self.npcs[&recipient];
                // Outside visual range, go to a known work site or home.
                // A report goal cannot track a moving recipient through a wall.
                self.travel(id, self.meeting_place(pos, peer), 1);
            }
            Goal::Retaliate => {
                // A lower-ID actor may already have killed the player.
                if self.player.alive() && in_reach(pos, self.player.position) {
                    let block = if guarding {
                        GUARD_BLOCK + (self.breath >= BREATH_MASTERY) as u16
                    } else {
                        0
                    };
                    let damage = self.npc_martial_damage(id);
                    self.damage(
                        id,
                        PLAYER,
                        self.wounded_strike(id, damage.saturating_sub(block)),
                        true,
                    );
                } else if self.player.alive() {
                    self.travel(id, self.player.position, 1);
                }
            }
            Goal::StockGarden | Goal::StockMarket | Goal::CollectFreight | Goal::DeliverFreight => {
                self.work_freight(id, goal);
            }
            Goal::Follow(site) => {
                if let Some(journey) = self.companion(id) {
                    self.travel(id, site, u32::from(journey.seen_at == self.tick));
                }
            }
            Goal::FieldCare => self.field_care(id),
            Goal::TeachAtHall => self.travel(id, Region::sites().hall, 0),
            Goal::BuyForLodge | Goal::PetitionHall => self.rival_work(id, goal),
            Goal::BuyForHall | Goal::SupplyHall => self.hall_steward_work(id, goal),
            Goal::RequestRelief | Goal::BuyRelief | Goal::DeliverRelief => {
                self.work_relief(id, goal);
            }
            Goal::SurveyQinghe | Goal::BuyLodgeAid | Goal::DeliverLodgeAid => {
                self.work_faction_agency(id, goal);
            }
            Goal::NegotiateContract(_) | Goal::ProcureContract(_) | Goal::FulfillContract(_) => {
                self.work_contract(id, goal);
            }
            Goal::AuditContract(need) => self.inspect_contract(id, need),
            Goal::DeliverTestimony { recipient, event } => {
                let meeting = self
                    .npcs
                    .get(&recipient)
                    .map(|peer| self.meeting_place(pos, peer));
                if let Some(destination) = meeting {
                    if in_reach(pos, destination) {
                        // Face-to-face presence queues the delayed institutional report.
                    } else {
                        self.travel(id, destination, 1);
                    }
                }
                let _ = event;
            }
            Goal::InvestigateDispute(dispute_id) => self.investigate_dispute(id, dispute_id),
        }
    }

    fn work_freight(&mut self, id: ActorId, goal: Goal) {
        let sites = Region::sites();
        let (site, loading, grower) = match goal {
            Goal::StockGarden => (sites.garden, true, true),
            Goal::StockMarket => (sites.market, false, true),
            Goal::CollectFreight => (sites.market, true, false),
            Goal::DeliverFreight => (self.scenario.shrine, false, false),
            _ => unreachable!("only freight goals reach this handler"),
        };
        // A carrier works from an adjacent square, so standing on a trading
        // tile does not disable the settlement's logistics.
        if !in_reach(self.npcs[&id].position, site) {
            self.travel(id, site, 1);
        } else if loading {
            self.load_regional_cargo(id, grower);
        } else {
            self.unload_regional_cargo(id, grower);
        }
    }

    fn retreat(&mut self, id: ActorId) {
        if self.hobbled(id) {
            return;
        }
        let npc = &self.npcs[&id];
        let threat = self.player.position;
        let current = distance(npc.position, threat);
        let safer = super::DIRECTIONS
            .into_iter()
            .enumerate()
            .filter_map(|(order, direction)| {
                moved(npc.position, direction).map(|cell| (order, cell))
            })
            .filter(|&(_, cell)| self.walkable(cell) && !self.occupied(cell))
            .filter(|&(_, cell)| distance(cell, threat) > current || !self.can_see(threat, cell))
            .max_by_key(|&(order, cell)| {
                (
                    !self.can_see(threat, cell),
                    distance(cell, threat),
                    Reverse(order),
                )
            });
        if let Some((_, cell)) = safer {
            self.npcs.get_mut(&id).expect("planned actor").position = cell;
        }
    }

    pub(super) fn travel(&mut self, id: ActorId, destination: Position, reach: u32) {
        if self.hobbled(id) {
            return;
        }
        if self.traffic.is_some() {
            self.travel_with_courtesy(id, destination, reach);
            return;
        }
        let position = self.npcs[&id].position;
        if let Some(next) = next_step(self.scenario, position, destination, reach, |cell| {
            self.walkable(cell) && !self.occupied(cell)
        }) {
            self.npcs.get_mut(&id).expect("planned actor").position = next;
        }
    }
}
