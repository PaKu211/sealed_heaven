//! Event-sourced checkpoint: scenario fingerprint, one fixed
//! record per committed turn, complete state digest, and an envelope checksum.
//! Replay reconstructs the NPC activity, relationships, receipts, queued reports,
//! effects, combat and stock state through the same canonical entry point.

use crate::encoding::sha256;

use super::{
    ActorId, AgencyEventKind, Allegiance, Ambition, AuditFinding, Background, Company,
    ContractEventKind, ContractMemory, ContractPhase, Contracts, DecisionReason, Direction,
    DisputePhase, Duty, FactionAgency, FactionEventKind, FactionPhase, Form, Goal, Good, HallRank,
    HierarchyEventKind, Holder, InfoLogKind, Information, Intent, Invitation, JourneyEvent,
    KnowledgeSource, LotEventKind, MAX_TICKS, MeridianEventKind, Need, Outcome, PoliticalEventKind,
    Politics, Position, Provenance, Region, Rejection, Relief, ReliefEventKind, ReliefPhase,
    Rivalry, Role, Rules, Scenario, School, SectDuty, SectEventKind, SectMember, Traffic,
    TrafficEventKind, World, WoundKind,
};

const MAGIC: &[u8; 12] = b"SH:C10:WORLD";
const HEADER: usize = MAGIC.len() + 2 + 32 + 4;
const INTENT_SIZE: usize = 5;
const FOOTER: usize = 64;
pub const MAX_SAVE_BYTES: usize = HEADER + 4 + MAX_TICKS as usize * INTENT_SIZE + FOOTER;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadError {
    LegacyCheckpoint,
    UnsupportedFormat,
    UnsupportedRevision(u16),
    ScenarioMismatch,
    InvalidEnvelope,
    ChecksumMismatch,
    InvalidIntent,
    StateMismatch,
}

pub(super) fn revision(rules: Rules) -> u16 {
    match rules {
        Rules::Original => 1,
        Rules::Qinghe => 2,
        Rules::Riverlands => 3,
        Rules::ReedHall => 4,
        Rules::RoadCompany => 5,
        Rules::SectRivalry => 6,
        Rules::SectPolitics => 7,
        Rules::TrafficCourtesy => 8,
        Rules::LivingAgency => 9,
        Rules::FactionAgency => 10,
        Rules::InstitutionalAgency => 11,
        Rules::ContractAccountability => 12,
        Rules::InformationDisputes => 13,
        Rules::PlayerAgency => 14,
        Rules::Consequences => 15,
        Rules::ViolenceLogistics => 16,
        Rules::PlayerReporting => 17,
        Rules::PhysicalRecovery => 18,
        Rules::ContestedResources => 19,
        Rules::SectCultivation => 20,
        Rules::MartialMentorship => 21,
        Rules::MeridianCultivation => 22,
        Rules::PersistentInjury => 23,
        Rules::AutonomousAgency => 24,
        Rules::SectHierarchy => 25,
        Rules::DutyCoherence => 26,
        Rules::GeneratedRiverlands => super::worldgen::GENERATED_REVISION,
        Rules::ExpandedRiverlands => super::expansion::REVISION,
        Rules::RegionalKnowledge => super::regional_news::REVISION,
        Rules::ExactLod => super::lod::REVISION,
        Rules::SettlementProduction => super::production::REVISION,
        Rules::RoadHistory => super::road_history::REVISION,
        Rules::EmergentRequests => super::requests::REVISION,
        Rules::PersonalFavors => super::favors::REVISION,
        Rules::ReciprocalBonds => super::reciprocity::REVISION,
        Rules::EncounterMemory => super::encounters::REVISION,
        Rules::SocialNetworks => super::social_network::REVISION,
        Rules::CollectiveAgency => super::cooperation::REVISION,
        Rules::DynamicObligations => super::obligations::REVISION,
        Rules::SituatedCommunication => super::communication::REVISION,
        Rules::InformationNetworks => super::rumors::REVISION,
        Rules::CausalNarratives => super::narrative::REVISION,
        Rules::PersonalDevelopment => super::development::REVISION,
        Rules::VoluntaryCircles => super::circles::REVISION,
        Rules::SituatedDiplomacy => super::diplomacy::REVISION,
    }
}

pub(super) fn encode(world: &World) -> Vec<u8> {
    let mut out = Wire(MAGIC.to_vec());
    out.u16(revision(world.rules));
    out.0.extend(bound_scenario_digest(world));
    out.count(world.journal.len());
    if let Some(region) = world.region() {
        out.u32(region.seed);
    }
    for &intent in &world.journal {
        out.intent(intent);
    }
    out.0.extend(state_digest(world));
    out.0.extend(sha256(&out.0));
    out.0
}

pub(super) fn decode(bytes: &[u8], scenario: &'static Scenario) -> Result<World, LoadError> {
    if bytes.starts_with(b"SH:C05:STATE") || bytes.starts_with(b"SH:C03:STATE") {
        return Err(LoadError::LegacyCheckpoint);
    }
    if !bytes.starts_with(MAGIC) {
        return Err(LoadError::UnsupportedFormat);
    }
    if !(HEADER + FOOTER..=MAX_SAVE_BYTES).contains(&bytes.len()) {
        return Err(LoadError::InvalidEnvelope);
    }
    let rules = match u16::from_le_bytes(bytes[12..14].try_into().unwrap()) {
        1 => Rules::Original,
        2 => Rules::Qinghe,
        3 => Rules::Riverlands,
        4 => Rules::ReedHall,
        5 => Rules::RoadCompany,
        6 => Rules::SectRivalry,
        7 => Rules::SectPolitics,
        8 => Rules::TrafficCourtesy,
        9 => Rules::LivingAgency,
        10 => Rules::FactionAgency,
        11 => Rules::InstitutionalAgency,
        12 => Rules::ContractAccountability,
        13 => Rules::InformationDisputes,
        14 => Rules::PlayerAgency,
        15 => Rules::Consequences,
        16 => Rules::ViolenceLogistics,
        17 => Rules::PlayerReporting,
        18 => Rules::PhysicalRecovery,
        19 => Rules::ContestedResources,
        20 => Rules::SectCultivation,
        21 => Rules::MartialMentorship,
        22 => Rules::MeridianCultivation,
        23 => Rules::PersistentInjury,
        24 => Rules::AutonomousAgency,
        25 => Rules::SectHierarchy,
        26 => Rules::DutyCoherence,
        super::worldgen::GENERATED_REVISION => Rules::GeneratedRiverlands,
        super::expansion::REVISION => Rules::ExpandedRiverlands,
        super::regional_news::REVISION => Rules::RegionalKnowledge,
        super::lod::REVISION => Rules::ExactLod,
        super::production::REVISION => Rules::SettlementProduction,
        super::road_history::REVISION => Rules::RoadHistory,
        super::requests::REVISION => Rules::EmergentRequests,
        super::favors::REVISION => Rules::PersonalFavors,
        super::reciprocity::REVISION => Rules::ReciprocalBonds,
        super::encounters::REVISION => Rules::EncounterMemory,
        super::social_network::REVISION => Rules::SocialNetworks,
        super::cooperation::REVISION => Rules::CollectiveAgency,
        super::obligations::REVISION => Rules::DynamicObligations,
        super::communication::REVISION => Rules::SituatedCommunication,
        super::rumors::REVISION => Rules::InformationNetworks,
        super::narrative::REVISION => Rules::CausalNarratives,
        super::development::REVISION => Rules::PersonalDevelopment,
        super::circles::REVISION => Rules::VoluntaryCircles,
        super::diplomacy::REVISION => Rules::SituatedDiplomacy,
        revision => return Err(LoadError::UnsupportedRevision(revision)),
    };
    let turns = u32::from_le_bytes(bytes[46..50].try_into().unwrap()) as usize;
    let header = HEADER + if rules.has_region() { 4 } else { 0 };
    if turns as u64 > MAX_TICKS || header + turns * INTENT_SIZE + FOOTER != bytes.len() {
        return Err(LoadError::InvalidEnvelope);
    }
    let checksum_at = bytes.len() - 32;
    if sha256(&bytes[..checksum_at]) != bytes[checksum_at..] {
        return Err(LoadError::ChecksumMismatch);
    }
    let seed = rules
        .has_region()
        .then(|| u32::from_le_bytes(bytes[HEADER..header].try_into().unwrap()));
    // Construct a disposable candidate only after validating the envelope.
    // For generated worlds this binds the seed-derived geography before replay.
    let mut world = match seed {
        Some(seed) => World::seeded(seed, rules),
        None => World::with_rules(scenario, rules),
    };
    if bound_scenario_digest(&world) != bytes[14..46] {
        return Err(LoadError::ScenarioMismatch);
    }

    let state_at = checksum_at - 32;
    let journal: Vec<_> = bytes[header..state_at]
        .as_chunks::<INTENT_SIZE>()
        .0
        .iter()
        .map(|record| read_intent(record, rules))
        .collect::<Result<_, _>>()?;
    for intent in journal {
        world
            .step(world.tick(), intent)
            .map_err(|_| LoadError::StateMismatch)?;
    }
    if state_digest(&world) != bytes[state_at..checksum_at] {
        return Err(LoadError::StateMismatch);
    }
    Ok(world)
}

pub(super) fn intent_is_recordable(intent: Intent, rules: Rules) -> bool {
    read_intent(&Wire::intent_record(intent), rules).is_ok_and(|decoded| decoded == intent)
}

