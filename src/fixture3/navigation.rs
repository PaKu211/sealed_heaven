//! Deterministic, integer-only search and authoritative actor movement.
use super::*;
use std::collections::VecDeque;

pub(super) fn path_cost(w: &World, steps: &[PathStep]) -> Result<u64, StepError> {
    steps.iter().try_fold(0u64, |sum, step| {
        let cost = match step {
            PathStep::Grid(_) => 1,
            PathStep::Edge(id) => {
                w.edges
                    .iter()
                    .find(|e| e.2 == *id)
                    .ok_or(StepError::InvalidState)?
                    .3
            }
        };
        sum.checked_add(cost).ok_or(StepError::CapacityExceeded)
    })
}

pub(crate) fn route(w: &World, from: u64, to: u64) -> Result<Option<Vec<PathStep>>, StepError> {
    if !w.sites.contains_key(&from) || !w.sites.contains_key(&to) {
        return Ok(None);
    }
    // The queue orders cost, then the entire (destination, edge) path.
    let mut queue = BTreeSet::from([(0u64, Vec::<(u64, u64)>::new(), from)]);
    let mut best = BTreeMap::from([(from, (0u64, Vec::<(u64, u64)>::new()))]);
    while let Some((cost, path, site)) = queue.pop_first() {
        if best.get(&site) != Some(&(cost, path.clone())) {
            continue;
        }
        if site == to {
            return Ok(Some(
                path.into_iter()
                    .map(|(_, edge)| PathStep::Edge(edge))
                    .collect(),
            ));
        }
        let mut successors: Vec<_> = w.edges.iter().filter(|e| e.0 == site).collect();
        successors.sort_by_key(|e| (e.1, e.2));
        for &&(_, dest, edge, duration) in &successors {
            let next_cost = cost
                .checked_add(duration)
                .ok_or(StepError::CapacityExceeded)?;
            let mut next_path = path.clone();
            next_path.push((dest, edge));
            let candidate = (next_cost, next_path.clone());
            if best.get(&dest).is_none_or(|old| candidate < *old) {
                best.insert(dest, candidate);
                queue.insert((next_cost, next_path, dest));
            }
        }
    }
    Ok(None)
}

fn cell(w: &World, x: u32, y: u32) -> bool {
    x < w.grid_w && y < w.grid_h && w.grid.get((y * w.grid_w + x) as usize) == Some(&1)
}

fn neighbor(w: &World, x: u32, y: u32, direction: u8) -> Option<(u32, u32)> {
    let (x, y) = match direction {
        1 => (x, y.checked_add(1)?),
        2 => (x.checked_add(1)?, y),
        3 => (x, y.checked_sub(1)?),
        4 => (x.checked_sub(1)?, y),
        _ => return None,
    };
    cell(w, x, y).then_some((x, y))
}

pub(super) fn search(w: &World, a: &Actor, t: &Task) -> Result<Option<Vec<PathStep>>, StepError> {
    if a.locus == 1 && t.target_locus == 1 {
        return route(w, a.site, t.target_site);
    }
    if a.locus != 2
        || t.target_locus != 2
        || a.site != t.target_site
        || a.site != w.grid_site
        || !cell(w, a.x, a.y)
        || !cell(w, t.target_x, t.target_y)
    {
        return Ok(None);
    }
    let mut queue = VecDeque::from([((a.x, a.y), Vec::new())]);
    let mut seen = BTreeSet::from([(a.x, a.y)]);
    while let Some(((x, y), path)) = queue.pop_front() {
        if (x, y) == (t.target_x, t.target_y) {
            return Ok(Some(path));
        }
        for direction in 1..=4 {
            if let Some(next) = neighbor(w, x, y, direction) {
                if seen.insert(next) {
                    let mut path = path.clone();
                    path.push(PathStep::Grid(direction));
                    queue.push_back((next, path));
                }
            }
        }
    }
    Ok(None)
}

