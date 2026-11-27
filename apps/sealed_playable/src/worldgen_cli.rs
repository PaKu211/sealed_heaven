//! Explicit C4.0 launch and read-only developer generator manifest.
use sealed_sim::living::expansion::{self, ExpandedRegion};
use sealed_sim::living::{
    content,
    worldgen::{self, GeneratedRegion},
};
use serde_json::json;

fn hex(digest: [u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn seed(value: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| "seed must be an unsigned 32-bit integer".into())
}

pub fn inspect(args: &[String]) -> Result<(), String> {
    if args.len() != 2 || args[0] != "--worldgen-info" {
        return Err("usage: --worldgen-info SEED".into());
    }
    let map = GeneratedRegion::generate(seed(&args[1])?);
    let digests = map.digests();
    let rows: Vec<String> = map
        .tiles()
        .as_chunks::<{ worldgen::WIDTH as usize }>()
        .0
        .iter()
        .map(|row| row.iter().map(|t| t.terrain.glyph()).collect())
        .collect();
    let fields:Vec<_>=map.tiles().as_chunks::<{ worldgen::WIDTH as usize }>().0.iter().map(|row|{
        row.iter().map(|t|json!({"terrain":t.terrain.id(),"elevation":t.elevation,"moisture":t.moisture,"qi":t.qi})).collect::<Vec<_>>()
    }).collect();
    let bridges = map.bridges().map(|p| [p.x, p.y]);
    let result = json!({
        "scope":"developer-only immutable regional generation diagnostic",
        "generator_version":worldgen::GENERATOR_VERSION,
        "checkpoint_revision":worldgen::GENERATED_REVISION,
        "base_content_revision":worldgen::BASE_REVISION,
        "content_sha256":content::CONTENT_SHA256,
        "seed":map.seed(),
        "width":worldgen::WIDTH,"height":worldgen::HEIGHT,
        "terrain_rows":rows,"tiles":fields,"river_x_by_row":map.river(),"bridges":bridges,
        "gate_distance":map.gate_distance(),
        "west_regrow_every":map.west_regrow_every(),"garden_regrow_every":map.garden_regrow_every(),
        "digests":{"geography":hex(digests.geography),"hydrology":hex(digests.hydrology),
            "ecology":hex(digests.ecology),"roads":hex(digests.roads),"world":hex(digests.world)}
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
    );
    Ok(())
}

pub fn inspect_expanded(args: &[String]) -> Result<(), String> {
    if args.len() != 2 || args[0] != "--expansion-info" {
        return Err("usage: --expansion-info SEED".into());
    }
    let seed = seed(&args[1])?;
    let map = ExpandedRegion::generate(seed);
    let base = GeneratedRegion::generate(seed);
    let rows: Vec<String> = map
        .tiles()
        .as_chunks::<{ expansion::WIDTH as usize }>()
        .0
        .iter()
        .map(|row| row.iter().map(|t| t.terrain.glyph()).collect())
        .collect();
    let fields:Vec<_>=map.tiles().as_chunks::<{expansion::WIDTH as usize}>().0.iter().map(|row|row.iter().map(|t|json!({"terrain":t.terrain.id(),"elevation":t.elevation,"moisture":t.moisture,"qi":t.qi})).collect::<Vec<_>>()).collect();
    let result = json!({
        "scope":"developer-only immutable eastern expansion diagnostic",
        "checkpoint_revision":expansion::REVISION,"generator_version":expansion::GENERATOR_VERSION,
        "content_sha256":content::CONTENT_SHA256,"overlay_sha256":expansion::CONTENT_SHA256,
        "seed":seed,"width":expansion::WIDTH,"height":expansion::HEIGHT,"terrain_rows":rows,"tiles":fields,
        "river_x_by_row":map.river(),"bridges":map.bridges().map(|p|[p.x,p.y]),
        "gate_distance":map.gate_distance(),"grove_regrow_every":map.grove_regrow_every(),
        "base_world_sha256":hex(base.digests().world),"world_sha256":hex(map.digest()),
        "authored_overlay":serde_json::from_str::<serde_json::Value>(expansion::manifest_json()).map_err(|e|e.to_string())?
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
    );
    Ok(())
}

pub fn launch(args: &[String]) -> Result<(), String> {
    let mut revision = None;
    let mut pixel = false;
    let mut debug = false;
    let mut selected_seed = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--c40" if revision.is_none() => revision = Some(27),
            "--c41" if revision.is_none() => revision = Some(28),
            "--c42" if revision.is_none() => revision = Some(29),
            "--c43" if revision.is_none() => revision = Some(30),
            "--pixel" if !pixel => pixel = true,
            "--debug-world" if !debug => debug = true,
            "--seed" if selected_seed.is_none() => {
                i += 1;
                selected_seed = Some(seed(args.get(i).ok_or("--seed requires a value")?)?);
            }
            other => {
                return Err(format!(
                    "unexpected or duplicate generated campaign argument: {other}"
                ));
            }
        }
        i += 1;
    }
    if revision.is_none() || (pixel && debug) {
        return Err(
            "usage: --c40 | --c41 | --c42 | --c43 [--seed N] [--pixel | --debug-world]".into(),
        );
    }
    let seed = selected_seed.unwrap_or(0);
    if pixel {
        #[cfg(feature = "desktop")]
        if revision == Some(30) {
            sealed_playable::ascii::run_lod_window(seed);
        } else if revision == Some(29) {
            sealed_playable::ascii::run_knowledge_window(seed);
        } else if revision == Some(28) {
            sealed_playable::ascii::run_expanded_window(seed);
        } else {
            sealed_playable::ascii::run_generated_window(seed);
        }
        #[cfg(not(feature = "desktop"))]
        return Err("The window requires the desktop feature.".into());
    } else {
        if revision == Some(30) {
            sealed_playable::ascii::run_lod_campaign(seed, debug);
        } else if revision == Some(29) {
            sealed_playable::ascii::run_knowledge_campaign(seed, debug);
        } else if revision == Some(28) {
            sealed_playable::ascii::run_expanded_campaign(seed, debug);
        } else {
            sealed_playable::ascii::run_generated_campaign(seed, debug);
        }
    }
    Ok(())
}
