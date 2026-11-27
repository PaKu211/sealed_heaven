//! Turn-based ASCII view of the Qinghe client.
//!
//! The map is drawn from the canonical checkpoint. Each command is one tick.
//! Rendering does not enter the simulation.

use crate::agency::{Action, Session};
use crate::clock;
use crate::locale::{Catalog, Locale};
use crate::supply::Phase;
use sealed_sim::player5::{
    CombatKind, Direction, Event, HEIGHT, Intent, Outcome, Position, Rejection, Target, WIDTH,
};
use std::fmt::Write;
use std::io::{self, BufRead, Write as IoWrite};

pub enum Command {
    Continue,
    Quit,
}

pub fn run() {
    crate::living_ascii::run(false);
}

pub fn run_developer_view() {
    crate::living_ascii::run(true);
}

/// C1.4 remains available with its original campaign revision and save path.
pub fn run_c14() {
    crate::living_ascii::run_c14();
}

pub fn run_c15() {
    crate::living_ascii::run_c15();
}

pub fn run_c16() {
    crate::living_ascii::run_c16();
}

pub fn run_c17() {
    crate::living_ascii::run_c17();
}

pub fn run_c18() {
    crate::living_ascii::run_c18();
}

pub fn run_c19() {
    crate::living_ascii::run_c19();
}

pub fn run_c20() {
    crate::living_ascii::run_c20();
}

pub fn run_c21() {
    crate::living_ascii::run_c21();
}

pub fn run_c22() {
    crate::living_ascii::run_c22();
}

pub fn run_c23() {
    crate::living_ascii::run_c23();
}

pub fn run_c24() {
    crate::living_ascii::run_c24();
}

pub fn run_c25() {
    crate::living_ascii::run_c25();
}

pub fn run_c27() {
    crate::living_ascii::run_c27();
}

pub fn run_c28() {
    crate::living_ascii::run_c28();
}

pub fn run_c29() {
    crate::living_ascii::run_c29();
}

pub fn run_c30() {
    crate::living_ascii::run_c30();
}

pub fn run_c31() {
    crate::living_ascii::run_c31();
}

pub fn run_c32() {
    crate::living_ascii::run_c32();
}

pub fn run_c33() {
    crate::living_ascii::run_c33();
}

pub fn run_c35() {
    crate::living_ascii::run_c35();
}

pub fn run_c34() {
    crate::living_ascii::run_c34();
}

pub fn run_sandbox_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_sandbox_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_sandbox_window(seed: u32) {
    crate::living_pixel::run_sandbox(seed);
}
pub fn run_governance_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_governance_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_governance_window(seed: u32) {
    crate::living_pixel::run_governance(seed);
}
pub fn run_diplomacy_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_diplomacy_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_diplomacy_window(seed: u32) {
    crate::living_pixel::run_diplomacy(seed);
}
pub fn run_circles_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_circles_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_circles_window(seed: u32) {
    crate::living_pixel::run_circles(seed);
}
pub fn run_development_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_development_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_development_window(seed: u32) {
    crate::living_pixel::run_development(seed);
}
pub fn run_narrative_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_narrative_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_narrative_window(seed: u32) {
    crate::living_pixel::run_narrative(seed);
}
pub fn run_rumor_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_rumor_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_rumor_window(seed: u32) {
    crate::living_pixel::run_rumor(seed);
}
pub fn run_communication_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_communication_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_communication_window(seed: u32) {
    crate::living_pixel::run_communication(seed);
}
pub fn run_obligations_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_obligations_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_obligations_window(seed: u32) {
    crate::living_pixel::run_obligations(seed);
}
pub fn run_cooperation_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_cooperation_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_cooperation_window(seed: u32) {
    crate::living_pixel::run_cooperation(seed);
}
pub fn run_social_network_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_social_network_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_social_network_window(seed: u32) {
    crate::living_pixel::run_social_network(seed);
}
pub fn run_encounter_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_encounter_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_encounter_window(seed: u32) {
    crate::living_pixel::run_encounters(seed);
}
pub fn run_bonds_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_bonds_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_bonds_window(seed: u32) {
    crate::living_pixel::run_bonds(seed);
}
pub fn run_favors_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_favors_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_favors_window(seed: u32) {
    crate::living_pixel::run_favors(seed);
}
pub fn run_requests_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_requests_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_requests_window(seed: u32) {
    crate::living_pixel::run_requests(seed);
}
pub fn run_history_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_history_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_history_window(seed: u32) {
    crate::living_pixel::run_history(seed);
}
pub fn run_production_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_production_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_production_window(seed: u32) {
    crate::living_pixel::run_production(seed);
}
pub fn run_lod_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_lod_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_lod_window(seed: u32) {
    crate::living_pixel::run_lod(seed);
}
pub fn run_knowledge_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_knowledge_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_knowledge_window(seed: u32) {
    crate::living_pixel::run_knowledge(seed);
}
pub fn run_expanded_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_expanded_campaign(seed, debug);
}
#[cfg(feature = "desktop")]
pub fn run_expanded_window(seed: u32) {
    crate::living_pixel::run_expanded(seed);
}
pub fn run_generated_campaign(seed: u32, debug: bool) {
    crate::living_ascii::run_generated_campaign(seed, debug);
}