fn read_intent(record: &[u8], rules: Rules) -> Result<Intent, LoadError> {
    let value = u32::from_le_bytes(record[1..].try_into().unwrap());
    let actor = ActorId::new(value);
    Ok(match (record[0], value) {
        (0, 0) => Intent::Wait,
        (1, 0) => Intent::Move(Direction::North),
        (1, 1) => Intent::Move(Direction::East),
        (1, 2) => Intent::Move(Direction::South),
        (1, 3) => Intent::Move(Direction::West),
        (2, _) => Intent::Talk(actor),
        (3, 0) => Intent::Gather,
        (4, 0) => Intent::Deliver,
        (5, _) => Intent::Attack(actor),
        (6, 0) => Intent::Guard,
        (7, 0) if rules != Rules::Original => Intent::Practice,
        (8, 0) if rules.has_region() => Intent::ChooseBackground(Background::Wanderer),
        (8, 1) if rules.has_region() => Intent::ChooseBackground(Background::Herbalist),
        (8, 2) if rules.has_region() => Intent::ChooseBackground(Background::Peddler),
        (9, 0) if rules.has_region() => Intent::BuyHerb,
        (10, 0) if rules.has_region() => Intent::SellHerb,
        (11, 0) if rules.has_region() => Intent::JoinHall,
        (12, 0) if rules.has_region() => Intent::Brew,
        (13, 0) if rules.has_region() => Intent::UseRemedy,
        (14, 0) if rules.has_school() => Intent::HallDuty,
        (15, 0) if rules.has_school() => Intent::StudyForm,
        (16, 0) if rules.has_school() => Intent::CenterQi,
        (17, _) if rules.has_school() => Intent::ReedStrike(actor),
        (18, _)
            if matches!(
                rules,
                Rules::RoadCompany
                    | Rules::SectRivalry
                    | Rules::SectPolitics
                    | Rules::TrafficCourtesy
                    | Rules::LivingAgency
                    | Rules::FactionAgency
                    | Rules::InstitutionalAgency
                    | Rules::ContractAccountability
                    | Rules::InformationDisputes
                    | Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::Invite(actor)
        }
        (19, _)
            if matches!(
                rules,
                Rules::RoadCompany
                    | Rules::SectRivalry
                    | Rules::SectPolitics
                    | Rules::TrafficCourtesy
                    | Rules::LivingAgency
                    | Rules::FactionAgency
                    | Rules::InstitutionalAgency
                    | Rules::ContractAccountability
                    | Rules::InformationDisputes
                    | Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::Provision(actor)
        }
        (20, _)
            if matches!(
                rules,
                Rules::RoadCompany
                    | Rules::SectRivalry
                    | Rules::SectPolitics
                    | Rules::TrafficCourtesy
                    | Rules::LivingAgency
                    | Rules::FactionAgency
                    | Rules::InstitutionalAgency
                    | Rules::ContractAccountability
                    | Rules::InformationDisputes
                    | Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::Part(actor)
        }
        (21, 0)
            if matches!(
                rules,
                Rules::SectRivalry
                    | Rules::SectPolitics
                    | Rules::TrafficCourtesy
                    | Rules::LivingAgency
                    | Rules::FactionAgency
                    | Rules::InstitutionalAgency
                    | Rules::ContractAccountability
                    | Rules::InformationDisputes
                    | Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::OfferAccord
        }
        (22, 0)
            if matches!(
                rules,
                Rules::SectRivalry
                    | Rules::SectPolitics
                    | Rules::TrafficCourtesy
                    | Rules::LivingAgency
                    | Rules::FactionAgency
                    | Rules::InstitutionalAgency
                    | Rules::ContractAccountability
                    | Rules::InformationDisputes
                    | Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::SupportHall
        }
        (23, 0)
            if matches!(
                rules,
                Rules::SectPolitics
                    | Rules::TrafficCourtesy
                    | Rules::LivingAgency
                    | Rules::FactionAgency
                    | Rules::InstitutionalAgency
                    | Rules::ContractAccountability
                    | Rules::InformationDisputes
                    | Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::PledgeLodge
        }
        (24, 0)
            if matches!(
                rules,
                Rules::SectPolitics
                    | Rules::TrafficCourtesy
                    | Rules::LivingAgency
                    | Rules::FactionAgency
                    | Rules::InstitutionalAgency
                    | Rules::ContractAccountability
                    | Rules::InformationDisputes
                    | Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::AidLodge
        }
        (25, 0)
            if matches!(
                rules,
                Rules::SectPolitics
                    | Rules::TrafficCourtesy
                    | Rules::LivingAgency
                    | Rules::FactionAgency
                    | Rules::InstitutionalAgency
                    | Rules::ContractAccountability
                    | Rules::InformationDisputes
                    | Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::RenounceLodge
        }
        (26, _)
            if matches!(
                rules,
                Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::Ask(actor)
        }
        (27, 0)
            if matches!(
                rules,
                Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::Examine
        }
        (28, _)
            if matches!(
                rules,
                Rules::PlayerAgency
                    | Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            let (to, event) = unpack_testimony(value);
            Intent::Testify { to, event }
        }
        (29, _)
            if matches!(
                rules,
                Rules::Consequences
                    | Rules::ViolenceLogistics
                    | Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::Take(actor)
        }
        (30, _)
            if matches!(
                rules,
                Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            let (to, body) = unpack_testimony(value);
            Intent::ReportBody {
                to,
                body: ActorId::new(body.0),
            }
        }
        (31, _)
            if matches!(
                rules,
                Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::ClaimLot(value)
        }
        (32, _)
            if matches!(
                rules,
                Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            let (to, id) = unpack_testimony(value);
            Intent::OfferLot { to, id: id.0 }
        }
        (33, _)
            if matches!(
                rules,
                Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::AttuneLot(value)
        }
        (34, _)
            if matches!(
                rules,
                Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::ChannelStrike(actor)
        }
        (35, _)
            if matches!(
                rules,
                Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::Drill(actor)
        }
        (36, _)
            if matches!(
                rules,
                Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::TechniqueStrike(actor)
        }
        (37, 0) if rules == Rules::MeridianCultivation => Intent::DrawQi,
        (38, 0) if rules == Rules::MeridianCultivation => Intent::CirculateQi,
        (39, 0) if rules == Rules::MeridianCultivation => Intent::MendMeridians,
        (37, 0) if rules == Rules::PersistentInjury => Intent::DrawQi,
        (38, 0) if rules == Rules::PersistentInjury => Intent::CirculateQi,
        (39, 0) if rules == Rules::PersistentInjury => Intent::MendMeridians,
        (40, 0) if rules == Rules::PersistentInjury => Intent::TreatWounds,
        (37, 0) if rules == Rules::AutonomousAgency => Intent::DrawQi,
        (38, 0) if rules == Rules::AutonomousAgency => Intent::CirculateQi,
        (39, 0) if rules == Rules::AutonomousAgency => Intent::MendMeridians,
        (40, 0) if rules == Rules::AutonomousAgency => Intent::TreatWounds,
        (37, 0)
            if matches!(
                rules,
                Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::DrawQi
        }
        (38, 0)
            if matches!(
                rules,
                Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::CirculateQi
        }
        (39, 0)
            if matches!(
                rules,
                Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::MendMeridians
        }
        (40, 0)
            if matches!(
                rules,
                Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::TreatWounds
        }
        (41, 0)
            if matches!(
                rules,
                Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::ClaimHallAllocation
        }
        (42, id)
            if matches!(
                rules,
                Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::RecoverCache(id)
        }
        (43, id)
            if matches!(
                rules,
                Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::EntrustCache(id)
        }
        (44, id)
            if matches!(
                rules,
                Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::AcceptRequest(id)
        }
        (45, id)
            if matches!(
                rules,
                Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::DeclineRequest(id)
        }
        (46, id)
            if matches!(
                rules,
                Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::AbandonRequest(id)
        }
        (47, id)
            if matches!(
                rules,
                Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::AcceptFavor(id)
        }
        (48, id)
            if matches!(
                rules,
                Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::DeclineFavor(id)
        }
        (49, id)
            if matches!(
                rules,
                Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::WithdrawFavor(id)
        }
        (50, id)
            if matches!(
                rules,
                Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::FundFavor(id)
        }
        (51, id)
            if matches!(
                rules,
                Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::RequestAid(ActorId::new(id))
        }
        (52, id)
            if matches!(
                rules,
                Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::RepayAid(ActorId::new(id))
        }
        (53, id)
            if matches!(
                rules,
                Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) =>
        {
            Intent::WaiveAid(ActorId::new(id))
        }
        _ => return Err(LoadError::InvalidIntent),
    })
}

fn bound_scenario_digest(world: &World) -> [u8; 32] {
    let authored = scenario_digest(world.scenario, world.rules);
    if let Some(map) = world.expanded_region() {
        let mut bytes = if world.rules == Rules::SituatedDiplomacy {
            b"SH:C58:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::VoluntaryCircles {
            b"SH:C57:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::PersonalDevelopment {
            b"SH:C56:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::CausalNarratives {
            b"SH:C55:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::InformationNetworks {
            b"SH:C54:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::SituatedCommunication {
            b"SH:C53:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::DynamicObligations {
            b"SH:C52:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::CollectiveAgency {
            b"SH:C51:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::SocialNetworks {
            b"SH:C50:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::EncounterMemory {
            b"SH:C49:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::ReciprocalBonds {
            b"SH:C48:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::PersonalFavors {
            b"SH:C47:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::EmergentRequests {
            b"SH:C46:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::RoadHistory {
            b"SH:C45:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::SettlementProduction {
            b"SH:C44:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::ExactLod {
            b"SH:C43:SCENARIO:v1\0".to_vec()
        } else if world.rules == Rules::RegionalKnowledge {
            b"SH:C42:SCENARIO:v1\0".to_vec()
        } else {
            b"SH:C41:SCENARIO:v1\0".to_vec()
        };
        bytes.extend_from_slice(&authored);
        bytes.extend_from_slice(&super::expansion::GENERATOR_VERSION.to_le_bytes());
        bytes.extend_from_slice(super::content::CONTENT_SHA256.as_bytes());
        bytes.extend_from_slice(super::expansion::CONTENT_SHA256.as_bytes());
        bytes.extend_from_slice(&map.digest());
        if let Some(history) = world.road_history() {
            bytes.extend_from_slice(&history.initial_digest());
        }
        return sha256(&bytes);
    }
    let Some(map) = world.generated_region() else {
        return authored;
    };
    let mut bytes = b"SH:C40:SCENARIO:v1\0".to_vec();
    bytes.extend_from_slice(&authored);
    bytes.extend_from_slice(&super::worldgen::GENERATOR_VERSION.to_le_bytes());
    bytes.extend_from_slice(super::content::CONTENT_SHA256.as_bytes());
    bytes.extend_from_slice(&map.digests().world);
    sha256(&bytes)
}

