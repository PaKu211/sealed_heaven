#[path = "build_support/registry.rs"]
mod registry;

use std::{env, fs, path::PathBuf};

// Pinned after comparing generated v1 content with the original scenarios and
// baseline checkpoints. Replacing legacy content requires a new rules binding.
const LEGACY_V1_SHA256: &str = "5d33f089bf1086ed4e9465f4ef3be8254b8090c6a35fcbdf9a2e9f9bb0451b80";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_support/registry.rs");
    println!("cargo:rerun-if-changed=content/v1/core.json");
    println!("cargo:rerun-if-changed=content/v1/sealed_heaven.json");
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let read = |name| {
        registry::read_bounded(&root.join("content/v1").join(name)).expect("read authored content")
    };
    let generated = registry::compile(&read("core.json"), &read("sealed_heaven.json"))
        .unwrap_or_else(|e| panic!("content registry rejected: {e}"));
    assert_eq!(
        generated.fingerprint, LEGACY_V1_SHA256,
        "legacy content is immutable; changing content v1 would invalidate historical rules"
    );
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));
    fs::write(out.join("living_registry.rs"), generated.rust).expect("write static registry");
    fs::write(out.join("living_registry.json"), generated.manifest).expect("write manifest");
}
