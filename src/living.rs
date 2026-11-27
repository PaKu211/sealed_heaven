//! Deterministic settlement simulation. The authored scenario supplies locations and
//! actor capabilities; a turn has no renderer, wall clock, or external dependency.

mod character_agency;
mod checkpoint;
pub mod circles;
pub mod communication;
mod company;
#[cfg(test)]
mod consequences_tests;
pub mod content;
mod contested;
mod contract_memory;
#[cfg(test)]
mod contract_memory_tests;
mod contracts;
#[cfg(test)]
mod contracts_tests;
pub mod cooperation;
pub mod development;
pub mod diplomacy;
pub mod encounters;
pub mod expansion;
mod faction_agency;
#[cfg(test)]
mod faction_agency_tests;
pub mod favors;
mod information;
#[cfg(test)]
mod information_tests;
mod injury;
pub mod lod;
mod martial;
mod meridian;
pub mod narrative;
mod navigation;
mod npc;
pub mod obligations;
#[cfg(test)]
mod player_agency_tests;
mod politics;
pub mod production;
mod qinghe;
pub mod reciprocity;
mod recovery;
mod region;
pub mod regional_news;
mod relief;
#[cfg(test)]
mod relief_tests;
pub mod requests;
mod rivalry;
pub mod road_history;
pub mod rumors;
mod school;
mod sect_hierarchy;
mod social;
pub mod social_network;
#[cfg(test)]
mod tests;
mod traffic;
pub mod worldgen;

use std::collections::{BTreeMap, BTreeSet};

pub use crate::ActorId;
pub use crate::player5::{Direction, Position};
pub use character_agency::{
    AgencyEvent, AgencyEventKind, Ambition, CLINIC_FEE, CharacterAgency, CharacterState,
    DecisionReason,
};
pub use checkpoint::{LoadError, MAX_SAVE_BYTES};
pub use company::{Company, CompanyEvent, Invitation, Journey, JourneyEvent};
pub use contested::{Contested, Good, Holder, Lot, LotEvent, LotEventKind};
pub use contract_memory::{AuditEvent, AuditFinding, ContractMemory, ContractReceipt};
pub use contracts::{
    Bid, Contract, ContractEvent, ContractEventKind, ContractPhase, Contracts, Need,
};
pub use faction_agency::{
    FactionAgency, FactionCase, FactionEvent, FactionEventKind, FactionPhase,
};
pub use information::{
    Dispute, DisputePhase, EventId, InfoEvent, InfoKnowledge, InfoLogEvent, InfoLogKind,
    InfoReport, Information, Provenance,
};
pub use injury::{Injuries, WoundEvent, WoundKind, Wounds};
pub use martial::{Form, Lesson, Martial, MartialSkill};
pub use meridian::{MeridianEvent, MeridianEventKind, MeridianState, Meridians};
pub use npc::{Activity, Goal, Npc, NpcDefinition, Role};
pub use politics::{Allegiance, PoliticalEvent, PoliticalEventKind, Politics};
pub use region::{Background, MarketQuote, Region, RegionSites};
pub use relief::{Relief, ReliefCase, ReliefEvent, ReliefEventKind, ReliefPhase};
pub use rivalry::{Rivalry, SectEvent, SectEventKind};
pub use school::{Duty, FORM_QI_COST, HallRank, MAX_QI, School, Succession};
pub use sect_hierarchy::{HierarchyEvent, HierarchyEventKind, SectDuty, SectHierarchy, SectMember};
pub use social::{
    BodyDiscovery, BodyReport, Incident, IncidentId, Knowledge, KnowledgeSource, RecoveryEvent,
    Relationship, Report, Social,
};
pub use traffic::{Traffic, TrafficEvent, TrafficEventKind};

pub const MAX_TICKS: u64 = 65_536;
pub const PLAYER: ActorId = ActorId::new(21);
const PLAYER_HP: u16 = 6;
const STRIKE: u16 = 2;
const GUARD_BLOCK: u16 = 1;
const SIGHT_RADIUS: u32 = 5;
const TURNS_PER_MINUTE: u64 = 2;
const MINUTES_PER_DAY: u64 = 1_440;
const DAWN_MINUTE: u64 = 360;
const DUSK_MINUTE: u64 = 1_080;
pub const BREATH_MASTERY: u8 = 3;
const DIRECTIONS: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Intent {
    Wait,
    Move(Direction),
    Talk(ActorId),
    Gather,
    Deliver,
    Attack(ActorId),
    Guard,
    Practice,
    ChooseBackground(Background),
    BuyHerb,
    SellHerb,
    JoinHall,
    Brew,
    UseRemedy,
    HallDuty,
    StudyForm,
    CenterQi,
    ReedStrike(ActorId),
    Invite(ActorId),
    Provision(ActorId),
    Part(ActorId),
    OfferAccord,
    SupportHall,
    PledgeLodge,
    AidLodge,
    RenounceLodge,
    /// Ask a nearby actor for one institutional finding they know.
    Ask(ActorId),
    /// Inspect the player's current tile for evidence that is actually there.
    Examine,
    /// Tell one known finding to one actor. The payload packs both identities.
    Testify {
        to: ActorId,
        event: EventId,
    },
    /// Take the herbs a dead carrier is no longer able to deliver.
    Take(ActorId),
    /// Tell a nearby person about a corpse the player actually discovered.
    ReportBody {
        to: ActorId,
        body: ActorId,
    },
    ClaimLot(u32),
    OfferLot {
        to: ActorId,
        id: u32,
    },
    AttuneLot(u32),
    ChannelStrike(ActorId),
    Drill(ActorId),
    TechniqueStrike(ActorId),
    DrawQi,
    CirculateQi,
    MendMeridians,
    TreatWounds,
    ClaimHallAllocation,
    RecoverCache(u32),
    EntrustCache(u32),
    AcceptRequest(u32),
    DeclineRequest(u32),
    AbandonRequest(u32),
    AcceptFavor(u32),
    DeclineFavor(u32),
    WithdrawFavor(u32),
    FundFavor(u32),
    RequestAid(ActorId),
    RepayAid(ActorId),
    WaiveAid(ActorId),
}

impl Intent {
    fn regional(self) -> bool {
        matches!(
            self,
            Self::ChooseBackground(_)
                | Self::BuyHerb
                | Self::SellHerb
                | Self::JoinHall
                | Self::Brew
                | Self::UseRemedy
        )
    }

