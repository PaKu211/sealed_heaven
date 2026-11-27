//! Capture deterministic real player choice traces for executable replay.
use sealed_sim::living::{World, requests::Site};
use serde_json::json;
use sha2::{Digest, Sha256};
#[path = "../tests/support/c46_trace.rs"]
mod trace;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn row(site: Site, choice: &str, id: u32, t: &trace::Trace, offer: &World) -> serde_json::Value {
    json!({"site":match site{Site::Shrine=>"shrine",Site::Market=>"market"},"choice":choice,"id":id,"seed":t.world.region().unwrap().seed,"tick":t.world.tick(),"offer_sha256":hex(&Sha256::digest(offer.checkpoint())),"checkpoint_sha256":hex(&Sha256::digest(t.world.checkpoint())),"state_sha256":hex(&t.world.state_digest()),"commands":t.commands})
}
fn main() {
    let mut rows = Vec::new();
    for site in Site::ALL {
        let j = trace::journey(site);
        let d = trace::decline(&j);
        let a = trace::abandon(&j);
        for (choice, t) in [("complete", &j.completed), ("decline", &d), ("abandon", &a)] {
            rows.push(row(site, choice, j.id, t, &j.offer.world));
        }
    }
    println!("{}", json!({"revision":33,"rows":rows}));
}