#[cfg(feature = "desktop")]
pub fn run_generated_window(seed: u32) {
    crate::living_pixel::run_generated(seed);
}

#[cfg(feature = "desktop")]
pub fn run_regional_window() {
    crate::living_pixel::run();
}

pub fn run_c09() {
    run_session(Session::memory());
}

/// Retain the exact revision-2 encounter as a manual regression surface.
pub fn run_c08() {
    run_session(Session::conflict());
}

fn run_session(mut session: Session) {
    let catalog = Catalog::runtime().expect("validated localization");
    if let Ok(script) = std::env::var("SEALED_ASCII_COMMANDS") {
        println!("{}", frame(&session, &catalog));
        for token in script.split(|c: char| c.is_whitespace() || c == ',') {
            if token.is_empty() {
                continue;
            }
            match apply(&mut session, token) {
                Command::Quit => break,
                Command::Continue => println!("{}", frame(&session, &catalog)),
            }
        }
        return;
    }
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    loop {
        println!("{}", frame(&session, &catalog));
        print!("> ");
        let _ = io::stdout().flush();
        let Some(Ok(line)) = lines.next() else {
            break;
        };
        if matches!(apply(&mut session, line.trim()), Command::Quit) {
            break;
        }
    }
}

pub fn apply(session: &mut Session, token: &str) -> Command {
    match token.trim().to_ascii_lowercase().as_str() {
        "q" | "quit" => Command::Quit,
        "w" | "north" => {
            one_turn(session, Intent::Move(Direction::North));
            Command::Continue
        }
        "d" | "east" => {
            one_turn(session, Intent::Move(Direction::East));
            Command::Continue
        }
        "s" | "south" => {
            one_turn(session, Intent::Move(Direction::South));
            Command::Continue
        }
        "a" | "west" => {
            one_turn(session, Intent::Move(Direction::West));
            Command::Continue
        }
        "e" | "use" => {
            one_turn(session, context_intent(session.selected));
            Command::Continue
        }
        "f" | "attack" => {
            one_turn(session, Intent::Attack);
            Command::Continue
        }
        "b" | "guard" => {
            one_turn(session, Intent::Guard);
            Command::Continue
        }
        "n" | "step" | "wait" => {
            if session.world().combat().is_some() {
                one_turn(session, Intent::Wait);
            } else {
                one_step(session);
            }
            Command::Continue
        }
        "t" | "tab" => {
            let all = targets();
            let index = all
                .iter()
                .position(|target| *target == session.selected)
                .unwrap_or(0);
            session.act(Action::Select(all[(index + 1) % all.len()]));
            Command::Continue
        }
        "p" | "pause" => {
            session.act(if session.paused {
                Action::Resume
            } else {
                Action::Pause
            });
            Command::Continue
        }
        "l" | "language" => {
            session.act(Action::Language(if session.locale == Locale::English {
                Locale::Indonesian
            } else {
                Locale::English
            }));
            Command::Continue
        }
        "r" | "reset" => {
            session.act(Action::Reset);
            Command::Continue
        }
        "v" | "save" => {
            session.act(Action::Save);
            Command::Continue
        }
        "o" | "load" => {
            session.act(Action::Load);
            Command::Continue
        }
        other => {
            let label = Catalog::runtime_ref()
                .map(|c| c.text(session.locale, "legacy.ascii.unknown_command"))
                .unwrap_or_else(|_| "unknown command".into());
            eprintln!("{label}: {other}");
            Command::Continue
        }
    }
}

