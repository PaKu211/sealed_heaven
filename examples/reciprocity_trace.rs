use sealed_sim::living::{ActorId, PLAYER};
use serde_json::json;
use sha2::{Digest, Sha256};
#[path = "../tests/support/c48_trace.rs"]
mod trace;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn row(choice: &str, t: &trace::Trace, actor: ActorId) -> serde_json::Value {
    let w = &t.world;
    let n = w.reciprocity().unwrap().net(PLAYER, actor);
    json!({"choice":choice,"actor":actor.get(),"seed":w.region().unwrap().seed,"tick":w.tick(),
        "checkpoint_sha256":hex(&Sha256::digest(w.checkpoint())),"state_sha256":hex(&w.state_digest()),
        "balance":n,"packed":w.npc(actor).unwrap().packed_herbs,
        "purse":w.region().unwrap().coins,"player_herbs":w.player().herbs,
        "last_aid":w.known_aid().last().map(|e|format!("{:?}",e.kind)),
        "outcome":format!("{:?}",w.latest_outcome()),"commands":t.commands})
}
fn main() {
    let j = trace::journey();
    let mut rows = [
        ("reserve", &j.reserve),
        ("return", &j.returned),
        ("waive", &j.waived),
        ("part", &j.parted),
        ("two", &j.two),
        ("one", &j.one),
    ]
    .into_iter()
    .map(|(choice, t)| row(choice, t, trace::TRAVELER))
    .collect::<Vec<_>>();
    if let Some(t) = trace::clinic_journey() {
        rows.push(row("repay", &t, trace::MEDIC));
    }
    println!("{}", json!({"revision":35,"rows":rows}));
}
