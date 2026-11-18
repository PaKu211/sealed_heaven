//! Tile BFS compared with a two-level regional search. This is not the frozen
//! navigator in `src/fixture3/navigation.rs`; the tile search copies its public
//! neighbor order so the comparison is about the algorithm, not a patched kernel.

use rand_chacha::ChaCha12Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use std::collections::VecDeque;
use std::time::Instant;

pub const REGION: u32 = 8;

#[derive(Clone, Debug)]
pub struct Map {
    pub width: u32,
    pub height: u32,
    open: Vec<bool>,
    component: Vec<u32>,
    adjacent: Vec<Vec<u32>>,
}

#[derive(Clone, Debug, Default)]
pub struct SearchStats {
    pub found: bool,
    pub tile_visits: u64,
    pub region_visits: u64,
    pub fallback: bool,
    pub steps: usize,
}

#[derive(Clone, Debug)]
pub struct Query {
    pub start: (u32, u32),
    pub goal: (u32, u32),
}

pub fn demo_wall(size: u32, seed: u64) -> Map {
    let mut open = vec![true; (size * size) as usize];
    for index in 0..size {
        close(&mut open, size, index, 0);
        close(&mut open, size, index, size - 1);
        close(&mut open, size, 0, index);
        close(&mut open, size, size - 1, index);
    }
    let wall = size / 2;
    let gap = size / 2;
    for y in 1..size - 1 {
        if y != gap {
            close(&mut open, size, wall, y);
        }
    }
    let mut rng = ChaCha12Rng::seed_from_u64(seed);
    let interior = ((size - 2) * (size - 2)) as usize;
    let extra = interior / 10;
    let mut placed = 0usize;
    while placed < extra {
        let x = 1 + range_u32(&mut rng, size - 2);
        let y = 1 + range_u32(&mut rng, size - 2);
        if x == wall || (x == wall && y == gap) {
            continue;
        }
        let index = (y * size + x) as usize;
        if open[index] {
            open[index] = false;
            placed += 1;
        }
    }
    Map::from_open(size, size, open)
}

impl Map {
    pub fn from_open(width: u32, height: u32, open: Vec<bool>) -> Self {
        let mut map = Self {
            width,
            height,
            open,
            component: Vec::new(),
            adjacent: Vec::new(),
        };
        map.rebuild();
        map
    }

    pub fn open_at(&self, x: u32, y: u32) -> bool {
        self.open[(y * self.width + x) as usize]
    }

