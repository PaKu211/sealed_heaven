use sealed_sim::living::{
    Intent, LoadError, World,
    expansion::CARRIER,
    lod::{Mode, REVISION},
};
use sha2::{Digest, Sha256};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn rechecksum(bytes: &mut [u8]) {
    let at = bytes.len() - 32;
    let digest = Sha256::digest(&bytes[..at]);
    bytes[at..].copy_from_slice(&digest);
}

#[test]
fn c43_checkpoint_binds_scheduler_and_never_relabels_historical_saves() {
    let mut w = World::exact_lod_campaign(7);
    for _ in 0..180 {
        w.step(w.tick(), Intent::Wait).unwrap();
    }
    let s = w.scheduler().unwrap();
    assert_eq!(s.entry(CARRIER).unwrap().mode, Mode::Scheduled);
    assert!(s.evaluation_totals().1 > 0);
    let bytes = w.checkpoint();
    assert_eq!(&bytes[12..14], &REVISION.to_le_bytes());
    assert_eq!(World::restore(&bytes).unwrap(), w);
    let mut bad = bytes.clone();
    let at = bad.len() - 64;
    bad[at] ^= 1;
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::StateMismatch));
    let mut bad = bytes.clone();
    bad[12..14].copy_from_slice(&29u16.to_le_bytes());
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::ScenarioMismatch));
    let old = World::regional_knowledge_campaign(7).checkpoint();
    let mut bad = old.clone();
    bad[12..14].copy_from_slice(&REVISION.to_le_bytes());
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::ScenarioMismatch));
    assert_eq!(World::restore(&old).unwrap().checkpoint(), old);
}

#[test]
fn exact_lod_v1_golden_checkpoints_and_schedule_are_exact() {
    let mut rows = Vec::new();
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::exact_lod_campaign(seed);
        let initial = hash(&w.checkpoint());
        for _ in 0..1000 {
            w.step(w.tick(), Intent::Wait).unwrap();
        }
        let (full, routine) = w.scheduler().unwrap().evaluation_totals();
        rows.push(format!(
            "{seed},{initial},{},{},{full},{routine}",
            hash(&w.checkpoint()),
            hex(&w.state_digest())
        ));
    }
    assert_eq!(
        rows,
        include_str!("fixtures/living/c43_exact_lod_v1.csv")
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
    );
}