pub fn frame(session: &Session, catalog: &Catalog) -> String {
    let world = session.world();
    let mut out = catalog.text(session.locale, "legacy.ascii.title");
    out.push('\n');
    let _ = writeln!(
        out,
        "{} {}  {} ({},{})  {}",
        catalog.text(session.locale, "legacy.ascii.tick"),
        world.tick(),
        catalog.text(session.locale, "legacy.ascii.player"),
        world.player_position().x,
        world.player_position().y,
        catalog.text(
            session.locale,
            if session.paused {
                "legacy.ascii.paused"
            } else {
                "legacy.ascii.running"
            }
        )
    );
    let _ = write!(out, "   ");
    for x in 0..WIDTH {
        let _ = write!(out, "{x:>2} ");
    }
    out.push('\n');
    for y in 0..HEIGHT {
        let _ = write!(out, "{y:2} ");
        for x in 0..WIDTH {
            out.push_str(&cell(world, Position { x, y }, session.selected));
        }
        out.push('\n');
    }
    out.push_str(&catalog.text(session.locale, "legacy.ascii.legend"));
    out.push('\n');
    out.push_str(&panel(session, catalog));
    out.push('\n');
    out.push_str(&catalog.text(session.locale, "legacy.ascii.controls"));
    out.push('\n');
    out
}

fn one_turn(session: &mut Session, intent: Intent) {
    let resume = !session.paused;
    if resume {
        session.act(Action::Pause);
    }
    session.act(Action::Intent(intent));
    session.act(Action::Step);
    if resume {
        session.act(Action::Resume);
    }
}

fn one_step(session: &mut Session) {
    let resume = !session.paused;
    if resume {
        session.act(Action::Pause);
    }
    session.act(Action::Step);
    if resume {
        session.act(Action::Resume);
    }
}

fn targets() -> Vec<Target> {
    [Target::Npc(1), Target::HerbPatch, Target::Shrine]
        .into_iter()
        .chain((2..=20).map(Target::Npc))
        .collect()
}

fn context_intent(target: Target) -> Intent {
    match target {
        Target::Npc(id) => Intent::Talk(id),
        Target::HerbPatch => Intent::Gather,
        Target::Shrine => Intent::Deliver,
    }
}

fn cell(world: &sealed_sim::player5::World, position: Position, selected: Target) -> String {
    let glyph = if world.player_position() == position {
        '@'
    } else if let Some(id) = (1..=20).find(|id| world.npc_position(*id) == Some(position)) {
        npc_glyph(id)
    } else if world
        .combat()
        .is_some_and(|combat| combat.foe == position && combat.foe_alive)
    {
        '!'
    } else if world.combat().is_some_and(|combat| combat.foe == position) {
        'x'
    } else if world.target_position(Target::HerbPatch) == Some(position) {
        '*'
    } else if world.target_position(Target::Shrine) == Some(position) {
        '+'
    } else if world.is_walkable(position) {
        '.'
    } else {
        '#'
    };
    if world.target_position(selected) == Some(position) {
        format!("[{glyph}]")
    } else {
        format!(" {glyph} ")
    }
}

fn npc_glyph(id: u32) -> char {
    if id == 1 {
        'Y'
    } else if id < 10 {
        char::from_digit(id, 10).unwrap_or('n')
    } else {
        char::from(b'a' + (id - 10) as u8)
    }
}

