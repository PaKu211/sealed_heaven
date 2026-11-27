//! Typed compiler for immutable authored living-world content.
//! This module is host-only: the simulation consumes generated static values.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::Write,
};

const MAX_BYTES: usize = 256 * 1024;
const SCHEMA: u32 = 1;
const LAST_REVISION: u16 = 26;
type Cell = [u32; 2];

macro_rules! record {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct $name { $($field: $ty),* }
    };
}
macro_rules! kind {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
        enum $name { $($variant),+ }
    };
}
kind!(ItemKind {
    Herb,
    Coin,
    Remedy,
    SpiritOre,
    JadeScript
});
kind!(Role {
    Medic,
    Watch,
    Supplier,
    Traveler,
    Grower,
    Courier,
    Mentor,
    HallDeputy,
    RivalEnvoy,
    HallSteward
});
kind!(Invitation {
    OpenRoad,
    SchoolMember,
    LodgeMember
});
kind!(BackgroundKind {
    Wanderer,
    Herbalist,
    Peddler
});
kind!(InstitutionKind { Hall, Lodge });
kind!(Rank {
    Apprentice,
    Disciple,
    SeniorDisciple,
    Elder
});
kind!(TechniqueKind {
    ReedStrike,
    ReedPalm,
    StoneFist
});
record!(Dependency {
    pack_id: String,
    content_version: u32
});
record!(Header { schema_version: u32, pack_id: String, content_version: u32, dependencies: Vec<Dependency> });
record!(Item {
    id: String,
    kind: ItemKind
});
record!(Tile {
    id: String,
    glyph: char,
    walkable: bool
});
record!(Recipe {
    id: String,
    input: String,
    input_count: u32,
    output: String,
    output_count: u32,
    effect_hp: u16
});
record!(CorePack { header: Header, items: Vec<Item>, tiles: Vec<Tile>, recipes: Vec<Recipe> });
record!(Actor {
    id: String, wire_id: u32, name: String, glyph: char, role: Role,
    spawn: Cell, home: Cell, route: Vec<Cell>, hp: u16, rest_threshold: u8,
    responds_to_attack: bool, shares_news: bool, report_to: Option<String>
});
record!(Map { id: String, rows: Vec<String> });
record!(Recruit {
    actor: String,
    invitation: Invitation
});
record!(Scenario {
    id: String, name: String, map: String, actors: Vec<String>, contacts: Vec<[String;2]>,
    recruits: Vec<Recruit>, entrance: Cell, patch: Cell, shrine: Cell, clinic: Cell,
    shrine_target: u32, initial_source: u32, regrow_every: u64
});
record!(Region {
    id: String,
    market: Cell,
    garden: Cell,
    hall: Cell,
    garden_capacity: u32,
    market_target: u32,
    market_reserve: u32,
    coin_budget: u32,
    stock_base: u32,
    stock_variants: u32,
    garden_regrow_every: u64,
    order_interval: u64,
    order_travel: u64,
    opening_order: u32,
    normal_buy: u32,
    normal_sell: u32,
    scarce_buy: u32,
    scarce_sell: u32
});
record!(Background {
    id: String,
    kind: BackgroundKind,
    coins: u32
});
record!(Office {
    actor: String,
    rank: Rank
});
record!(Institution { id: String, kind: InstitutionKind, opening_coins: u32, offices: Vec<Office> });
record!(Sect {
    contest_threshold: u8,
    max_pressure: u8,
    accord_turns: u64,
    disciple_merit: u8,
    senior_merit: u8,
    max_qi: u8
});
record!(Technique {
    id: String,
    kind: TechniqueKind,
    teacher: String,
    institution: String,
    herb_cost: u32,
    qi_cost: u8,
    damage: u16,
    exertion: u8,
    parry: u8,
    lessons: u8,
    tuition: u32
});
record!(Lot {
    id: String,
    wire_id: u32,
    item: String,
    position: Cell,
    minimum_revision: u16
});
record!(Campaign {
    first: u16,
    last: u16,
    even: String,
    odd: String
});
record!(WorldPack {
    header: Header, actors: Vec<Actor>, maps: Vec<Map>, scenarios: Vec<Scenario>,
    region: Region, backgrounds: Vec<Background>, institutions: Vec<Institution>,
    sect: Sect, techniques: Vec<Technique>, lots: Vec<Lot>, campaigns: Vec<Campaign>
});
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub kind: &'static str,
    pub wire_id: Option<u32>,
    pub binding: Option<String>,
}
#[derive(Debug)]
pub struct Generated {
    pub fingerprint: String,
    pub rust: String,
    pub manifest: String,
}
fn require(test: bool, message: impl FnOnce() -> String) -> Result<(), String> {
    if test { Ok(()) } else { Err(message()) }
}
fn name_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
fn label_ok(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
}
fn header_order(headers: &[&Header]) -> Result<Vec<String>, String> {
    let mut index = BTreeMap::new();
    for h in headers {
        require(h.schema_version == SCHEMA, || {
            format!("unsupported schema {}", h.schema_version)
        })?;
        require(name_ok(&h.pack_id) && h.content_version > 0, || {
            format!("invalid pack {}", h.pack_id)
        })?;
        require(index.insert(h.pack_id.as_str(), *h).is_none(), || {
            format!("duplicate pack {}", h.pack_id)
        })?;
    }
    for h in headers {
        let mut seen = BTreeSet::new();
        for d in &h.dependencies {
            require(seen.insert(d.pack_id.as_str()), || {
                format!("duplicate dependency {} -> {}", h.pack_id, d.pack_id)
            })?;
            let target = index
                .get(d.pack_id.as_str())
                .ok_or_else(|| format!("missing dependency {}", d.pack_id))?;
            require(target.content_version == d.content_version, || {
                format!("dependency version mismatch {}", d.pack_id)
            })?;
        }
    }
    let mut done = BTreeSet::new();
    let mut order = Vec::new();
    while done.len() < index.len() {
        let next = index.iter().find(|(id, h)| {
            !done.contains(**id)
                && h.dependencies
                    .iter()
                    .all(|d| done.contains(d.pack_id.as_str()))
        });
        let Some((&id, _)) = next else {
            return Err("dependency cycle".into());
        };
        done.insert(id);
        order.push(id.to_owned());
    }
    Ok(order)
}
fn register(
    entries: &mut BTreeMap<String, Entry>,
    namespace: &str,
    id: &str,
    kind: &'static str,
    wire_id: Option<u32>,
) -> Result<(), String> {
    let Some((ns, local)) = id.split_once(':') else {
        return Err(format!("unqualified ID {id}"));
    };
    require(ns == namespace && name_ok(local), || {
        format!("invalid namespace or ID {id}")
    })?;
    require(
        entries
            .insert(
                id.to_owned(),
                Entry {
                    id: id.to_owned(),
                    kind,
                    wire_id,
                    binding: None,
                },
            )
            .is_none(),
        || format!("duplicate definition ID {id}"),
    )
}
fn unique<T: Ord>(values: impl IntoIterator<Item = T>, context: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for v in values {
        require(seen.insert(v), || format!("duplicate {context}"))?;
    }
    Ok(())
}
fn map_passable(map: &Map, [x, y]: Cell) -> bool {
    map.rows
        .get(y as usize)
        .and_then(|r| r.as_bytes().get(x as usize))
        == Some(&b'.')
}
fn reachable(map: &Map, start: Cell) -> BTreeSet<Cell> {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([start]);
    while let Some([x, y]) = queue.pop_front() {
        if !map_passable(map, [x, y]) || !visited.insert([x, y]) {
            continue;
        }
        for (nx, ny) in [
            (x.checked_sub(1), Some(y)),
            (x.checked_add(1), Some(y)),
            (Some(x), y.checked_sub(1)),
            (Some(x), y.checked_add(1)),
        ] {
            if let (Some(nx), Some(ny)) = (nx, ny) {
                queue.push_back([nx, ny]);
            }
        }
    }
    visited
}
fn actor<'a>(world: &'a WorldPack, id: &str) -> Result<&'a Actor, String> {
    world
        .actors
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| format!("unknown actor {id}"))
}
fn map<'a>(world: &'a WorldPack, id: &str) -> Result<&'a Map, String> {
    world
        .maps
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("unknown map {id}"))
}
fn item<'a>(core: &'a CorePack, id: &str) -> Result<&'a Item, String> {
    core.items
        .iter()
        .find(|x| x.id == id)
        .ok_or_else(|| format!("unknown item {id}"))
}
fn scenario<'a>(world: &'a WorldPack, id: &str) -> Result<&'a Scenario, String> {
    world
        .scenarios
        .iter()
        .find(|x| x.id == id)
        .ok_or_else(|| format!("unknown scenario {id}"))
}
fn validate(
    core: &CorePack,
    world: &WorldPack,
) -> Result<(Vec<String>, BTreeMap<String, Entry>), String> {
    let order = header_order(&[&core.header, &world.header])?;
    require(
        core.header.pack_id == "core" && world.header.pack_id == "sealed_heaven",
        || "unsupported pack namespace".into(),
    )?;
    require(
        core.header.dependencies.is_empty()
            && world.header.dependencies.len() == 1
            && world.header.dependencies[0].pack_id == "core",
        || "world must depend on core".into(),
    )?;
    let mut entries = BTreeMap::new();
    let mut reg = |ns, id: &str, kind, wire| register(&mut entries, ns, id, kind, wire);
    for x in &core.items {
        reg("core", &x.id, "item", None)?;
    }
    for x in &core.tiles {
        reg("core", &x.id, "tile", None)?;
    }
    for x in &core.recipes {
        reg("core", &x.id, "recipe", None)?;
    }
    for x in &world.actors {
        reg("sealed_heaven", &x.id, "actor", Some(x.wire_id))?;
    }
    for x in &world.maps {
        reg("sealed_heaven", &x.id, "map", None)?;
    }
    for x in &world.scenarios {
        reg("sealed_heaven", &x.id, "scenario", None)?;
    }
    reg("sealed_heaven", &world.region.id, "region", None)?;
    for x in &world.backgrounds {
        reg("sealed_heaven", &x.id, "background", None)?;
    }
    for x in &world.institutions {
        reg("sealed_heaven", &x.id, "institution", None)?;
    }
    for x in &world.techniques {
        reg("sealed_heaven", &x.id, "technique", None)?;
    }
    for x in &world.lots {
        reg("sealed_heaven", &x.id, "lot", Some(x.wire_id))?;
    }
    // A stable identity cannot silently become another engine-facing type.
    let mut bind = |id: &str, value: String| entries.get_mut(id).unwrap().binding = Some(value);
    for x in &core.items {
        bind(&x.id, format!("{:?}", x.kind));
    }
    for x in &core.tiles {
        bind(&x.id, format!("{}:{}", x.glyph, x.walkable));
    }
    for x in &world.actors {
        bind(&x.id, format!("{:?}", x.role));
    }
    for x in &world.backgrounds {
        bind(&x.id, format!("{:?}", x.kind));
    }
    for x in &world.institutions {
        bind(&x.id, format!("{:?}", x.kind));
    }
    for x in &world.techniques {
        bind(&x.id, format!("{:?}", x.kind));
    }
    for x in &world.lots {
        bind(&x.id, x.item.clone());
    }
    require(
        core.items.len() == 5 && core.tiles.len() == 2 && core.recipes.len() == 1,
        || "unsupported core binding count".into(),
    )?;
    unique(core.items.iter().map(|x| x.kind), "item kind")?;
    unique(core.tiles.iter().map(|x| x.glyph), "tile glyph")?;
    require(
        core.tiles.iter().any(|x| x.glyph == '.' && x.walkable)
            && core.tiles.iter().any(|x| x.glyph == '#' && !x.walkable),
        || "tile semantics incompatible with grid engine".into(),
    )?;
    let recipe = &core.recipes[0];
    require(
        item(core, &recipe.input)?.kind == ItemKind::Herb
            && item(core, &recipe.output)?.kind == ItemKind::Remedy,
        || "unsupported recipe item types".into(),
    )?;
    require(
        (1..=64).contains(&recipe.input_count)
            && recipe.output_count == 1
            && (1..=6).contains(&recipe.effect_hp),
        || "invalid remedy recipe".into(),
    )?;
    require(
        (1..=32).contains(&world.actors.len())
            && (1..=32).contains(&world.maps.len())
            && (1..=64).contains(&world.scenarios.len()),
        || "invalid registry size".into(),
    )?;
    unique(world.actors.iter().map(|x| x.wire_id), "actor wire ID")?;
    for a in &world.actors {
        require(
            (1..=255).contains(&a.wire_id)
                && a.wire_id != 21
                && a.glyph.is_ascii_alphanumeric()
                && label_ok(&a.name)
                && a.hp > 0
                && (1..=40).contains(&a.rest_threshold)
                && a.route.len() <= 256,
            || format!("invalid actor {}", a.id),
        )?;
        if let Some(peer) = &a.report_to {
            actor(world, peer)?;
            require(peer != &a.id, || format!("self report {}", a.id))?;
        }
    }
    for m in &world.maps {
        require(
            (1..=64).contains(&m.rows.len())
                && m.rows
                    .first()
                    .is_some_and(|row| (1..=64).contains(&row.len()))
                && m.rows.iter().all(|row| {
                    row.len() == m.rows[0].len() && row.bytes().all(|b| matches!(b, b'.' | b'#'))
                }),
            || format!("invalid map {}", m.id),
        )?;
    }
    let mut used_actors = BTreeSet::new();
    let mut used_maps = BTreeSet::new();
    for s in &world.scenarios {
        require(
            label_ok(&s.name)
                && !s.actors.is_empty()
                && s.actors.len() <= 32
                && (1..=65_536).contains(&s.shrine_target)
                && s.initial_source <= 65_536
                && s.regrow_every > 0,
            || format!("invalid scenario {}", s.id),
        )?;
        let m = map(world, &s.map)?;
        used_maps.insert(s.map.as_str());
        let connected = reachable(m, s.entrance);
        require(
            [s.entrance, s.patch, s.shrine, s.clinic]
                .iter()
                .all(|p| connected.contains(p)),
            || format!("blocked or disconnected site in {}", s.id),
        )?;
        let mut ids = BTreeSet::new();
        let mut spawns = BTreeSet::from([s.entrance]);
        let mut glyphs = BTreeSet::new();
        for id in &s.actors {
            let a = actor(world, id)?;
            used_actors.insert(id.as_str());
            require(
                ids.insert(id.as_str()) && spawns.insert(a.spawn) && glyphs.insert(a.glyph),
                || format!("duplicate actor, spawn or glyph in {}", s.id),
            )?;
            require(
                [a.spawn, a.home]
                    .iter()
                    .chain(a.route.iter())
                    .all(|p| connected.contains(p)),
                || format!("actor {} disconnected in {}", id, s.id),
            )?;
        }
        for id in &s.actors {
            let a = actor(world, id)?;
            require(
                a.report_to
                    .as_ref()
                    .is_none_or(|peer| ids.contains(peer.as_str())),
                || format!("report target outside {}", s.id),
            )?;
        }
        let mut contacts = BTreeSet::new();
        for [a, b] in &s.contacts {
            require(
                a != b && ids.contains(a.as_str()) && ids.contains(b.as_str()),
                || format!("invalid contact in {}", s.id),
            )?;
            require(contacts.insert((a.min(b), a.max(b))), || {
                format!("duplicate contact in {}", s.id)
            })?;
        }
        unique(s.recruits.iter().map(|r| r.actor.as_str()), "recruit")?;
        for r in &s.recruits {
            require(ids.contains(r.actor.as_str()), || {
                format!("unknown recruit {}", r.actor)
            })?;
        }
    }
    require(
        used_actors.len() == world.actors.len() && used_maps.len() == world.maps.len(),
        || "unreferenced actor or map".into(),
    )?;
    let r = &world.region;
    require(
        r.market_reserve < r.market_target
            && r.market_target <= r.garden_capacity
            && r.garden_capacity <= 65_536
            && r.stock_variants > 0
            && r.stock_base
                .checked_add(r.stock_variants - 1)
                .is_some_and(|v| v <= 65_536)
            && r.coin_budget <= 65_536
            && r.garden_regrow_every > 0
            && r.order_interval > 0
            && r.order_travel > 0
            && r.order_travel < r.order_interval
            && r.opening_order <= 65_536
            && r.normal_buy > r.normal_sell
            && r.scarce_buy > r.scarce_sell
            && r.scarce_buy >= r.normal_buy
            && r.scarce_sell >= r.normal_sell
            && r.scarce_buy < 65_536,
        || "invalid region economy".into(),
    )?;
    require(world.backgrounds.len() == 3, || {
        "missing background binding".into()
    })?;
    unique(world.backgrounds.iter().map(|b| b.kind), "background kind")?;
    require(
        world.backgrounds.iter().all(|b| b.coins <= r.coin_budget),
        || "background exceeds opening coin budget".into(),
    )?;
    require(world.institutions.len() == 2, || {
        "missing institution binding".into()
    })?;
    unique(
        world.institutions.iter().map(|i| i.kind),
        "institution kind",
    )?;
    let mut office_roles = BTreeSet::new();
    for institution in &world.institutions {
        require(institution.opening_coins <= 65_536, || {
            "institution treasury out of range".into()
        })?;
        unique(
            institution.offices.iter().map(|o| o.actor.as_str()),
            "institution office",
        )?;
        for o in &institution.offices {
            let a = actor(world, &o.actor)?;
            require(
                institution.kind == InstitutionKind::Hall && office_roles.insert(a.role),
                || "unsupported or duplicate institution office".into(),
            )?;
        }
    }
    require(
        office_roles == BTreeSet::from([Role::Mentor, Role::HallDeputy, Role::HallSteward]),
        || "missing hall office".into(),
    )?;
    let sect = &world.sect;
    require(
        sect.contest_threshold > 0
            && sect.contest_threshold <= sect.max_pressure
            && sect.max_pressure <= 64
            && sect.accord_turns > 0
            && sect.disciple_merit > 0
            && sect.disciple_merit < sect.senior_merit
            && sect.max_qi > 0,
        || "invalid sect progression".into(),
    )?;
    require(world.techniques.len() == 3, || {
        "missing technique binding".into()
    })?;
    unique(world.techniques.iter().map(|t| t.kind), "technique kind")?;
    for t in &world.techniques {
        let teacher = actor(world, &t.teacher)?;
        let institution = world
            .institutions
            .iter()
            .find(|i| i.id == t.institution)
            .ok_or_else(|| format!("unknown institution {}", t.institution))?;
        let (role, kind) = if t.kind == TechniqueKind::StoneFist {
            (Role::RivalEnvoy, InstitutionKind::Lodge)
        } else {
            (Role::Mentor, InstitutionKind::Hall)
        };
        require(
            teacher.role == role && institution.kind == kind && (1..=64).contains(&t.damage),
            || format!("invalid technique {}", t.id),
        )?;
        if t.kind == TechniqueKind::ReedStrike {
            require(
                t.herb_cost > 0
                    && t.herb_cost <= 64
                    && t.qi_cost > 0
                    && t.qi_cost <= sect.max_qi
                    && t.exertion == 0
                    && t.parry == 0
                    && t.lessons == 0
                    && t.tuition == 0,
                || "invalid reed strike".into(),
            )?;
        } else {
            require(
                t.herb_cost == 0
                    && t.qi_cost == 0
                    && (1..=2).contains(&t.exertion)
                    && t.parry <= 1
                    && (1..=3).contains(&t.lessons)
                    && t.tuition > 0
                    && t.tuition <= 64,
                || format!("invalid martial technique {}", t.id),
            )?;
        }
    }
    require(world.lots.len() <= 64, || "too many initial lots".into())?;
    unique(world.lots.iter().map(|l| l.wire_id), "lot wire ID")?;
    for l in &world.lots {
        require(
            l.wire_id <= 0x00ff_ffff
                && matches!(l.minimum_revision, 19 | 20)
                && matches!(
                    item(core, &l.item)?.kind,
                    ItemKind::SpiritOre | ItemKind::JadeScript
                ),
            || format!("invalid lot {}", l.id),
        )?;
    }
    let mut next = 1;
    let mut bound = BTreeSet::new();
    for c in &world.campaigns {
        require(
            c.first == next && c.first <= c.last && c.last <= LAST_REVISION,
            || "campaign revision gap, overlap or unsupported revision".into(),
        )?;
        next = c.last + 1;
        for id in [&c.even, &c.odd] {
            let s = scenario(world, id)?;
            bound.insert(id.as_str());
            if c.last >= 3 {
                let m = map(world, &s.map)?;
                let reachable = reachable(m, s.entrance);
                require(
                    [r.market, r.garden, r.hall]
                        .iter()
                        .all(|p| reachable.contains(p)),
                    || format!("regional site disconnected in {}", s.id),
                )?;
                for lot in &world.lots {
                    if lot.minimum_revision <= c.last {
                        require(reachable.contains(&lot.position), || {
                            format!("lot {} disconnected in {}", lot.id, s.id)
                        })?;
                    }
                }
            }
        }
    }
    require(
        next == LAST_REVISION + 1 && bound.len() == world.scenarios.len(),
        || "incomplete campaign bindings".into(),
    )?;
    Ok((order, entries))
}

