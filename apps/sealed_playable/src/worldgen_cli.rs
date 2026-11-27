//! Explicit C4.0 launch and read-only developer generator manifest.
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

pub fn launch(args: &[String]) -> Result<(), String> {
    let mut c40 = false;
    let mut pixel = false;
    let mut debug = false;
    let mut selected_seed = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--c40" if !c40 => c40 = true,
            "--pixel" if !pixel => pixel = true,
            "--debug-world" if !debug => debug = true,
            "--seed" if selected_seed.is_none() => {
                i += 1;
                selected_seed = Some(seed(args.get(i).ok_or("--seed requires a value")?)?);
            }
            other => return Err(format!("unexpected or duplicate C4.0 argument: {other}")),
        }
        i += 1;
    }
    if !c40 || (pixel && debug) {
        return Err("usage: --c40 [--seed N] [--pixel | --debug-world]".into());
    }
    let seed = selected_seed.unwrap_or(0);
    if pixel {
        #[cfg(feature = "desktop")]
        sealed_playable::ascii::run_generated_window(seed);
        #[cfg(not(feature = "desktop"))]
        return Err("The window requires the desktop feature.".into());
    } else {
        sealed_playable::ascii::run_generated_campaign(seed, debug);
    }
    Ok(())
}