fn panel(session: &Session, catalog: &Catalog) -> String {
    let world = session.world();
    let locale = session.locale;
    let t = |key: &str| catalog.text(locale, key);
    let minute = clock::minute_of_day(world.tick());
    let (hour, mins) = clock::clock_hhmm(minute);
    let mut text = format!(
        "\n{}: {}   {}: {}   {}: {}\n{}\n{}: {}\n",
        t("c05.herbs"),
        world.player_herbs(),
        t("c05.source"),
        world.source_herbs(),
        t("c05.delivered"),
        world.delivered_herbs(),
        if world.completed() {
            t("c05.complete")
        } else if world.requested() {
            t("c05.request")
        } else {
            t("c05.unmet")
        },
        t("c05.target"),
        target_name(session.selected, catalog, locale)
    );
    let _ = writeln!(
        text,
        "{}: {:02}:{:02}  {} {}  {}",
        t("c06.clock"),
        hour,
        mins,
        t("c06.day"),
        clock::day_number(world.tick()),
        t(if clock::is_night(minute) {
            "c06.night"
        } else {
            "c06.daylight"
        })
    );
    if let Some(combat) = world.combat() {
        let _ = writeln!(
            text,
            "{} {}: {}  {} {}: {}{}",
            t("c08.hp"),
            t("c05.player"),
            combat.player_hp,
            t("c08.foe"),
            t("c08.hp"),
            combat.foe_hp,
            if combat.guard {
                format!("  {}", t("c08.guard"))
            } else {
                String::new()
            }
        );
        for event in combat.log.iter().rev().take(3) {
            let verb = match event.kind {
                CombatKind::Hit { amount } => format!("{} {amount}", t("c08.hit")),
                CombatKind::Death => t("c08.death"),
            };
            let _ = writeln!(
                text,
                "#{} t{} {} {} -> {}",
                event.id, event.tick, verb, event.source, event.target
            );
        }
    }
    if let Some(memory) = world.memory() {
        let _ = writeln!(
            text,
            "{}: {}",
            t("c09.healer_attitude"),
            t(if memory.reported {
                "c09.wary"
            } else {
                "c09.neutral"
            })
        );
        // Pending reports are intentionally hidden from the player view. They
        // remain inspectable through the canonical domain API in debug/tests.
        if memory.reported
            && let Some(incident) = memory.incident
        {
            let _ = writeln!(
                text,
                "{}: NPC {} / #{} t{}",
                t("c09.reason"),
                incident.witness_id,
                incident.death_event_id,
                incident.report_tick
            );
        }
    }
    if let Ok(supply) = session.supply().status() {
        let _ = writeln!(
            text,
            "{}  {}: {}  {}: {}  {}: {} / {}  {}: {}  {}: {}  {}: {}  {}",
            t("c07.supply"),
            t("c07.demand"),
            supply.demand,
            t("c07.worker"),
            supply.worker,
            t("c07.progress"),
            supply.deliveries,
            supply.demand,
            t("c07.carrier"),
            supply.carrier,
            t("c07.shrine"),
            supply.shrine,
            t("c07.total"),
            supply.total,
            t(phase_key(supply.phase))
        );
    }
    let _ = writeln!(text, "{}", t(&session.status));
    for event in session.recent.iter().rev().take(4) {
        let _ = writeln!(text, "{}", event_text(event, catalog, locale));
    }
    if !session.diagnostic.is_empty() {
        let _ = writeln!(text, "{}", session.diagnostic);
    }
    text
}

fn target_name(target: Target, catalog: &Catalog, locale: Locale) -> String {
    let key = match target {
        Target::Npc(1) => "c05.healer",
        Target::HerbPatch => "c05.patch",
        Target::Shrine => "c05.shrine",
        Target::Npc(_) => "inspector.npc",
    };
    let name = catalog.text(locale, key);
    match target {
        Target::Npc(id) if id != 1 => format!("{name} {id}"),
        _ => name,
    }
}

