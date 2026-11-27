#[path = "../build_support/migration.rs"]
mod migration;
#[path = "../build_support/registry.rs"]
mod registry;

use registry::{Generated, compile};
use sealed_sim::living::{ActorId, Background, Intent, World, content};
use serde_json::{Value, json};
const CORE: &str = include_str!("../content/v1/core.json");
const WORLD: &str = include_str!("../content/v1/sealed_heaven.json");
fn parsed() -> (Value, Value) {
    (
        serde_json::from_str(CORE).unwrap(),
        serde_json::from_str(WORLD).unwrap(),
    )
}
fn build(core: &Value, world: &Value) -> Result<Generated, String> {
    compile(&core.to_string(), &world.to_string())
}
fn invalid(f: impl FnOnce(&mut Value, &mut Value), needle: &str) {
    let (mut core, mut world) = parsed();
    f(&mut core, &mut world);
    let error = build(&core, &world).unwrap_err();
    assert!(error.contains(needle), "expected {needle:?}, got {error:?}");
}
#[test]
fn canonical_registry_is_pinned_and_order_independent_for_object_keys() {
    let baseline = compile(CORE, WORLD).unwrap();
    let (core, world) = parsed();
    let reserialized = build(&core, &world).unwrap();
    assert_eq!(baseline.fingerprint, content::CONTENT_SHA256);
    assert_eq!(baseline.fingerprint, reserialized.fingerprint);
    assert_eq!(baseline.rust, reserialized.rust);
    assert_eq!(baseline.manifest, content::manifest_json());
    assert_eq!(baseline.manifest, reserialized.manifest);
    assert_eq!(content::DEFINITIONS.len(), 43);
    assert!(
        content::DEFINITIONS
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
    );
    for d in content::DEFINITIONS {
        assert_eq!(content::definition(d.id), Some(d));
    }
    assert_eq!(
        content::definition("sealed_heaven:luo_xiu")
            .unwrap()
            .wire_id,
        Some(34)
    );
    assert_eq!(
        content::definition("sealed_heaven:ore_west")
            .unwrap()
            .wire_id,
        Some(0)
    );
    assert!(content::definition("missing:actor").is_none());
    for rev in 1..=26 {
        assert_eq!(
            content::identity_for_revision(rev).unwrap().sha256,
            baseline.fingerprint
        );
    }
    assert!(content::identity_for_revision(0).is_none());
    assert_eq!(
        content::identity_for_revision(27),
        content::identity_for_revision(26)
    );
    let expanded = content::identity_for_revision(28).unwrap();
    assert_eq!(expanded.sha256, content::CONTENT_SHA256);
    assert_eq!(
        expanded.overlay_sha256,
        Some(sealed_sim::living::expansion::CONTENT_SHA256)
    );
    assert!(
        content::identity_for_revision(27)
            .unwrap()
            .overlay_sha256
            .is_none()
    );
    assert_eq!(
        content::identity_for_revision(29),
        content::identity_for_revision(28)
    );
    assert_eq!(
        content::identity_for_revision(30),
        content::identity_for_revision(29)
    );
    assert_eq!(
        content::identity_for_revision(31),
        content::identity_for_revision(30)
    );
    assert_eq!(
        content::identity_for_revision(32),
        content::identity_for_revision(31)
    );
    assert_eq!(
        content::identity_for_revision(33),
        content::identity_for_revision(32)
    );
    assert_eq!(
        content::identity_for_revision(34),
        content::identity_for_revision(33)
    );
    assert_eq!(
        content::identity_for_revision(35),
        content::identity_for_revision(34)
    );
    for revision in 36..=sealed_sim::living::rumors::REVISION {
        assert_eq!(
            content::identity_for_revision(revision),
            content::identity_for_revision(35)
        );
    }
    assert!(content::identity_for_revision(u16::MAX).is_none());
}
#[test]
fn duplicate_unknown_and_invalid_schema_fields_are_rejected() {
    assert!(
        compile(
            &CORE.replacen(
                "\"schema_version\": 1",
                "\"schema_version\": 1, \"schema_version\": 1",
                1
            ),
            WORLD
        )
        .unwrap_err()
        .contains("duplicate field")
    );
    invalid(
        |c, _| c["header"]["schema_version"] = json!(2),
        "unsupported schema",
    );
    invalid(
        |c, _| c["items"][0]["unexpected"] = json!(true),
        "unknown field",
    );
    invalid(
        |c, _| c["items"][0]["kind"] = json!("Unknown"),
        "unknown variant",
    );
    invalid(
        |c, _| c["items"][0]["id"] = json!("invalid namespace:herb"),
        "invalid namespace",
    );
    invalid(
        |c, _| c["items"][1]["id"] = c["items"][0]["id"].clone(),
        "duplicate definition ID",
    );
}
#[test]
fn dependency_resolution_rejects_missing_versions_duplicates_and_cycles() {
    invalid(
        |_, w| w["header"]["dependencies"][0]["pack_id"] = json!("uninstalled"),
        "missing dependency",
    );
    invalid(
        |_, w| w["header"]["dependencies"][0]["content_version"] = json!(2),
        "dependency version mismatch",
    );
    invalid(
        |_, w| {
            let d = w["header"]["dependencies"][0].clone();
            w["header"]["dependencies"].as_array_mut().unwrap().push(d);
        },
        "duplicate dependency",
    );
    invalid(
        |c, _| {
            c["header"]["dependencies"] = json!([{"pack_id":"sealed_heaven","content_version":1}])
        },
        "dependency cycle",
    );
}
#[test]
fn actor_identity_and_reference_errors_are_rejected() {
    invalid(
        |_, w| w["actors"][1]["wire_id"] = json!(1),
        "duplicate actor wire ID",
    );
    invalid(
        |_, w| w["actors"][1]["wire_id"] = json!(21),
        "invalid actor",
    );
    invalid(
        |_, w| w["actors"][1]["wire_id"] = json!(256),
        "invalid actor",
    );
    invalid(|_, w| w["actors"][1]["hp"] = json!(0), "invalid actor");
    invalid(
        |_, w| w["actors"][1]["report_to"] = json!("sealed_heaven:unknown"),
        "unknown actor",
    );
    invalid(
        |_, w| w["actors"][1]["report_to"] = w["actors"][1]["id"].clone(),
        "self report",
    );
    invalid(
        |_, w| w["actors"][1]["report_to"] = json!("sealed_heaven:mei_fen"),
        "report target outside",
    );
    invalid(
        |_, w| w["actors"][1]["spawn"] = w["actors"][0]["spawn"].clone(),
        "duplicate actor, spawn or glyph",
    );
    invalid(
        |_, w| w["actors"][1]["glyph"] = w["actors"][0]["glyph"].clone(),
        "duplicate actor, spawn or glyph",
    );
}
#[test]
fn invalid_map_and_disconnected_physical_sites_are_rejected() {
    invalid(|_, w| w["maps"][0]["rows"][0] = json!("x"), "invalid map");
    invalid(
        |_, w| w["scenarios"][0]["map"] = json!("sealed_heaven:missing"),
        "unknown map",
    );
    invalid(
        |_, w| w["scenarios"][0]["patch"] = json!([99, 1]),
        "blocked or disconnected site",
    );
    invalid(
        |_, w| w["actors"][0]["route"] = json!([[6, 1]]),
        "disconnected",
    );
    invalid(
        |_, w| w["region"]["market"] = json!([99, 1]),
        "regional site disconnected",
    );
    invalid(
        |_, w| w["lots"][0]["position"] = json!([99, 1]),
        "disconnected",
    );
    invalid(
        |_, w| {
            let rows = w["maps"][0]["rows"].as_array_mut().unwrap();
            for row in rows {
                let mut b = row.as_str().unwrap().as_bytes().to_vec();
                b[6] = b'#';
                *row = json!(String::from_utf8(b).unwrap());
            }
        },
        "disconnected",
    );
}
#[test]
fn scenario_relationships_and_campaign_ranges_are_validated() {
    invalid(
        |_, w| w["scenarios"][0]["actors"][0] = json!("sealed_heaven:missing"),
        "unknown actor",
    );
    invalid(
        |_, w| {
            let a = w["scenarios"][0]["contacts"][0].clone();
            w["scenarios"][0]["contacts"]
                .as_array_mut()
                .unwrap()
                .push(a);
        },
        "duplicate contact",
    );
    invalid(
        |_, w| {
            w["scenarios"][0]["contacts"][0] =
                json!(["sealed_heaven:yao_lian", "sealed_heaven:yao_lian"])
        },
        "invalid contact",
    );
    invalid(
        |_, w| {
            w["scenarios"][0]["recruits"] =
                json!([{"actor":"sealed_heaven:luo_xiu","invitation":"OpenRoad"}])
        },
        "unknown recruit",
    );
    invalid(
        |_, w| w["campaigns"][1]["first"] = json!(4),
        "campaign revision gap",
    );
    invalid(
        |_, w| w["campaigns"][4]["last"] = json!(27),
        "campaign revision gap",
    );
    invalid(
        |_, w| w["campaigns"][1]["even"] = json!("sealed_heaven:missing"),
        "unknown scenario",
    );
}
#[test]
fn typed_items_recipes_and_economic_bounds_are_validated() {
    invalid(
        |c, _| c["recipes"][0]["input"] = json!("core:unknown"),
        "unknown item",
    );
    invalid(
        |c, _| c["recipes"][0]["output"] = json!("core:coin"),
        "unsupported recipe",
    );
    invalid(
        |c, _| c["recipes"][0]["input_count"] = json!(0),
        "invalid remedy recipe",
    );
    invalid(
        |_, w| w["region"]["stock_variants"] = json!(0),
        "invalid region economy",
    );
    invalid(
        |_, w| w["region"]["stock_base"] = json!(u32::MAX),
        "invalid region economy",
    );
    invalid(
        |_, w| w["region"]["order_interval"] = json!(0),
        "invalid region economy",
    );
    invalid(
        |_, w| w["region"]["market_reserve"] = json!(6),
        "invalid region economy",
    );
    invalid(
        |_, w| w["backgrounds"][0]["coins"] = json!(25),
        "background exceeds",
    );
    invalid(
        |_, w| w["lots"][0]["item"] = json!("core:coin"),
        "invalid lot",
    );
    invalid(
        |_, w| w["lots"][1]["wire_id"] = json!(0),
        "duplicate lot wire ID",
    );
}
#[test]
fn institution_and_technique_references_are_validated() {
    invalid(
        |_, w| w["institutions"][0]["offices"][0]["actor"] = json!("sealed_heaven:missing"),
        "unknown actor",
    );
    invalid(
        |_, w| w["institutions"][0]["opening_coins"] = json!(65_537),
        "institution treasury",
    );
    invalid(
        |_, w| w["sect"]["senior_merit"] = json!(1),
        "invalid sect progression",
    );
    invalid(
        |_, w| w["techniques"][0]["qi_cost"] = json!(4),
        "invalid reed strike",
    );
    invalid(
        |_, w| w["techniques"][1]["institution"] = json!("sealed_heaven:stonebank"),
        "invalid technique",
    );
    invalid(
        |_, w| w["techniques"][1]["teacher"] = json!("sealed_heaven:missing"),
        "unknown actor",
    );
    invalid(
        |_, w| w["techniques"][2]["exertion"] = json!(255),
        "invalid martial technique",
    );
}
#[test]
fn unknown_and_same_version_content_changes_cannot_be_activated() {
    let (core, mut world) = parsed();
    let base = build(&core, &world).unwrap();
    assert_eq!(
        migration::plan_upgrade(&base, &base).unwrap()["status"],
        "NO_CHANGE"
    );
    world["region"]["coin_budget"] = json!(25);
    let changed = build(&core, &world).unwrap();
    assert_ne!(changed.fingerprint, base.fingerprint);
    assert!(
        migration::plan_upgrade(&base, &changed)
            .unwrap_err()
            .contains("increment version")
    );
    world["header"]["content_version"] = json!(2);
    let next = build(&core, &world).unwrap();
    let plan = migration::plan_upgrade(&base, &next).unwrap();
    assert_eq!(plan["status"], "REQUIRES_NEW_RULES_REVISION");
    assert_eq!(plan["changed_packs"].as_array().unwrap().len(), 1);
    assert_eq!(plan["save_migration_performed"], false);
    assert_eq!(plan["runtime_activation_performed"], false);
    assert_eq!(
        World::duty_coherence_campaign(0)
            .region()
            .unwrap()
            .market_coins,
        24
    );
    world["header"]["content_version"] = json!(3);
    assert!(
        migration::plan_upgrade(&base, &build(&core, &world).unwrap())
            .unwrap_err()
            .contains("increment version")
    );
}
#[test]
fn core_change_requires_updated_dependency_and_world_version() {
    let (mut core, mut world) = parsed();
    let base = build(&core, &world).unwrap();
    core["recipes"][0]["effect_hp"] = json!(3);
    core["header"]["content_version"] = json!(2);
    assert!(
        build(&core, &world)
            .unwrap_err()
            .contains("dependency version mismatch")
    );
    world["header"]["dependencies"][0]["content_version"] = json!(2);
    assert!(
        migration::plan_upgrade(&base, &build(&core, &world).unwrap())
            .unwrap_err()
            .contains("increment version")
    );
    world["header"]["content_version"] = json!(2);
    assert_eq!(
        migration::plan_upgrade(&base, &build(&core, &world).unwrap()).unwrap()["changed_packs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn migration_rejects_removal_or_rebinding_of_wire_identity() {
    let (core, mut world) = parsed();
    let base = build(&core, &world).unwrap();
    world["header"]["content_version"] = json!(2);
    world["actors"][0]["wire_id"] = json!(4);
    assert!(
        migration::plan_upgrade(&base, &build(&core, &world).unwrap())
            .unwrap_err()
            .contains("rebound stable ID")
    );
    let (_, mut world) = parsed();
    world["header"]["content_version"] = json!(2);
    world["lots"].as_array_mut().unwrap().pop();
    assert!(
        migration::plan_upgrade(&base, &build(&core, &world).unwrap())
            .unwrap_err()
            .contains("removed stable ID")
    );
}
#[test]
fn migration_rejects_rebinding_semantic_types_without_a_wire_id() {
    let (core, mut world) = parsed();
    let base = build(&core, &world).unwrap();
    world["header"]["content_version"] = json!(2);
    world["backgrounds"][0]["kind"] = json!("Peddler");
    world["backgrounds"][2]["kind"] = json!("Wanderer");
    let next = build(&core, &world).unwrap();
    assert!(
        migration::plan_upgrade(&base, &next)
            .unwrap_err()
            .contains("rebound stable ID")
    );
}

#[test]
fn oversized_and_invalid_utf8_pack_files_are_rejected() {
    assert!(
        compile(&" ".repeat(262145), WORLD)
            .unwrap_err()
            .contains("byte limit")
    );
    let dir = std::env::temp_dir().join(format!("sealed-c37-registry-test-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let file = dir.join("draft.json");
    std::fs::write(&file, [0xff]).unwrap();
    assert!(registry::read_bounded(&file).is_err());
    std::fs::write(&file, vec![b' '; 262145]).unwrap();
    assert!(
        registry::read_bounded(&file)
            .unwrap_err()
            .contains("byte limit")
    );
    std::fs::remove_file(&file).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}
#[test]
fn generated_content_is_used_by_actual_world_and_is_not_mutable_state() {
    let mut world = World::duty_coherence_campaign(0);
    let initial = world.checkpoint();
    assert_eq!(world.npcs().len(), 10);
    assert_eq!(
        world.npc(ActorId::new(34)).unwrap().definition.name,
        "Luo Xiu"
    );
    assert_eq!(world.region().unwrap().market_coins, 24);
    assert_eq!(world.politics().unwrap().hall_coins, 18);
    assert_eq!(world.rivalry().unwrap().lodge_coins, 16);
    assert_eq!(world.contested().unwrap().lots.len(), 4);
    assert_eq!(
        World::contested_resources(0)
            .contested()
            .unwrap()
            .lots
            .len(),
        3
    );
    assert_eq!(World::riverlands(0).npcs().len(), 7);
    world
        .step(0, Intent::ChooseBackground(Background::Peddler))
        .unwrap();
    assert_eq!(world.region().unwrap().coins, 6);
    assert_ne!(world.checkpoint(), initial);
    assert_eq!(
        content::identity_for_revision(world.checkpoint_revision())
            .unwrap()
            .sha256,
        content::CONTENT_SHA256
    );
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
}