    pub fn rebuild(&mut self) {
        self.component = vec![0; self.open.len()];
        let mut next = 1u32;
        for index in 0..self.open.len() {
            if !self.open[index] || self.component[index] != 0 {
                continue;
            }
            let mut queue = VecDeque::from([index]);
            self.component[index] = next;
            while let Some(current) = queue.pop_front() {
                let x = (current as u32) % self.width;
                let y = (current as u32) / self.width;
                for neighbor in neighbors(self.width, self.height, x, y) {
                    let next_index = (neighbor.1 * self.width + neighbor.0) as usize;
                    if self.open[next_index] && self.component[next_index] == 0 {
                        self.component[next_index] = next;
                        queue.push_back(next_index);
                    }
                }
            }
            next += 1;
        }
        let regions = self.region_count();
        let mut adjacent = vec![Vec::new(); regions as usize];
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.open_at(x, y) {
                    continue;
                }
                let region = self.region(x, y);
                for (nx, ny, _) in neighbors(self.width, self.height, x, y) {
                    if self.open_at(nx, ny) {
                        let other = self.region(nx, ny);
                        if other != region {
                            let list = &mut adjacent[region as usize];
                            if !list.contains(&other) {
                                list.push(other);
                            }
                        }
                    }
                }
            }
        }
        for list in &mut adjacent {
            list.sort_unstable();
        }
        self.adjacent = adjacent;
    }

    pub fn block_open_fraction(&mut self, numerator: u32, denominator: u32, seed: u64) -> usize {
        let candidates: Vec<usize> = self
            .open
            .iter()
            .enumerate()
            .filter_map(|(index, open)| open.then_some(index))
            .collect();
        let target = candidates.len() * numerator as usize / denominator as usize;
        let mut rng = ChaCha12Rng::seed_from_u64(seed);
        let mut order = candidates;
        for index in (1..order.len()).rev() {
            let swap = range_usize(&mut rng, index + 1);
            order.swap(index, swap);
        }
        for index in order.into_iter().take(target) {
            self.open[index] = false;
        }
        self.rebuild();
        target
    }

    pub fn tile_bfs(&self, start: (u32, u32), goal: (u32, u32)) -> (Option<Vec<u8>>, SearchStats) {
        self.bfs(start, goal, |_| true, false)
    }

    pub fn hierarchical(
        &self,
        start: (u32, u32),
        goal: (u32, u32),
    ) -> (Option<Vec<u8>>, SearchStats) {
        if !self.open_at(start.0, start.1) || !self.open_at(goal.0, goal.1) {
            return (None, SearchStats::default());
        }
        let start_component = self.component[(start.1 * self.width + start.0) as usize];
        let goal_component = self.component[(goal.1 * self.width + goal.0) as usize];
        if start_component != goal_component {
            return (None, SearchStats::default());
        }
        let start_region = self.region(start.0, start.1);
        let goal_region = self.region(goal.0, goal.1);
        if start_region == goal_region {
            return self.bfs(start, goal, |_| true, false);
        }
        let (regions, region_visits) = self.region_path(start_region, goal_region);
        let Some(regions) = regions else {
            return (
                None,
                SearchStats {
                    region_visits,
                    ..SearchStats::default()
                },
            );
        };
        let allowed =
            |cell: (u32, u32)| regions.binary_search(&self.region(cell.0, cell.1)).is_ok();
        let (path, mut stats) = self.bfs(start, goal, allowed, false);
        stats.region_visits = region_visits;
        if path.is_some() {
            return (path, stats);
        }
        let (fallback, mut fallback_stats) = self.bfs(start, goal, |_| true, false);
        fallback_stats.tile_visits += stats.tile_visits;
        fallback_stats.region_visits = region_visits;
        fallback_stats.fallback = true;
        (fallback, fallback_stats)
    }

    fn region_count(&self) -> u32 {
        self.width.div_ceil(REGION) * self.height.div_ceil(REGION)
    }

    fn region(&self, x: u32, y: u32) -> u32 {
        let columns = self.width.div_ceil(REGION);
        (y / REGION) * columns + (x / REGION)
    }

    fn region_path(&self, start: u32, goal: u32) -> (Option<Vec<u32>>, u64) {
        let mut queue = VecDeque::from([start]);
        let mut parent = vec![u32::MAX; self.adjacent.len()];
        parent[start as usize] = start;
        let mut visits = 0u64;
        while let Some(region) = queue.pop_front() {
            visits += 1;
            if region == goal {
                break;
            }
            for &next in &self.adjacent[region as usize] {
                if parent[next as usize] == u32::MAX {
                    parent[next as usize] = region;
                    queue.push_back(next);
                }
            }
        }
        if parent[goal as usize] == u32::MAX {
            return (None, visits);
        }
        let mut path = Vec::new();
        let mut cursor = goal;
        loop {
            path.push(cursor);
            if cursor == start {
                break;
            }
            cursor = parent[cursor as usize];
        }
        path.reverse();
        path.sort_unstable();
        let unique = path.clone();
        (Some(unique), visits)
    }

    fn bfs(
        &self,
        start: (u32, u32),
        goal: (u32, u32),
        allowed: impl Fn((u32, u32)) -> bool,
        _fallback: bool,
    ) -> (Option<Vec<u8>>, SearchStats) {
        if start == goal && self.open_at(start.0, start.1) && allowed(start) {
            return (
                Some(Vec::new()),
                SearchStats {
                    found: true,
                    ..SearchStats::default()
                },
            );
        }
        let mut queue = VecDeque::from([(start, Vec::new())]);
        let mut seen = vec![false; self.open.len()];
        seen[(start.1 * self.width + start.0) as usize] = true;
        let mut visits = 0u64;
        while let Some(((x, y), path)) = queue.pop_front() {
            visits += 1;
            if (x, y) == goal {
                return (
                    Some(path.clone()),
                    SearchStats {
                        found: true,
                        tile_visits: visits,
                        steps: path.len(),
                        ..SearchStats::default()
                    },
                );
            }
            for (nx, ny, direction) in neighbors(self.width, self.height, x, y) {
                let index = (ny * self.width + nx) as usize;
                if self.open[index] && !seen[index] && allowed((nx, ny)) {
                    seen[index] = true;
                    let mut next = path.clone();
                    next.push(direction);
                    queue.push_back(((nx, ny), next));
                }
            }
        }
        (
            None,
            SearchStats {
                tile_visits: visits,
                ..SearchStats::default()
            },
        )
    }
}

