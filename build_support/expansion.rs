//! Host-only compiler for the immutable Pineford overlay. Base v1 is not edited.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt::Write};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Pack {
    schema_version: u32,
    pack_id: String,
    content_version: u32,
    base_sha256: String,
    name: String,
    width: u32,
    height: u32,
    grove: [u32; 2],
    depot: [u32; 2],
    grove_capacity: u32,
    depot_target: u32,
    depot_reserve: u32,
    depot_initial: u32,
    coin_budget: u32,
    caravan_load: u32,
    willowford_buffer: u32,
    regrow_fastest: u64,
    regrow_step: u64,
    normal_buy: u32,
    normal_sell: u32,
    scarce_buy: u32,
    scarce_sell: u32,
    actors: [Actor; 2],
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Actor {
    id: String,
    wire_id: u32,
    name: String,
    glyph: char,
    role: Role,
    spawn: [u32; 2],
    home: [u32; 2],
    hp: u16,
    rest_threshold: u8,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
enum Role {
    FrontierCarrier,
    FrontierGrower,
}

fn cell([x, y]: [u32; 2]) -> String {
    format!("Position {{x:{x},y:{y}}}")
}
fn valid_text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 80 && !s.chars().any(char::is_control)
}
fn valid_id(s: &str) -> bool {
    s.strip_prefix("sealed_heaven:").is_some_and(|tail| {
        !tail.is_empty()
            && tail
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    })
}
fn constant(out: &mut String, n: &str, ty: &str, v: impl std::fmt::Display) {
    writeln!(out, "pub const {n}: {ty} = {v};").unwrap();
}

pub struct Generated {
    pub rust: String,
    pub manifest: String,
    pub sha256: String,
}

