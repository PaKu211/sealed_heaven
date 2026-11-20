//! Fixture-3 semantic checks. Parsing and integrity checks finish before these
//! checks run; no caller can observe the intermediate, unvalidated world.
use super::*;

fn require(ok: bool) -> Result<(), RestoreError> {
    if ok {
        Ok(())
    } else {
        Err(RestoreError::InvalidState)
    }
}

fn cursor(cursor: u64, ids: impl Iterator<Item = u64>) -> Result<(), RestoreError> {
    require(cursor != 0 && ids.into_iter().all(|id| id != 0 && id < cursor))
}

fn increasing<T: Ord>(items: impl IntoIterator<Item = T>) -> bool {
    let mut previous = None;
    for value in items {
        if previous.as_ref().is_some_and(|old| old >= &value) {
            return false;
        }
        previous = Some(value);
    }
    true
}

pub(super) fn owner_exists(w: &World, kind: u8, id: u64) -> bool {
    if id == 0 {
        return false;
    }
    match kind {
        1 => u32::try_from(id)
            .ok()
            .is_some_and(|id| w.actors.contains_key(&id)),
        3 => w.sites.contains_key(&id),
        2 | 4 | 5 => w.holders.get(&id) == Some(&kind),
        _ => false,
    }
}

pub(super) fn target_valid(w: &World, locus: u8, site: u64, x: u32, y: u32) -> bool {
    match locus {
        1 => w.sites.contains_key(&site) && x == 0 && y == 0,
        2 => site != 0 && site == w.grid_site && x < w.grid_w && y < w.grid_h,
        _ => false,
    }
}

fn position(w: &World, a: &Actor) -> Result<(), RestoreError> {
    match a.locus {
        1 | 2 => {
            if a.edge != 0 || a.traversed != 0 || (a.locus == 1 && (a.x != 0 || a.y != 0)) {
                return Err(RestoreError::Corrupt);
            }
            require(target_valid(w, a.locus, a.site, a.x, a.y))
        }
        3 => {
            if a.site != 0 || a.x != 0 || a.y != 0 {
                return Err(RestoreError::Corrupt);
            }
            let edge = w
                .edges
                .iter()
                .find(|e| e.2 == a.edge)
                .ok_or(RestoreError::InvalidState)?;
            if a.traversed > edge.3 {
                return Err(RestoreError::Corrupt);
            }
            Ok(())
        }
        _ => Err(RestoreError::Corrupt),
    }
}

fn completion(
    w: &World,
    key: CommandKey,
    submitted: u64,
    resolved: u64,
) -> Result<(), RestoreError> {
    let task = w.tasks.get(&resolved).ok_or(RestoreError::InvalidState)?;
    require(
        resolved != 0
            && key.actor_id.get() == task.actor_id
            && key.producer_id == 1
            && key.target_tick <= w.tick
            && (submitted == 0 || submitted == resolved),
    )?;
    Ok(())
}