pub fn neighbors(width: u32, height: u32, x: u32, y: u32) -> Vec<(u32, u32, u8)> {
    let mut found = Vec::with_capacity(4);
    let candidates = [
        (x, y.wrapping_add(1), 1u8),
        (x.wrapping_add(1), y, 2),
        (x, y.wrapping_sub(1), 3),
        (x.wrapping_sub(1), y, 4),
    ];
    for (nx, ny, direction) in candidates {
        if direction == 3 && y == 0 || direction == 4 && x == 0 {
            continue;
        }
        if nx < width && ny < height {
            found.push((nx, ny, direction));
        }
    }
    found
}

pub fn queries(map: &Map, count: usize, seed: u64) -> Vec<Query> {
    let open: Vec<(u32, u32)> = (0..map.height)
        .flat_map(|y| (0..map.width).map(move |x| (x, y)))
        .filter(|(x, y)| map.open_at(*x, *y))
        .collect();
    let mut rng = ChaCha12Rng::seed_from_u64(seed);
    let mut queries = Vec::with_capacity(count);
    while queries.len() < count && !open.is_empty() {
        let start = open[range_usize(&mut rng, open.len())];
        let goal = open[range_usize(&mut rng, open.len())];
        queries.push(Query { start, goal });
    }
    queries
}

#[derive(Clone, Debug)]
pub struct PathBenchmark {
    pub size: u32,
    pub queries: usize,
    pub tile_mean_ns: u128,
    pub tile_p95_ns: u128,
    pub hierarchical_mean_ns: u128,
    pub hierarchical_p95_ns: u128,
    pub tile_mean_visits: f64,
    pub hierarchical_mean_visits: f64,
    pub agreement: usize,
    pub hierarchical_fallbacks: usize,
    pub component_rejections: usize,
    pub invalidation_ns: u128,
    pub invalidated_cells: usize,
    pub cache_drop_ns: u128,
    pub cached_paths: usize,
}

