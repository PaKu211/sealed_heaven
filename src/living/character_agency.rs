//! Persistent personal motives, finances and bounded decision evidence.
//!
//! The existing world systems still propose and execute goals. This module
//! records why a character changed course and supplies one material livelihood
//! fallback when an injured character cannot afford clinic care.

use std::collections::BTreeMap;

use super::navigation::in_reach;
use super::{ActorId, Goal, Npc, Position, Region, Role, Scenario, World};

pub const CLINIC_FEE: u32 = 1;
const HISTORY_CAP: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ambition {
    CommunityService,
    Prosperity,
    MartialMastery,
    Cultivation,
    InstitutionalDuty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionReason {
    ImmediateDanger,
    PhysicalRecovery,
    TreatmentFunds,
    MedicalDuty,
    Cultivation,
    MartialAmbition,
    InstitutionalObligation,
    KnownOpportunity,
    Fatigue,
    Livelihood,
    Routine,
    ResumingAmbition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Decision {
    pub goal: Goal,
    pub reason: DecisionReason,
    pub score: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CharacterState {
    pub coins: u32,
    /// Herbs gathered specifically as personal livelihood stock. This is
    /// provenance over `Npc::herbs`, not a second quantity ledger.
    pub livelihood_herbs: u8,
    pub ambition: Ambition,
    pub suspended: Option<Goal>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgencyEventKind {
    GoalChanged,
    WorkGathered,
    WorkSold,
    CarePaid,
    CareBartered,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgencyEvent {
    pub id: u32,
    pub at: u64,
    pub actor: ActorId,
    pub kind: AgencyEventKind,
    pub goal: Option<Goal>,
    pub reason: Option<DecisionReason>,
    pub amount: u32,
    pub other: Option<ActorId>,
    pub site: Position,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterAgency {
    pub people: BTreeMap<ActorId, CharacterState>,
    /// Fees paid when a medic treats their own wounds at the clinic.
    pub clinic_coins: u32,
    pub history: Vec<AgencyEvent>,
    pub next_event: u32,
}

impl CharacterAgency {
    pub(super) fn new(scenario: &Scenario) -> Self {
        let people = scenario
            .npcs
            .iter()
            .map(|npc| {
                let ambition = match npc.role {
                    Role::Medic | Role::Watch => Ambition::CommunityService,
                    Role::Supplier | Role::Grower | Role::Courier | Role::Traveler => {
                        Ambition::Prosperity
                    }
                    Role::Mentor | Role::HallDeputy => Ambition::MartialMastery,
                    Role::RivalEnvoy => Ambition::Cultivation,
                    Role::HallSteward => Ambition::InstitutionalDuty,
                };
                (
                    npc.id,
                    CharacterState {
                        coins: 0,
                        livelihood_herbs: 0,
                        ambition,
                        suspended: None,
                    },
                )
            })
            .collect();
        Self {
            people,
            clinic_coins: 0,
            history: Vec::new(),
            next_event: 0,
        }
    }
}

impl World {
    pub fn character_agency(&self) -> Option<&CharacterAgency> {
        self.character_agency.as_ref()
    }

    pub(super) fn personal_care_goal(&self, actor: ActorId) -> Goal {
        let state = self.character_agency().expect("agency rules").people[&actor];
        if self.supply.shrine > 0 && state.coins >= CLINIC_FEE {
            return Goal::SeekWoundCare;
        }
        if state.livelihood_herbs > 0 {
            if self.supply.shrine == 0 {
                return Goal::SeekWoundCare;
            }
            let can_sell = !self.is_night()
                && self.region().is_some_and(|region| {
                    region.market_coins >= self.market_quote().expect("regional rules").sell
                });
            return if can_sell {
                Goal::SellForCare
            } else {
                Goal::SeekWoundCare
            };
        }
        Goal::GatherForCare
    }

    pub(super) fn reconcile_character_agency_inventory(&mut self) {
        let Some(agency) = self.character_agency.as_mut() else {
            return;
        };
        for (&actor, state) in &mut agency.people {
            state.livelihood_herbs = state
                .livelihood_herbs
                .min(provenance_capacity(self.npcs[&actor].herbs));
        }
    }

    pub(super) fn commit_decision(&mut self, actor: ActorId, mut decision: Decision) {
        let Some(agency) = self.character_agency.as_ref() else {
            return;
        };
        let previous = self.npcs[&actor].activity.goal;
        if previous == decision.goal {
            return;
        }
        let suspended = agency.people[&actor].suspended;
        let interrupting = matches!(
            decision.reason,
            DecisionReason::ImmediateDanger
                | DecisionReason::PhysicalRecovery
                | DecisionReason::TreatmentFunds
        );
        let resuming = suspended == Some(decision.goal) && !interrupting;
        let state = self
            .character_agency
            .as_mut()
            .expect("checked agency")
            .people
            .get_mut(&actor)
            .expect("authored actor");
        if resuming {
            state.suspended = None;
            decision.reason = DecisionReason::ResumingAmbition;
        } else if interrupting && state.suspended.is_none() && resumable(previous) {
            state.suspended = Some(previous);
        }
        self.record_agency_event(
            actor,
            AgencyEventKind::GoalChanged,
            Some(decision.goal),
            Some(decision.reason),
            decision.score.into(),
            None,
        );
    }

    pub(super) fn work_for_care(&mut self, actor: ActorId, goal: Goal) {
        let physical_herbs = self.npcs[&actor].herbs;
        let state = self
            .character_agency
            .as_mut()
            .expect("agency rules")
            .people
            .get_mut(&actor)
            .expect("authored actor");
        state.livelihood_herbs = state
            .livelihood_herbs
            .min(provenance_capacity(physical_herbs));
        let sites = Region::sites();
        match goal {
            Goal::GatherForCare => {
                if !in_reach(self.npcs[&actor].position, sites.garden) {
                    self.travel(actor, sites.garden, 1);
                    return;
                }
                if self.region().is_none_or(|region| region.garden_herbs == 0) {
                    return;
                }
                self.region.as_mut().unwrap().garden_herbs -= 1;
                self.npcs.get_mut(&actor).unwrap().herbs += 1;
                self.character_agency
                    .as_mut()
                    .unwrap()
                    .people
                    .get_mut(&actor)
                    .unwrap()
                    .livelihood_herbs += 1;
                self.record_agency_event(
                    actor,
                    AgencyEventKind::WorkGathered,
                    Some(goal),
                    Some(DecisionReason::Livelihood),
                    1,
                    None,
                );
            }
            Goal::SellForCare => {
                if !in_reach(self.npcs[&actor].position, sites.market) {
                    self.travel(actor, sites.market, 1);
                    return;
                }
                if self.is_night()
                    || self.character_agency().unwrap().people[&actor].livelihood_herbs == 0
                {
                    return;
                }
                let price = self.market_quote().unwrap().sell;
                if self.region().unwrap().market_coins < price {
                    return;
                }
                let region = self.region.as_mut().unwrap();
                region.market_coins -= price;
                region.market_herbs += 1;
                self.npcs.get_mut(&actor).unwrap().herbs -= 1;
                let state = self
                    .character_agency
                    .as_mut()
                    .unwrap()
                    .people
                    .get_mut(&actor)
                    .unwrap();
                state.livelihood_herbs -= 1;
                state.coins += price;
                self.record_agency_event(
                    actor,
                    AgencyEventKind::WorkSold,
                    Some(goal),
                    Some(DecisionReason::Livelihood),
                    price,
                    None,
                );
            }
            _ => unreachable!("only livelihood goals reach this handler"),
        }
    }

    /// Revalidated at the clinic immediately before medicine is consumed.
    pub(super) fn pay_for_npc_care(&mut self, patient: ActorId, medic: ActorId) -> bool {
        let Some(agency) = self.character_agency.as_mut() else {
            return true;
        };
        let state = agency.people.get_mut(&patient).expect("authored patient");
        state.livelihood_herbs = state
            .livelihood_herbs
            .min(provenance_capacity(self.npcs[&patient].herbs));
        let state = *state;
        if self.supply.shrine > 0 && state.coins >= CLINIC_FEE {
            let agency = self.character_agency.as_mut().unwrap();
            agency.people.get_mut(&patient).unwrap().coins -= CLINIC_FEE;
            if patient == medic {
                agency.clinic_coins += CLINIC_FEE;
            } else {
                agency.people.get_mut(&medic).unwrap().coins += CLINIC_FEE;
            }
            self.record_agency_event(
                patient,
                AgencyEventKind::CarePaid,
                Some(Goal::SeekWoundCare),
                Some(DecisionReason::PhysicalRecovery),
                CLINIC_FEE,
                Some(medic),
            );
            return true;
        }
        if state.livelihood_herbs == 0 {
            return false;
        }
        self.npcs.get_mut(&patient).unwrap().herbs -= 1;
        self.character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&patient)
            .unwrap()
            .livelihood_herbs -= 1;
        // The bartered herb enters the clinic stock before the treatment below
        // consumes one unit. This preserves the physical medicine ledger.
        self.supply.shrine += 1;
        self.record_agency_event(
            patient,
            AgencyEventKind::CareBartered,
            Some(Goal::SeekWoundCare),
            Some(DecisionReason::PhysicalRecovery),
            1,
            Some(medic),
        );
        true
    }

    fn record_agency_event(
        &mut self,
        actor: ActorId,
        kind: AgencyEventKind,
        goal: Option<Goal>,
        reason: Option<DecisionReason>,
        amount: u32,
        other: Option<ActorId>,
    ) {
        let site = self.npcs[&actor].position;
        let witnessed = self.player.alive() && self.perceives(self.player.position, site);
        let agency = self.character_agency.as_mut().expect("agency rules");
        let event = AgencyEvent {
            id: agency.next_event,
            at: self.tick,
            actor,
            kind,
            goal,
            reason,
            amount,
            other,
            site,
            witnessed,
        };
        agency.next_event = agency.next_event.checked_add(1).expect("agency event id");
        if agency.history.len() == HISTORY_CAP {
            agency.history.remove(0);
        }
        agency.history.push(event);
    }
}

fn resumable(goal: Goal) -> bool {
    matches!(
        goal,
        Goal::MartialPractice(_)
            | Goal::MeridianDraw
            | Goal::MeridianCirculate
            | Goal::TeachAtHall
            | Goal::Patrol
    )
}

fn provenance_capacity(physical_herbs: u32) -> u8 {
    u8::try_from(physical_herbs).unwrap_or(u8::MAX)
}

pub(super) fn decision_for(goal: Goal, reason: DecisionReason, score: u16) -> Decision {
    Decision {
        goal,
        reason,
        score,
    }
}

pub(super) fn ambition_reason(npc: &Npc, goal: Goal) -> DecisionReason {
    match goal {
        Goal::MartialPractice(_) | Goal::MartialRest => DecisionReason::MartialAmbition,
        Goal::MeridianDraw | Goal::MeridianCirculate | Goal::MeridianCare => {
            DecisionReason::Cultivation
        }
        _ if matches!(npc.definition.role, Role::Mentor | Role::HallSteward) => {
            DecisionReason::InstitutionalObligation
        }
        _ => DecisionReason::Routine,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Direction, Intent, MartialSkill, Outcome, PLAYER, Wounds};

    const DEPUTY: ActorId = ActorId::new(34);
    const MENTOR: ActorId = ActorId::new(33);
    const MEDIC: ActorId = ActorId::new(1);

    fn turn(world: &mut World) -> Outcome {
        world.step(world.tick(), Intent::Wait).unwrap()
    }

    fn apply_revision_23_wound_trace(world: &mut World) {
        for direction in [Direction::North, Direction::East, Direction::East] {
            world.step(world.tick(), Intent::Move(direction)).unwrap();
        }
        world.step(world.tick(), Intent::Attack(MEDIC)).unwrap();
        for (direction, count) in [
            (Direction::North, 3),
            (Direction::East, 9),
            (Direction::South, 2),
            (Direction::North, 2),
            (Direction::West, 6),
            (Direction::South, 3),
            (Direction::West, 2),
        ] {
            for _ in 0..count {
                world.step(world.tick(), Intent::Move(direction)).unwrap();
            }
        }
        world.step(world.tick(), Intent::Attack(MEDIC)).unwrap();
        for _ in 0..120 {
            turn(world);
        }
    }

    fn coin_total(world: &World) -> u32 {
        world.region().unwrap().market_coins
            + world.region().unwrap().coins
            + world.politics().unwrap().hall_coins
            + world.rivalry().unwrap().lodge_coins
            + world.character_agency().unwrap().clinic_coins
            + world
                .character_agency()
                .unwrap()
                .people
                .values()
                .map(|state| state.coins)
                .sum::<u32>()
    }

    #[test]
    fn injured_character_works_pays_for_care_and_resumes_partial_training() {
        let mut world = World::autonomous_agency(0);
        world.supply.shrine = 2;
        world.npcs.get_mut(&MEDIC).unwrap().position = world.scenario.clinic;
        world.npcs.get_mut(&DEPUTY).unwrap().position = Region::sites().hall;
        world.npcs.get_mut(&MENTOR).unwrap().position = Region::sites().hall;
        world.martial.as_mut().unwrap().skills.insert(
            DEPUTY,
            MartialSkill {
                teacher: Some(MENTOR),
                lessons: 1,
                ..MartialSkill::default()
            },
        );
        world.npcs.get_mut(&DEPUTY).unwrap().activity.goal = Goal::MartialPractice(MENTOR);
        world
            .injuries
            .as_mut()
            .unwrap()
            .people
            .insert(DEPUTY, Wounds { arms: 1, legs: 0 });
        world.npcs.get_mut(&DEPUTY).unwrap().hp = 4;

        assert_eq!(world.choose_goal(&world.npcs[&DEPUTY]), Goal::GatherForCare);
        let initial_coins = coin_total(&world);
        let initial_consumed = world.supply.consumed;

        for _ in 0..220 {
            turn(&mut world);
            let skill = world.martial().unwrap().skills[&DEPUTY];
            if !world.injuries().unwrap().people[&DEPUTY].injured()
                && skill.lessons >= 2
                && world.character_agency().unwrap().people[&DEPUTY]
                    .suspended
                    .is_none()
            {
                break;
            }
        }

        let state = world.character_agency().unwrap().people[&DEPUTY];
        assert_eq!(state.livelihood_herbs, 0);
        assert_eq!(world.character_agency().unwrap().people[&MEDIC].coins, 1);
        assert_eq!(coin_total(&world), initial_coins);
        assert!(world.supply.consumed > initial_consumed);
        assert!(!world.injuries().unwrap().people[&DEPUTY].injured());
        assert_eq!(
            world.martial().unwrap().skills[&DEPUTY].teacher,
            Some(MENTOR)
        );
        assert!(world.martial().unwrap().skills[&DEPUTY].lessons >= 2);
        assert!(
            world
                .character_agency()
                .unwrap()
                .history
                .iter()
                .any(|event| {
                    event.actor == DEPUTY && event.kind == AgencyEventKind::WorkGathered
                })
        );
        assert!(
            world
                .character_agency()
                .unwrap()
                .history
                .iter()
                .any(|event| { event.actor == DEPUTY && event.kind == AgencyEventKind::WorkSold })
        );
        assert!(
            world
                .character_agency()
                .unwrap()
                .history
                .iter()
                .any(|event| {
                    event.actor == DEPUTY && event.reason == Some(DecisionReason::ResumingAmbition)
                })
        );
    }

    #[test]
    fn failed_care_never_charges_money_or_consumes_medicine() {
        let mut world = World::autonomous_agency(0);
        world.supply.shrine = 1;
        world.npcs.get_mut(&DEPUTY).unwrap().position = world.scenario.clinic;
        world
            .injuries
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap()
            .arms = 1;
        world
            .character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap()
            .coins = 1;
        world.npcs.get_mut(&MEDIC).unwrap().position = Position { x: 0, y: 0 };

        world.work_wound_care(DEPUTY);
        assert_eq!(world.character_agency().unwrap().people[&DEPUTY].coins, 1);
        assert_eq!(world.character_agency().unwrap().people[&MEDIC].coins, 0);
        assert_eq!(world.supply.shrine, 1);
        assert_eq!(world.supply.consumed, 0);

        world.npcs.get_mut(&MEDIC).unwrap().position = world.scenario.clinic;
        world.supply.shrine = 0;
        world.work_wound_care(DEPUTY);
        assert_eq!(world.character_agency().unwrap().people[&DEPUTY].coins, 1);
        assert_eq!(world.character_agency().unwrap().people[&MEDIC].coins, 0);
        assert_eq!(world.supply.consumed, 0);

        world.supply.shrine = 1;
        let state = world
            .character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap();
        state.coins = 0;
        state.livelihood_herbs = 1;
        world.npcs.get_mut(&DEPUTY).unwrap().herbs = 0;
        world.work_wound_care(DEPUTY);
        assert_eq!(
            world.character_agency().unwrap().people[&DEPUTY].livelihood_herbs,
            0
        );
        assert_eq!(world.supply.shrine, 1);
        assert_eq!(world.supply.consumed, 0);
    }

    #[test]
    fn injured_medic_uses_the_clinic_area_and_pays_its_till_for_self_care() {
        let mut world = World::autonomous_agency(0);
        world.supply.shrine = 1;
        world.npcs.get_mut(&MEDIC).unwrap().position = Position { x: 3, y: 4 };
        world
            .injuries
            .as_mut()
            .unwrap()
            .people
            .get_mut(&MEDIC)
            .unwrap()
            .arms = 1;
        world
            .character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&MEDIC)
            .unwrap()
            .coins = 1;

        world.work_wound_care(MEDIC);
        assert!(in_reach(world.npcs[&MEDIC].position, world.scenario.clinic));
        assert!(!world.injuries().unwrap().people[&MEDIC].injured());
        assert_eq!(world.character_agency().unwrap().people[&MEDIC].coins, 0);
        assert_eq!(world.character_agency().unwrap().clinic_coins, 1);
        assert_eq!(world.supply.shrine, 0);
        assert_eq!(world.supply.consumed, 1);
    }

    #[test]
    fn livelihood_herb_bootstraps_care_when_clinic_stock_is_empty() {
        let mut world = World::autonomous_agency(0);
        world.supply.shrine = 0;
        world.npcs.get_mut(&MEDIC).unwrap().position = world.scenario.clinic;
        world.npcs.get_mut(&DEPUTY).unwrap().position = Position { x: 3, y: 4 };
        world
            .injuries
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap()
            .arms = 1;

        assert_eq!(world.wound_goal(DEPUTY), Some(Goal::GatherForCare));
        world.npcs.get_mut(&DEPUTY).unwrap().herbs = 1;
        world
            .character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap()
            .livelihood_herbs = 1;
        assert_eq!(world.wound_goal(DEPUTY), Some(Goal::SeekWoundCare));

        world.work_wound_care(DEPUTY);
        assert!(!world.injuries().unwrap().people[&DEPUTY].injured());
        assert_eq!(world.npcs[&DEPUTY].herbs, 0);
        assert_eq!(
            world.character_agency().unwrap().people[&DEPUTY].livelihood_herbs,
            0
        );
        assert_eq!(world.supply.shrine, 0);
        assert_eq!(world.supply.consumed, 1);
        assert!(
            world
                .character_agency()
                .unwrap()
                .history
                .iter()
                .any(|event| {
                    event.actor == DEPUTY && event.kind == AgencyEventKind::CareBartered
                })
        );
    }

    #[test]
    fn committed_boundary_reconciles_external_herb_custody_changes() {
        let mut world = World::autonomous_agency(0);
        world.npcs.get_mut(&DEPUTY).unwrap().herbs = 0;
        world
            .character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap()
            .livelihood_herbs = 1;

        turn(&mut world);
        assert_eq!(
            world.character_agency().unwrap().people[&DEPUTY].livelihood_herbs,
            0
        );
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
    }

    #[test]
    fn one_medic_commits_at_most_one_paid_wound_treatment_per_turn() {
        let second_patient = ActorId::new(2);
        let mut world = World::autonomous_agency(0);
        world.supply.shrine = 2;
        world.npcs.get_mut(&MEDIC).unwrap().position = world.scenario.clinic;
        world.npcs.get_mut(&second_patient).unwrap().position = Position { x: 3, y: 4 };
        world.npcs.get_mut(&DEPUTY).unwrap().position = Position { x: 2, y: 3 };
        world
            .injuries
            .as_mut()
            .unwrap()
            .people
            .get_mut(&second_patient)
            .unwrap()
            .arms = 1;
        world
            .injuries
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap()
            .arms = 1;
        world
            .character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&second_patient)
            .unwrap()
            .coins = 1;
        world
            .character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap()
            .coins = 1;

        world.traffic.as_mut().unwrap().begin_tick();
        world.work_wound_care(second_patient);
        world.work_wound_care(DEPUTY);
        assert!(!world.injuries().unwrap().people[&second_patient].injured());
        assert!(world.injuries().unwrap().people[&DEPUTY].injured());
        assert_eq!(
            world.character_agency().unwrap().people[&second_patient].coins,
            0
        );
        assert_eq!(world.character_agency().unwrap().people[&DEPUTY].coins, 1);
        assert_eq!(world.character_agency().unwrap().people[&MEDIC].coins, 1);
        assert_eq!(world.supply.consumed, 1);

        world.traffic.as_mut().unwrap().begin_tick();
        world.work_wound_care(DEPUTY);
        assert!(!world.injuries().unwrap().people[&DEPUTY].injured());
        assert_eq!(world.character_agency().unwrap().people[&MEDIC].coins, 2);
        assert_eq!(world.supply.consumed, 2);
    }

    #[test]
    fn revision_24_inherits_physical_ward_protection() {
        for mut world in [World::persistent_injury(0), World::autonomous_agency(0)] {
            world.contested.as_mut().unwrap().wards = 1;
            let hp = world.player.hp;
            world.damage(MEDIC, PLAYER, 2, false);
            assert_eq!(world.player.hp, hp - 1);
            assert_eq!(world.contested().unwrap().wards, 0);
        }
    }

    #[test]
    fn ambition_and_suspended_pursuit_raise_valid_goal_priority() {
        let mut world = World::autonomous_agency(0);
        let decision = world.choose_decision(&world.npcs[&DEPUTY]);
        assert!(matches!(decision.goal, Goal::MartialPractice(_)));
        assert_eq!(decision.score, 68);

        world
            .character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap()
            .ambition = Ambition::Prosperity;
        let decision = world.choose_decision(&world.npcs[&DEPUTY]);
        assert_eq!(decision.score, 64);
        world
            .character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&DEPUTY)
            .unwrap()
            .suspended = Some(decision.goal);
        assert_eq!(world.choose_decision(&world.npcs[&DEPUTY]).score, 67);
    }

    #[test]
    fn agency_history_is_bounded_with_monotonic_ids_and_enters_replay_digest() {
        let mut world = World::autonomous_agency(0);
        for index in 0..(HISTORY_CAP + 24) {
            let goal = if index % 2 == 0 {
                Goal::Patrol
            } else {
                Goal::Rest
            };
            world.npcs.get_mut(&DEPUTY).unwrap().activity.goal = if goal == Goal::Patrol {
                Goal::Rest
            } else {
                Goal::Patrol
            };
            world.commit_decision(DEPUTY, decision_for(goal, DecisionReason::Routine, 20));
        }
        let agency = world.character_agency().unwrap();
        assert_eq!(agency.history.len(), HISTORY_CAP);
        assert_eq!(agency.next_event, (HISTORY_CAP + 24) as u32);
        assert_eq!(agency.history.first().unwrap().id, 24);
        assert!(
            agency
                .history
                .windows(2)
                .all(|events| events[0].id + 1 == events[1].id)
        );

        let mut replayed = World::autonomous_agency(0);
        for _ in 0..40 {
            turn(&mut replayed);
        }
        assert_eq!(World::restore(&replayed.checkpoint()).unwrap(), replayed);
    }

    #[test]
    fn baseline_revision_21_22_and_23_checkpoints_remain_byte_exact() {
        type Constructor = fn(u32) -> World;
        let cases: [(&[u8], Constructor); 3] = [
            (
                include_bytes!("../../tests/fixtures/living/revision21.bin"),
                World::martial_mentorship,
            ),
            (
                include_bytes!("../../tests/fixtures/living/revision22.bin"),
                World::meridian_cultivation,
            ),
            (
                include_bytes!("../../tests/fixtures/living/revision23.bin"),
                World::persistent_injury,
            ),
        ];
        for (golden, constructor) in cases {
            let mut restored = World::restore(golden).unwrap();
            assert_eq!(restored.checkpoint(), golden);

            let mut rebuilt = constructor(7);
            for _ in 0..64 {
                turn(&mut rebuilt);
            }
            assert_eq!(rebuilt.checkpoint(), golden);

            for _ in 0..40 {
                assert_eq!(turn(&mut restored), turn(&mut rebuilt));
                assert_eq!(restored.checkpoint(), rebuilt.checkpoint());
            }
        }
    }

    #[test]
    fn baseline_revision_23_wounded_medic_behavior_remains_byte_exact() {
        let golden = include_bytes!("../../tests/fixtures/living/revision23_wounded.bin");
        let mut restored = World::restore(golden).unwrap();
        assert_eq!(restored.checkpoint(), golden);

        let mut rebuilt = World::persistent_injury(0);
        apply_revision_23_wound_trace(&mut rebuilt);
        assert_eq!(rebuilt.checkpoint(), golden);

        for _ in 0..40 {
            assert_eq!(turn(&mut restored), turn(&mut rebuilt));
            assert_eq!(restored.checkpoint(), rebuilt.checkpoint());
        }
    }

    #[test]
    fn multi_seed_thousand_turn_campaigns_are_deterministic_and_replayable() {
        for seed in [0, 1, 2, 7] {
            let mut left = World::autonomous_agency(seed);
            let mut right = World::autonomous_agency(seed);
            for turn_index in 1..=1_000 {
                assert_eq!(turn(&mut left), turn(&mut right));
                if matches!(turn_index, 200 | 500 | 1_000) {
                    assert_eq!(left.checkpoint(), right.checkpoint());
                    left = World::restore(&left.checkpoint()).unwrap();
                    assert_eq!(left, right);
                    for (&actor, state) in &left.character_agency().unwrap().people {
                        assert!(u32::from(state.livelihood_herbs) <= left.npcs[&actor].herbs);
                    }
                }
            }
            assert!(left.character_agency().unwrap().history.len() <= HISTORY_CAP);
        }
    }
}
