//! Historical campaign selection, backed by the immutable v1 registry.
use super::{Rules, Scenario, checkpoint, content};
pub(super) use content::SCENARIO_QINGHE as QINGHE;

pub(super) fn region(seed: u32, rules: Rules) -> &'static Scenario {
    content::scenario_for_revision(checkpoint::revision(rules), seed)
        .expect("every supported rules revision has a content binding")
}