pub(super) fn validate(
    w: &World,
    tables_sorted: bool,
    keys_sorted: bool,
    traces_sorted: bool,
) -> Result<(), RestoreError> {
    require(!w.tasks.keys().any(|id| w.reservations.contains_key(id)))?;
    cursor(w.next_actor_id, w.actors.keys().map(|id| u64::from(*id)))?;
    cursor(
        w.next_record_id,
        w.tasks.keys().chain(w.reservations.keys()).copied(),
    )?;
    cursor(w.next_stock_id, w.stocks.keys().copied())?;
    cursor(w.next_transaction_id, w.transactions.keys().copied())?;
    require(w.next_event_id != 0)?;
    require(tables_sorted)?;

    require(w.sites.iter().all(|(id, flag)| *id != 0 && *flag <= 1))?;
    require(
        w.holders
            .iter()
            .all(|(id, kind)| *id != 0 && matches!(kind, 2 | 4 | 5)),
    )?;
    let mut edge_ids = BTreeSet::new();
    for &(from, to, id, _) in &w.edges {
        require(
            id != 0
                && edge_ids.insert(id)
                && w.sites.contains_key(&from)
                && w.sites.contains_key(&to),
        )?;
    }
    require(increasing(w.edges.iter().map(|e| (e.0, e.1, e.2))))?;
    if (w.grid_site == 0) != (w.grid_w == 0 && w.grid_h == 0) || w.grid.iter().any(|byte| *byte > 1)
    {
        return Err(RestoreError::Corrupt);
    }
    require(
        w.sites
            .iter()
            .all(|(site, flag)| (*flag == 1) == (*site == w.grid_site)),
    )?;
    require(w.grid_site == 0 || w.sites.contains_key(&w.grid_site))?;

    for (&id, a) in &w.actors {
        require(
            a.enabled <= 1 && matches!(a.mode, 1 | 2) && a.channels.iter().all(|c| *c <= 1000),
        )?;
        position(w, a)?;
        require(a.home == 0 || w.sites.contains_key(&a.home))?;
        if a.task_id != 0 {
            let task = w.tasks.get(&a.task_id).ok_or(RestoreError::InvalidState)?;
            require(task.actor_id == id && task.stage < 6)?;
        }
    }
    for stock in w.stocks.values() {
        require(
            matches!(stock.class, 1 | 2)
                && stock.definition_id != 0
                && owner_exists(w, stock.owner_kind, stock.owner_id),
        )?;
    }

    let mut current = BTreeSet::new();
    let mut unconsumed_keys = BTreeSet::new();
    for (&id, task) in &w.tasks {
        let actor = w
            .actors
            .get(&task.actor_id)
            .ok_or(RestoreError::InvalidState)?;
        require(
            task.goal <= 5 && matches!(task.recipe_id, 0 | 1) && (1..=8).contains(&task.stage),
        )?;
        require(
            task.created <= w.tick
                && target_valid(
                    w,
                    task.target_locus,
                    task.target_site,
                    task.target_x,
                    task.target_y,
                ),
        )?;
        require(task.path_index <= task.steps.len() as u64)?;
        for step in &task.steps {
            require(match step {
                PathStep::Edge(edge) => edge_ids.contains(edge),
                PathStep::Grid(direction) => (1..=4).contains(direction),
            })?;
        }
        if task.stage < 6 {
            require(
                task.interruption == 0 && current.insert(task.actor_id) && actor.task_id == id,
            )?;
        } else {
            require(
                task.unconsumed.is_empty()
                    && if task.stage == 6 {
                        task.interruption == 0
                    } else {
                        (1..=11).contains(&task.interruption)
                    },
            )?;
        }
        require(increasing(task.reservations.iter().copied()))?;
        for reservation in &task.reservations {
            require(
                w.reservations
                    .get(reservation)
                    .is_some_and(|r| r.task_id == id),
            )?;
        }
        require(increasing(task.unconsumed.iter().map(|r| r.key)))?;
        for record in &task.unconsumed {
            completion(w, record.key, record.submitted, record.resolved)?;
            require(
                record.resolved == id
                    && unconsumed_keys.insert(record.key)
                    && !w.registry.contains_key(&record.key)
                    && !w.pending.contains_key(&record.key),
            )?;
        }
        // Planned legs are declarations, not committed effects. Their class,
        // stock, owner and quantity failures belong to work, preserving rollback.
    }
    let mut held: BTreeMap<u64, u64> = BTreeMap::new();
    for (&id, r) in &w.reservations {
        let task = w.tasks.get(&r.task_id).ok_or(RestoreError::InvalidState)?;
        let stock = w
            .stocks
            .get(&r.stock_id)
            .ok_or(RestoreError::InvalidState)?;
        require(
            r.claimant == task.actor_id
                && r.quantity != 0
                && r.expiry_tick > r.acquired_tick
                && r.acquired_tick <= w.tick
                && matches!(r.state, 1 | 2)
                && task.reservations.binary_search(&id).is_ok(),
        )?;
        if r.state == 1 {
            require(task.stage < 6 && w.tick < r.expiry_tick)?;
            let total = held.entry(r.stock_id).or_default();
            *total = total
                .checked_add(r.quantity)
                .ok_or(RestoreError::InvalidState)?;
            require(*total <= stock.quantity)?;
        }
    }

    let mut transaction_tasks = BTreeSet::new();
    let mut transaction_keys = BTreeSet::new();
    for transaction in w.transactions.values() {
        require(transaction.applied == 1)?;
        let task = w
            .tasks
            .get(&transaction.task_id)
            .ok_or(RestoreError::InvalidState)?;
        require(
            task.stage == 6
                && transaction_tasks.insert(transaction.task_id)
                && transaction_keys.insert(transaction.key)
                && transaction.key.target_tick <= w.tick
                && transaction.key.actor_id.get() == task.actor_id,
        )?;
        match transaction.source {
            1 => {
                let record = w
                    .registry
                    .get(&transaction.key)
                    .ok_or(RestoreError::InvalidState)?;
                require(
                    transaction.key.producer_id == 1
                        && record.role == 1
                        && record.resolved == transaction.task_id,
                )?;
            }
            2 => require(
                transaction.key.producer_id == 0
                    && transaction.key.sequence == 0
                    && !w.registry.contains_key(&transaction.key),
            )?,
            _ => return Err(RestoreError::InvalidState),
        }
        for leg in &transaction.legs {
            require(matches!(leg.op, 1..=4) && matches!(leg.class, 1 | 2) && leg.quantity != 0)?;
            let stock = w
                .stocks
                .get(&leg.stock_id)
                .ok_or(RestoreError::InvalidState)?;
            require(stock.definition_id == leg.definition_id && stock.class == leg.class)?;
            match leg.op {
                1 | 3 => {
                    require(leg.owner_kind_after == 0 && leg.owner_id_after == 0 && leg.class == 1)?
                }
                _ => require(
                    owner_exists(w, leg.owner_kind_after, leg.owner_id_after)
                        && stock.owner_kind == leg.owner_kind_after
                        && stock.owner_id == leg.owner_id_after,
                )?,
            }
        }
    }
    for (&id, task) in &w.tasks {
        require(task.stage != 6 || transaction_tasks.contains(&id))?;
    }
    require(keys_sorted)?;
    let mut role_one_tasks = BTreeSet::new();
    for (&key, r) in &w.registry {
        completion(w, key, r.submitted, r.resolved)?;
        let task = &w.tasks[&r.resolved]; // completion checked this reference.
        require(task.stage >= 6 && !w.pending.contains_key(&key))?;
        match r.role {
            1 => require(
                role_one_tasks.insert(r.resolved)
                    && w.transactions
                        .values()
                        .any(|t| t.source == 1 && t.key == key && t.task_id == r.resolved),
            )?,
            2 => require(!transaction_keys.contains(&key))?,
            _ => return Err(RestoreError::InvalidState),
        }
    }
    for (&key, &payload) in &w.pending {
        require(
            key.target_tick > w.tick
                && key.producer_id == 1
                && w.actors.contains_key(&key.actor_id.get()),
        )?;
        require(super::payload_shape_ok(w, Command::new(key, payload)))?;
        if let Payload::CompleteTask(0) = payload {
            require(w.pending.iter().any(|(earlier, p)| {
                earlier < &key
                    && earlier.actor_id == key.actor_id
                    && earlier.target_tick == key.target_tick
                    && matches!(p, Payload::BeginTask { .. })
            }))?;
        }
    }
    if w.edges.iter().any(|edge| edge.3 == 0) {
        return Err(RestoreError::Corrupt);
    }
    require(traces_sorted)?;
    for (&id, bytes) in &w.traces {
        require(w.actors.contains_key(&id) && bytes.len() == 320)?;
        let evaluated = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        let task = u64::from_le_bytes(bytes[17..25].try_into().unwrap());
        require(
            evaluated <= w.tick
                && bytes[16] <= 5
                && if bytes[16] == 0 {
                    task == 0
                } else {
                    w.tasks.get(&task).is_some_and(|t| t.actor_id == id)
                },
        )?;
        for index in 0..5 {
            let at = 25 + index * 59;
            require(bytes[at] == index as u8 + 1 && bytes[at + 1] <= 1 && bytes[at + 2] <= 11)?;
        }
    }
    Ok(())
}
