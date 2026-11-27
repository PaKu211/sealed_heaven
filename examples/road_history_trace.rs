//! Developer replay fixture capture. Uses the same real canonical walking
//! trace as the client tests, then emits both branch commands and their hashes.
//! Never re-bless an existing golden automatically.
use sealed_sim::living::Intent;
use serde_json::json;
use sha2::{Digest, Sha256};
#[path = "../apps/sealed_playable/tests/support/c45_trace.rs"]
mod trace;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn main() {
    let mut rows = Vec::new();
    for entrust in [false, true] {
        let (mut w, mut commands) = trace::trace(entrust);
        let before = hex(&Sha256::digest(w.checkpoint()));
        let seed = w.region().unwrap().seed;
        let intent = if entrust {
            Intent::EntrustCache(1)
        } else {
            Intent::RecoverCache(1)
        };
        w.step(w.tick(), intent).unwrap();
        commands.push(if entrust {
            "entrustcache"
        } else {
            "recovercache"
        });
        rows.push(json!({"seed":seed,"choice":if entrust{"entrust"}else{"take"},"tick":w.tick(),"pre_choice_sha256":before,"checkpoint_sha256":hex(&Sha256::digest(w.checkpoint())),"state_sha256":hex(&w.state_digest()),"commands":commands}));
    }
    println!("{}", json!({"revision":32,"rows":rows}));
}