pub fn compile(raw: &str, base_raw: &str, base_fingerprint: &str) -> Result<Generated, String> {
    if raw.len() > 16 * 1024 {
        return Err("expansion pack too large".into());
    }
    let p: Pack = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let base: serde_json::Value = serde_json::from_str(base_raw).map_err(|e| e.to_string())?;
    if p.schema_version != 1
        || p.content_version != 1
        || p.pack_id != "sealed_heaven:pineford"
        || p.base_sha256 != base_fingerprint
        || p.width != 54
        || p.height != 15
        || !valid_text(&p.name)
    {
        return Err("invalid expansion identity, dependency or bounds".into());
    }
    let in_town = |[x, y]: [u32; 2]| (43..=53).contains(&x) && (0..15).contains(&y);
    if !in_town(p.grove)
        || !in_town(p.depot)
        || p.grove == p.depot
        || !(1..=64).contains(&p.grove_capacity)
        || !(1..=64).contains(&p.depot_target)
        || p.depot_initial > p.depot_target
        || p.depot_reserve >= p.depot_target
        || !(1..=8).contains(&p.caravan_load)
        || !(1..=8).contains(&p.willowford_buffer)
        || p.coin_budget == 0
        || p.coin_budget > 1024
        || !(1..=100).contains(&p.regrow_fastest)
        || !(1..=100).contains(&p.regrow_step)
        || p.normal_sell == 0
        || p.normal_sell >= p.normal_buy
        || p.scarce_sell >= p.scarce_buy
        || p.scarce_buy <= p.normal_buy
        || p.scarce_sell < p.normal_sell
        || p.scarce_buy > 64
    {
        return Err("invalid expansion sites or economy".into());
    }
    let original = base["actors"]
        .as_array()
        .ok_or("missing validated base actors")?;
    let mut ids: BTreeSet<String> = original
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_owned())
        .collect();
    let mut wires: BTreeSet<u32> = original
        .iter()
        .map(|a| a["wire_id"].as_u64().unwrap() as u32)
        .collect();
    let mut glyphs: BTreeSet<char> = original
        .iter()
        .map(|a| a["glyph"].as_str().unwrap().chars().next().unwrap())
        .collect();
    let mut spawns = BTreeSet::new();
    let mut homes = BTreeSet::new();
    if original.len() != 10
        || !ids.insert(p.pack_id.clone())
        || p.actors[0].role != Role::FrontierCarrier
        || p.actors[1].role != Role::FrontierGrower
    {
        return Err("invalid base actor count or new actor bindings".into());
    }
    for a in &p.actors {
        if !valid_id(&a.id)
            || !ids.insert(a.id.clone())
            || !(37..=255).contains(&a.wire_id)
            || !wires.insert(a.wire_id)
            || !valid_text(&a.name)
            || !a.glyph.is_ascii_uppercase()
            || !glyphs.insert(a.glyph)
            || !in_town(a.spawn)
            || !in_town(a.home)
            || !spawns.insert(a.spawn)
            || !homes.insert(a.home)
            || !(1..=100).contains(&a.hp)
            || !(1..=40).contains(&a.rest_threshold)
        {
            return Err("invalid or duplicate expansion actor".into());
        }
    }
    if p.actors[0].wire_id != 37 || p.actors[1].wire_id != 38 {
        return Err("stable actor binding changed".into());
    }
    // Canonical typed representation: normalized object keys and explicit schema.
    let canonical = serde_json::to_vec(&serde_json::to_value(&p).unwrap()).unwrap();
    let mut hash = Sha256::new();
    hash.update(b"SH:C41:CONTENT:v1\0");
    hash.update(&canonical);
    let sha256 = hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let mut rust = String::from("// @generated by expansion compiler; do not edit.\n");
    writeln!(rust, "pub const CONTENT_SHA256: &str = {:?};", sha256).unwrap();
    writeln!(rust, "pub const TOWN_NAME: &str = {:?};", p.name).unwrap();
    writeln!(rust, "pub const PACK_ID: &str = {:?};", p.pack_id).unwrap();
    for (name, v) in [
        ("WIDTH", p.width),
        ("HEIGHT", p.height),
        ("GROVE_CAPACITY", p.grove_capacity),
        ("DEPOT_TARGET", p.depot_target),
        ("DEPOT_RESERVE", p.depot_reserve),
        ("DEPOT_INITIAL", p.depot_initial),
        ("COIN_BUDGET", p.coin_budget),
        ("CARAVAN_LOAD", p.caravan_load),
        ("WILLOWFORD_BUFFER", p.willowford_buffer),
        ("NORMAL_BUY", p.normal_buy),
        ("NORMAL_SELL", p.normal_sell),
        ("SCARCE_BUY", p.scarce_buy),
        ("SCARCE_SELL", p.scarce_sell),
    ] {
        constant(&mut rust, name, "u32", v);
    }
    constant(&mut rust, "REGROW_FASTEST", "u64", p.regrow_fastest);
    constant(&mut rust, "REGROW_STEP", "u64", p.regrow_step);
    constant(&mut rust, "GROVE", "Position", cell(p.grove));
    constant(&mut rust, "DEPOT", "Position", cell(p.depot));
    rust.push_str("const EXTRA_NPCS: [NpcDefinition;2] = [\n");
    for a in &p.actors {
        writeln!(rust,"NpcDefinition {{ id:ActorId::new({}),name:{:?},glyph:{:?},role:Role::{:?},spawn:{},home:{},route:&[],hp:{},rest_threshold:{},responds_to_attack:false,shares_news:true,report_to:None }},",a.wire_id,a.name,a.glyph,a.role,cell(a.spawn),cell(a.home),a.hp,a.rest_threshold).unwrap();
    }
    rust.push_str("];\n");
    for (name, map_id) in [
        ("RIDGE", "sealed_heaven:ridge_map"),
        ("LOWER", "sealed_heaven:lower_map"),
    ] {
        let rows = base["maps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == map_id)
            .ok_or("missing base map")?["rows"]
            .as_array()
            .unwrap();
        let mut wide = Vec::new();
        for y in 0..15 {
            let left = rows
                .get(y)
                .map(|row| row.as_str().unwrap().to_owned())
                .unwrap_or("#".repeat(30));
            wide.push(left + &".".repeat(24));
        }
        writeln!(rust, "const TERRAIN_{name}: &[&str] = &{:?};", wide).unwrap();
    }
    let mut defs = vec![
        (p.pack_id.clone(), "settlement", None, None),
        (format!("{}_depot", p.pack_id), "site", None, None),
        (format!("{}_grove", p.pack_id), "site", None, None),
    ];
    for actor in &p.actors {
        defs.push((
            actor.id.clone(),
            "actor",
            Some(actor.wire_id),
            Some(format!("{:?}", actor.role)),
        ));
    }
    defs.sort_by(|a, b| a.0.cmp(&b.0));
    if defs.windows(2).any(|w| w[0].0 == w[1].0) {
        return Err("duplicate derived overlay identity".into());
    }
    // Base v1 is already validated. Check all top-level definition lists for
    // stable string ID collisions with the extension.
    for (id, _, _, _) in &defs {
        if base
            .as_object()
            .unwrap()
            .values()
            .filter_map(|v| v.as_array())
            .flatten()
            .any(|v| v.get("id").and_then(|v| v.as_str()) == Some(id))
        {
            return Err("overlay definition collides with base".into());
        }
    }
    rust.push_str("pub const DEFINITIONS: &[content::Definition] = &[\n");
    for (id, kind, wire, binding) in defs {
        let binding = binding.map_or_else(|| "None".to_owned(), |v| format!("Some({v:?})"));
        writeln!(
            rust,
            "content::Definition {{id:{id:?},kind:{kind:?},wire_id:{wire:?},binding:{binding}}},"
        )
        .unwrap();
    }
    rust.push_str("];\n");
    let manifest =
        serde_json::to_string_pretty(&serde_json::json!({"content_sha256":sha256,"pack":p}))
            .unwrap();
    Ok(Generated {
        rust,
        manifest,
        sha256,
    })
}