pub fn benchmark(size: u32, query_count: usize, seed: u64) -> PathBenchmark {
    let map = demo_wall(size, seed);
    let queries = queries(&map, query_count, seed ^ 0xA5A5);
    let mut tile_times = Vec::with_capacity(queries.len());
    let mut hierarchical_times = Vec::with_capacity(queries.len());
    let mut tile_visits = 0u64;
    let mut hierarchical_visits = 0u64;
    let mut agreement = 0usize;
    let mut fallbacks = 0usize;
    let mut rejections = 0usize;
    type CachedPath = ((u32, u32), (u32, u32), Vec<u8>);
    let mut cache: Vec<CachedPath> = Vec::new();
    for query in &queries {
        let started = Instant::now();
        let (tile, tile_stats) = map.tile_bfs(query.start, query.goal);
        tile_times.push(started.elapsed().as_nanos());
        tile_visits += tile_stats.tile_visits;
        let started = Instant::now();
        let (hierarchical, hierarchical_stats) = map.hierarchical(query.start, query.goal);
        hierarchical_times.push(started.elapsed().as_nanos());
        hierarchical_visits += hierarchical_stats.tile_visits + hierarchical_stats.region_visits;
        fallbacks += usize::from(hierarchical_stats.fallback);
        if tile.is_none()
            && map.component[(query.start.1 * map.width + query.start.0) as usize]
                != map.component[(query.goal.1 * map.width + query.goal.0) as usize]
        {
            rejections += 1;
        }
        if paths_agree(
            &map,
            query.start,
            query.goal,
            tile.as_deref(),
            hierarchical.as_deref(),
        ) {
            agreement += 1;
        }
        if let Some(path) = tile.clone() {
            cache.push((query.start, query.goal, path));
        }
    }
    let mut changed = map.clone();
    let started = Instant::now();
    let invalidated_cells = changed.block_open_fraction(1, 10, seed ^ 0x0B57);
    let invalidation_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    cache.retain(|(start, _goal, path)| path_is_open(&changed, *start, path));
    let cache_drop_ns = started.elapsed().as_nanos();
    tile_times.sort_unstable();
    hierarchical_times.sort_unstable();
    let denom = queries.len() as f64;
    PathBenchmark {
        size,
        queries: queries.len(),
        tile_mean_ns: mean(&tile_times),
        tile_p95_ns: crate::nearest_rank(&tile_times, 0.95),
        hierarchical_mean_ns: mean(&hierarchical_times),
        hierarchical_p95_ns: crate::nearest_rank(&hierarchical_times, 0.95),
        tile_mean_visits: tile_visits as f64 / denom,
        hierarchical_mean_visits: hierarchical_visits as f64 / denom,
        agreement,
        hierarchical_fallbacks: fallbacks,
        component_rejections: rejections,
        invalidation_ns,
        invalidated_cells,
        cache_drop_ns,
        cached_paths: cache.len(),
    }
}

pub fn paths_agree(
    map: &Map,
    start: (u32, u32),
    goal: (u32, u32),
    left: Option<&[u8]>,
    right: Option<&[u8]>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            walk(map, start, left) == Some(goal) && walk(map, start, right) == Some(goal)
        }
        _ => false,
    }
}

pub fn path_is_open(map: &Map, start: (u32, u32), path: &[u8]) -> bool {
    walk(map, start, path).is_some()
}

fn walk(map: &Map, start: (u32, u32), path: &[u8]) -> Option<(u32, u32)> {
    let mut x = start.0;
    let mut y = start.1;
    if !map.open_at(x, y) {
        return None;
    }
    for direction in path {
        let next = match direction {
            1 => (x, y.checked_add(1)?),
            2 => (x.checked_add(1)?, y),
            3 => (x, y.checked_sub(1)?),
            4 => (x.checked_sub(1)?, y),
            _ => return None,
        };
        if next.0 >= map.width || next.1 >= map.height || !map.open_at(next.0, next.1) {
            return None;
        }
        (x, y) = next;
    }
    Some((x, y))
}

fn mean(values: &[u128]) -> u128 {
    values.iter().sum::<u128>() / values.len() as u128
}

fn close(open: &mut [bool], width: u32, x: u32, y: u32) {
    open[(y * width + x) as usize] = false;
}

pub fn parallel_digest(map: &Map, queries: &[Query], workers: usize) -> [u8; 32] {
    assert!(workers > 0);
    let mut partials = Vec::with_capacity(workers);
    std::thread::scope(|scope| {
        let mut joins = Vec::with_capacity(workers);
        for worker in 0..workers {
            joins.push(scope.spawn(move || {
                let mut digests = Vec::new();
                for (index, query) in queries.iter().enumerate() {
                    if index % workers != worker {
                        continue;
                    }
                    let (path, _) = map.hierarchical(query.start, query.goal);
                    digests.push((index, crate::sha256(path.as_deref().unwrap_or(&[]))));
                }
                digests
            }));
        }
        for join in joins {
            partials.push(join.join().expect("path worker"));
        }
    });
    let mut ordered = partials.concat();
    ordered.sort_by_key(|(index, _)| *index);
    let flat: Vec<u8> = ordered.into_iter().flat_map(|(_, digest)| digest).collect();
    crate::sha256(&flat)
}

fn range_u32(rng: &mut ChaCha12Rng, limit: u32) -> u32 {
    (rng.next_u64() % u64::from(limit)) as u32
}

fn range_usize(rng: &mut ChaCha12Rng, limit: usize) -> usize {
    (rng.next_u64() as usize) % limit
}
