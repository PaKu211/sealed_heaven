//! Immutable, typed content generated at build time from content/v1.
//! Inventory, prices, relationships and other evolving state remain in World.
use super::{
    ActorId, Background, Form, Good, HallRank, Invitation, NpcDefinition, Position, RegionSites,
    Role, Scenario,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Definition {
    pub id: &'static str,
    pub kind: &'static str,
    /// Existing wire identity for an actor or initial physical lot.
    pub wire_id: Option<u32>,
    pub binding: Option<&'static str>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentIdentity {
    pub schema_version: u32,
    pub core_version: u32,
    pub world_version: u32,
    /// Fingerprint of the immutable base registry.
    pub sha256: &'static str,
    /// Additional authored overlay required for this rules revision.
    pub overlay_sha256: Option<&'static str>,
}

pub(super) struct MartialProfile {
    pub damage: u16,
    pub exertion: u8,
    pub parry: u8,
    pub lessons: u8,
    pub tuition: u32,
}

include!(concat!(env!("OUT_DIR"), "/living_registry.rs"));

/// Lookup stable namespaced IDs without a heap allocation or a hash seed.
pub fn definition(id: &str) -> Option<&'static Definition> {
    DEFINITIONS
        .binary_search_by_key(&id, |d| d.id)
        .ok()
        .map(|index| &DEFINITIONS[index])
}

/// Resolve a definition only within a known revision. The old manifest and
/// definition() API intentionally describe the base registry alone.
pub fn definition_for_revision(revision: u16, id: &str) -> Option<&'static Definition> {
    let identity = identity_for_revision(revision)?;
    definition(id).or_else(|| {
        identity.overlay_sha256?;
        super::expansion::DEFINITIONS
            .binary_search_by_key(&id, |d| d.id)
            .ok()
            .map(|index| &super::expansion::DEFINITIONS[index])
    })
}

/// Legacy checkpoints encode their rules revision and scenario digest.
/// All historical revisions are permanently bound to the exact v1 pack.
/// Never guess content identity for an unknown future revision.
pub fn identity_for_revision(revision: u16) -> Option<ContentIdentity> {
    // Revision 27 derives its geography from this exact authored v1 pack.
    // The separate generator version and world digest bind the generated map.
    (1..=super::expansion::REVISION)
        .contains(&revision)
        .then_some(ContentIdentity {
            schema_version: SCHEMA_VERSION,
            core_version: CORE_VERSION,
            world_version: CONTENT_VERSION,
            sha256: CONTENT_SHA256,
            overlay_sha256: (revision == super::expansion::REVISION)
                .then_some(super::expansion::CONTENT_SHA256),
        })
}

/// Diagnostics only; does not construct or advance a world.
pub fn manifest_json() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/living_registry.json"))
}