fn cell([x, y]: Cell) -> String {
    format!("Position {{ x: {x}, y: {y} }}")
}
fn ident(s: &str) -> String {
    s.split_once(':').unwrap().1.to_ascii_uppercase()
}
fn aid(a: &Actor) -> String {
    format!("ActorId::new({})", a.wire_id)
}
fn constant<T: std::fmt::Display>(out: &mut String, name: &str, ty: &str, value: T) {
    writeln!(out, "pub(super) const {name}: {ty} = {value};").unwrap();
}
fn emit(
    core: &CorePack,
    world: &WorldPack,
    entries: &BTreeMap<String, Entry>,
    fingerprint: &str,
) -> String {
    let mut out = String::from("// @generated by typed registry compiler; do not edit.\n");
    writeln!(out, "pub const CONTENT_SHA256: &str = {:?};", fingerprint).unwrap();
    writeln!(
        out,
        "pub const CONTENT_VERSION: u32 = {};",
        world.header.content_version
    )
    .unwrap();
    writeln!(
        out,
        "pub const CORE_VERSION: u32 = {};",
        core.header.content_version
    )
    .unwrap();
    writeln!(out, "pub const SCHEMA_VERSION: u32 = {SCHEMA};").unwrap();
    for a in &world.actors {
        let name = ident(&a.id);
        let route = a
            .route
            .iter()
            .copied()
            .map(cell)
            .collect::<Vec<_>>()
            .join(",");
        let report = a.report_to.as_ref().map_or_else(
            || "None".to_owned(),
            |id| format!("Some({})", aid(actor(world, id).unwrap())),
        );
        writeln!(out,"const ACTOR_{name}: NpcDefinition = NpcDefinition {{ id: {}, name: {:?}, glyph: {:?}, role: Role::{:?}, spawn: {}, home: {}, route: &[{}], hp: {}, rest_threshold: {}, responds_to_attack: {}, shares_news: {}, report_to: {} }};",aid(a),a.name,a.glyph,a.role,cell(a.spawn),cell(a.home),route,a.hp,a.rest_threshold,a.responds_to_attack,a.shares_news,report).unwrap();
    }
    for m in &world.maps {
        writeln!(out, "const MAP_{}: &[&str] = &{:?};", ident(&m.id), m.rows).unwrap();
    }
    for s in &world.scenarios {
        let m = map(world, &s.map).unwrap();
        let actors = s
            .actors
            .iter()
            .map(|id| format!("ACTOR_{}", ident(id)))
            .collect::<Vec<_>>()
            .join(",");
        let contacts = s
            .contacts
            .iter()
            .map(|[a, b]| {
                format!(
                    "({}, {})",
                    aid(actor(world, a).unwrap()),
                    aid(actor(world, b).unwrap())
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let recruits = s
            .recruits
            .iter()
            .map(|r| {
                format!(
                    "({}, Invitation::{:?})",
                    aid(actor(world, &r.actor).unwrap()),
                    r.invitation
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        writeln!(out,"pub(super) static SCENARIO_{}: Scenario = Scenario {{name: {:?}, width: {}, height: {}, terrain: MAP_{}, entrance: {}, patch: {}, shrine: {}, clinic: {}, shrine_target: {}, initial_source: {}, regrow_every: {}, npcs: &[{}], contacts: &[{}], recruits: &[{}]}};",ident(&s.id),s.name,m.rows[0].len(),m.rows.len(),ident(&s.map),cell(s.entrance),cell(s.patch),cell(s.shrine),cell(s.clinic),s.shrine_target,s.initial_source,s.regrow_every,actors,contacts,recruits).unwrap();
    }
    writeln!(out,"pub(super) fn scenario_for_revision(revision:u16,seed:u32)->Option<&'static Scenario>{{ Some(match (revision,seed&1) {{").unwrap();
    for c in &world.campaigns {
        writeln!(
            out,
            "({}..={}, 0) => &SCENARIO_{}, ({}..={}, _) => &SCENARIO_{},",
            c.first,
            c.last,
            ident(&c.even),
            c.first,
            c.last,
            ident(&c.odd)
        )
        .unwrap();
    }
    out.push_str("_=>return None, }) }\n");
    let r = &world.region;
    writeln!(
        out,
        "pub(super) const REGION_SITES: RegionSites = RegionSites {{market:{},garden:{},hall:{}}};",
        cell(r.market),
        cell(r.garden),
        cell(r.hall)
    )
    .unwrap();
    for (name, v) in [
        ("GARDEN_CAPACITY", r.garden_capacity),
        ("MARKET_TARGET", r.market_target),
        ("MARKET_RESERVE", r.market_reserve),
        ("COIN_BUDGET", r.coin_budget),
        ("STOCK_BASE", r.stock_base),
        ("STOCK_VARIANTS", r.stock_variants),
        ("OPENING_ORDER", r.opening_order),
        ("NORMAL_BUY", r.normal_buy),
        ("NORMAL_SELL", r.normal_sell),
        ("SCARCE_BUY", r.scarce_buy),
        ("SCARCE_SELL", r.scarce_sell),
    ] {
        constant(&mut out, name, "u32", v);
    }
    for (name, v) in [
        ("GARDEN_REGROW_EVERY", r.garden_regrow_every),
        ("ORDER_INTERVAL", r.order_interval),
        ("ORDER_TRAVEL", r.order_travel),
        ("ACCORD_TURNS", world.sect.accord_turns),
    ] {
        constant(&mut out, name, "u64", v);
    }
    let recipe = &core.recipes[0];
    constant(&mut out, "REMEDY_HERBS", "u32", recipe.input_count);
    constant(&mut out, "REMEDY_OUTPUT", "u32", recipe.output_count);
    constant(&mut out, "REMEDY_HP", "u16", recipe.effect_hp);
    for (name, v) in [
        ("CONTEST_THRESHOLD", world.sect.contest_threshold),
        ("MAX_PRESSURE", world.sect.max_pressure),
        ("DISCIPLE_MERIT", world.sect.disciple_merit),
        ("SENIOR_MERIT", world.sect.senior_merit),
        ("MAX_QI", world.sect.max_qi),
    ] {
        constant(&mut out, name, "u8", v);
    }
    out.push_str("pub(super) fn starting_coins(background:Background)->u32 { match background {\n");
    for b in &world.backgrounds {
        writeln!(out, "Background::{:?}=>{},", b.kind, b.coins).unwrap();
    }
    out.push_str("} }\n");
    out.push_str("pub(super) fn hall_rank_for_role(role:Role)->Option<HallRank> { match role {\n");
    for i in &world.institutions {
        for office in &i.offices {
            writeln!(
                out,
                "Role::{:?}=>Some(HallRank::{:?}),",
                actor(world, &office.actor).unwrap().role,
                office.rank
            )
            .unwrap();
        }
    }
    out.push_str("_=>None,} }\n");
    for i in &world.institutions {
        constant(
            &mut out,
            if i.kind == InstitutionKind::Hall {
                "HALL_OPENING_COINS"
            } else {
                "LODGE_OPENING_COINS"
            },
            "u32",
            i.opening_coins,
        );
    }
    let strike = world
        .techniques
        .iter()
        .find(|t| t.kind == TechniqueKind::ReedStrike)
        .unwrap();
    constant(&mut out, "FORM_HERBS", "u32", strike.herb_cost);
    constant(&mut out, "FORM_QI_COST", "u8", strike.qi_cost);
    constant(&mut out, "FORM_DAMAGE", "u16", strike.damage);
    out.push_str("pub(super) fn martial_profile(form:Form)->MartialProfile { match form {\n");
    for t in &world.techniques {
        if t.kind != TechniqueKind::ReedStrike {
            writeln!(out,"Form::{:?}=>MartialProfile{{damage:{},exertion:{},parry:{},lessons:{},tuition:{}}},",t.kind,t.damage,t.exertion,t.parry,t.lessons,t.tuition).unwrap();
        }
    }
    out.push_str("} }\n");
    out.push_str("pub(super) fn teaching_form(role:Role)->Option<Form> { match role {\n");
    for t in &world.techniques {
        if t.kind != TechniqueKind::ReedStrike {
            writeln!(
                out,
                "Role::{:?}=>Some(Form::{:?}),",
                actor(world, &t.teacher).unwrap().role,
                t.kind
            )
            .unwrap();
        }
    }
    out.push_str("_=>None,} }\n");
    out.push_str("pub(super) const LOTS: &[(u32,Good,Position,u16)] = &[\n");
    for l in &world.lots {
        writeln!(
            out,
            "({},Good::{:?},{},{}),",
            l.wire_id,
            item(core, &l.item).unwrap().kind,
            cell(l.position),
            l.minimum_revision
        )
        .unwrap();
    }
    out.push_str("];\n");
    out.push_str("pub static DEFINITIONS: &[Definition] = &[\n");
    for e in entries.values() {
        writeln!(
            out,
            "Definition{{id:{:?},kind:{:?},wire_id:{:?},binding:{:?}}},",
            e.id, e.kind, e.wire_id, e.binding
        )
        .unwrap();
    }
    out.push_str("];\n");
    out
}
/// A bounded read protects both build-time and authoring-tool ingestion.
pub fn read_bounded(path: &std::path::Path) -> Result<String, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("content pack exceeds byte limit".into());
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn hex(bytes: impl IntoIterator<Item = u8>) -> String {
    let mut result = String::with_capacity(64);
    for byte in bytes {
        write!(result, "{byte:02x}").unwrap();
    }
    result
}

pub fn compile(core_text: &str, world_text: &str) -> Result<Generated, String> {
    require(
        core_text.len() <= MAX_BYTES && world_text.len() <= MAX_BYTES,
        || "content pack exceeds byte limit".into(),
    )?;
    let core: CorePack = serde_json::from_str(core_text).map_err(|e| format!("core: {e}"))?;
    let world: WorldPack = serde_json::from_str(world_text).map_err(|e| format!("world: {e}"))?;
    let (order, entries) = validate(&core, &world)?;
    let core_bytes = serde_json::to_vec(&core).map_err(|e| e.to_string())?;
    let world_bytes = serde_json::to_vec(&world).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(b"SH:CONTENT:REGISTRY:v1\0");
    for bytes in [&core_bytes, &world_bytes] {
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    let fingerprint = hex(hasher.finalize());
    let core_sha = hex(Sha256::digest(&core_bytes));
    let world_sha = hex(Sha256::digest(&world_bytes));
    let rust = emit(&core, &world, &entries, &fingerprint);
    let manifest=serde_json::to_string_pretty(&serde_json::json!({
        "schema_version":SCHEMA, "content_sha256":fingerprint,
        "pack_order":order,"packs":[core.header,world.header],
        "pack_fingerprints":[
            {"pack_id":core.header.pack_id,"content_version":core.header.content_version,"sha256":core_sha},
            {"pack_id":world.header.pack_id,"content_version":world.header.content_version,"sha256":world_sha}
        ],
        "supported_checkpoint_revisions":[1,LAST_REVISION],
        "definitions":entries.values().collect::<Vec<_>>(),
        "campaigns":world.campaigns,
        "compatibility":"This compiler validates the v1 schema and historical binding shape. Only the fingerprint pinned by build.rs is active. Draft validation does not activate or migrate saves."
    })).map_err(|e|e.to_string())?;
    Ok(Generated {
        fingerprint,
        rust,
        manifest,
    })
}
