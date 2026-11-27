//! Historical campaign selection, backed by the immutable v1 registry.
use super::{Rules, Scenario, checkpoint, content};
pub(super) use content::SCENARIO_QINGHE as QINGHE;

pub(super) fn region(seed: u32, rules: Rules) -> &'static Scenario {
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
    ) {
        return super::expansion::scenario(seed);
    }
    let revision = if rules == Rules::GeneratedRiverlands {
        super::worldgen::BASE_REVISION
    } else {
        checkpoint::revision(rules)
    };
    content::scenario_for_revision(revision, seed)
        .expect("every supported rules revision has a content binding")
}
