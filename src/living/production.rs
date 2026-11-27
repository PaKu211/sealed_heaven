//! Exact L2 site processes. The calendar indexes due work, not ownership.
//! All actual herbs remain in the existing physical inventories.
use super::{
    Position, Region, World,
    content::{GARDEN_CAPACITY, ORDER_INTERVAL, ORDER_TRAVEL},
    expansion::{GROVE, GROVE_CAPACITY},
    navigation::in_reach,
};
use std::collections::{BTreeMap, BTreeSet};

pub const REVISION: u16 = 31;

/// Ordinal order is also the legacy within-turn phase order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum Job {
    WestGrowth,
    GardenGrowth,
    OrderArrival,
    OrderDispatch,
    GroveGrowth,
}
impl Job {
    pub const ALL: [Self; 5] = [
        Self::WestGrowth,
        Self::GardenGrowth,
        Self::OrderArrival,
        Self::OrderDispatch,
        Self::GroveGrowth,
    ];
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Record {
    pub executions: u64,
    /// Natural growth produces herbs. Order jobs count requested herbs only.
    pub quantity: u64,
    pub last_tick: u64,
    pub last_amount: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Production {
    agenda: BTreeSet<(u64, Job)>,
    records: BTreeMap<Job, Record>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SiteReport {
    pub job: Job,
    pub position: Position,
    pub stock: u32,
    pub capacity: u32,
    pub next_cycle: u64,
    pub interval: u64,
}
impl Production {
    pub(super) fn new(world: &World) -> Self {
        let mut state = Self {
            agenda: BTreeSet::new(),
            records: Job::ALL
                .into_iter()
                .map(|job| (job, Record::default()))
                .collect(),
        };
        for job in Job::ALL {
            if let Some(interval) = world.production_interval(job) {
                assert!(interval > 0);
                state.schedule(interval, job);
            }
        }
        state
    }
    fn schedule(&mut self, tick: u64, job: Job) {
        assert!(
            self.next_due(job).is_none(),
            "job already has a pending execution"
        );
        assert!(self.agenda.insert((tick, job)), "duplicate production job");
    }
    fn pop_due(&mut self, tick: u64) -> Option<Job> {
        let &(due, job) = self.agenda.first()?;
        if due > tick {
            return None;
        }
        assert_eq!(due, tick, "overdue production job");
        self.agenda.pop_first();
        Some(job)
    }
    fn record(&mut self, tick: u64, job: Job, amount: u32) {
        let record = self.records.get_mut(&job).expect("known site job");
        record.executions += 1;
        record.quantity += u64::from(amount);
        record.last_tick = tick;
        record.last_amount = amount;
    }
    pub fn agenda(&self) -> &BTreeSet<(u64, Job)> {
        &self.agenda
    }
    pub fn record_for(&self, job: Job) -> Record {
        self.records[&job]
    }
    pub fn next_due(&self, job: Job) -> Option<u64> {
        self.agenda
            .iter()
            .find_map(|&(tick, k)| (job == k).then_some(tick))
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"SH:C44:PRODUCTION:v1\0".to_vec();
        out.extend((self.agenda.len() as u32).to_le_bytes());
        for &(tick, job) in &self.agenda {
            out.extend(tick.to_le_bytes());
            out.push(job as u8);
        }
        out.extend((self.records.len() as u32).to_le_bytes());
        for (&job, record) in &self.records {
            out.push(job as u8);
            for n in [record.executions, record.quantity, record.last_tick] {
                out.extend(n.to_le_bytes());
            }
            out.extend(record.last_amount.to_le_bytes());
        }
        out
    }
}
impl World {
    pub fn production(&self) -> Option<&Production> {
        self.production.as_ref()
    }
    pub(super) fn production_interval(&self, job: Job) -> Option<u64> {
        let map = self
            .generated_region()
            .expect("production inherits generated map");
        match job {
            Job::WestGrowth => Some(map.west_regrow_every()),
            Job::GardenGrowth => Some(map.garden_regrow_every()),
            Job::OrderArrival => None,
            Job::OrderDispatch => Some(ORDER_INTERVAL),
            Job::GroveGrowth => Some(
                self.expanded_region()
                    .expect("expanded map")
                    .grove_regrow_every(),
            ),
        }
    }
    /// A local inspection, never a remote real-time oracle. No read mutates state.
    pub fn local_production(&self) -> Vec<SiteReport> {
        let Some(state) = self.production() else {
            return Vec::new();
        };
        let player = self.player();
        if !player.alive() {
            return Vec::new();
        }
        [
            (
                Job::WestGrowth,
                self.scenario.patch,
                self.supply.source,
                self.scenario.initial_source,
            ),
            (
                Job::GardenGrowth,
                Region::sites().garden,
                self.region().unwrap().garden_herbs,
                GARDEN_CAPACITY,
            ),
            (
                Job::GroveGrowth,
                GROVE,
                self.frontier().unwrap().grove_herbs,
                GROVE_CAPACITY,
            ),
        ]
        .into_iter()
        .filter(|&(_, position, _, _)| {
            in_reach(player.position, position) && self.player_sees(position)
        })
        .map(|(job, position, stock, capacity)| SiteReport {
            job,
            position,
            stock,
            capacity,
            next_cycle: state.next_due(job).expect("recurring job"),
            interval: self.production_interval(job).unwrap(),
        })
        .collect()
    }
    pub(super) fn advance_production(&mut self) {
        let Some(mut state) = self.production.take() else {
            self.regrow();
            self.advance_region();
            self.advance_frontier();
            return;
        };
        while let Some(job) = state.pop_due(self.tick) {
            let amount = match job {
                Job::WestGrowth => grow(
                    &mut self.supply.source,
                    &mut self.supply.regrown,
                    self.scenario.initial_source,
                ),
                Job::GardenGrowth => {
                    let r = self.region.as_mut().expect("regional site");
                    grow(&mut r.garden_herbs, &mut r.garden_regrown, GARDEN_CAPACITY)
                }
                Job::GroveGrowth => {
                    let f = self.frontier.as_mut().expect("frontier site");
                    grow(&mut f.grove_herbs, &mut f.regrown, GROVE_CAPACITY)
                }
                Job::OrderArrival => {
                    let r = self.region.as_mut().expect("regional order");
                    let (due, amount) = r.order_in_transit.take().expect("scheduled order");
                    assert_eq!(due, self.tick);
                    r.order_remaining = amount;
                    amount
                }
                Job::OrderDispatch => {
                    let amount = self.west_shortage();
                    let r = self.region.as_mut().expect("regional order");
                    // Only a request travels. This is not physical cargo.
                    if let Some((due, _)) = r.order_in_transit {
                        assert!(state.agenda.remove(&(due, Job::OrderArrival)));
                    }
                    let due = self.tick + ORDER_TRAVEL;
                    assert!(due > self.tick);
                    r.order_in_transit = Some((due, amount));
                    state.schedule(due, Job::OrderArrival);
                    amount
                }
            };
            state.record(self.tick, job, amount);
            if let Some(interval) = self.production_interval(job) {
                state.schedule(self.tick + interval, job);
            }
        }
        self.production = Some(state);
    }
}
fn grow(stock: &mut u32, produced: &mut u32, cap: u32) -> u32 {
    if *stock >= cap {
        0
    } else {
        *stock += 1;
        *produced += 1;
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Direction, Intent, MAX_TICKS, Outcome, Rules};
    fn compare(a: &World, b: &World) {
        let mut projected = a.clone();
        projected.production = None;
        projected.rules = Rules::ExactLod;
        assert_eq!(&projected, b, "gameplay divergence at tick {}", b.tick());
        assert_eq!(projected.state_digest(), b.state_digest());
    }
    fn both(a: &mut World, b: &mut World, intent: Intent) -> Outcome {
        let result = a.step(a.tick(), intent);
        assert_eq!(result, b.step(b.tick(), intent));
        compare(a, b);
        result.unwrap()
    }
    fn verify_agenda(w: &World) {
        let p = w.production().unwrap();
        let tick = w.tick();
        assert!(p.agenda().len() >= 4 && p.agenda().len() <= 5);
        assert!(p.agenda().iter().all(|(due, _)| *due > tick));
        for job in Job::ALL {
            let record = p.record_for(job);
            if let Some(interval) = w.production_interval(job) {
                assert_eq!(record.executions, tick / interval);
                assert_eq!(record.last_tick, (tick / interval) * interval);
                assert_eq!(p.next_due(job), Some((tick / interval + 1) * interval));
            } else {
                assert_eq!(
                    p.next_due(job),
                    w.region().unwrap().order_in_transit.map(|(due, _)| due)
                );
                assert_eq!(
                    record.executions,
                    tick.saturating_sub(ORDER_TRAVEL) / ORDER_INTERVAL
                );
            }
        }
        for (job, regrown) in [
            (Job::WestGrowth, w.supply().regrown),
            (Job::GardenGrowth, w.region().unwrap().garden_regrown),
            (Job::GroveGrowth, w.frontier().unwrap().regrown),
        ] {
            let record = p.record_for(job);
            assert_eq!(record.quantity, u64::from(regrown));
            assert!(record.quantity <= record.executions && record.last_amount <= 1);
        }
        let dispatched = p.record_for(Job::OrderDispatch);
        let arrived = p.record_for(Job::OrderArrival);
        let pending = w.region().unwrap().order_in_transit;
        assert_eq!(
            dispatched.executions,
            arrived.executions + u64::from(pending.is_some())
        );
        assert_eq!(
            dispatched.quantity,
            arrived.quantity + pending.map_or(0, |(_, amount)| u64::from(amount))
        );
    }

    #[test]
    fn scheduled_site_work_matches_c43_every_tick_and_preserves_order_accounting() {
        for seed in [0, 1, 2, 7, 42, u32::MAX] {
            let mut a = World::settlement_production_campaign(seed);
            let mut b = World::exact_lod_campaign(seed);
            compare(&a, &b);
            verify_agenda(&a);
            for _ in 0..2000 {
                both(&mut a, &mut b, Intent::Wait);
                verify_agenda(&a);
                if matches!(a.tick(), 48 | 77 | 78 | 1000 | 2000) {
                    assert_eq!(World::restore(&a.checkpoint()).unwrap(), a);
                }
            }
            let p = a.production().unwrap();
            println!(
                "seed={seed} growth_jobs={} herbs_produced={} orders_dispatched={} orders_arrived={}",
                [Job::WestGrowth, Job::GardenGrowth, Job::GroveGrowth]
                    .iter()
                    .map(|&j| p.record_for(j).executions)
                    .sum::<u64>(),
                [Job::WestGrowth, Job::GardenGrowth, Job::GroveGrowth]
                    .iter()
                    .map(|&j| p.record_for(j).quantity)
                    .sum::<u64>(),
                p.record_for(Job::OrderDispatch).executions,
                p.record_for(Job::OrderArrival).executions
            );
            assert!(p.record_for(Job::GroveGrowth).quantity > 0);
        }
    }

    #[test]
    fn mixed_actions_and_contention_keep_existing_gameplay_exact() {
        let actions = [
            Intent::Move(Direction::North),
            Intent::Move(Direction::East),
            Intent::Move(Direction::South),
            Intent::Move(Direction::West),
            Intent::Wait,
            Intent::Guard,
            Intent::Gather,
            Intent::Deliver,
            Intent::BuyHerb,
            Intent::SellHerb,
            Intent::Talk(super::super::ActorId::new(1)),
            Intent::Attack(super::super::ActorId::new(1)),
        ];
        for seed in [0, 1, 7, 42] {
            let mut a = World::settlement_production_campaign(seed);
            let mut b = World::exact_lod_campaign(seed);
            let mut rng = u64::from(seed) + 1;
            for _ in 0..1000 {
                rng = rng
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                both(
                    &mut a,
                    &mut b,
                    actions[((rng >> 32) as usize) % actions.len()],
                );
                verify_agenda(&a);
            }
            assert_eq!(World::restore(&a.checkpoint()).unwrap(), a);
        }
    }

    #[test]
    fn growth_checks_have_exactly_once_capacity_and_boundary_semantics() {
        let mut w = World::settlement_production_campaign(7);
        let interval = w.production_interval(Job::WestGrowth).unwrap();
        for tick in 1..=interval {
            w.tick = tick;
            w.advance_production();
        }
        assert_eq!(
            w.production()
                .unwrap()
                .record_for(Job::WestGrowth)
                .last_amount,
            0
        );
        assert_eq!(w.supply.regrown, 0);
        w.supply.source -= 1;
        for tick in interval + 1..2 * interval {
            w.tick = tick;
            w.advance_production();
            assert_eq!(w.supply.regrown, 0);
        }
        w.tick = 2 * interval;
        w.advance_production();
        assert_eq!(w.supply.source, w.scenario.initial_source);
        assert_eq!(w.supply.regrown, 1);
        assert_eq!(
            w.production().unwrap().record_for(Job::WestGrowth).quantity,
            1
        );
        let before = w.clone();
        w.advance_production();
        assert_eq!(w, before);
        let result = w.step(w.tick() - 1, Intent::Wait);
        assert!(result.is_err());
        assert_eq!(w, before);
    }

    #[test]
    fn local_inspection_never_reveals_remote_stock_and_is_read_only() {
        let mut w = World::settlement_production_campaign(7);
        for (job, pos) in [
            (Job::WestGrowth, w.scenario.patch),
            (Job::GardenGrowth, Region::sites().garden),
            (Job::GroveGrowth, GROVE),
        ] {
            w.player.position = pos;
            let hash = w.state_digest();
            let reports = w.local_production();
            assert_eq!(reports.len(), 1);
            assert_eq!(reports[0].job, job);
            assert_eq!(reports[0].next_cycle, reports[0].interval);
            assert_eq!(hash, w.state_digest());
        }
        w.player.hp = 0;
        assert!(w.local_production().is_empty());
        assert!(World::exact_lod_campaign(7).local_production().is_empty());
    }

    #[test]
    fn future_agenda_and_execution_record_are_hash_bound() {
        let mut w = World::settlement_production_campaign(7);
        for _ in 0..48 {
            w.step(w.tick(), Intent::Wait).unwrap();
        }
        assert!(
            w.production()
                .unwrap()
                .next_due(Job::OrderArrival)
                .is_some()
        );
        let before = w.checkpoint();
        assert_eq!(World::restore(&before).unwrap(), w);
        let mut changed = w.clone();
        let p = changed.production.as_mut().unwrap();
        let tick = p.next_due(Job::OrderArrival).unwrap();
        assert!(p.agenda.remove(&(tick, Job::OrderArrival)));
        p.schedule(tick + 1, Job::OrderArrival);
        assert_ne!(changed.state_digest(), w.state_digest());
        assert_eq!(
            World::restore(&changed.checkpoint()),
            Err(crate::living::LoadError::StateMismatch)
        );
        let mut changed = w.clone();
        changed
            .production
            .as_mut()
            .unwrap()
            .records
            .get_mut(&Job::WestGrowth)
            .unwrap()
            .quantity += 1;
        assert_eq!(
            World::restore(&changed.checkpoint()),
            Err(crate::living::LoadError::StateMismatch)
        );
    }

    #[test]
    #[should_panic(expected = "job already has a pending execution")]
    fn duplicate_job_identity_at_another_tick_is_rejected() {
        let mut w = World::settlement_production_campaign(7);
        let p = w.production.as_mut().unwrap();
        let due = p.next_due(Job::WestGrowth).unwrap();
        p.schedule(due + 1, Job::WestGrowth);
    }

    #[test]
    fn actual_journey_across_all_production_sites_matches_reference() {
        use crate::living::navigation::{distance, next_step};
        let mut a = World::settlement_production_campaign(7);
        let mut b = World::exact_lod_campaign(7);
        both(
            &mut a,
            &mut b,
            Intent::ChooseBackground(crate::living::Background::Herbalist),
        );
        for (job, target) in [
            (Job::WestGrowth, a.scenario.patch),
            (Job::GardenGrowth, Region::sites().garden),
            (Job::GroveGrowth, GROVE),
        ] {
            for _ in 0..500 {
                let from = a.player.position;
                if distance(from, target) <= 1 {
                    break;
                }
                let next = next_step(a.scenario, from, target, 1, |p| {
                    a.walkable(p) && !a.occupied(p)
                });
                let intent = next.map_or(Intent::Wait, |p| {
                    Intent::Move(if p.x > from.x {
                        Direction::East
                    } else if p.x < from.x {
                        Direction::West
                    } else if p.y > from.y {
                        Direction::South
                    } else {
                        Direction::North
                    })
                });
                both(&mut a, &mut b, intent);
            }
            let before = a.local_production();
            assert_eq!(before.len(), 1, "site not reached");
            assert_eq!(before[0].job, job);
            both(&mut a, &mut b, Intent::Gather);
            let after = a.local_production();
            assert_eq!(after.len(), 1);
            assert_eq!(after[0].job, job);
            verify_agenda(&a);
            assert_eq!(World::restore(&a.checkpoint()).unwrap(), a);
        }
    }

    #[test]
    fn full_horizon_does_not_overflow_due_dates_or_counts() {
        let mut w = World::settlement_production_campaign(42);
        // Exercise the site calendar at the complete supported logical horizon
        // without claiming this fixture simulates actors or combat.
        for tick in 1..=MAX_TICKS {
            w.tick = tick;
            w.advance_production();
            verify_agenda(&w);
        }
        assert!(
            w.production()
                .unwrap()
                .agenda()
                .iter()
                .all(|&(due, _)| due > MAX_TICKS)
        );
    }
}
