//! Offline content authoring tool; uses the same compiler as the game build.
#[path = "../build_support/migration.rs"]
mod migration;
#[path = "../build_support/registry.rs"]
mod registry;
use std::{env, process::ExitCode};
fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: content_tool validate|plan-upgrade <core.json> <world.json>".into());
    }
    let core = registry::read_bounded(std::path::Path::new(&args[1]))?;
    let world = registry::read_bounded(std::path::Path::new(&args[2]))?;
    let draft = registry::compile(&core, &world)?;
    match args[0].to_str() {
        Some("validate") => {
            let mut report: serde_json::Value =
                serde_json::from_str(&draft.manifest).map_err(|e| e.to_string())?;
            report["generated_rust_bytes"] = serde_json::json!(draft.rust.len());
            report["validation"] = "PASS_STRUCTURAL_ONLY".into();
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            );
        }
        Some("plan-upgrade") => {
            let base = registry::compile(
                include_str!("../content/v1/core.json"),
                include_str!("../content/v1/sealed_heaven.json"),
            )?;
            let report = migration::plan_upgrade(&base, &draft)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            );
        }
        _ => return Err("unknown content tool action".into()),
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Content rejected: {e}");
            ExitCode::from(2)
        }
    }
}
