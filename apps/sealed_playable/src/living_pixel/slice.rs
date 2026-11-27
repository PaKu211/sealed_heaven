//! C6.0 projection and modal keyboard handling, separate from legacy UI.
use super::{
    Color, FOG, Game, INK, MOSS, OCHRE, Position, Projection, SIDE, STONE, Session, draw_icon,
    nearby, outcome_label, sprite,
};
use crate::living_ascii::field::{Page, wrap_lines};
use bevy::prelude::*;

const MENU_ROWS: usize = 8;
const READ_ROWS: usize = 16;
const PANEL: Color = Color::srgb(0.082, 0.109, 0.108);
const DIM: Color = Color::srgb(0.48, 0.56, 0.51);
const LINE: Color = Color::srgb(0.20, 0.28, 0.26);
const SELECTED: Color = Color::srgb(0.20, 0.28, 0.22);

pub(super) fn notice_lines(s: &str) -> Vec<String> {
    wrap_lines(s, 96)
}
pub(super) fn keyboard(keys: &ButtonInput<KeyCode>, game: &mut Game) -> bool {
    if keys.just_pressed(KeyCode::Tab) {
        return false;
    } // standard language path
    let s = &mut game.session;
    if keys.just_pressed(KeyCode::Escape) && s.field.modal() {
        let reader = s.field.reader.is_some();
        s.field.close();
        if !reader {
            s.notice.clear();
        }
        return true;
    }
    if let Some(page) = s.field.page {
        let len = s.field_entries(page).len();
        if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::ArrowRight) {
            s.input(&format!(
                "field {}",
                page.shift(keys.just_pressed(KeyCode::ArrowRight)).slug()
            ));
        } else if keys.just_pressed(KeyCode::ArrowUp) && len > 0 {
            s.field.cursor = (s.field.cursor + len - 1) % len;
        } else if keys.just_pressed(KeyCode::ArrowDown) && len > 0 {
            s.field.cursor = (s.field.cursor + 1) % len;
        } else if keys.just_pressed(KeyCode::PageDown) && len > 0 {
            s.field.cursor = (s.field.cursor + MENU_ROWS).min(len - 1);
        } else if keys.just_pressed(KeyCode::PageUp) {
            s.field.cursor = s.field.cursor.saturating_sub(MENU_ROWS);
        } else if keys.just_pressed(KeyCode::Enter) && len > 0 {
            s.input(&format!("choose {}", s.field.cursor.min(len - 1) + 1));
        }
        return true; // modal never forwards movement, attacks, saves or waits
    }
    if let Some(page) = s.field.reader {
        let count = notice_lines(&s.notice).len().div_ceil(READ_ROWS).max(1);
        if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::PageDown) {
            s.field.reader = Some((page + 1).min(count - 1));
        }
        if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::PageUp) {
            s.field.reader = Some(page.saturating_sub(1));
        }
        if keys.just_pressed(KeyCode::Enter) {
            s.input("field journal");
        }
        return true;
    }
    if s.field.atlas {
        if keys.just_pressed(KeyCode::Enter) {
            s.input("field travel");
        }
        return true;
    }
    if keys.just_pressed(KeyCode::Enter) {
        s.input("field local");
        return true;
    }
    if keys.just_pressed(KeyCode::PageDown) {
        if s.notice.is_empty() {
            s.notice = outcome_message(s);
        }
        s.field.reader = Some(0);
        return true;
    }
    if keys.just_pressed(KeyCode::Space) && keys.pressed(KeyCode::ShiftRight) {
        s.input("go");
        return true;
    }
    // Arrow keys are movement only outside a modal.
    for (key, command) in [
        (KeyCode::ArrowUp, "north"),
        (KeyCode::ArrowRight, "east"),
        (KeyCode::ArrowDown, "south"),
        (KeyCode::ArrowLeft, "west"),
    ] {
        if keys.just_pressed(key) {
            s.input(command);
            return true;
        }
    }
    false
}
fn text_at(c: &mut Commands, s: impl Into<String>, x: f32, y: f32, size: f32, color: Color) {
    c.spawn((
        Projection,
        Text2d::new(s.into()),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
        Transform::from_xyz(x, y, 3.0),
    ));
}
fn panel(c: &mut Commands, x: f32, y: f32, w: f32, h: f32) {
    sprite(c, Vec2::new(x, y), -1.0, Vec2::new(w, h), PANEL);
}
fn rule(c: &mut Commands, x: f32, y: f32, w: f32) {
    sprite(c, Vec2::new(x, y), 0.0, Vec2::new(w, 1.0), LINE);
}
fn page_label(c: &mut Commands, s: &Session, at: usize, total: usize, y: f32) {
    text_at(
        c,
        s.field_fmt(
            "pixel.pages",
            &[
                ("page", &(at + 1).to_string()),
                ("total", &total.to_string()),
            ],
        ),
        0.0,
        y,
        13.0,
        DIM,
    );
}
fn lines_at(
    c: &mut Commands,
    lines: impl IntoIterator<Item = String>,
    x: f32,
    top: f32,
    size: f32,
    spacing: f32,
    color: Color,
) {
    for (i, line) in lines.into_iter().enumerate() {
        text_at(c, line, x, top - i as f32 * spacing, size, color);
    }
}
fn header(c: &mut Commands, s: &Session) {
    let w = &s.world;
    let p = w.player();
    let r = w.region().unwrap();
    text_at(c, "SEALED HEAVEN   /   C6.0", 0.0, 330.0, 24.0, INK);
    text_at(
        c,
        s.field_fmt(
            "pixel.turn",
            &[
                ("tick", &w.tick().to_string()),
                ("hp", &p.hp.to_string()),
                ("herbs", &p.herbs.to_string()),
                ("coins", &r.coins.to_string()),
                ("breath", &w.breath_progress().to_string()),
            ],
        ),
        0.0,
        295.0,
        16.0,
        OCHRE,
    );
    rule(c, 0.0, 275.0, 1030.0);
}
fn menu(c: &mut Commands, s: &Session, page: Page) {
    panel(c, 0.0, -15.0, 1030.0, 550.0);
    let tabs = Page::ALL
        .into_iter()
        .map(|p| {
            if p == page {
                format!("[ {} ]", s.field_page_name(p))
            } else {
                s.field_page_name(p).into()
            }
        })
        .collect::<Vec<_>>()
        .join("   /   ");
    text_at(c, tabs, 0.0, 236.0, 15.0, OCHRE);
    rule(c, 0.0, 211.0, 970.0);
    let entries = s.field_entries(page);
    let cursor = s.field.cursor.min(entries.len().saturating_sub(1));
    let page_index = cursor / MENU_ROWS;
    let total = entries.len().div_ceil(MENU_ROWS).max(1);
    for (i, entry) in entries
        .iter()
        .enumerate()
        .skip(page_index * MENU_ROWS)
        .take(MENU_ROWS)
    {
        let y = 181.0 - (i % MENU_ROWS) as f32 * 51.0;
        if i == cursor {
            sprite(
                c,
                Vec2::new(0.0, y - 8.0),
                0.0,
                Vec2::new(970.0, 45.0),
                SELECTED,
            );
        }
        let mut label = format!(
            "{}  {}. {}",
            if i == cursor { ">" } else { " " },
            i + 1,
            entry.label
        );
        // Command is not needed for confirmation, but makes all actions auditable.
        label.push_str(&format!("   [{}]", entry.command));
        lines_at(
            c,
            wrap_lines(&label, 95).into_iter().take(2),
            0.0,
            y,
            15.0,
            18.0,
            if i == cursor { INK } else { DIM },
        );
    }
    if entries.is_empty() {
        text_at(c, s.field_text("pixel.empty"), 0.0, 155.0, 16.0, DIM);
    }
    page_label(c, s, page_index, total, -262.0);
    text_at(
        c,
        s.field_text("pixel.menu.control"),
        0.0,
        -326.0,
        15.0,
        INK,
    );
}
fn reader(c: &mut Commands, s: &Session, page: usize) {
    panel(c, 0.0, -15.0, 1030.0, 550.0);
    let lines = notice_lines(&s.notice);
    let total = lines.len().div_ceil(READ_ROWS).max(1);
    let page = page.min(total - 1);
    lines_at(
        c,
        lines.into_iter().skip(page * READ_ROWS).take(READ_ROWS),
        0.0,
        231.0,
        16.0,
        30.0,
        INK,
    );
    page_label(c, s, page, total, -262.0);
    text_at(
        c,
        s.field_text("pixel.reader.control"),
        0.0,
        -326.0,
        14.0,
        INK,
    );
}
fn terrain_color(symbol: char) -> Color {
    match symbol {
        '?' => FOG,
        '#' => STONE,
        '~' => Color::srgb(0.12, 0.28, 0.43),
        '%' => Color::srgb(0.42, 0.29, 0.16),
        ':' => Color::srgb(0.34, 0.30, 0.21),
        ';' => Color::srgb(0.10, 0.22, 0.14),
        ',' => Color::srgb(0.28, 0.38, 0.23),
        _ => MOSS,
    }
}
fn atlas(c: &mut Commands, s: &Session) {
    let w = &s.world;
    let sc = w.scenario();
    let p = w.player().position;
    let cell_size = 17.0;
    panel(c, 0.0, 0.0, 1030.0, 490.0);
    text_at(
        c,
        s.field_fmt(
            "atlas.count",
            &[
                ("count", &w.exploration().unwrap().count().to_string()),
                ("total", &(sc.width * sc.height).to_string()),
            ],
        ),
        0.0,
        212.0,
        18.0,
        OCHRE,
    );
    for y in 0..sc.height {
        for x in 0..sc.width {
            let pos = Position { x, y };
            let symbol = w.charted_symbol(pos).unwrap_or('?');
            let at = Vec2::new(
                (x as f32 - (sc.width - 1) as f32 / 2.0) * cell_size,
                35.0 + ((sc.height - 1) as f32 / 2.0 - y as f32) * cell_size,
            );
            let marked = w.known_landmarks().any(|l| l.position(w) == pos);
            let color = if pos == p {
                INK
            } else if marked {
                OCHRE
            } else {
                terrain_color(symbol)
            };
            sprite(c, at, 0.0, Vec2::splat(cell_size - 1.0), color);
            if marked && pos != p {
                text_at(c, symbol.to_string(), at.x, at.y, 11.0, FOG);
            }
        }
    }
    lines_at(
        c,
        wrap_lines(s.field_text("atlas.legend"), 96),
        0.0,
        -121.0,
        15.0,
        23.0,
        INK,
    );
    let sites = w
        .known_landmarks()
        .map(|l| s.landmark_name(l))
        .collect::<Vec<_>>()
        .join("  /  ");
    lines_at(
        c,
        wrap_lines(&sites, 96).into_iter().take(3),
        0.0,
        -170.0,
        15.0,
        20.0,
        DIM,
    );
    lines_at(
        c,
        wrap_lines(&s.route_line(), 96),
        0.0,
        -250.0,
        15.0,
        20.0,
        OCHRE,
    );
    text_at(
        c,
        s.field_text("pixel.atlas.control"),
        0.0,
        -326.0,
        15.0,
        INK,
    );
}
fn outcome_message(s: &Session) -> String {
    let w = &s.world;
    let mut message = if s.notice.is_empty() {
        w.latest_outcome().map_or_else(
            || s.field_text("pixel.no_local").into(),
            |o| outcome_label(s.locale, w, o),
        )
    } else {
        s.notice.clone()
    };
    if s.notice.is_empty() {
        if let Some(line) = s.latest_quest_notice() {
            message.push('\n');
            message.push_str(&line);
        }
        if let Some(line) = s.latest_favor_notice() {
            message.push('\n');
            message.push_str(&line);
        }
        if let Some(line) = s.latest_aid_notice() {
            message.push('\n');
            message.push_str(&line);
        }
    }
    message
}
fn normal(c: &mut Commands, g: &Game) {
    let s = &g.session;
    let w = &s.world;
    let p = w.player();
    let sc = w.scenario();
    panel(c, -215.0, 52.0, 584.0, 415.0);
    panel(c, 300.0, 52.0, 430.0, 415.0);
    text_at(
        c,
        s.field_fmt(
            "pixel.position",
            &[
                ("x", &p.position.x.to_string()),
                ("y", &p.position.y.to_string()),
                ("count", &w.exploration().unwrap().count().to_string()),
            ],
        ),
        -215.0,
        234.0,
        15.0,
        OCHRE,
    );
    let width = sc.width.min(17);
    let height = sc.height.min(9);
    let left = p.position.x.saturating_sub(width / 2).min(sc.width - width);
    let top = p
        .position
        .y
        .saturating_sub(height / 2)
        .min(sc.height - height);
    for y in top..top + height {
        for x in left..left + width {
            let cell = Position { x, y };
            let glyph = s.tile_symbol(cell);
            let at = Vec2::new(
                -215.0 + ((x - left) as f32 - (width - 1) as f32 / 2.0) * SIDE,
                56.0 + ((height - 1) as f32 / 2.0 - (y - top) as f32) * SIDE,
            );
            let visible = w.player_sees(cell) || cell == p.position;
            let color = terrain_color(glyph);
            sprite(c, at, 0.0, Vec2::splat(SIDE - 1.0), color);
            draw_icon(c, at, glyph);
            if !visible && glyph != '?' {
                sprite(
                    c,
                    at,
                    2.0,
                    Vec2::splat(SIDE - 1.0),
                    Color::srgba(0.02, 0.04, 0.04, 0.52),
                );
            }
            if let Some(l) = s.field.waypoint
                && w.exploration().unwrap().knows(cell)
                && l.position(w) == cell
            {
                sprite(
                    c,
                    at + Vec2::new(0.0, 13.0),
                    2.5,
                    Vec2::new(SIDE - 2.0, 3.0),
                    OCHRE,
                );
            }
        }
    }
    sprite(c, Vec2::new(-397.0, -119.0), 0.0, Vec2::splat(10.0), MOSS);
    text_at(c, s.field_text("pixel.current"), -323.0, -119.0, 13.0, INK);
    sprite(c, Vec2::new(-181.0, -119.0), 0.0, Vec2::splat(10.0), DIM);
    text_at(c, s.field_text("pixel.memory"), -88.0, -119.0, 13.0, DIM);
    text_at(c, s.field_text("pixel.here"), 300.0, 234.0, 17.0, OCHRE);
    rule(c, 300.0, 212.0, 380.0);
    let local = s.field_entries(Page::Local);
    let mut hints = local
        .iter()
        .filter(|e| !e.command.starts_with("wait"))
        .take(6)
        .map(|e| e.label.clone())
        .collect::<Vec<_>>();
    if let Some(hint) = s.local_quest_hint() {
        hints.insert(0, hint);
    }
    if let Some(hint) = s.local_favor_hint() {
        hints.insert(0, hint);
    }
    if hints.is_empty() {
        hints.push(s.field_text("pixel.no_local").into());
    }
    if w.player_sees(sealed_sim::living::Region::sites().hall) {
        hints.insert(0, s.hall_hint());
    } else if let Some(h) = s.frontier_hint() {
        hints.insert(0, h);
    }
    let hints = hints.join("\n");
    lines_at(
        c,
        wrap_lines(&hints, 44).into_iter().take(10),
        300.0,
        185.0,
        14.0,
        20.0,
        INK,
    );
    rule(c, 300.0, -29.0, 380.0);
    lines_at(
        c,
        wrap_lines(&s.route_line(), 44).into_iter().take(3),
        300.0,
        -52.0,
        15.0,
        19.0,
        OCHRE,
    );
    if s.field.waypoint.is_some() {
        lines_at(
            c,
            wrap_lines(s.field_text("pixel.route"), 44)
                .into_iter()
                .take(2),
            300.0,
            -115.0,
            12.0,
            16.0,
            DIM,
        );
    }
    panel(c, 0.0, -231.0, 1030.0, 127.0);
    let message = outcome_message(s);
    let lines = notice_lines(&message);
    lines_at(
        c,
        lines.iter().take(3).cloned(),
        0.0,
        -192.0,
        15.0,
        20.0,
        INK,
    );
    if lines.len() > 3 {
        text_at(c, s.field_text("pixel.last"), 0.0, -264.0, 13.0, DIM);
    }
    let local = nearby(s);
    if let Some(a) = g.target.filter(|id| local.contains(id))
        && let Some(n) = w.npc(a)
    {
        text_at(
            c,
            format!("T: {} #{}", n.definition.name, a.get()),
            0.0,
            -293.0,
            13.0,
            OCHRE,
        );
    }
    text_at(c, s.field_text("pixel.control"), 0.0, -326.0, 15.0, INK);
}
pub(super) fn draw(c: &mut Commands, g: &Game) {
    let s = &g.session;
    header(c, s);
    if let Some(page) = s.field.page {
        menu(c, s, page);
    } else if let Some(page) = s.field.reader {
        reader(c, s, page);
    } else if s.field.atlas {
        atlas(c, s);
    } else {
        normal(c, g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living_ascii::sandbox_session;
    use crate::locale::Locale;
    use bevy::window::PrimaryWindow;
    use sealed_sim::living::{Background, Direction, Intent, World};
    fn press(app: &mut App, key: KeyCode, shift: bool) {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear();
            keys.release(KeyCode::ShiftRight);
            if shift {
                keys.press(KeyCode::ShiftRight);
            }
            keys.release(key);
            keys.press(key);
        }
        app.update();
    }
    #[test]
    fn c60_modal_navigation_isolated_and_menu_action_matches_direct_intent() {
        let mut app = App::new();
        app.insert_resource(Game {
            session: sandbox_session(3, false),
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (super::super::keyboard, super::super::draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let start = app.world().resource::<Game>().session.world.checkpoint();
        press(&mut app, KeyCode::Enter, false);
        press(&mut app, KeyCode::KeyW, false);
        press(&mut app, KeyCode::Space, false);
        press(&mut app, KeyCode::KeyF, false);
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            start
        );
        press(&mut app, KeyCode::ArrowDown, false);
        press(&mut app, KeyCode::Enter, false);
        let mut expected = World::sandbox_campaign(3);
        expected
            .step(0, Intent::ChooseBackground(Background::Herbalist))
            .unwrap();
        assert_eq!(app.world().resource::<Game>().session.world, expected);
        press(&mut app, KeyCode::Enter, false);
        press(&mut app, KeyCode::ArrowRight, false);
        assert_eq!(
            app.world().resource::<Game>().session.field.page,
            Some(Page::Travel)
        );
        press(&mut app, KeyCode::Enter, false);
        assert!(app.world().resource::<Game>().session.field.atlas);
        press(&mut app, KeyCode::Space, false);
        assert_eq!(app.world().resource::<Game>().session.world, expected);
        press(&mut app, KeyCode::Enter, false);
        let index = app
            .world()
            .resource::<Game>()
            .session
            .field_entries(Page::Travel)
            .iter()
            .position(|e| e.command == "route shrine")
            .unwrap();
        for _ in 0..index {
            press(&mut app, KeyCode::ArrowDown, false);
        }
        press(&mut app, KeyCode::Enter, false);
        assert!(
            app.world()
                .resource::<Game>()
                .session
                .field
                .reader
                .is_some()
        );
        press(&mut app, KeyCode::PageDown, false);
        assert_eq!(app.world().resource::<Game>().session.world, expected);
        press(&mut app, KeyCode::Escape, false);
        press(&mut app, KeyCode::Space, true);
        let d = expected
            .charted_route(sealed_sim::living::exploration::Landmark::Shrine)
            .unwrap()[0];
        expected.step(expected.tick(), Intent::Move(d)).unwrap();
        assert_eq!(app.world().resource::<Game>().session.world, expected);
        press(&mut app, KeyCode::Tab, false);
        assert_eq!(
            app.world().resource::<Game>().session.locale,
            Locale::Indonesian
        );
        press(&mut app, KeyCode::Enter, false);
        let mut q = app.world_mut().query::<&Text2d>();
        let texts = q
            .iter(app.world())
            .map(|x| x.0.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(texts.contains("Tindakan lokal") && texts.contains("Enter konfirmasi"));
        assert_eq!(app.world().resource::<Game>().session.world, expected);
        press(&mut app, KeyCode::Escape, false);
        press(&mut app, KeyCode::ArrowDown, false);
        expected
            .step(expected.tick(), Intent::Move(Direction::South))
            .unwrap();
        assert_eq!(app.world().resource::<Game>().session.world, expected);
    }
}
