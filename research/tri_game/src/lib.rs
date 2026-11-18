//! Post-C0.5 experiments. This crate calls the public simulation API and does not
//! replace the canonical scheduler, navigator, or resource ledger.

pub mod energy;
pub mod kernel;
pub mod logistics;
pub mod path;
pub mod schedule;

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).into()
}

pub fn nearest_rank(sorted: &[u128], percentile: f64) -> u128 {
    assert!(!sorted.is_empty());
    let rank = (percentile * sorted.len() as f64).ceil() as usize - 1;
    sorted[rank.min(sorted.len() - 1)]
}