fn scenario_digest(scenario: &Scenario, rules: Rules) -> [u8; 32] {
    // Bump the rules identifier when changing gameplay semantics. Static map
    // and NPC definition edits are fingerprinted automatically.
    let mut out = Wire(match rules {
        Rules::Original => b"SH:C10:RULES:v1".to_vec(),
        Rules::Qinghe => b"SH:C10:RULES:v2".to_vec(),
        Rules::Riverlands => b"SH:C10:RULES:v3".to_vec(),
        Rules::ReedHall => b"SH:C10:RULES:v4".to_vec(),
        Rules::RoadCompany => b"SH:C10:RULES:v5".to_vec(),
        Rules::SectRivalry => b"SH:C10:RULES:v6".to_vec(),
        Rules::SectPolitics => b"SH:C10:RULES:v7".to_vec(),
        Rules::TrafficCourtesy => b"SH:C10:RULES:v8".to_vec(),
        Rules::LivingAgency => b"SH:C10:RULES:v9".to_vec(),
        Rules::FactionAgency => b"SH:C10:RULES:v10".to_vec(),
        Rules::InstitutionalAgency => b"SH:C10:RULES:v11".to_vec(),
        Rules::ContractAccountability => b"SH:C10:RULES:v12".to_vec(),
        Rules::InformationDisputes => b"SH:C10:RULES:v13".to_vec(),
        Rules::PlayerAgency => b"SH:C10:RULES:v14".to_vec(),
        Rules::Consequences => b"SH:C10:RULES:v15".to_vec(),
        Rules::ViolenceLogistics => b"SH:C10:RULES:v16".to_vec(),
        Rules::PlayerReporting => b"SH:C10:RULES:v17".to_vec(),
        Rules::PhysicalRecovery => b"SH:C10:RULES:v18".to_vec(),
        Rules::ContestedResources => b"SH:C10:RULES:v19".to_vec(),
        Rules::SectCultivation => b"SH:C10:RULES:v20".to_vec(),
        Rules::MartialMentorship => b"SH:C10:RULES:v21".to_vec(),
        Rules::MeridianCultivation => b"SH:C10:RULES:v22".to_vec(),
        Rules::PersistentInjury => b"SH:C10:RULES:v23".to_vec(),
        Rules::AutonomousAgency => b"SH:C10:RULES:v24".to_vec(),
        Rules::SectHierarchy => b"SH:C10:RULES:v25".to_vec(),
        Rules::DutyCoherence => b"SH:C10:RULES:v26".to_vec(),
        Rules::GeneratedRiverlands => b"SH:C10:RULES:v27".to_vec(),
        Rules::ExpandedRiverlands => b"SH:C10:RULES:v28".to_vec(),
        Rules::RegionalKnowledge => b"SH:C10:RULES:v29".to_vec(),
        Rules::ExactLod => b"SH:C10:RULES:v30".to_vec(),
        Rules::SettlementProduction => b"SH:C10:RULES:v31".to_vec(),
        Rules::RoadHistory => b"SH:C10:RULES:v32".to_vec(),
        Rules::EmergentRequests => b"SH:C10:RULES:v33".to_vec(),
        Rules::PersonalFavors => b"SH:C10:RULES:v34".to_vec(),
        Rules::ReciprocalBonds => b"SH:C10:RULES:v35".to_vec(),
        Rules::EncounterMemory => b"SH:C10:RULES:v36".to_vec(),
        Rules::SocialNetworks => b"SH:C10:RULES:v37".to_vec(),
        Rules::CollectiveAgency => b"SH:C10:RULES:v38".to_vec(),
        Rules::DynamicObligations => b"SH:C10:RULES:v39".to_vec(),
        Rules::SituatedCommunication => b"SH:C10:RULES:v40".to_vec(),
        Rules::InformationNetworks => b"SH:C10:RULES:v41".to_vec(),
        Rules::CausalNarratives => b"SH:C10:RULES:v42".to_vec(),
        Rules::PersonalDevelopment => b"SH:C10:RULES:v43".to_vec(),
        Rules::VoluntaryCircles => b"SH:C10:RULES:v44".to_vec(),
        Rules::SituatedDiplomacy => b"SH:C10:RULES:v45".to_vec(),
    });
    out.u64(MAX_TICKS);
    out.string(scenario.name);
    out.u32(scenario.width);
    out.u32(scenario.height);
    for row in scenario.terrain {
        out.string(row);
    }
    for cell in [
        scenario.entrance,
        scenario.patch,
        scenario.shrine,
        scenario.clinic,
    ] {
        out.position(cell);
    }
    if rules.has_region() {
        for site in Region::sites().positions() {
            out.position(site);
        }
    }
    out.u32(scenario.shrine_target);
    out.u32(scenario.initial_source);
    out.u64(scenario.regrow_every);
    out.count(scenario.npcs.len());
    for npc in scenario.npcs {
        out.actor(npc.id);
        out.string(npc.name);
        out.u32(u32::from(npc.glyph));
        out.u8(match npc.role {
            Role::Medic => 0,
            Role::Watch => 1,
            Role::Supplier => 2,
            Role::Traveler => 3,
            Role::Grower => 4,
            Role::Courier => 5,
            Role::Mentor => 6,
            Role::HallDeputy => 7,
            Role::RivalEnvoy => 8,
            Role::HallSteward => 9,
            Role::FrontierCarrier => 10,
            Role::FrontierGrower => 11,
        });
        out.position(npc.spawn);
        out.position(npc.home);
        out.count(npc.route.len());
        for &cell in npc.route {
            out.position(cell);
        }
        out.u16(npc.hp);
        out.u8(npc.rest_threshold);
        out.u8(u8::from(npc.responds_to_attack));
        out.u8(u8::from(npc.shares_news));
        out.u8(u8::from(npc.report_to.is_some()));
        if let Some(peer) = npc.report_to {
            out.actor(peer);
        }
    }
    out.count(scenario.contacts.len());
    for &(a, b) in scenario.contacts {
        out.actor(a);
        out.actor(b);
    }
    if matches!(
        rules,
        Rules::RoadCompany
            | Rules::SectRivalry
            | Rules::SectPolitics
            | Rules::TrafficCourtesy
            | Rules::LivingAgency
            | Rules::FactionAgency
            | Rules::InstitutionalAgency
            | Rules::ContractAccountability
            | Rules::InformationDisputes
            | Rules::PlayerAgency
            | Rules::Consequences
            | Rules::ViolenceLogistics
            | Rules::PlayerReporting
            | Rules::PhysicalRecovery
            | Rules::ContestedResources
            | Rules::SectCultivation
            | Rules::MartialMentorship
            | Rules::MeridianCultivation
            | Rules::PersistentInjury
            | Rules::AutonomousAgency
            | Rules::SectHierarchy
            | Rules::DutyCoherence
            | Rules::GeneratedRiverlands
            | Rules::ExpandedRiverlands
            | Rules::RegionalKnowledge
            | Rules::ExactLod
            | Rules::SettlementProduction
            | Rules::RoadHistory
            | Rules::EmergentRequests
            | Rules::PersonalFavors
            | Rules::ReciprocalBonds
            | Rules::EncounterMemory
            | Rules::SocialNetworks
            | Rules::CollectiveAgency
            | Rules::DynamicObligations
            | Rules::SituatedCommunication
            | Rules::InformationNetworks
            | Rules::CausalNarratives
            | Rules::PersonalDevelopment
            | Rules::VoluntaryCircles
            | Rules::SituatedDiplomacy
    ) {
        out.count(scenario.recruits.len());
        for &(actor, rule) in scenario.recruits {
            out.actor(actor);
            out.u8(match rule {
                Invitation::OpenRoad => 0,
                Invitation::SchoolMember => 1,
                Invitation::LodgeMember => 2,
            });
        }
    }
    sha256(&out.0)
}