pub(super) fn spend_edge(w: &mut World, actor_id: u32) -> Result<(), StepError> {
    let a = w.actors.get_mut(&actor_id).ok_or(StepError::InvalidState)?;
    let edge = w
        .edges
        .iter()
        .find(|e| e.2 == a.edge)
        .ok_or(StepError::InvalidState)?;
    if a.traversed > edge.3 {
        return Err(StepError::InvalidState);
    }
    if a.traversed < edge.3 {
        a.traversed = a
            .traversed
            .checked_add(1)
            .ok_or(StepError::CapacityExceeded)?;
    }
    if a.traversed == edge.3 {
        a.locus = 1;
        a.site = edge.1;
        a.edge = 0;
        a.traversed = 0;
    }
    Ok(())
}

pub(super) fn step(w: &mut World, task_id: u64) -> Result<(), StepError> {
    let t = &w.tasks[&task_id];
    let actor_id = t.actor_id;
    let index = usize::try_from(t.path_index).map_err(|_| StepError::InvalidState)?;
    let step = t.steps.get(index).cloned();
    let a = &w.actors[&actor_id];
    if a.locus == 3 {
        if step.as_ref().is_some_and(|s| *s != PathStep::Edge(a.edge)) {
            return Err(StepError::InvalidState);
        }
        spend_edge(w, actor_id)?;
        if w.actors[&actor_id].locus == 1 && step.is_some() {
            w.tasks.get_mut(&task_id).unwrap().path_index += 1;
        }
        return Ok(());
    }
    match step.ok_or(StepError::InvalidState)? {
        PathStep::Edge(id) => {
            let edge = w
                .edges
                .iter()
                .find(|e| e.2 == id)
                .ok_or(StepError::InvalidState)?;
            if a.locus != 1 || a.site != edge.0 {
                return Err(StepError::InvalidState);
            }
            let a = w.actors.get_mut(&actor_id).unwrap();
            a.locus = 3;
            a.site = 0;
            a.edge = id;
            a.traversed = 0;
            spend_edge(w, actor_id)?;
            if w.actors[&actor_id].locus == 1 {
                w.tasks.get_mut(&task_id).unwrap().path_index += 1;
            }
        }
        PathStep::Grid(direction) => {
            if a.locus != 2 || a.site != w.grid_site {
                return Err(StepError::InvalidState);
            }
            let (x, y) = neighbor(w, a.x, a.y, direction).ok_or(StepError::InvalidState)?;
            let a = w.actors.get_mut(&actor_id).unwrap();
            a.x = x;
            a.y = y;
            w.tasks.get_mut(&task_id).unwrap().path_index += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_uses_cost_then_entire_lexicographic_path_and_checks_overflow() {
        // Algorithm-only graphs, not modifications to the approved fixture.
        let mut w = World::standard(100);
        w.sites.insert(3, 0);
        w.edges = vec![(1, 2, 7, 1), (1, 3, 8, 1), (2, 9, 10, 1), (3, 9, 11, 1)];
        assert_eq!(
            route(&w, 1, 9).unwrap().unwrap(),
            [PathStep::Edge(7), PathStep::Edge(10)]
        );
        w.edges.push((1, 9, 12, 1));
        assert_eq!(route(&w, 1, 9).unwrap().unwrap(), [PathStep::Edge(12)]);
        w.edges = vec![(1, 2, 7, u64::MAX), (2, 9, 10, 1)];
        assert_eq!(route(&w, 1, 9), Err(StepError::CapacityExceeded));
        w.insert_actor(1, 0);
        let a = w.actors.get_mut(&1).unwrap();
        a.next_wake = 1;
        a.home = 9;
        let mut sim = into_simulation(w);
        let before = sim.domain_checkpoint_bytes();
        assert_eq!(sim.drive(0, &[]), Err(StepError::CapacityExceeded));
        assert_eq!(sim.domain_checkpoint_bytes(), before);
    }
}