fn event_text(event: &Event, catalog: &Catalog, locale: Locale) -> String {
    let label = match event.outcome {
        Outcome::Moved => catalog.text(locale, "c05.moved"),
        Outcome::Requested => catalog.text(locale, "c05.requested"),
        Outcome::Gathered => catalog.text(locale, "c05.gathered"),
        Outcome::Delivered => catalog.text(locale, "c05.delivered_ok"),
        Outcome::Waited => catalog.text(locale, "c08.waited"),
        Outcome::Struck => catalog.text(locale, "c08.struck"),
        Outcome::Guarded => catalog.text(locale, "c08.guarded"),
        Outcome::Rejected(reason) => format!(
            "{}: {}",
            catalog.text(locale, "c05.rejected"),
            catalog.text(locale, rejection_key(reason))
        ),
    };
    format!("#{} t{} {label}", event.id, event.tick)
}

fn rejection_key(rejection: Rejection) -> &'static str {
    match rejection {
        Rejection::OutOfBounds | Rejection::Terrain => "c05.r_terrain",
        Rejection::Occupied => "c05.r_occupied",
        Rejection::OutOfRange => "c05.r_out_of_range",
        Rejection::UnknownTarget => "c05.r_npc",
        Rejection::RequestNotAccepted => "c05.r_request",
        Rejection::Empty => "c05.r_empty",
        Rejection::AlreadyCompleted => "c05.r_complete",
        Rejection::Overflow => "c05.r_overflow",
        Rejection::Incapacitated => "c08.r_dead",
    }
}

fn phase_key(phase: Phase) -> &'static str {
    match phase {
        Phase::Requested => "c07.phase.requested",
        Phase::Assigned => "c07.phase.assigned",
        Phase::InProgress => "c07.phase.progress",
        Phase::Completed => "c07.phase.completed",
        Phase::Rejected => "c07.phase.rejected",
        Phase::Cancelled => "c07.phase.cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_cells(frame: &str, y: u32) -> Vec<String> {
        let line = frame
            .lines()
            .find(|line| line.starts_with(&format!("{y:2} ")))
            .unwrap();
        line[3..]
            .chars()
            .collect::<Vec<_>>()
            .chunks(3)
            .map(|chunk| chunk.iter().collect())
            .collect()
    }

    #[test]
    fn the_opening_map_marks_the_player_wall_patch_and_shrine() {
        let session = Session::default();
        let frame = frame(&session, &Catalog::agency().unwrap());
        let cells = |y| row_cells(&frame, y);
        assert_eq!(cells(4)[0], " @ ");
        assert_eq!(cells(1)[6], " # ");
        assert_eq!(cells(2)[9], " * ");
        assert_eq!(cells(5)[3], " + ");
        assert!(frame.contains("[Y]"), "{frame}");
        assert!(frame.contains("Shrine supply"));
        assert!(frame.contains("Carried: 12"));
        assert!(frame.contains("At the shrine: 0"));
    }

    #[test]
    fn one_east_command_moves_the_player_one_cell() {
        let mut session = Session::default();
        apply(&mut session, "east");
        assert_eq!(session.world().player_position(), Position { x: 1, y: 4 });
        assert_eq!(session.world().tick(), 1);
        let frame = frame(&session, &Catalog::agency().unwrap());
        assert_eq!(row_cells(&frame, 4)[1], " @ ");
        assert!(frame.contains("Moved one cell") || frame.contains("Bergerak"));
    }

    #[test]
    fn language_and_target_commands_do_not_advance_the_tick() {
        let mut session = Session::default();
        apply(&mut session, "l");
        apply(&mut session, "t");
        assert_eq!(session.locale, Locale::Indonesian);
        assert_eq!(session.selected, Target::HerbPatch);
        assert_eq!(session.world().tick(), 0);
        let frame = frame(&session, &Catalog::agency().unwrap());
        assert!(frame.contains("[*]"));
        assert!(frame.contains("Pasokan kuil"));
    }
}
