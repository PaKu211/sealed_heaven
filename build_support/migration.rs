//! Offline authoring upgrade admission. It never opens or writes a save.
use super::registry::Generated;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Compare validated packs, forbid stable ID removal/rebinding, and require
/// exactly one version increment for each changed pack. The resulting plan
/// is advisory: activation requires a separately implemented rules revision.
pub fn plan_upgrade(base: &Generated, candidate: &Generated) -> Result<Value, String> {
    let old: Value = serde_json::from_str(&base.manifest).map_err(|e| e.to_string())?;
    let new: Value = serde_json::from_str(&candidate.manifest).map_err(|e| e.to_string())?;
    let index = |v: &Value, field: &str, key: &str| -> Result<BTreeMap<String, Value>, String> {
        let mut m = BTreeMap::new();
        for entry in v[field]
            .as_array()
            .ok_or_else(|| format!("missing {field}"))?
        {
            let id = entry[key]
                .as_str()
                .ok_or_else(|| format!("missing {key}"))?;
            if m.insert(id.to_owned(), entry.clone()).is_some() {
                return Err(format!("duplicate {key}"));
            }
        }
        Ok(m)
    };
    let old_defs = index(&old, "definitions", "id")?;
    let new_defs = index(&new, "definitions", "id")?;
    for (id, definition) in &old_defs {
        match new_defs.get(id) {
            None => {
                return Err(format!(
                    "removed stable ID {id}; explicit migration required"
                ));
            }
            Some(next)
                if next["kind"] != definition["kind"]
                    || next["wire_id"] != definition["wire_id"]
                    || next["binding"] != definition["binding"] =>
            {
                return Err(format!(
                    "rebound stable ID {id}; explicit migration required"
                ));
            }
            _ => {}
        }
    }
    let old_packs = index(&old, "pack_fingerprints", "pack_id")?;
    let new_packs = index(&new, "pack_fingerprints", "pack_id")?;
    if !old_packs.keys().eq(new_packs.keys()) {
        return Err("pack set changed; new schema required".into());
    }
    let mut changes = Vec::new();
    for (id, before) in &old_packs {
        let after = &new_packs[id];
        let from = before["content_version"]
            .as_u64()
            .ok_or("invalid prior version")?;
        let to = after["content_version"]
            .as_u64()
            .ok_or("invalid candidate version")?;
        if before["sha256"] != after["sha256"] {
            if from.checked_add(1) != Some(to) {
                return Err(format!(
                    "changed pack {id} must increment version exactly once"
                ));
            }
            changes.push(json!({"pack_id":id,"from":from,"to":to,"from_sha256":before["sha256"],"to_sha256":after["sha256"]}));
        } else if from != to {
            return Err(format!("inconsistent unchanged pack version {id}"));
        }
    }
    let added: Vec<_> = new_defs
        .keys()
        .filter(|id| !old_defs.contains_key(*id))
        .cloned()
        .collect();
    let changed = base.fingerprint != candidate.fingerprint;
    Ok(json!({
        "status": if changed {"REQUIRES_NEW_RULES_REVISION"} else {"NO_CHANGE"},
        "from_sha256":base.fingerprint, "to_sha256":candidate.fingerprint,
        "changed_packs":changes, "added_ids":added,
        "preserved_ids":old_defs.len(),
        "legacy_checkpoint_policy":"Keep v1 and the existing revision bindings unchanged.",
        "activation_policy":"New revision and explicit checkpoint content identity; separately review and test any state migration.",
        "save_migration_performed":false, "runtime_activation_performed":false
    }))
}