    fn school(self) -> bool {
        matches!(
            self,
            Self::HallDuty | Self::StudyForm | Self::CenterQi | Self::ReedStrike(_)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    Dead,
    Impassable,
    Occupied,
    UnknownActor,
    OutOfReach,
    Refused,
    HealerAway,
    Empty,
    NoCargo,
    PracticeComplete,
    BackgroundLocked,
    MarketClosed,
    NotEnoughCoins,
    HallClosed,
    AlreadyEnrolled,
    PracticeRequired,
    MembershipRequired,
    NoRemedy,
    Uninjured,
    DutyComplete,
    RankRequired,
    FormAlreadyKnown,
    FormRequired,
    QiFull,
    InsufficientQi,
    IntroductionRequired,
    BoundToDuty,
    CompanyFull,
    NotCompanion,
    PackFull,
    SchoolContested,
    OathConflict,
    FactionTrustRequired,
    /// The tile has no institutional finding to examine.
    NoEvidence,
    Overtrained,
    MeridiansBlocked,
    Hobbled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Waited,
    Moved,
    Spoke(ActorId),
    Gathered,
    Delivered,
    Contribution {
        witnesses: u8,
    },
    Practiced {
        progress: u8,
    },
    BackgroundChosen(Background),
    Traded {
        bought: bool,
        coins: u32,
    },
    Enrolled,
    Brewed,
    UsedRemedy {
        healed: u16,
    },
    GatheredPair,
    DutyAccepted,
    DutyDelivered,
    DutyReported,
    FormLearned,
    QiCentered {
        total: u8,
    },
    FormStruck(ActorId),
    JoinedCompany(ActorId),
    Provisioned(ActorId),
    Parted(ActorId),
    LodgeAccord {
        until: u64,
    },
    HallSupported,
    LodgePledged,
    LodgeAided,
    LodgeRenounced,
    Struck(ActorId),
    Guarded,
    Treated {
        by: ActorId,
        healed: u16,
    },
    Learned(EventId),
    Briefed {
        from: ActorId,
        event: EventId,
    },
    Testified {
        to: ActorId,
        event: EventId,
    },
    /// Herbs moved from a corpse onto the player. Nothing was created.
    Took {
        from: ActorId,
        herbs: u32,
    },
    BodyReported {
        to: ActorId,
        body: ActorId,
    },
    LotClaimed(u32),
    LotSold {
        to: ActorId,
        id: u32,
        coins: u32,
    },
    LotAttuned(u32),
    ChannelStruck(ActorId),
    MartialDrilled {
        teacher: ActorId,
        learned: bool,
    },
    TechniqueStruck {
        target: ActorId,
        form: Form,
    },
    QiDrawn(u8),
    QiCirculated {
        channels: u8,
        blockage: u8,
    },
    MeridiansMended,
    WoundTreated,
    HallAllocationClaimed,
    CacheRecovered {
        id: u32,
        herbs: u32,
    },
    CacheEntrusted {
        id: u32,
        to: ActorId,
        herbs: u32,
    },
    RequestAccepted(u32),
    RequestDeclined(u32),
    RequestAbandoned(u32),
    FavorAccepted(u32),
    FavorDeclined(u32),
    FavorWithdrawn(u32),
    FavorFunded {
        id: u32,
        actor: ActorId,
        coins: u32,
    },
    ReciprocalAid {
        actor: ActorId,
        asset: reciprocity::Asset,
    },
    AidRepaid(ActorId),
    AidWaived(ActorId),
    Rejected(Rejection),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepError {
    WrongBoundary { expected: u64, received: u64 },
    HorizonReached,
    UnavailableInOriginalRules,
    RegionalCampaignRequired,
    SchoolCampaignRequired,
    CompanyCampaignRequired,
    RivalryCampaignRequired,
    PoliticsCampaignRequired,
    InvalidBodyReport,
    ContestedCampaignRequired,
    UnrecordableIntent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Player {
    pub position: Position,
    pub hp: u16,
    pub herbs: u32,
}

impl Player {
    pub fn alive(self) -> bool {
        self.hp > 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Supply {
    pub source: u32,
    pub shrine: u32,
    pub npc_deliveries: u32,
    pub player_deliveries: u32,
    pub consumed: u32,
    pub regrown: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scenario {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub(crate) terrain: &'static [&'static str],
    pub(crate) entrance: Position,
    pub patch: Position,
    pub shrine: Position,
    pub clinic: Position,
    pub shrine_target: u32,
    pub(crate) initial_source: u32,
    pub(crate) regrow_every: u64,
    pub(crate) npcs: &'static [NpcDefinition],
    pub(crate) contacts: &'static [(ActorId, ActorId)],
    pub(crate) recruits: &'static [(ActorId, Invitation)],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    scenario: &'static Scenario,
    generated: Option<std::sync::Arc<worldgen::GeneratedRegion>>,
    expanded: Option<std::sync::Arc<expansion::ExpandedRegion>>,
    frontier: Option<expansion::Frontier>,
    regional_news: Option<regional_news::RegionalNews>,
    scheduler: Option<lod::Scheduler>,
    production: Option<production::Production>,
    road_history: Option<road_history::RoadHistory>,
    requests: Option<requests::Requests>,
    favors: Option<favors::Favors>,
    reciprocity: Option<reciprocity::Reciprocity>,
    encounters: Option<encounters::Encounters>,
    social_network: Option<social_network::SocialNetwork>,
    cooperation: Option<cooperation::Cooperation>,
    obligations: Option<obligations::Obligations>,
    communication: Option<communication::Communication>,
    rumors: Option<rumors::Rumors>,
    narrative: Option<narrative::Narrative>,
    development: Option<development::Development>,
    circles: Option<circles::Circles>,
    diplomacy: Option<diplomacy::Diplomacy>,
    tick: u64,
    player: Player,
    npcs: BTreeMap<ActorId, Npc>,
    supply: Supply,
    social: Social,
    journal: Vec<Intent>,
    latest: Option<Outcome>,
    rules: Rules,
    breath: u8,
    region: Option<Region>,
    school: Option<School>,
    company: Option<Company>,
    rivalry: Option<Rivalry>,
    politics: Option<Politics>,
    traffic: Option<Traffic>,
    relief: Option<Relief>,
    faction_agency: Option<FactionAgency>,
    contracts: Option<Contracts>,
    contract_memory: Option<ContractMemory>,
    information: Option<Information>,
    contested: Option<Contested>,
    martial: Option<Martial>,
    meridians: Option<Meridians>,
    injuries: Option<Injuries>,
    character_agency: Option<CharacterAgency>,
    sect_hierarchy: Option<SectHierarchy>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rules {
    Original,
    Qinghe,
    Riverlands,
    ReedHall,
    RoadCompany,
    SectRivalry,
    SectPolitics,
    TrafficCourtesy,
    LivingAgency,
    FactionAgency,
    InstitutionalAgency,
    ContractAccountability,
    InformationDisputes,
    PlayerAgency,
    Consequences,
    ViolenceLogistics,
    PlayerReporting,
    PhysicalRecovery,
    ContestedResources,
    SectCultivation,
    MartialMentorship,
    MeridianCultivation,
    PersistentInjury,
    AutonomousAgency,
    SectHierarchy,
    DutyCoherence,
    GeneratedRiverlands,
    ExpandedRiverlands,
    RegionalKnowledge,
    ExactLod,
    SettlementProduction,
    RoadHistory,
    EmergentRequests,
    PersonalFavors,
    ReciprocalBonds,
    EncounterMemory,
    SocialNetworks,
    CollectiveAgency,
    DynamicObligations,
    SituatedCommunication,
    InformationNetworks,
    CausalNarratives,
    PersonalDevelopment,
    VoluntaryCircles,
    SituatedDiplomacy,
}

impl Rules {
    fn has_duty_coherence(self) -> bool {
        matches!(
            self,
            Self::DutyCoherence
                | Self::GeneratedRiverlands
                | Self::ExpandedRiverlands
                | Self::RegionalKnowledge
                | Self::ExactLod
                | Self::SettlementProduction
                | Self::RoadHistory
                | Self::EmergentRequests
                | Self::PersonalFavors
                | Self::ReciprocalBonds
                | Self::EncounterMemory
                | Self::SocialNetworks
                | Self::CollectiveAgency
                | Self::DynamicObligations
                | Self::SituatedCommunication
                | Self::InformationNetworks
                | Self::CausalNarratives
                | Self::PersonalDevelopment
                | Self::VoluntaryCircles
                | Self::SituatedDiplomacy
        )
    }

    fn has_region(self) -> bool {
        self == Self::Riverlands || self.has_school()
    }

    fn has_school(self) -> bool {
        matches!(
            self,
            Self::ReedHall
                | Self::RoadCompany
                | Self::SectRivalry
                | Self::SectPolitics
                | Self::TrafficCourtesy
                | Self::LivingAgency
                | Self::FactionAgency
                | Self::InstitutionalAgency
                | Self::ContractAccountability
                | Self::InformationDisputes
                | Self::PlayerAgency
                | Self::Consequences
                | Self::ViolenceLogistics
                | Self::PlayerReporting
                | Self::PhysicalRecovery
                | Self::ContestedResources
                | Self::SectCultivation
                | Self::MartialMentorship
                | Self::MeridianCultivation
                | Self::PersistentInjury
                | Self::AutonomousAgency
                | Self::SectHierarchy
                | Self::DutyCoherence
                | Self::GeneratedRiverlands
                | Self::ExpandedRiverlands
                | Self::RegionalKnowledge
                | Self::ExactLod
                | Self::SettlementProduction
                | Self::RoadHistory
                | Self::EmergentRequests
                | Self::PersonalFavors
                | Self::ReciprocalBonds
                | Self::EncounterMemory
                | Self::SocialNetworks
                | Self::CollectiveAgency
                | Self::DynamicObligations
                | Self::SituatedCommunication
                | Self::InformationNetworks
                | Self::CausalNarratives
                | Self::PersonalDevelopment
                | Self::VoluntaryCircles
                | Self::SituatedDiplomacy
        )
    }
}

impl World {
    pub fn qinghe() -> Self {
        Self::new(&qinghe::QINGHE)
    }

    pub fn riverlands(seed: u32) -> Self {
        Self::seeded(seed, Rules::Riverlands)
    }

    pub fn reed_hall(seed: u32) -> Self {
        Self::seeded(seed, Rules::ReedHall)
    }

    pub fn road_company(seed: u32) -> Self {
        Self::seeded(seed, Rules::RoadCompany)
    }

    pub fn sect_rivalry(seed: u32) -> Self {
        Self::seeded(seed, Rules::SectRivalry)
    }

    pub fn sect_politics(seed: u32) -> Self {
        Self::seeded(seed, Rules::SectPolitics)
    }

    pub fn traffic_courtesy(seed: u32) -> Self {
        Self::seeded(seed, Rules::TrafficCourtesy)
    }

    pub fn living_agency(seed: u32) -> Self {
        Self::seeded(seed, Rules::LivingAgency)
    }

    pub fn faction_agency(seed: u32) -> Self {
        Self::seeded(seed, Rules::FactionAgency)
    }

    pub fn institutional_agency(seed: u32) -> Self {
        Self::seeded(seed, Rules::InstitutionalAgency)
    }

    pub fn contract_accountability(seed: u32) -> Self {
        Self::seeded(seed, Rules::ContractAccountability)
    }

    pub fn information_disputes(seed: u32) -> Self {
        Self::seeded(seed, Rules::InformationDisputes)
    }

    pub fn player_agency(seed: u32) -> Self {
        Self::seeded(seed, Rules::PlayerAgency)
    }

    pub fn consequences(seed: u32) -> Self {
        Self::seeded(seed, Rules::Consequences)
    }

    pub fn violence_logistics(seed: u32) -> Self {
        Self::seeded(seed, Rules::ViolenceLogistics)
    }

    pub fn player_reporting(seed: u32) -> Self {
        Self::seeded(seed, Rules::PlayerReporting)
    }

    pub fn physical_recovery(seed: u32) -> Self {
        Self::seeded(seed, Rules::PhysicalRecovery)
    }

    pub fn contested_resources(seed: u32) -> Self {
        Self::seeded(seed, Rules::ContestedResources)
    }

    pub fn sect_cultivation(seed: u32) -> Self {
        Self::seeded(seed, Rules::SectCultivation)
    }

    pub fn martial_mentorship(seed: u32) -> Self {
        Self::seeded(seed, Rules::MartialMentorship)
    }

    pub fn meridian_cultivation(seed: u32) -> Self {
        Self::seeded(seed, Rules::MeridianCultivation)
    }

    pub fn persistent_injury(seed: u32) -> Self {
        Self::seeded(seed, Rules::PersistentInjury)
    }

    pub fn autonomous_agency(seed: u32) -> Self {
        Self::seeded(seed, Rules::AutonomousAgency)
    }

    pub fn sect_hierarchy_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::SectHierarchy)
    }

    /// C3.5.1: a new rules revision, not a silent change to C3.5 replay.
    pub fn duty_coherence_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::DutyCoherence)
    }

    /// C4.0: generated regional geography, with the existing living society.
    pub fn generated_riverlands(seed: u32) -> Self {
        Self::seeded(seed, Rules::GeneratedRiverlands)
    }

    /// C4.1: continuous eastern expansion with a third settlement.
    pub fn expanded_riverlands(seed: u32) -> Self {
        Self::seeded(seed, Rules::ExpandedRiverlands)
    }

    /// C4.2: situated regional knowledge and causal event memory.
    pub fn regional_knowledge_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::RegionalKnowledge)
    }

    /// C4.3: exact scheduled named actors and indexed navigation.
    pub fn exact_lod_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::ExactLod)
    }

    /// C4.4: deterministic settlement site production and order scheduling.
    pub fn settlement_production_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::SettlementProduction)
    }

    /// C4.5: seed-bound prehistory and physical relief caches.
    pub fn road_history_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::RoadHistory)
    }

    /// C4.6: material needs become optional player commitments.
    pub fn emergent_requests_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::EmergentRequests)
    }

    /// C4.7: personal, optional promises with physical aid and local trust.
    pub fn personal_favors_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::PersonalFavors)
    }

    /// C4.8: voluntary bilateral gratitude and material reciprocity.
    pub fn reciprocal_bonds_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::ReciprocalBonds)
    }

    /// C4.9: situated conversation memory and distinct encounters.
    pub fn encounter_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::EncounterMemory)
    }

    /// C5.0: local autonomous greetings and causal NPC social memory.
    pub fn social_network_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::SocialNetworks)
    }

    /// C5.1: voluntary local freight teams and material project outcomes.
    pub fn cooperation_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::CollectiveAgency)
    }

    /// C5.2: situated acknowledgement of material commitments.
    pub fn obligations_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::DynamicObligations)
    }

    /// C5.3: physical delivery of situated commitment reports.
    pub fn communication_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::SituatedCommunication)
    }

    /// C5.4: dated competing accounts and situated verification.
    pub fn rumor_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::InformationNetworks)
    }

    /// C5.5: canonical causal episodes and situated narrative memory.
    pub fn narrative_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::CausalNarratives)
    }

    /// C5.6: evidence-based personal development and adaptive pursuits.
    pub fn development_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::PersonalDevelopment)
    }

    /// C5.7: voluntary situated associations.
    pub fn circles_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::VoluntaryCircles)
    }

    /// C5.8: locally negotiated intergroup and institutional liaisons.
    pub fn diplomacy_campaign(seed: u32) -> Self {
        Self::seeded(seed, Rules::SituatedDiplomacy)
    }

    fn seeded(seed: u32, rules: Rules) -> Self {
        let mut world = Self::with_rules(qinghe::region(seed, rules), rules);
        if matches!(
            rules,
            Rules::GeneratedRiverlands
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
            let base = worldgen::GeneratedRegion::generate(seed);
            if matches!(
                rules,
                Rules::ExpandedRiverlands
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
                world.expanded = Some(std::sync::Arc::new(expansion::ExpandedRegion::from_base(
                    seed, &base,
                )));
                world.frontier = Some(expansion::Frontier::new());
            }
            world.generated = Some(std::sync::Arc::new(base));
        }
        assert!(
            Region::sites()
                .positions()
                .into_iter()
                .all(|site| world.walkable(site))
        );
        world.region = Some(Region::new(seed));
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
        ) {
            world.road_history = Some(road_history::RoadHistory::new(&mut world, seed));
            world.observe_road_history();
        }
        if matches!(
            rules,
            Rules::RegionalKnowledge
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
            world.init_regional_news();
        }
        if matches!(
            rules,
            Rules::ExactLod
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
            world.scheduler = Some(lod::Scheduler::new(&world));
        }
        if matches!(
            rules,
            Rules::SettlementProduction
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
            world.production = Some(production::Production::new(&world));
        }
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
        ) {
            world.requests = Some(requests::Requests::default());
            world.advance_requests();
        }
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
        ) {
            world.favors = Some(favors::Favors::default());
        }
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
        ) {
            world.reciprocity = Some(reciprocity::Reciprocity::default());
        }
        if matches!(
            rules,
            Rules::EncounterMemory
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
            world.encounters = Some(encounters::Encounters::default());
        }
        if matches!(
            rules,
            Rules::SocialNetworks
                | Rules::CollectiveAgency
                | Rules::DynamicObligations
                | Rules::SituatedCommunication
                | Rules::InformationNetworks
                | Rules::CausalNarratives
                | Rules::PersonalDevelopment
                | Rules::VoluntaryCircles
                | Rules::SituatedDiplomacy
        ) {
            world.social_network = Some(social_network::SocialNetwork::default());
        }
        if matches!(
            rules,
            Rules::CollectiveAgency
                | Rules::DynamicObligations
                | Rules::SituatedCommunication
                | Rules::InformationNetworks
                | Rules::CausalNarratives
                | Rules::PersonalDevelopment
                | Rules::VoluntaryCircles
                | Rules::SituatedDiplomacy
        ) {
            world.cooperation = Some(cooperation::Cooperation::default());
        }
        if matches!(
            rules,
            Rules::DynamicObligations
                | Rules::SituatedCommunication
                | Rules::InformationNetworks
                | Rules::CausalNarratives
                | Rules::PersonalDevelopment
                | Rules::VoluntaryCircles
                | Rules::SituatedDiplomacy
        ) {
            world.obligations = Some(obligations::Obligations::default());
        }
        if matches!(
            rules,
            Rules::SituatedCommunication
                | Rules::InformationNetworks
                | Rules::CausalNarratives
                | Rules::PersonalDevelopment
                | Rules::VoluntaryCircles
                | Rules::SituatedDiplomacy
        ) {
            world.communication = Some(communication::Communication::default());
        }
        if matches!(
            rules,
            Rules::InformationNetworks
                | Rules::CausalNarratives
                | Rules::PersonalDevelopment
                | Rules::VoluntaryCircles
                | Rules::SituatedDiplomacy
        ) {
            world.rumors = Some(rumors::Rumors::default());
            world.observe_regional(0);
        }
        if matches!(
            rules,
            Rules::CausalNarratives
                | Rules::PersonalDevelopment
                | Rules::VoluntaryCircles
                | Rules::SituatedDiplomacy
        ) {
            world.narrative = Some(narrative::Narrative::default());
        }
        if matches!(
            rules,
            Rules::PersonalDevelopment | Rules::VoluntaryCircles | Rules::SituatedDiplomacy
        ) {
            world.development = Some(development::Development::new(world.npcs.keys().copied()));
        }
        if matches!(rules, Rules::VoluntaryCircles | Rules::SituatedDiplomacy) {
            world.circles = Some(circles::Circles::default());
        }
        if rules == Rules::SituatedDiplomacy {
            world.diplomacy = Some(diplomacy::Diplomacy::default());
        }
        world
    }

    fn new(scenario: &'static Scenario) -> Self {
        Self::with_rules(scenario, Rules::Qinghe)
    }

    fn with_rules(scenario: &'static Scenario, rules: Rules) -> Self {
        assert!(
            scenario.valid(),
            "invalid authored settlement: {}",
            scenario.name
        );
        Self {
            scenario,
            generated: None,
            expanded: None,
            frontier: None,
            regional_news: None,
            scheduler: None,
            production: None,
            road_history: None,
            requests: None,
            favors: None,
            reciprocity: None,
            encounters: None,
            social_network: None,
            cooperation: None,
            obligations: None,
            communication: None,
            rumors: None,
            narrative: None,
            development: None,
            circles: None,
            diplomacy: None,
            tick: 0,
            player: Player {
                position: scenario.entrance,
                hp: PLAYER_HP,
                herbs: 0,
            },
            npcs: scenario
                .npcs
                .iter()
                .map(|definition| (definition.id, Npc::new(definition)))
                .collect(),
            supply: Supply {
                source: scenario.initial_source,
                shrine: 0,
                npc_deliveries: 0,
                player_deliveries: 0,
                consumed: 0,
                regrown: 0,
            },
            social: {
                let mut social = Social::new(scenario.contacts);
                if matches!(
                    rules,
                    Rules::InformationDisputes
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
                    // Watch, Mentor and Hall steward can brief each other in person.
                    social.link(ActorId::new(2), ActorId::new(33));
                    social.link(ActorId::new(2), ActorId::new(36));
                }
                social
            },
            journal: Vec::new(),
            latest: None,
            rules,
            breath: 0,
            region: None,
            school: rules.has_school().then(|| School::new(scenario)),
            company: matches!(
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
            )
            .then(Company::default),
            rivalry: matches!(
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
            )
            .then(Rivalry::new),
            politics: matches!(
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
            )
            .then(Politics::new),
            traffic: matches!(
                rules,
                Rules::TrafficCourtesy
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
            )
            .then(Traffic::new),
            relief: matches!(rules, Rules::LivingAgency | Rules::FactionAgency).then(Relief::new),
            faction_agency: (rules == Rules::FactionAgency).then(FactionAgency::new),
            contracts: matches!(
                rules,
                Rules::InstitutionalAgency
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
            )
            .then(Contracts::new),
            contract_memory: matches!(
                rules,
                Rules::ContractAccountability
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
            )
            .then(ContractMemory::default),
            information: matches!(
                rules,
                Rules::InformationDisputes
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
            )
            .then(Information::default),
            contested: matches!(
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
            )
            .then(|| {
                Contested::new(matches!(
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
                ))
            }),
            martial: matches!(
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
            )
            .then(|| Martial::new(scenario)),
            meridians: matches!(
                rules,
                Rules::MeridianCultivation
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
            )
            .then(|| Meridians::new(scenario)),
            injuries: matches!(
                rules,
                Rules::PersistentInjury
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
            )
            .then(|| Injuries::new(scenario)),
            character_agency: matches!(
                rules,
                Rules::AutonomousAgency
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
            )
            .then(|| CharacterAgency::new(scenario)),
            sect_hierarchy: matches!(
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
            )
            .then(|| SectHierarchy::new(scenario)),
        }
    }

    pub fn scenario(&self) -> &'static Scenario {
        self.scenario
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// The current campaign opens at dawn. The same turn-derived clock drives
    /// both the settlement's routine and its presentation.
    pub fn calendar_tick(&self) -> u64 {
        self.tick
            + if self.original_rules() {
                0
            } else {
                DAWN_MINUTE * TURNS_PER_MINUTE
            }
    }

    pub fn is_night(&self) -> bool {
        let minute = self.calendar_tick() / TURNS_PER_MINUTE % MINUTES_PER_DAY;
        !(DAWN_MINUTE..DUSK_MINUTE).contains(&minute)
    }

    pub fn player(&self) -> Player {
        self.player
    }

    pub fn npcs(&self) -> impl ExactSizeIterator<Item = &Npc> {
        self.npcs.values()
    }

    pub fn npc(&self, id: ActorId) -> Option<&Npc> {
        self.npcs.get(&id)
    }

    pub fn supply(&self) -> Supply {
        self.supply
    }

    pub fn social(&self) -> &Social {
        &self.social
    }

    pub fn original_rules(&self) -> bool {
        self.rules == Rules::Original
    }

    pub fn breath_progress(&self) -> u8 {
        self.breath
    }

    pub fn region(&self) -> Option<&Region> {
        self.region.as_ref()
    }

    pub fn school(&self) -> Option<&School> {
        self.school.as_ref()
    }

    pub fn company(&self) -> Option<&Company> {
        self.company.as_ref()
    }

    pub fn hall_rank(&self) -> Option<HallRank> {
        if let Some(hierarchy) = self.sect_hierarchy() {
            return hierarchy.player.map(|member| member.rank);
        }
        self.region()?.enrolled.then(|| {
            if self
                .school()
                .is_some_and(|school| school.duty == Duty::Reported)
            {
                HallRank::Disciple
            } else {
                HallRank::Apprentice
            }
        })
    }

    pub fn latest_outcome(&self) -> Option<Outcome> {
        self.latest
    }

    pub fn generated_region(&self) -> Option<&worldgen::GeneratedRegion> {
        self.generated.as_deref()
    }

    pub fn walkable(&self, position: Position) -> bool {
        if let Some(map) = self.expanded_region() {
            map.walkable(position)
        } else {
            self.generated_region().map_or_else(
                || self.scenario.walkable(position),
                |map| map.walkable(position),
            )
        }
    }

    pub fn can_see(&self, from: Position, to: Position) -> bool {
        if let Some(map) = self.expanded_region() {
            navigation::can_see_with(from, to, |p| map.walkable(p))
        } else if let Some(map) = self.generated_region() {
            navigation::can_see_with(from, to, |p| map.walkable(p))
        } else {
            navigation::can_see(self.scenario, from, to)
        }
    }

    pub fn player_sees(&self, target: Position) -> bool {
        self.player.alive() && self.perceives(self.player.position, target)
    }

    fn perceives(&self, observer: Position, target: Position) -> bool {
        navigation::distance(observer, target) <= SIGHT_RADIUS && self.can_see(observer, target)
    }

    /// Boundary admission precedes mutation. Retrying a committed tick cannot apply
    /// another action, strike, delivery, or social effect. A wait also works after death.
    pub fn step(&mut self, boundary: u64, intent: Intent) -> Result<Outcome, StepError> {
        if boundary != self.tick {
            return Err(StepError::WrongBoundary {
                expected: self.tick,
                received: boundary,
            });
        }
        if self.tick >= MAX_TICKS {
            return Err(StepError::HorizonReached);
        }
        if self.original_rules() && intent == Intent::Practice {
            return Err(StepError::UnavailableInOriginalRules);
        }
        if intent.regional() && self.region.is_none() {
            return Err(StepError::RegionalCampaignRequired);
        }
        if intent.school() && self.school.is_none() {
            return Err(StepError::SchoolCampaignRequired);
        }

        if matches!(
            intent,
            Intent::Invite(_) | Intent::Provision(_) | Intent::Part(_)
        ) && self.company.is_none()
        {
            return Err(StepError::CompanyCampaignRequired);
        }

        if matches!(intent, Intent::OfferAccord | Intent::SupportHall) && self.rivalry.is_none() {
            return Err(StepError::RivalryCampaignRequired);
        }
        if matches!(
            intent,
            Intent::PledgeLodge | Intent::AidLodge | Intent::RenounceLodge
        ) && self.politics.is_none()
        {
            return Err(StepError::PoliticsCampaignRequired);
        }
        if let Intent::ReportBody { to, body } = intent
            && (!matches!(
                self.rules,
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
            ) || to.get() > 255
                || body.get() > 0x00ff_ffff)
        {
            return Err(StepError::InvalidBodyReport);
        }
        if matches!(intent, Intent::ClaimLot(_) | Intent::OfferLot { .. })
            && (self.contested.is_none()
                || matches!(intent, Intent::OfferLot { to, id } if to.get() > 255 || id > 0x00ff_ffff))
        {
            return Err(StepError::ContestedCampaignRequired);
        }
        if matches!(intent, Intent::AttuneLot(_) | Intent::ChannelStrike(_))
            && !matches!(
                self.rules,
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
            )
        {
            return Err(StepError::ContestedCampaignRequired);
        }
        if matches!(intent, Intent::Drill(_) | Intent::TechniqueStrike(_)) && self.martial.is_none()
        {
            return Err(StepError::ContestedCampaignRequired);
        }
        if matches!(
            intent,
            Intent::DrawQi | Intent::CirculateQi | Intent::MendMeridians
        ) && self.meridians.is_none()
        {
            return Err(StepError::ContestedCampaignRequired);
        }
        if intent == Intent::TreatWounds && self.injuries.is_none() {
            return Err(StepError::ContestedCampaignRequired);
        }
        if intent == Intent::ClaimHallAllocation && self.sect_hierarchy.is_none() {
            return Err(StepError::SchoolCampaignRequired);
        }

        // Reject intents that the current revision cannot replay exactly.
        if !checkpoint::intent_is_recordable(intent, self.rules) {
            return Err(StepError::UnrecordableIntent);
        }

        self.tick += 1;
        self.expire_inquiries();
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.begin_tick();
        }
        // 1. Arriving information is available to all decisions on this tick.
        self.deliver_reports();
        self.deliver_body_reports();
        self.deliver_info_reports();
        self.deliver_regional_reports();
        self.advance_information_recovery();
        self.advance_production();
        self.advance_requests();
        self.advance_favors();
        // 2. Apply the player's action and capture observation at the point of contact.
        if self.tick == 1 && !matches!(intent, Intent::ChooseBackground(_)) {
            self.select_default_background();
        }
        let outcome = self.apply_player(intent).unwrap_or_else(Outcome::Rejected);
        self.observe_road_history();
        self.advance_relief();
        self.advance_faction_agency();
        self.advance_contracts();
        self.update_succession();
        self.sync_hierarchy_office();
        self.update_journey();
        self.observe_regional(0);
        // 3. Decide from one snapshot; resolve movement, work and combat by actor ID.
        self.reconcile_cooperation();
        self.advance_obligations();
        self.advance_npcs(intent == Intent::Guard && outcome == Outcome::Guarded);
        self.retire_scheduled_dead();
        self.advance_reciprocity();
        self.reconcile_cooperation();
        self.observe_cooperation_receipts();
        self.advance_obligations();
        self.delegate_messages();
        self.advance_social_network();
        self.observe_road_history();
        self.advance_requests();
        self.advance_favors();
        if intent == Intent::Wait {
            self.recover_martial(PLAYER);
        }
        self.observe_lots();
        self.observe_bodies();
        self.queue_body_reports();
        // 4. Queue new conversations from the committed positions.
        self.observe_player_death();
        self.queue_reports();
        self.queue_info_reports();
        self.observe_regional(1);
        self.advance_circles();
        self.advance_diplomacy();
        self.advance_rumors();
        self.expire_inquiries();
        self.prune_circle_leads();
        self.queue_regional_reports();
        self.reconcile_character_agency_inventory();
        self.reconcile_sect_hierarchy_inventory();
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.finish_tick();
        }

        self.journal.push(intent);
        self.latest = Some(outcome);
        Ok(outcome)
    }

    fn apply_player(&mut self, intent: Intent) -> Result<Outcome, Rejection> {
        if intent == Intent::Wait {
            return Ok(Outcome::Waited);
        }
        if !self.player.alive() {
            return Err(Rejection::Dead);
        }
        match intent {
            Intent::Wait => unreachable!(),
            Intent::Move(direction) => {
                if self.hobbled(PLAYER) {
                    return Err(Rejection::Hobbled);
                }
                let destination = navigation::moved(self.player.position, direction)
                    .filter(|&position| self.walkable(position))
                    .ok_or(Rejection::Impassable)?;
                if self.traffic.is_some() {
                    self.move_player_with_courtesy(destination)?;
                } else {
                    if self.occupied(destination) {
                        return Err(Rejection::Occupied);
                    }
                    self.player.position = destination;
                }
                Ok(Outcome::Moved)
            }
            Intent::Talk(id) => self.talk(id),
            Intent::Gather => self.gather(),
            Intent::Deliver => {
                self.require_reach(self.scenario.shrine)?;
                if self.player.herbs == 0 {
                    return Err(Rejection::NoCargo);
                }
                let shortage = self.supply.shrine < self.scenario.shrine_target;
                self.player.herbs -= 1;
                self.supply.shrine += 1;
                self.supply.player_deliveries += 1;
                self.request_contribution(requests::Site::Shrine);
                self.record_regional_event(
                    regional_news::Site::QingheShrine,
                    regional_news::EventKind::ShrineContributed,
                    Some(PLAYER),
                    1,
                );
                if !self.original_rules() && shortage {
                    let witnesses: Vec<_> = self
                        .npcs
                        .values()
                        .filter(|npc| {
                            npc.alive()
                                && matches!(npc.definition.role, Role::Medic | Role::Supplier)
                        })
                        .filter(|npc| {
                            self.perceives(npc.position, self.scenario.shrine)
                                && self.perceives(npc.position, self.player.position)
                        })
                        .map(Npc::id)
                        .collect();
                    for &witness in &witnesses {
                        self.social.aided(PLAYER, witness);
                    }
                    return Ok(Outcome::Contribution {
                        witnesses: witnesses.len() as u8,
                    });
                }
                Ok(Outcome::Delivered)
            }
            Intent::Attack(id) => self.strike(id, false),
            Intent::Guard => Ok(Outcome::Guarded),
            Intent::Practice => {
                self.require_reach(self.scenario.shrine)?;
                if self.breath == BREATH_MASTERY {
                    return Err(Rejection::PracticeComplete);
                }
                if self.player.herbs == 0 {
                    return Err(Rejection::NoCargo);
                }
                self.player.herbs -= 1;
                self.breath += 1;
                Ok(Outcome::Practiced {
                    progress: self.breath,
                })
            }
            Intent::ChooseBackground(background) => self.choose_background(background),
            Intent::BuyHerb => self.trade_herb(true),
            Intent::SellHerb => self.trade_herb(false),
            Intent::JoinHall => self.join_hall(),
            Intent::Brew => self.brew_remedy(),
            Intent::UseRemedy => self.use_remedy(),
            Intent::HallDuty => self.hall_duty(),
            Intent::StudyForm => self.study_form(),
            Intent::CenterQi => self.center_qi(),
            Intent::ReedStrike(id) => self.strike(id, true),
            Intent::Invite(id) => self.invite(id),
            Intent::Provision(id) => self.provision(id),
            Intent::Part(id) => self.part(id),
            Intent::OfferAccord => self.accord_with_lodge(),
            Intent::SupportHall => self.support_reed_hall(),
            Intent::PledgeLodge => self.pledge_lodge(),
            Intent::AidLodge => self.aid_lodge(),
            Intent::RenounceLodge => self.renounce_lodge(),
            Intent::Ask(id) => self.ask_witness(id),
            Intent::Examine => self.examine_site(),
            Intent::Testify { to, event } => self.testify(to, event),
            Intent::Take(id) => self.take_cargo(id),
            Intent::ReportBody { to, body } => self.report_body(to, body),
            Intent::ClaimLot(id) => self.claim_lot(id),
            Intent::OfferLot { to, id } => self.offer_lot(to, id),
            Intent::AttuneLot(id) => self.attune_lot(id),
            Intent::ChannelStrike(id) => self.channel_strike(id),
            Intent::Drill(id) => self.drill(id),
            Intent::TechniqueStrike(id) => self.technique_strike(id),
            Intent::DrawQi => self.draw_qi(),
            Intent::CirculateQi => self.circulate_qi(),
            Intent::MendMeridians => self.mend_meridians(),
            Intent::TreatWounds => self.treat_wounds(),
            Intent::ClaimHallAllocation => self.claim_hall_allocation(),
            Intent::RecoverCache(id) => self.recover_cache(id, false),
            Intent::EntrustCache(id) => self.recover_cache(id, true),
            Intent::AcceptRequest(id) => self.choose_request(id, true),
            Intent::DeclineRequest(id) => self.choose_request(id, false),
            Intent::AbandonRequest(id) => self.abandon_request(id),
            Intent::AcceptFavor(id) => self.choose_favor(id, true),
            Intent::DeclineFavor(id) => self.choose_favor(id, false),
            Intent::WithdrawFavor(id) => self.withdraw_favor(id),
            Intent::FundFavor(id) => self.fund_favor(id),
            Intent::RequestAid(id) => self.request_aid(id),
            Intent::RepayAid(id) => self.repay_aid(id),
            Intent::WaiveAid(id) => self.waive_aid(id),
        }
    }

    pub fn is_player_agency(&self) -> bool {
        matches!(
            self.rules,
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
        )
    }

    pub fn is_consequences(&self) -> bool {
        matches!(
            self.rules,
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
        )
    }

    pub fn is_violence_logistics(&self) -> bool {
        matches!(
            self.rules,
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
        )
    }

    pub fn is_player_reporting(&self) -> bool {
        matches!(
            self.rules,
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
        )
    }

    pub fn is_physical_recovery(&self) -> bool {
        matches!(
            self.rules,
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
        )
    }

    pub fn is_sect_cultivation(&self) -> bool {
        matches!(
            self.rules,
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
        )
    }

    fn grants_player_agency(&self) -> bool {
        self.is_player_agency()
    }

    fn take_cargo(&mut self, id: ActorId) -> Result<Outcome, Rejection> {
        if !self.is_consequences() {
            return Err(Rejection::Refused);
        }
        if !self.player.alive() {
            return Err(Rejection::Dead);
        }
        let npc = self.npcs.get(&id).ok_or(Rejection::UnknownActor)?;
        if npc.alive() {
            return Err(Rejection::Refused);
        }
        let position = npc.position;
        self.require_reach(position)?;
        let herbs = npc.herbs;
        if herbs == 0 {
            return Err(Rejection::Empty);
        }
        self.player
            .herbs
            .checked_add(herbs)
            .ok_or(Rejection::PackFull)?;
        self.npcs.get_mut(&id).expect("known corpse").herbs = 0;
        self.player.herbs += herbs;
        self.witness_transfer(id, position);
        Ok(Outcome::Took { from: id, herbs })
    }

    fn participant_alive(&self, id: ActorId) -> bool {
        if id == PLAYER {
            self.player.alive()
        } else {
            self.npcs.get(&id).is_some_and(|npc| npc.alive())
        }
    }

    fn participant_position(&self, id: ActorId) -> Option<Position> {
        if id == PLAYER {
            Some(self.player.position)
        } else {
            self.npcs.get(&id).map(|npc| npc.position)
        }
    }

    fn strike(&mut self, id: ActorId, with_form: bool) -> Result<Outcome, Rejection> {
        let target = self
            .npcs
            .get(&id)
            .filter(|npc| npc.alive())
            .ok_or(Rejection::UnknownActor)?;
        self.require_reach(target.position)?;
        let (damage, outcome) = if with_form {
            self.spend_form_qi()?;
            (content::FORM_DAMAGE, Outcome::FormStruck(id))
        } else {
            (STRIKE, Outcome::Struck(id))
        };
        self.damage(PLAYER, id, self.wounded_strike(PLAYER, damage), false);
        Ok(outcome)
    }

    fn talk(&mut self, id: ActorId) -> Result<Outcome, Rejection> {
        let npc = self
            .npcs
            .get(&id)
            .filter(|npc| npc.alive())
            .ok_or(Rejection::UnknownActor)?;
        self.require_reach(npc.position)?;
        if self.social.relationship(id, PLAYER).refuses_aid() {
            return Err(Rejection::Refused);
        }
        if npc.definition.role == Role::Medic && self.player.hp < PLAYER_HP {
            if npc.position != self.scenario.clinic {
                return Err(Rejection::HealerAway);
            }
            if self.supply.shrine == 0 {
                return Err(Rejection::Empty);
            }
            let place = npc.position;
            let healed = self.treat(PLAYER);
            self.social.aided(id, PLAYER);
            self.remember_encounter(id, place, encounters::Kind::Care);
            self.remember_aid(id, PLAYER, reciprocity::Kind::ClinicCare);
            return Ok(Outcome::Treated { by: id, healed });
        }
        let place = npc.position;
        if self.remember_encounter(id, place, encounters::Kind::Talk) {
            self.social.meet(id, PLAYER);
        }
        self.regional_conversation(id);
        self.rumor_conversation(id);
        self.disclose_development(id);
        self.disclose_circle(id);
        self.disclose_diplomacy(id);
        self.discuss_favor(id);
        Ok(Outcome::Spoke(id))
    }

    fn require_reach(&self, target: Position) -> Result<(), Rejection> {
        navigation::in_reach(self.player.position, target)
            .then_some(())
            .ok_or(Rejection::OutOfReach)
    }

    fn occupied(&self, position: Position) -> bool {
        (self.player.alive() && self.player.position == position)
            || self
                .npcs
                .values()
                .any(|npc| npc.alive() && npc.position == position)
    }

    fn regrow(&mut self) {
        let cadence = self.generated_region().map_or(
            self.scenario.regrow_every,
            worldgen::GeneratedRegion::west_regrow_every,
        );
        if self.tick.is_multiple_of(cadence) && self.supply.source < self.scenario.initial_source {
            self.supply.source += 1;
            self.supply.regrown += 1;
        }
    }

    /// Called only for live patients with a deficit and available medicine.
    fn treat(&mut self, patient: ActorId) -> u16 {
        let (hp, maximum) = if patient == PLAYER {
            (&mut self.player.hp, PLAYER_HP)
        } else {
            let npc = self.npcs.get_mut(&patient).expect("known patient");
            (&mut npc.hp, npc.definition.hp)
        };
        let healed = (maximum - *hp).min(2);
        *hp += healed;
        self.supply.shrine -= 1;
        self.supply.consumed += 1;
        healed
    }

    pub fn checkpoint_revision(&self) -> u16 {
        checkpoint::revision(self.rules)
    }

    pub fn checkpoint(&self) -> Vec<u8> {
        checkpoint::encode(self)
    }

    pub fn restore(bytes: &[u8]) -> Result<Self, LoadError> {
        checkpoint::decode(bytes, &qinghe::QINGHE)
    }

    pub fn state_digest(&self) -> [u8; 32] {
        checkpoint::state_digest(self)
    }
}

