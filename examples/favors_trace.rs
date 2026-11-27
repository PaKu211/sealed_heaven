//! Record actual canonical travel favor branches.
use sealed_sim::living::World;
use serde_json::json;
use sha2::{Digest, Sha256};
#[path = "../tests/support/c47_trace.rs"]
mod trace;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn row(choice: &str, id: u32, t: &trace::Trace, offer: &World) -> serde_json::Value {
    let actor = trace::TRAVELER;
    let r = t
        .world
        .favors()
        .unwrap()
        .record(actor)
        .unwrap()
        .current
        .unwrap();
    json!({"choice":choice,"id":id,"actor":actor.get(),"seed":t.world.region().unwrap().seed,"tick":t.world.tick(),
        "offer_tick":offer.tick(),"offer_sha256":hex(&Sha256::digest(offer.checkpoint())),
        "checkpoint_sha256":hex(&Sha256::digest(t.world.checkpoint())),"state_sha256":hex(&t.world.state_digest()),
        "regard":t.world.social().relationship(actor,sealed_sim::living::PLAYER).regard,
        "regard_change":r.regard_change,"status":format!("{:?}",r.status),"commands":t.commands})
}
fn main() {
    let j = trace::journey();
    let d = trace::decline(&j);
    let w = trace::withdraw(&j);
    let p = trace::part(&j);
    let u = trace::unpromised(&j);
    let rows = [
        ("fulfill", &j.fulfilled),
        ("decline", &d),
        ("withdraw", &w),
        ("part", &p),
        ("unpromised", &u),
    ]
    .into_iter()
    .map(|(choice, t)| row(choice, j.id, t, &j.offer.world))
    .collect::<Vec<_>>();
    println!("{}", json!({"revision":34,"rows":rows}));
}
