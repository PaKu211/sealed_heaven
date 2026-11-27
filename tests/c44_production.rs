use sealed_sim::living::{
    Intent, LoadError, World,
    production::{Job, REVISION},
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
fn c44_checkpoint_isolated_from_all_historical_rules() {
    let mut w = World::settlement_production_campaign(7);
    for _ in 0..77 {
        w.step(w.tick(), Intent::Wait).unwrap();
    }
    let bytes = w.checkpoint();
    assert_eq!(&bytes[12..14], &REVISION.to_le_bytes());
    assert_eq!(
        w.production().unwrap().next_due(Job::OrderArrival),
        Some(78)
    );
    let mut restored = World::restore(&bytes).unwrap();
    assert_eq!(restored, w);
    restored.step(77, Intent::Wait).unwrap();
    w.step(77, Intent::Wait).unwrap();
    assert_eq!(w, restored);
    assert_eq!(
        w.production()
            .unwrap()
            .record_for(Job::OrderArrival)
            .executions,
        1
    );
    let mut bad = bytes.clone();
    let at = bad.len() - 64;
    bad[at] ^= 1;
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::StateMismatch));
    let mut bad = bytes.clone();
    bad[12..14].copy_from_slice(&30u16.to_le_bytes());
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::ScenarioMismatch));
    let old = World::exact_lod_campaign(7).checkpoint();
    let mut bad = old.clone();
    bad[12..14].copy_from_slice(&REVISION.to_le_bytes());
    rechecksum(&mut bad);
    assert_eq!(World::restore(&bad), Err(LoadError::ScenarioMismatch));
    assert_eq!(World::restore(&old).unwrap().checkpoint(), old);
}

#[test]
fn production_v1_golden_checkpoints_and_calendar_are_exact() {
    let mut rows = Vec::new();
    for seed in [0, 1, 2, 7, 42, u32::MAX] {
        let mut w = World::settlement_production_campaign(seed);
        let initial = hash(&w.checkpoint());
        for _ in 0..1000 {
            w.step(w.tick(), Intent::Wait).unwrap();
        }
        let p = w.production().unwrap();
        let counts = Job::ALL.map(|j| p.record_for(j));
        let executions = counts.iter().map(|r| r.executions).sum::<u64>();
        let grown = [Job::WestGrowth, Job::GardenGrowth, Job::GroveGrowth]
            .iter()
            .map(|&j| p.record_for(j).quantity)
            .sum::<u64>();
        rows.push(format!(
            "{seed},{initial},{},{},{executions},{grown}",
            hash(&w.checkpoint()),
            hex(&w.state_digest())
        ));
    }
    assert_eq!(
        rows,
        include_str!("fixtures/living/c44_production_v1.csv")
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
    );
}