pub(super) fn state_digest(world: &World) -> [u8; 32] {
    let mut out = Wire(match world.rules {
        Rules::Original => b"SH:C10:STATE:v1".to_vec(),
        Rules::Qinghe => b"SH:C10:STATE:v2".to_vec(),
        Rules::Riverlands => b"SH:C10:STATE:v3".to_vec(),
        Rules::ReedHall => b"SH:C10:STATE:v4".to_vec(),
        Rules::RoadCompany => b"SH:C10:STATE:v5".to_vec(),
        Rules::SectRivalry => b"SH:C10:STATE:v6".to_vec(),
        Rules::SectPolitics => b"SH:C10:STATE:v7".to_vec(),
        Rules::TrafficCourtesy => b"SH:C10:STATE:v8".to_vec(),
        Rules::LivingAgency => b"SH:C10:STATE:v9".to_vec(),
        Rules::FactionAgency => b"SH:C10:STATE:v10".to_vec(),
        Rules::InstitutionalAgency => b"SH:C10:STATE:v11".to_vec(),
        Rules::ContractAccountability => b"SH:C10:STATE:v12".to_vec(),
        Rules::InformationDisputes => b"SH:C10:STATE:v13".to_vec(),
        Rules::PlayerAgency => b"SH:C10:STATE:v14".to_vec(),
        Rules::Consequences => b"SH:C10:STATE:v15".to_vec(),
        Rules::ViolenceLogistics => b"SH:C10:STATE:v16".to_vec(),
        Rules::PlayerReporting => b"SH:C10:STATE:v17".to_vec(),
        Rules::PhysicalRecovery => b"SH:C10:STATE:v18".to_vec(),
        Rules::ContestedResources => b"SH:C10:STATE:v19".to_vec(),
        Rules::SectCultivation => b"SH:C10:STATE:v20".to_vec(),
        Rules::MartialMentorship => b"SH:C10:STATE:v21".to_vec(),
        Rules::MeridianCultivation => b"SH:C10:STATE:v22".to_vec(),
        Rules::PersistentInjury => b"SH:C10:STATE:v23".to_vec(),
        Rules::AutonomousAgency => b"SH:C10:STATE:v24".to_vec(),
        Rules::SectHierarchy => b"SH:C10:STATE:v25".to_vec(),
        Rules::DutyCoherence => b"SH:C10:STATE:v26".to_vec(),
        Rules::GeneratedRiverlands => b"SH:C10:STATE:v27".to_vec(),
        Rules::ExpandedRiverlands => b"SH:C10:STATE:v28".to_vec(),
        Rules::RegionalKnowledge => b"SH:C10:STATE:v29".to_vec(),
        Rules::ExactLod => b"SH:C10:STATE:v30".to_vec(),
        Rules::SettlementProduction => b"SH:C10:STATE:v31".to_vec(),
        Rules::RoadHistory => b"SH:C10:STATE:v32".to_vec(),
        Rules::EmergentRequests => b"SH:C10:STATE:v33".to_vec(),
        Rules::PersonalFavors => b"SH:C10:STATE:v34".to_vec(),
        Rules::ReciprocalBonds => b"SH:C10:STATE:v35".to_vec(),
        Rules::EncounterMemory => b"SH:C10:STATE:v36".to_vec(),
        Rules::SocialNetworks => b"SH:C10:STATE:v37".to_vec(),
        Rules::CollectiveAgency => b"SH:C10:STATE:v38".to_vec(),
        Rules::DynamicObligations => b"SH:C10:STATE:v39".to_vec(),
        Rules::SituatedCommunication => b"SH:C10:STATE:v40".to_vec(),
        Rules::InformationNetworks => b"SH:C10:STATE:v41".to_vec(),
        Rules::CausalNarratives => b"SH:C10:STATE:v42".to_vec(),
        Rules::PersonalDevelopment => b"SH:C10:STATE:v43".to_vec(),
        Rules::VoluntaryCircles => b"SH:C10:STATE:v44".to_vec(),
        Rules::SituatedDiplomacy => b"SH:C10:STATE:v45".to_vec(),
    });
    out.0.extend(bound_scenario_digest(world));
    out.u64(world.tick);
    out.position(world.player.position);
    out.u16(world.player.hp);
    out.u32(world.player.herbs);
    if !world.original_rules() {
        out.u8(world.breath);
    }
    if let Some(region) = world.region() {
        out.region(region);
    }
    if let Some(f) = world.frontier() {
        for n in [
            f.grove_herbs,
            f.depot_herbs,
            f.depot_coins,
            f.regrown,
            f.stocked,
            f.shipped_to_willowford,
            f.player_bought,
            f.player_sold,
        ] {
            out.u32(n);
        }
    }
    if let Some(school) = world.school() {
        out.school(school);
        if let Some(company) = world.company() {
            out.u8(u8::from(school.vacancy_incident.is_some()));
            if let Some(cause) = school.vacancy_incident {
                out.u32(cause.0);
            }
            out.count(school.history.len());
            for event in &school.history {
                out.u64(event.at);
                out.actor(event.former);
                out.optional_actor(event.successor);
            }
            out.company(company);
        }
    }
    if let Some(rivalry) = world.rivalry() {
        out.rivalry(rivalry);
    }
    if let Some(politics) = world.politics() {
        out.politics(politics);
    }
    if let Some(traffic) = world.traffic() {
        out.traffic(traffic);
    }
    if let Some(relief) = world.relief() {
        out.relief(relief);
    }
    if let Some(agency) = world.faction_agency_state() {
        out.faction_agency(agency);
    }
    if let Some(contracts) = world.contracts() {
        out.contracts(contracts);
    }
    if let Some(memory) = world.contract_memory() {
        out.contract_memory(memory);
    }
    if let Some(information) = world.information() {
        out.information(information);
    }
    for quantity in [
        world.supply.source,
        world.supply.shrine,
        world.supply.npc_deliveries,
        world.supply.player_deliveries,
        world.supply.consumed,
        world.supply.regrown,
    ] {
        out.u32(quantity);
    }
    out.count(world.npcs.len());
    for (&id, npc) in &world.npcs {
        out.actor(id);
        out.position(npc.position);
        out.u16(npc.hp);
        out.u8(npc.fatigue);
        out.u32(npc.herbs);
        if matches!(
            world.rules,
            Rules::RoadCompany
                | Rules::SectRivalry
                | Rules::SectPolitics
                | Rules::TrafficCourtesy
                | Rules::LivingAgency
                | Rules::FactionAgency
                | Rules::InstitutionalAgency
                | Rules::ContractAccountability
                | Rules::InformationDisputes
                | Rules::PlayerAgency
                | Rules::Consequences
                | Rules::ViolenceLogistics
                | Rules::PlayerReporting
                | Rules::PhysicalRecovery
                | Rules::ContestedResources
                | Rules::SectCultivation
                | Rules::MartialMentorship
                | Rules::MeridianCultivation
                | Rules::PersistentInjury
                | Rules::AutonomousAgency
                | Rules::SectHierarchy
                | Rules::DutyCoherence
                | Rules::GeneratedRiverlands
                | Rules::ExpandedRiverlands
                | Rules::RegionalKnowledge
                | Rules::ExactLod
                | Rules::SettlementProduction
                | Rules::RoadHistory
                | Rules::EmergentRequests
                | Rules::PersonalFavors
                | Rules::ReciprocalBonds
                | Rules::EncounterMemory
                | Rules::SocialNetworks
                | Rules::CollectiveAgency
                | Rules::DynamicObligations
                | Rules::SituatedCommunication
                | Rules::InformationNetworks
                | Rules::CausalNarratives
                | Rules::PersonalDevelopment
                | Rules::VoluntaryCircles
                | Rules::SituatedDiplomacy
        ) {
            out.u8(npc.packed_herbs);
        }
        out.count(npc.waypoint);
        out.u8(u8::from(npc.last_investigated.is_some()));
        if let Some(incident) = npc.last_investigated {
            out.u32(incident.0);
        }
        out.goal(npc.activity.goal);
        out.u64(npc.activity.since);
    }
    let social = &world.social;
    out.count(social.incidents.len());
    for incident in &social.incidents {
        out.u32(incident.id.0);
        out.u64(incident.tick);
        out.actor(incident.source);
        out.actor(incident.target);
        out.position(incident.position);
        out.u16(incident.damage);
        out.u8(u8::from(incident.fatal));
        out.u8(u8::from(incident.retaliation));
    }
    out.count(social.relations.len());
    for (&(actor, other), relation) in &social.relations {
        out.actor(actor);
        out.actor(other);
        out.u8(relation.regard.to_le_bytes()[0]);
        out.u8(relation.familiarity);
    }
    out.count(social.known.len());
    for (&actor, receipts) in &social.known {
        out.actor(actor);
        out.count(receipts.len());
        for (&incident, receipt) in receipts {
            out.u32(incident.0);
            out.u64(receipt.acquired);
            out.u8(receipt.regard_change.to_le_bytes()[0]);
            match receipt.source {
                KnowledgeSource::Observed => out.u8(0),
                KnowledgeSource::Reported {
                    sender,
                    witness,
                    hops,
                } => {
                    out.u8(1);
                    out.actor(sender);
                    out.actor(witness);
                    out.u8(hops);
                }
            }
        }
    }
    out.count(social.pending.len());
    for report in social.pending.values() {
        out.u32(report.incident.0);
        out.actor(report.sender);
        out.actor(report.recipient);
        out.actor(report.witness);
        out.u8(report.hops);
        out.u64(report.sent);
        out.u64(report.due);
    }
    if matches!(
        world.rules,
        Rules::ViolenceLogistics
            | Rules::PlayerReporting
            | Rules::PhysicalRecovery
            | Rules::ContestedResources
            | Rules::SectCultivation
            | Rules::MartialMentorship
            | Rules::MeridianCultivation
            | Rules::PersistentInjury
            | Rules::AutonomousAgency
            | Rules::SectHierarchy
            | Rules::DutyCoherence
            | Rules::GeneratedRiverlands
            | Rules::ExpandedRiverlands
            | Rules::RegionalKnowledge
            | Rules::ExactLod
            | Rules::SettlementProduction
            | Rules::RoadHistory
            | Rules::EmergentRequests
            | Rules::PersonalFavors
            | Rules::ReciprocalBonds
            | Rules::EncounterMemory
            | Rules::SocialNetworks
            | Rules::CollectiveAgency
            | Rules::DynamicObligations
            | Rules::SituatedCommunication
            | Rules::InformationNetworks
            | Rules::CausalNarratives
            | Rules::PersonalDevelopment
            | Rules::VoluntaryCircles
            | Rules::SituatedDiplomacy
    ) {
        out.count(social.discoveries.len());
        for discovery in social.discoveries.values() {
            out.actor(discovery.observer);
            out.actor(discovery.body);
            out.u64(discovery.at);
            out.optional_actor(discovery.nearby);
            match discovery.source {
                KnowledgeSource::Observed => out.u8(0),
                KnowledgeSource::Reported {
                    sender,
                    witness,
                    hops,
                } => {
                    out.u8(1);
                    out.actor(sender);
                    out.actor(witness);
                    out.u8(hops);
                }
            }
            if matches!(
                world.rules,
                Rules::PlayerReporting
                    | Rules::PhysicalRecovery
                    | Rules::ContestedResources
                    | Rules::SectCultivation
                    | Rules::MartialMentorship
                    | Rules::MeridianCultivation
                    | Rules::PersistentInjury
                    | Rules::AutonomousAgency
                    | Rules::SectHierarchy
                    | Rules::DutyCoherence
                    | Rules::GeneratedRiverlands
                    | Rules::ExpandedRiverlands
                    | Rules::RegionalKnowledge
                    | Rules::ExactLod
                    | Rules::SettlementProduction
                    | Rules::RoadHistory
                    | Rules::EmergentRequests
                    | Rules::PersonalFavors
                    | Rules::ReciprocalBonds
                    | Rules::EncounterMemory
                    | Rules::SocialNetworks
                    | Rules::CollectiveAgency
                    | Rules::DynamicObligations
                    | Rules::SituatedCommunication
                    | Rules::InformationNetworks
                    | Rules::CausalNarratives
                    | Rules::PersonalDevelopment
                    | Rules::VoluntaryCircles
                    | Rules::SituatedDiplomacy
            ) {
                out.u8(u8::from(discovery.prior_report.is_some()));
                if let Some((at, source)) = discovery.prior_report {
                    out.u64(at);
                    match source {
                        KnowledgeSource::Observed => out.u8(0),
                        KnowledgeSource::Reported {
                            sender,
                            witness,
                            hops,
                        } => {
                            out.u8(1);
                            out.actor(sender);
                            out.actor(witness);
                            out.u8(hops);
                        }
                    }
                }
            }
        }
        out.count(social.body_reports.len());
        for report in social.body_reports.values() {
            out.actor(report.sender);
            out.actor(report.recipient);
            out.actor(report.body);
            out.u64(report.due);
        }
    }
    if matches!(
        world.rules,
        Rules::PhysicalRecovery
            | Rules::ContestedResources
            | Rules::SectCultivation
            | Rules::MartialMentorship
            | Rules::MeridianCultivation
            | Rules::PersistentInjury
            | Rules::AutonomousAgency
            | Rules::SectHierarchy
            | Rules::DutyCoherence
            | Rules::GeneratedRiverlands
            | Rules::ExpandedRiverlands
            | Rules::RegionalKnowledge
            | Rules::ExactLod
            | Rules::SettlementProduction
            | Rules::RoadHistory
            | Rules::EmergentRequests
            | Rules::PersonalFavors
            | Rules::ReciprocalBonds
            | Rules::EncounterMemory
            | Rules::SocialNetworks
            | Rules::CollectiveAgency
            | Rules::DynamicObligations
            | Rules::SituatedCommunication
            | Rules::InformationNetworks
            | Rules::CausalNarratives
            | Rules::PersonalDevelopment
            | Rules::VoluntaryCircles
            | Rules::SituatedDiplomacy
    ) {
        out.count(social.recovery_cargo.len());
        for (&actor, &herbs) in &social.recovery_cargo {
            out.actor(actor);
            out.u32(herbs);
        }
        out.count(social.recoveries.len());
        for event in &social.recoveries {
            out.u32(event.id);
            out.u64(event.at);
            out.actor(event.actor);
            out.actor(event.body);
            out.u32(event.herbs);
            out.u8(u8::from(event.delivered));
            out.u8(u8::from(event.witnessed));
        }
    }
    if let Some(contested) = world.contested() {
        out.count(contested.lots.len());
        for (&id, lot) in &contested.lots {
            out.u32(id);
            out.u8(match lot.good {
                Good::SpiritOre => 0,
                Good::JadeScript => 1,
            });
            match lot.holder {
                Holder::Ground(site) => {
                    out.u8(0);
                    out.position(site);
                }
                Holder::Carried(actor) => {
                    out.u8(1);
                    out.actor(actor);
                }
                Holder::Deposited(actor) => {
                    out.u8(2);
                    out.actor(actor);
                }
                Holder::Consumed(actor) => {
                    out.u8(3);
                    out.actor(actor);
                }
            }
        }
        out.count(contested.known.len());
        for (&(actor, id), &site) in &contested.known {
            out.actor(actor);
            out.u32(id);
            out.position(site);
        }
        out.count(contested.history.len());
        for event in &contested.history {
            out.u32(event.id);
            out.u64(event.at);
            out.u32(event.lot);
            out.actor(event.actor);
            out.u8(match event.kind {
                LotEventKind::Claimed => 0,
                LotEventKind::Sold => 1,
                LotEventKind::Deposited => 2,
                LotEventKind::Dropped => 3,
                LotEventKind::Attuned => 4,
            });
            out.position(event.position);
            out.u8(u8::from(event.witnessed));
        }
        out.count(contested.prestige.len());
        for (&actor, &value) in &contested.prestige {
            out.actor(actor);
            out.u32(value);
        }
        if matches!(
            world.rules,
            Rules::SectCultivation
                | Rules::MartialMentorship
                | Rules::MeridianCultivation
                | Rules::PersistentInjury
                | Rules::AutonomousAgency
                | Rules::SectHierarchy
                | Rules::DutyCoherence
                | Rules::GeneratedRiverlands
                | Rules::ExpandedRiverlands
                | Rules::RegionalKnowledge
                | Rules::ExactLod
                | Rules::SettlementProduction
                | Rules::RoadHistory
                | Rules::EmergentRequests
                | Rules::PersonalFavors
                | Rules::ReciprocalBonds
                | Rules::EncounterMemory
                | Rules::SocialNetworks
                | Rules::CollectiveAgency
                | Rules::DynamicObligations
                | Rules::SituatedCommunication
                | Rules::InformationNetworks
                | Rules::CausalNarratives
                | Rules::PersonalDevelopment
                | Rules::VoluntaryCircles
                | Rules::SituatedDiplomacy
        ) {
            out.u8(contested.wards);
            out.u8(contested.script_charges);
        }
    }
    if let Some(martial) = world.martial() {
        out.count(martial.skills.len());
        for (&actor, skill) in &martial.skills {
            out.actor(actor);
            out.u8(match skill.form {
                None => 0,
                Some(Form::ReedPalm) => 1,
                Some(Form::StoneFist) => 2,
            });
            out.optional_actor(skill.teacher);
            out.u8(skill.lessons);
            out.u8(skill.strain);
            out.u8(skill.parry);
        }
        out.count(martial.lessons.len());
        for lesson in &martial.lessons {
            out.u32(lesson.id);
            out.u64(lesson.at);
            out.actor(lesson.student);
            out.actor(lesson.teacher);
            out.u8(match lesson.form {
                Form::ReedPalm => 1,
                Form::StoneFist => 2,
            });
            out.u8(u8::from(lesson.learned));
            out.u8(u8::from(lesson.witnessed));
        }
    }
    if let Some(meridians) = world.meridians() {
        out.count(meridians.people.len());
        for (&actor, state) in &meridians.people {
            out.actor(actor);
            out.u8(state.qi);
            out.u8(state.channels);
            out.u8(state.blockage);
        }
        out.count(meridians.events.len());
        for event in &meridians.events {
            out.u32(event.id);
            out.u64(event.at);
            out.actor(event.actor);
            out.u8(match event.kind {
                MeridianEventKind::Drawn => 0,
                MeridianEventKind::Opened => 1,
                MeridianEventKind::Obstructed => 2,
                MeridianEventKind::Treated => 3,
                MeridianEventKind::Spent => 4,
            });
            out.position(event.site);
            out.u8(u8::from(event.witnessed));
        }
    }
    if let Some(injuries) = world.injuries() {
        out.count(injuries.people.len());
        for (&actor, wound) in &injuries.people {
            out.actor(actor);
            out.u8(wound.arms);
            out.u8(wound.legs);
        }
        out.count(injuries.events.len());
        for event in &injuries.events {
            out.u32(event.id);
            out.u64(event.at);
            out.actor(event.actor);
            out.u8(match event.kind {
                WoundKind::Arms => 0,
                WoundKind::Legs => 1,
                WoundKind::Treated => 2,
            });
            out.position(event.site);
            out.u8(u8::from(event.witnessed));
        }
    }
    if let Some(agency) = world.character_agency() {
        out.count(agency.people.len());
        for (&actor, state) in &agency.people {
            out.actor(actor);
            out.u32(state.coins);
            out.u8(state.livelihood_herbs);
            out.ambition(state.ambition);
            out.u8(u8::from(state.suspended.is_some()));
            if let Some(goal) = state.suspended {
                out.goal(goal);
            }
        }
        out.u32(agency.clinic_coins);
        out.count(agency.history.len());
        for event in &agency.history {
            out.u32(event.id);
            out.u64(event.at);
            out.actor(event.actor);
            out.u8(match event.kind {
                AgencyEventKind::GoalChanged => 0,
                AgencyEventKind::WorkGathered => 1,
                AgencyEventKind::WorkSold => 2,
                AgencyEventKind::CarePaid => 3,
                AgencyEventKind::CareBartered => 4,
            });
            out.u8(u8::from(event.goal.is_some()));
            if let Some(goal) = event.goal {
                out.goal(goal);
            }
            out.u8(u8::from(event.reason.is_some()));
            if let Some(reason) = event.reason {
                out.decision_reason(reason);
            }
            out.u32(event.amount);
            out.optional_actor(event.other);
            out.position(event.site);
            out.u8(u8::from(event.witnessed));
        }
        out.u32(agency.next_event);
    }
    if let Some(hierarchy) = world.sect_hierarchy() {
        out.u8(u8::from(hierarchy.player.is_some()));
        if let Some(player) = hierarchy.player {
            out.sect_member(player);
        }
        out.count(hierarchy.members.len());
        for (&actor, &member) in &hierarchy.members {
            out.actor(actor);
            out.sect_member(member);
        }
        out.count(hierarchy.history.len());
        for event in &hierarchy.history {
            out.u32(event.id);
            out.u64(event.at);
            out.actor(event.actor);
            out.u8(match event.kind {
                HierarchyEventKind::DutyAccepted => 0,
                HierarchyEventKind::DutyDelivered => 1,
                HierarchyEventKind::MeritEarned => 2,
                HierarchyEventKind::Promoted => 3,
                HierarchyEventKind::AllocationClaimed => 4,
                HierarchyEventKind::OfficeAssumed => 5,
            });
            out.hall_rank(event.rank);
            out.u8(event.merit);
            out.u8(u8::from(event.witnessed));
        }
        out.u32(hierarchy.next_event);
    }
    if let Some(news) = world.regional_news() {
        let bytes = news.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(scheduler) = world.scheduler() {
        let bytes = scheduler.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(production) = world.production() {
        let bytes = production.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(history) = world.road_history() {
        let bytes = history.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(requests) = world.requests() {
        let bytes = requests.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(favors) = world.favors() {
        let bytes = favors.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(reciprocity) = world.reciprocity() {
        let bytes = reciprocity.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(encounters) = world.encounters() {
        let bytes = encounters.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(network) = world.social_network() {
        let bytes = network.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(cooperation) = world.cooperation() {
        let bytes = cooperation.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(obligations) = world.obligations() {
        let bytes = obligations.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(communication) = world.communication() {
        let bytes = communication.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(rumors) = world.rumors() {
        let bytes = rumors.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(narrative) = world.narrative() {
        let bytes = narrative.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(development) = world.development() {
        let bytes = development.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(circles) = world.circles() {
        let bytes = circles.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    if let Some(d) = world.diplomacy() {
        let bytes = d.canonical_bytes();
        out.count(bytes.len());
        out.0.extend(bytes);
    }
    out.count(world.journal.len());
    for &intent in &world.journal {
        out.intent(intent);
    }
    out.outcome(world.latest);
    sha256(&out.0)
}

struct Wire(Vec<u8>);

impl Wire {
    fn u8(&mut self, value: u8) {
        self.0.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.0.extend(value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.0.extend(value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.0.extend(value.to_le_bytes());
    }

    fn count(&mut self, value: usize) {
        self.u32(u32::try_from(value).expect("bounded checkpoint field"));
    }

    fn string(&mut self, value: &str) {
        self.count(value.len());
        self.0.extend(value.as_bytes());
    }

    fn actor(&mut self, value: ActorId) {
        self.u32(value.get());
    }

    fn position(&mut self, value: Position) {
        self.u32(value.x);
        self.u32(value.y);
    }

    fn region(&mut self, region: &Region) {
        self.u32(region.seed);
        self.u8(region
            .background
            .map_or(0, |background| background_code(background) + 1));
        for value in [
            region.coins,
            region.market_coins,
            region.market_herbs,
            region.garden_herbs,
            region.garden_regrown,
            region.shipped_to_qinghe,
            region.hall_offerings,
            region.remedies,
            region.remedies_used,
            region.order_remaining,
        ] {
            self.u32(value);
        }
        self.u8(u8::from(region.enrolled));
        self.u8(u8::from(region.order_in_transit.is_some()));
        if let Some((due, amount)) = region.order_in_transit {
            self.u64(due);
            self.u32(amount);
        }
    }

    fn school(&mut self, school: &School) {
        self.optional_actor(school.leader);
        self.u8(match school.duty {
            Duty::Unassigned => 0,
            Duty::Carrying => 1,
            Duty::Delivered => 2,
            Duty::Reported => 3,
        });
        self.u8(u8::from(school.form_learned));
        self.u8(school.qi);
        self.u8(u8::from(school.succession.is_some()));
        if let Some(succession) = school.succession {
            self.u64(succession.at);
            self.actor(succession.former);
            self.optional_actor(succession.successor);
        }
    }

    fn company(&mut self, company: &Company) {
        self.u32(company.herbs_used);
        self.u8(u8::from(company.journey.is_some()));
        if let Some(journey) = company.journey {
            self.actor(journey.actor);
            self.position(journey.last_seen);
            self.u64(journey.seen_at);
        }
        self.count(company.history.len());
        for event in &company.history {
            self.u64(event.at);
            self.actor(event.actor);
            self.position(event.place);
            self.u8(u8::from(event.witnessed));
            self.u8(match event.kind {
                JourneyEvent::Joined => 0,
                JourneyEvent::Dismissed => 1,
                JourneyEvent::LostContact => 2,
                JourneyEvent::Distrust => 3,
                JourneyEvent::Office => 4,
                JourneyEvent::Fallen => 5,
                JourneyEvent::PlayerFallen => 6,
                JourneyEvent::FieldCare => 7,
            });
        }
    }

    fn rivalry(&mut self, rivalry: &Rivalry) {
        self.u8(rivalry.pressure);
        for value in [
            rivalry.lodge_coins,
            rivalry.market_purchases,
            rivalry.petitions,
            rivalry.consumed_herbs,
            rivalry.accord_offerings,
            rivalry.hall_patronage,
        ] {
            self.u32(value);
        }
        self.u64(rivalry.peace_until);
        self.count(rivalry.history.len());
        for event in &rivalry.history {
            self.u64(event.at);
            self.actor(event.actor);
            self.position(event.position);
            self.u8(u8::from(event.witnessed));
            self.u8(match event.kind {
                SectEventKind::Purchased => 0,
                SectEventKind::Petitioned => 1,
                SectEventKind::Accord => 2,
                SectEventKind::Patronage => 3,
            });
        }
    }

    fn politics(&mut self, politics: &Politics) {
        self.u8(match politics.allegiance {
            Allegiance::Independent => 0,
            Allegiance::Stonebank => 1,
        });
        for value in [
            politics.hall_coins,
            politics.hall_purchases,
            politics.hall_supplies,
            politics.lodge_oaths,
            politics.lodge_aid,
        ] {
            self.u32(value);
        }
        self.u8(politics.hall_favor.to_le_bytes()[0]);
        self.u8(politics.lodge_favor.to_le_bytes()[0]);
        self.count(politics.history.len());
        for event in &politics.history {
            self.u64(event.at);
            self.actor(event.actor);
            self.position(event.position);
            self.u8(u8::from(event.witnessed));
            self.u8(match event.kind {
                PoliticalEventKind::HallPurchase => 0,
                PoliticalEventKind::HallSupply => 1,
                PoliticalEventKind::LodgeOath => 2,
                PoliticalEventKind::LodgeAid => 3,
                PoliticalEventKind::LodgeRenunciation => 4,
                PoliticalEventKind::HallPatronage => 5,
            });
        }
    }

    fn traffic(&mut self, traffic: &Traffic) {
        self.u32(traffic.swaps);
        self.u32(traffic.approach_steps);
        self.count(traffic.stalls.len());
        for (&actor, &stalled) in &traffic.stalls {
            self.actor(actor);
            self.u8(stalled);
        }
        self.count(traffic.history.len());
        for event in &traffic.history {
            self.u64(event.at);
            self.actor(event.mover);
            self.actor(event.other);
            self.position(event.from);
            self.position(event.to);
            self.u8(match event.kind {
                TrafficEventKind::PlayerPass => 0,
                TrafficEventKind::NpcPass => 1,
            });
            self.u8(u8::from(event.witnessed));
        }
    }

    fn relief(&mut self, relief: &Relief) {
        self.u32(relief.requests);
        self.u32(relief.deliveries);
        self.u32(relief.failures);
        self.u64(relief.last_closed);
        self.u8(u8::from(relief.active.is_some()));
        if let Some(case) = relief.active {
            self.u64(case.opened);
            self.actor(case.messenger);
            self.actor(case.sponsor);
            self.u8(match case.phase {
                ReliefPhase::SeekingSponsor => 0,
                ReliefPhase::Procuring => 1,
                ReliefPhase::Delivering => 2,
            });
        }
        self.count(relief.history.len());
        for event in &relief.history {
            self.u64(event.at);
            self.actor(event.actor);
            self.position(event.position);
            self.u8(u8::from(event.witnessed));
            self.u8(match event.kind {
                ReliefEventKind::Observed => 0,
                ReliefEventKind::Reported => 1,
                ReliefEventKind::Purchased => 2,
                ReliefEventKind::Redirected => 3,
                ReliefEventKind::Delivered => 4,
                ReliefEventKind::Withdrawn => 5,
                ReliefEventKind::Failed => 6,
            });
        }
    }

    fn faction_agency(&mut self, agency: &FactionAgency) {
        self.u32(agency.surveys);
        self.u32(agency.lodge_deliveries);
        self.u32(agency.hall_deliveries);
        self.u8(agency.public_standing.to_le_bytes()[0]);
        self.u64(agency.last_closed);
        self.u8(u8::from(agency.active.is_some()));
        if let Some(case) = agency.active {
            self.u64(case.opened);
            self.actor(case.envoy);
            self.u8(match case.phase {
                FactionPhase::Survey => 0,
                FactionPhase::Procuring => 1,
                FactionPhase::Delivering => 2,
            });
        }
        self.count(agency.history.len());
        for event in &agency.history {
            self.u64(event.at);
            self.actor(event.actor);
            self.position(event.position);
            self.u8(u8::from(event.witnessed));
            self.u8(match event.kind {
                FactionEventKind::SurveyClear => 0,
                FactionEventKind::ShortageSeen => 1,
                FactionEventKind::Purchased => 2,
                FactionEventKind::Redirected => 3,
                FactionEventKind::Delivered => 4,
                FactionEventKind::HallDelivered => 5,
                FactionEventKind::Withdrawn => 6,
                FactionEventKind::Failed => 7,
            });
        }
    }

    fn contracts(&mut self, registry: &Contracts) {
        self.u32(registry.opened);
        self.u32(registry.completed);
        self.u32(registry.failed);
        for &tick in &registry.last_closed {
            self.u64(tick);
        }
        self.count(registry.active.len());
        for case in &registry.active {
            self.u32(case.id);
            self.need(case.need);
            self.actor(case.requester);
            self.u64(case.opened);
            self.u8(match case.phase {
                ContractPhase::Soliciting => 0,
                ContractPhase::Awarding => 1,
                ContractPhase::Procuring => 2,
                ContractPhase::Delivering => 3,
            });
            self.count(case.next_candidate);
            self.optional_actor(case.provider);
            self.count(case.bids.len());
            for bid in &case.bids {
                self.actor(bid.provider);
                self.u32(bid.quoted_coins);
                self.u8(bid.local_trust.to_le_bytes()[0]);
            }
        }
        self.count(registry.history.len());
        for event in &registry.history {
            self.u64(event.at);
            self.u32(event.case_id);
            self.need(event.need);
            self.actor(event.actor);
            self.position(event.position);
            self.u8(u8::from(event.witnessed));
            self.u8(match event.kind {
                ContractEventKind::Observed => 0,
                ContractEventKind::Offered => 1,
                ContractEventKind::Declined => 2,
                ContractEventKind::Awarded => 3,
                ContractEventKind::Purchased => 4,
                ContractEventKind::Redirected => 5,
                ContractEventKind::Delivered => 6,
                ContractEventKind::Failed => 7,
                ContractEventKind::Withdrawn => 8,
                ContractEventKind::ResolvedLocally => 9,
            });
        }
    }

    fn contract_memory(&mut self, memory: &ContractMemory) {
        self.count(memory.receipts.len());
        for receipt in &memory.receipts {
            self.u32(receipt.case_id);
            self.need(receipt.need);
            self.actor(receipt.requester);
            self.actor(receipt.provider);
            self.u64(receipt.opened);
            self.u64(receipt.closed);
            self.u8(u8::from(receipt.delivered));
            self.u8(u8::from(receipt.inspected));
        }
        self.count(memory.regard.len());
        for (&(requester, provider), &score) in &memory.regard {
            self.actor(requester);
            self.actor(provider);
            self.u8(score.to_le_bytes()[0]);
        }
        self.count(memory.history.len());
        for event in &memory.history {
            self.u64(event.at);
            self.u32(event.case_id);
            self.need(event.need);
            self.actor(event.requester);
            self.actor(event.provider);
            self.position(event.site);
            self.u8(match event.finding {
                AuditFinding::Delivered => 0,
                AuditFinding::Unfulfilled => 1,
            });
            self.u8(u8::from(event.witnessed));
        }
    }

    fn information(&mut self, information: &Information) {
        self.count(information.events.len());
        for event in &information.events {
            self.u32(event.id.0);
            self.u64(event.tick);
            self.u32(event.case_id);
            self.need(event.need);
            self.actor(event.requester);
            self.actor(event.provider);
            self.u8(match event.finding {
                AuditFinding::Delivered => 0,
                AuditFinding::Unfulfilled => 1,
            });
            self.position(event.site);
        }
        self.count(information.known.len());
        for (&actor, receipts) in &information.known {
            self.actor(actor);
            self.count(receipts.len());
            for (&event, receipt) in receipts {
                self.u32(event.0);
                self.u64(receipt.acquired);
                match receipt.provenance {
                    Provenance::Firsthand { observer } => {
                        self.u8(0);
                        self.actor(observer);
                    }
                    Provenance::Reported {
                        sender,
                        origin,
                        hops,
                    } => {
                        self.u8(1);
                        self.actor(sender);
                        self.actor(origin);
                        self.u8(hops);
                    }
                }
            }
        }
        self.count(information.pending.len());
        for report in information.pending.values() {
            self.u32(report.event.0);
            self.actor(report.sender);
            self.actor(report.recipient);
            self.actor(report.origin);
            self.u8(report.hops);
            self.u64(report.sent);
            self.u64(report.due);
        }
        self.count(information.disputes.len());
        for dispute in &information.disputes {
            self.u32(dispute.id);
            self.u64(dispute.opened);
            self.actor(dispute.institution);
            self.actor(dispute.subject);
            self.u32(dispute.event.0);
            self.need(dispute.need);
            self.u8(match dispute.phase {
                DisputePhase::Open => 0,
                DisputePhase::Investigating => 1,
                DisputePhase::Confirmed => 2,
                DisputePhase::Refuted => 3,
                DisputePhase::Suspended => 4,
                DisputePhase::Restored => 5,
            });
            self.u8(u8::from(dispute.resolved_at.is_some()));
            if let Some(at) = dispute.resolved_at {
                self.u64(at);
            }
        }
        self.count(information.history.len());
        for event in &information.history {
            self.u64(event.at);
            self.u8(match event.kind {
                InfoLogKind::Learned => 0,
                InfoLogKind::DisputeOpened => 1,
                InfoLogKind::DisputeUpdated => 2,
                InfoLogKind::TestimonyQueued => 3,
            });
            self.actor(event.actor);
            self.optional_actor(event.other);
            self.u32(event.event.0);
            self.u8(u8::from(event.dispute.is_some()));
            if let Some(id) = event.dispute {
                self.u32(id);
            }
            self.u8(u8::from(event.phase.is_some()));
            if let Some(phase) = event.phase {
                self.u8(match phase {
                    DisputePhase::Open => 0,
                    DisputePhase::Investigating => 1,
                    DisputePhase::Confirmed => 2,
                    DisputePhase::Refuted => 3,
                    DisputePhase::Suspended => 4,
                    DisputePhase::Restored => 5,
                });
            }
            self.u8(u8::from(event.witnessed));
        }
        self.u32(information.next_dispute);
        self.u32(information.next_event);
    }

    fn need(&mut self, need: Need) {
        self.u8(match need {
            Need::ShrineMedicine => 0,
            Need::HallOfferings => 1,
        });
    }

    fn optional_actor(&mut self, actor: Option<ActorId>) {
        self.u8(u8::from(actor.is_some()));
        if let Some(actor) = actor {
            self.actor(actor);
        }
    }

    fn ambition(&mut self, ambition: Ambition) {
        self.u8(match ambition {
            Ambition::CommunityService => 0,
            Ambition::Prosperity => 1,
            Ambition::MartialMastery => 2,
            Ambition::Cultivation => 3,
            Ambition::InstitutionalDuty => 4,
        });
    }

    fn decision_reason(&mut self, reason: DecisionReason) {
        self.u8(match reason {
            DecisionReason::ImmediateDanger => 0,
            DecisionReason::PhysicalRecovery => 1,
            DecisionReason::TreatmentFunds => 2,
            DecisionReason::MedicalDuty => 3,
            DecisionReason::Cultivation => 4,
            DecisionReason::MartialAmbition => 5,
            DecisionReason::InstitutionalObligation => 6,
            DecisionReason::KnownOpportunity => 7,
            DecisionReason::Fatigue => 8,
            DecisionReason::Livelihood => 9,
            DecisionReason::Routine => 10,
            DecisionReason::ResumingAmbition => 11,
        });
    }

    fn hall_rank(&mut self, rank: HallRank) {
        self.u8(match rank {
            HallRank::Apprentice => 0,
            HallRank::Disciple => 1,
            HallRank::SeniorDisciple => 2,
            HallRank::Elder => 3,
        });
    }

    fn sect_member(&mut self, member: SectMember) {
        self.hall_rank(member.rank);
        self.u8(member.merit);
        self.u8(match member.duty {
            SectDuty::Unassigned => 0,
            SectDuty::Carrying => 1,
            SectDuty::Delivered => 2,
        });
        self.u8(u8::from(member.allocation_claimed));
    }

    // Both admission and checkpoint serialization share the same fixed record.
    fn intent_record(intent: Intent) -> [u8; INTENT_SIZE] {
        let (tag, payload): (u8, u32) = match intent {
            Intent::Wait => (0, 0),
            Intent::Move(Direction::North) => (1, 0),
            Intent::Move(Direction::East) => (1, 1),
            Intent::Move(Direction::South) => (1, 2),
            Intent::Move(Direction::West) => (1, 3),
            Intent::Talk(id) => (2, id.get()),
            Intent::Gather => (3, 0),
            Intent::Deliver => (4, 0),
            Intent::Attack(id) => (5, id.get()),
            Intent::Guard => (6, 0),
            Intent::Practice => (7, 0),
            Intent::ChooseBackground(background) => (8, u32::from(background_code(background))),
            Intent::BuyHerb => (9, 0),
            Intent::SellHerb => (10, 0),
            Intent::JoinHall => (11, 0),
            Intent::Brew => (12, 0),
            Intent::UseRemedy => (13, 0),
            Intent::HallDuty => (14, 0),
            Intent::StudyForm => (15, 0),
            Intent::CenterQi => (16, 0),
            Intent::ReedStrike(id) => (17, id.get()),
            Intent::Invite(id) => (18, id.get()),
            Intent::Provision(id) => (19, id.get()),
            Intent::Part(id) => (20, id.get()),
            Intent::OfferAccord => (21, 0),
            Intent::SupportHall => (22, 0),
            Intent::PledgeLodge => (23, 0),
            Intent::AidLodge => (24, 0),
            Intent::RenounceLodge => (25, 0),
            Intent::Ask(id) => (26, id.get()),
            Intent::Examine => (27, 0),
            Intent::Testify { to, event } => (28, pack_testimony(to, event)),
            Intent::Take(id) => (29, id.get()),
            Intent::ReportBody { to, body } => (30, pack_testimony(to, super::EventId(body.get()))),
            Intent::ClaimLot(id) => (31, id),
            Intent::OfferLot { to, id } => (32, pack_testimony(to, super::EventId(id))),
            Intent::AttuneLot(id) => (33, id),
            Intent::ChannelStrike(id) => (34, id.get()),
            Intent::Drill(id) => (35, id.get()),
            Intent::TechniqueStrike(id) => (36, id.get()),
            Intent::DrawQi => (37, 0),
            Intent::CirculateQi => (38, 0),
            Intent::MendMeridians => (39, 0),
            Intent::TreatWounds => (40, 0),
            Intent::ClaimHallAllocation => (41, 0),
            Intent::RecoverCache(id) => (42, id),
            Intent::EntrustCache(id) => (43, id),
            Intent::AcceptRequest(id) => (44, id),
            Intent::DeclineRequest(id) => (45, id),
            Intent::AbandonRequest(id) => (46, id),
            Intent::AcceptFavor(id) => (47, id),
            Intent::DeclineFavor(id) => (48, id),
            Intent::WithdrawFavor(id) => (49, id),
            Intent::FundFavor(id) => (50, id),
            Intent::RequestAid(id) => (51, id.get()),
            Intent::RepayAid(id) => (52, id.get()),
            Intent::WaiveAid(id) => (53, id.get()),
        };
        let [a, b, c, d] = payload.to_le_bytes();
        [tag, a, b, c, d]
    }

    fn intent(&mut self, intent: Intent) {
        self.0.extend(Self::intent_record(intent));
    }

    fn goal(&mut self, goal: Goal) {
        self.u8(match goal {
            Goal::Idle => 0,
            Goal::Rest => 1,
            Goal::Patrol => 2,
            Goal::TendClinic => 3,
            Goal::CollectHerbs => 4,
            Goal::DeliverHerbs => 5,
            Goal::SeekCare => 6,
            Goal::Treat(_) => 7,
            Goal::Investigate(_) => 8,
            Goal::Report { .. } => 9,
            Goal::Retreat => 10,
            Goal::Retaliate => 11,
            Goal::StockGarden => 12,
            Goal::StockMarket => 13,
            Goal::CollectFreight => 14,
            Goal::DeliverFreight => 15,
            Goal::TeachAtHall => 16,
            Goal::Follow(_) => 17,
            Goal::FieldCare => 18,
            Goal::BuyForLodge => 19,
            Goal::PetitionHall => 20,
            Goal::BuyForHall => 21,
            Goal::SupplyHall => 22,
            Goal::RequestRelief => 23,
            Goal::BuyRelief => 24,
            Goal::DeliverRelief => 25,
            Goal::CampRest => 26,
            Goal::SurveyQinghe => 27,
            Goal::BuyLodgeAid => 28,
            Goal::DeliverLodgeAid => 29,
            Goal::NegotiateContract(_) => 30,
            Goal::ProcureContract(_) => 31,
            Goal::FulfillContract(_) => 32,
            Goal::AuditContract(_) => 33,
            Goal::DeliverTestimony { .. } => 34,
            Goal::InvestigateDispute(_) => 35,
            Goal::InvestigateBody(_) => 36,
            Goal::RecoverBody(_) => 37,
            Goal::ReturnRecoveredCargo => 38,
            Goal::ClaimLot(_) => 39,
            Goal::DeliverLot(_) => 40,
            Goal::MartialPractice(_) => 41,
            Goal::MartialRest => 42,
            Goal::MeridianDraw => 43,
            Goal::MeridianCirculate => 44,
            Goal::MeridianCare => 45,
            Goal::SeekWoundCare => 46,
            Goal::GatherForCare => 47,
            Goal::SellForCare => 48,
            Goal::SectDuty => 49,
            Goal::GatherFrontier => 50,
            Goal::StockFrontierDepot => 51,
            Goal::CollectFrontierFreight => 52,
            Goal::DeliverFrontierFreight => 53,
            Goal::SurveyWillowford => 54,
            Goal::SurveyPinefordDepot => 55,
            Goal::CarryMessage(_) => 56,
            Goal::VerifyRumor(_) => 57,
        });
        match goal {
            Goal::Treat(patient) => self.actor(patient),
            Goal::Follow(site) => self.position(site),
            Goal::Investigate(incident) => self.u32(incident.0),
            Goal::InvestigateBody(body) => self.actor(body),
            Goal::RecoverBody(body) => self.actor(body),
            Goal::ClaimLot(id) | Goal::DeliverLot(id) => self.u32(id),
            Goal::MartialPractice(teacher) => self.actor(teacher),
            Goal::Report {
                recipient,
                incident,
            } => {
                self.actor(recipient);
                self.u32(incident.0);
            }
            Goal::NegotiateContract(need)
            | Goal::ProcureContract(need)
            | Goal::FulfillContract(need) => self.need(need),
            Goal::AuditContract(need) => self.need(need),
            Goal::DeliverTestimony { recipient, event } => {
                self.actor(recipient);
                self.u32(event.0);
            }
            Goal::InvestigateDispute(id) | Goal::CarryMessage(id) => self.u32(id),
            Goal::VerifyRumor(site) => self.u8(site as u8),
            _ => {}
        }
    }

    fn outcome(&mut self, outcome: Option<Outcome>) {
        self.u8(match outcome {
            None => 0,
            Some(Outcome::Waited) => 1,
            Some(Outcome::Moved) => 2,
            Some(Outcome::Spoke(_)) => 3,
            Some(Outcome::Gathered) => 4,
            Some(Outcome::Delivered) => 5,
            Some(Outcome::Struck(_)) => 6,
            Some(Outcome::Guarded) => 7,
            Some(Outcome::Treated { .. }) => 8,
            Some(Outcome::Rejected(_)) => 9,
            Some(Outcome::Contribution { .. }) => 10,
            Some(Outcome::Practiced { .. }) => 11,
            Some(Outcome::BackgroundChosen(_)) => 12,
            Some(Outcome::Traded { .. }) => 13,
            Some(Outcome::Enrolled) => 14,
            Some(Outcome::Brewed) => 15,
            Some(Outcome::UsedRemedy { .. }) => 16,
            Some(Outcome::GatheredPair) => 17,
            Some(Outcome::DutyAccepted) => 18,
            Some(Outcome::DutyDelivered) => 19,
            Some(Outcome::DutyReported) => 20,
            Some(Outcome::FormLearned) => 21,
            Some(Outcome::QiCentered { .. }) => 22,
            Some(Outcome::FormStruck(_)) => 23,
            Some(Outcome::JoinedCompany(_)) => 24,
            Some(Outcome::Provisioned(_)) => 25,
            Some(Outcome::Parted(_)) => 26,
            Some(Outcome::LodgeAccord { .. }) => 27,
            Some(Outcome::HallSupported) => 28,
            Some(Outcome::LodgePledged) => 29,
            Some(Outcome::LodgeAided) => 30,
            Some(Outcome::LodgeRenounced) => 31,
            Some(Outcome::Learned(_)) => 32,
            Some(Outcome::Briefed { .. }) => 33,
            Some(Outcome::Testified { .. }) => 34,
            Some(Outcome::Took { .. }) => 35,
            Some(Outcome::BodyReported { .. }) => 36,
            Some(Outcome::LotClaimed(_)) => 37,
            Some(Outcome::LotSold { .. }) => 38,
            Some(Outcome::LotAttuned(_)) => 39,
            Some(Outcome::ChannelStruck(_)) => 40,
            Some(Outcome::MartialDrilled { .. }) => 41,
            Some(Outcome::TechniqueStruck { .. }) => 42,
            Some(Outcome::QiDrawn(_)) => 43,
            Some(Outcome::QiCirculated { .. }) => 44,
            Some(Outcome::MeridiansMended) => 45,
            Some(Outcome::WoundTreated) => 46,
            Some(Outcome::HallAllocationClaimed) => 47,
            Some(Outcome::CacheRecovered { .. }) => 48,
            Some(Outcome::CacheEntrusted { .. }) => 49,
            Some(Outcome::RequestAccepted(_)) => 50,
            Some(Outcome::RequestDeclined(_)) => 51,
            Some(Outcome::RequestAbandoned(_)) => 52,
            Some(Outcome::FavorAccepted(_)) => 53,
            Some(Outcome::FavorDeclined(_)) => 54,
            Some(Outcome::FavorWithdrawn(_)) => 55,
            Some(Outcome::FavorFunded { .. }) => 56,
            Some(Outcome::ReciprocalAid { .. }) => 57,
            Some(Outcome::AidRepaid(_)) => 58,
            Some(Outcome::AidWaived(_)) => 59,
        });
        match outcome {
            Some(
                Outcome::Spoke(actor)
                | Outcome::Struck(actor)
                | Outcome::FormStruck(actor)
                | Outcome::JoinedCompany(actor)
                | Outcome::Provisioned(actor)
                | Outcome::Parted(actor),
            ) => self.actor(actor),
            Some(Outcome::Treated { by, healed }) => {
                self.actor(by);
                self.u16(healed);
            }
            Some(Outcome::Contribution { witnesses }) => self.u8(witnesses),
            Some(Outcome::Practiced { progress }) => self.u8(progress),
            Some(Outcome::BackgroundChosen(background)) => self.u8(background_code(background)),
            Some(Outcome::Traded { bought, coins }) => {
                self.u8(u8::from(bought));
                self.u32(coins);
            }
            Some(Outcome::UsedRemedy { healed }) => self.u16(healed),
            Some(Outcome::QiCentered { total }) => self.u8(total),
            Some(Outcome::LodgeAccord { until }) => self.u64(until),
            Some(Outcome::Learned(event)) => self.u32(event.0),
            Some(Outcome::Briefed { from, event })
            | Some(Outcome::Testified { to: from, event }) => {
                self.actor(from);
                self.u32(event.0);
            }
            Some(Outcome::Took { from, herbs }) => {
                self.actor(from);
                self.u32(herbs);
            }
            Some(Outcome::BodyReported { to, body }) => {
                self.actor(to);
                self.actor(body);
            }
            Some(Outcome::LotClaimed(id)) => self.u32(id),
            Some(Outcome::LotSold { to, id, coins }) => {
                self.actor(to);
                self.u32(id);
                self.u32(coins);
            }
            Some(Outcome::LotAttuned(id)) => self.u32(id),
            Some(Outcome::ChannelStruck(id)) => self.actor(id),
            Some(Outcome::MartialDrilled { teacher, learned }) => {
                self.actor(teacher);
                self.u8(u8::from(learned));
            }
            Some(Outcome::TechniqueStruck { target, form }) => {
                self.actor(target);
                self.u8(match form {
                    Form::ReedPalm => 1,
                    Form::StoneFist => 2,
                });
            }
            Some(Outcome::QiDrawn(qi)) => self.u8(qi),
            Some(Outcome::QiCirculated { channels, blockage }) => {
                self.u8(channels);
                self.u8(blockage);
            }
            Some(Outcome::CacheRecovered { id, herbs }) => {
                self.u32(id);
                self.u32(herbs);
            }
            Some(Outcome::CacheEntrusted { id, to, herbs }) => {
                self.u32(id);
                self.actor(to);
                self.u32(herbs);
            }
            Some(
                Outcome::RequestAccepted(id)
                | Outcome::RequestDeclined(id)
                | Outcome::RequestAbandoned(id),
            ) => self.u32(id),
            Some(
                Outcome::FavorAccepted(id)
                | Outcome::FavorDeclined(id)
                | Outcome::FavorWithdrawn(id),
            ) => self.u32(id),
            Some(Outcome::FavorFunded { id, actor, coins }) => {
                self.u32(id);
                self.actor(actor);
                self.u32(coins);
            }
            Some(Outcome::ReciprocalAid { actor, asset }) => {
                self.actor(actor);
                self.u8(asset as u8);
            }
            Some(Outcome::AidRepaid(actor) | Outcome::AidWaived(actor)) => self.actor(actor),
            Some(Outcome::Rejected(reason)) => self.u8(match reason {
                Rejection::Dead => 0,
                Rejection::Impassable => 1,
                Rejection::Occupied => 2,
                Rejection::UnknownActor => 3,
                Rejection::OutOfReach => 4,
                Rejection::Refused => 5,
                Rejection::HealerAway => 6,
                Rejection::Empty => 7,
                Rejection::NoCargo => 8,
                Rejection::PracticeComplete => 9,
                Rejection::BackgroundLocked => 10,
                Rejection::MarketClosed => 11,
                Rejection::NotEnoughCoins => 12,
                Rejection::HallClosed => 13,
                Rejection::AlreadyEnrolled => 14,
                Rejection::PracticeRequired => 15,
                Rejection::MembershipRequired => 16,
                Rejection::NoRemedy => 17,
                Rejection::Uninjured => 18,
                Rejection::DutyComplete => 19,
                Rejection::RankRequired => 20,
                Rejection::FormAlreadyKnown => 21,
                Rejection::FormRequired => 22,
                Rejection::QiFull => 23,
                Rejection::InsufficientQi => 24,
                Rejection::IntroductionRequired => 25,
                Rejection::BoundToDuty => 26,
                Rejection::CompanyFull => 27,
                Rejection::NotCompanion => 28,
                Rejection::PackFull => 29,
                Rejection::SchoolContested => 30,
                Rejection::OathConflict => 31,
                Rejection::FactionTrustRequired => 32,
                Rejection::NoEvidence => 33,
                Rejection::Overtrained => 34,
                Rejection::MeridiansBlocked => 35,
                Rejection::Hobbled => 36,
            }),
            _ => {}
        }
    }
}

fn background_code(background: Background) -> u8 {
    match background {
        Background::Wanderer => 0,
        Background::Herbalist => 1,
        Background::Peddler => 2,
    }
}

fn pack_testimony(actor: ActorId, event: super::EventId) -> u32 {
    (actor.get() << 24) | (event.0 & 0x00FF_FFFF)
}

fn unpack_testimony(payload: u32) -> (ActorId, super::EventId) {
    (
        ActorId::new(payload >> 24),
        super::EventId(payload & 0x00FF_FFFF),
    )
}