impl Scenario {
    fn valid(&self) -> bool {
        if !(1..=64).contains(&self.width)
            || !(1..=64).contains(&self.height)
            || self.terrain.len() != self.height as usize
            || self.terrain.iter().any(|row| {
                row.len() != self.width as usize || !row.bytes().all(|b| matches!(b, b'.' | b'#'))
            })
            || self.regrow_every == 0
            || self.initial_source > 65_536
            || self.shrine_target == 0
            || self.npcs.is_empty()
            || self.npcs.len() > 32
        {
            return false;
        }
        let ids: BTreeSet<_> = self.npcs.iter().map(|npc| npc.id).collect();
        let starts: BTreeSet<_> = self.npcs.iter().map(|npc| npc.spawn).collect();
        let glyphs: BTreeSet<_> = self.npcs.iter().map(|npc| npc.glyph).collect();
        ids.len() == self.npcs.len()
            && starts.len() == self.npcs.len()
            && glyphs.len() == self.npcs.len()
            && !ids.contains(&PLAYER)
            && !ids.contains(&ActorId::new(0))
            && !starts.contains(&self.entrance)
            && [self.entrance, self.clinic, self.patch, self.shrine]
                .into_iter()
                .all(|cell| self.walkable(cell))
            && self.npcs.iter().all(|npc| {
                self.walkable(npc.spawn)
                    && self.walkable(npc.home)
                    && npc.route.iter().all(|&cell| self.walkable(cell))
                    && npc.glyph.is_ascii_alphanumeric()
                    && npc.hp > 0
                    && (1..=40).contains(&npc.rest_threshold)
                    && npc
                        .report_to
                        .is_none_or(|peer| peer != npc.id && ids.contains(&peer))
            })
            && self.recruits.iter().all(|(id, _)| ids.contains(id))
            && self
                .recruits
                .iter()
                .map(|(id, _)| id)
                .collect::<BTreeSet<_>>()
                .len()
                == self.recruits.len()
            && self
                .contacts
                .iter()
                .all(|(a, b)| a != b && ids.contains(a) && ids.contains(b))
    }

    fn walkable(&self, position: Position) -> bool {
        position.x < self.width
            && position.y < self.height
            && self.terrain[position.y as usize].as_bytes()[position.x as usize] != b'#'
    }
}
