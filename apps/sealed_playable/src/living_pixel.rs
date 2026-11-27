//! Desktop projection of the regional campaign. Keyboard actions enter the
//! same session as terminal commands, and rendering never advances the clock.

use bevy::prelude::*;
use sealed_sim::living::{ActorId, BREATH_MASTERY, MAX_QI, Position, Region};

use crate::{
    clock,
    living_ascii::{Session, campaign_session, outcome_label, text},
    locale::Locale,
};

const SIDE: f32 = 32.0;
const INK: Color = Color::srgb(0.94, 0.90, 0.78);
const MOSS: Color = Color::srgb(0.18, 0.26, 0.22);
const OCHRE: Color = Color::srgb(0.80, 0.63, 0.36);
const STONE: Color = Color::srgb(0.30, 0.35, 0.38);
const FOG: Color = Color::srgb(0.07, 0.09, 0.10);

#[derive(Resource)]
struct Game {
    session: Session,
    target: Option<ActorId>,
}

#[derive(Component)]
struct Projection;

pub(crate) fn run() {
    App::new()
        .insert_resource(Game {
            session: campaign_session(false, 25),
            target: None,
        })
        .insert_resource(ClearColor(Color::srgb(0.048, 0.062, 0.065)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sealed Heaven — Riverlands".into(),
                resolution: (900, 660).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (keyboard, draw).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

const COMMANDS: &[(KeyCode, &str)] = &[
    (KeyCode::KeyW, "w"),
    (KeyCode::KeyA, "a"),
    (KeyCode::KeyS, "s"),
    (KeyCode::KeyD, "d"),
    (KeyCode::Space, "wait"),
    (KeyCode::KeyB, "guard"),
    (KeyCode::KeyG, "gather"),
    (KeyCode::KeyH, "deliver"),
    (KeyCode::KeyM, "practice"),
    (KeyCode::KeyU, "buy"),
    (KeyCode::KeyI, "sell"),
    (KeyCode::KeyJ, "join"),
    (KeyCode::KeyK, "brew"),
    (KeyCode::KeyL, "remedy"),
    (KeyCode::KeyY, "duty"),
    (KeyCode::KeyZ, "study"),
    (KeyCode::KeyX, "center"),
    (KeyCode::KeyC, "accord"),
    (KeyCode::KeyV, "sponsor"),
    (KeyCode::Digit4, "pledge"),
    (KeyCode::Digit5, "aidlodge"),
    (KeyCode::Digit6, "renounce"),
    (KeyCode::Digit1, "background wanderer"),
    (KeyCode::Digit2, "background herbalist"),
    (KeyCode::Digit3, "background peddler"),
    (KeyCode::F5, "save"),
    (KeyCode::F9, "load"),
];

const TARGET_COMMANDS: &[(KeyCode, &str)] = &[
    (KeyCode::KeyE, "talk"),
    (KeyCode::KeyF, "attack"),
    (KeyCode::KeyP, "form"),
    (KeyCode::KeyN, "invite"),
    (KeyCode::KeyO, "pack"),
    (KeyCode::KeyR, "part"),
];

fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    if keys.just_pressed(KeyCode::Tab) {
        let command = if game.session.locale == Locale::English {
            "lang id"
        } else {
            "lang en"
        };
        game.session.input(command);
    } else if keys.just_pressed(KeyCode::KeyT) {
        let nearby = nearby(&game.session);
        let next = nearby
            .iter()
            .position(|&id| Some(id) == game.target)
            .map_or(0, |index| index + 1);
        game.target = nearby
            .get(next)
            .copied()
            .or_else(|| nearby.first().copied());
    } else if let Some((_, command)) = TARGET_COMMANDS
        .iter()
        .find(|(key, _)| keys.just_pressed(*key))
    {
        let nearby = nearby(&game.session);
        let target = game
            .target
            .filter(|id| nearby.contains(id))
            .or_else(|| nearby.first().copied());
        match target {
            Some(id) => {
                game.session.input(&format!("{command} {}", id.get()));
            }
            None => {
                game.session.input(command);
            }
        }
    } else if let Some((_, command)) = COMMANDS.iter().find(|(key, _)| keys.just_pressed(*key)) {
        game.session.input(command);
    }
}

fn nearby(session: &Session) -> Vec<ActorId> {
    let player = session.world.player().position;
    session
        .world
        .npcs()
        .filter(|npc| {
            npc.alive()
                && session.world.player_sees(npc.position)
                && npc.position.x.abs_diff(player.x) + npc.position.y.abs_diff(player.y) <= 1
        })
        .map(|npc| npc.id())
        .collect()
}

fn draw(mut commands: Commands, game: Res<Game>, previous: Query<Entity, With<Projection>>) {
    if !game.is_changed() {
        return;
    }
    for entity in &previous {
        commands.entity(entity).despawn();
    }
    let session = &game.session;
    let world = &session.world;
    let player = world.player();
    let scenario = world.scenario();
    let tr = |en, id| text(session.locale, en, id);
    let sites = Region::sites();
    let distance =
        |site: Position| player.position.x.abs_diff(site.x) + player.position.y.abs_diff(site.y);
    let near_hall = world.player_sees(sites.hall);
    let near_market = world.player_sees(sites.market);

    label(
        &mut commands,
        format!("Sealed Heaven   /   {}", scenario.name),
        257.0,
        27.0,
        INK,
    );
    let (hour, minute) = clock::clock_hhmm(clock::minute_of_day(world.calendar_tick()));
    let mut stats = format!(
        "{} {}   {hour:02}:{minute:02}   |   HP {}   |   {} {}   |   {} {}/{}",
        tr("Day", "Hari"),
        clock::day_number(world.calendar_tick()),
        player.hp,
        tr("Herbs", "Herba"),
        player.herbs,
        tr("Breath", "Napas"),
        world.breath_progress(),
        BREATH_MASTERY,
    );
    if let Some(region) = world.region() {
        stats.push_str(&format!(
            "   |   {} {}   |   {} {}",
            tr("Coins", "Koin"),
            region.coins,
            tr("Remedies", "Obat"),
            region.remedies
        ));
    }
    if let Some(school) = world.school().filter(|school| school.form_learned) {
        stats.push_str(&format!("   |   Qi {}/{MAX_QI}", school.qi));
    }
    label(&mut commands, stats, 219.0, 16.0, OCHRE);

    let width = scenario.width.min(17);
    let left = player
        .position
        .x
        .saturating_sub(width / 2)
        .min(scenario.width - width);
    for y in 0..scenario.height {
        for x in left..left + width {
            let cell = Position { x, y };
            let symbol = session.tile_symbol(cell);
            let at = Vec2::new(
                ((x - left) as f32 - (width - 1) as f32 / 2.0) * SIDE,
                ((scenario.height - 1) as f32 / 2.0 - y as f32) * SIDE + 37.0,
            );
            let tile_color = match symbol {
                '?' => FOG,
                '#' => STONE,
                _ if (x + y) % 2 == 0 => MOSS,
                _ => Color::srgb(0.21, 0.29, 0.24),
            };
            sprite(&mut commands, at, 0.0, Vec2::splat(SIDE - 1.0), tile_color);
            draw_icon(&mut commands, at, symbol);
        }
    }

    let event = world
        .company()
        .and_then(|company| company.history.last())
        .filter(|event| event.witnessed && event.at == world.tick());
    let local = if let Some(event) = event {
        session.company_line(event)
    } else if let Some(hint) = session.duty_hint() {
        hint.into()
    } else if world.player_sees(scenario.shrine) {
        format!(
            "{} {}/{}",
            tr("Shrine herbs", "Herba kuil"),
            world.supply().shrine,
            scenario.shrine_target
        )
    } else if world.region().is_some()
        && near_hall
        && (!near_market || distance(sites.hall) < distance(sites.market))
    {
        session.hall_hint()
    } else if let Some(region) = world.region().filter(|_| near_market) {
        if world.is_night() {
            tr(
                "Willowford market opens at dawn",
                "Pasar Willowford buka saat fajar",
            )
            .into()
        } else {
            let quote = world.market_quote().expect("regional market");
            format!(
                "{} {}   |   U {} {}   I {} {}",
                tr("Market herbs", "Herba pasar"),
                region.market_herbs,
                tr("buy", "beli"),
                quote.buy,
                tr("sell", "jual"),
                quote.sell
            )
        }
    } else {
        tr(
            "Willowford lies east of Qinghe.",
            "Willowford terletak di timur Qinghe.",
        )
        .into()
    };
    label(&mut commands, local, -132.0, 16.0, OCHRE);
    let message = if session.notice.is_empty() {
        world.latest_outcome().map_or_else(
            || {
                tr(
                    "Choose a background   1 Wanderer   2 Herbalist   3 Peddler",
                    "Pilih latar   1 Pengembara   2 Pemetik herba   3 Pedagang keliling",
                )
                .into()
            },
            |event| outcome_label(session.locale, world, event),
        )
    } else {
        session.notice.clone()
    };
    label(&mut commands, message, -171.0, 16.0, INK);
    let nearby = nearby(session);
    let target = game
        .target
        .filter(|id| nearby.contains(id))
        .or_else(|| nearby.first().copied());
    if let Some(npc) = target.and_then(|id| world.npc(id)) {
        label(
            &mut commands,
            format!(
                "T {}   {}",
                tr("next target", "target berikutnya"),
                npc.definition.name
            ),
            -198.0,
            14.0,
            OCHRE,
        );
    }
    label(&mut commands, tr("WASD move   Space wait   E talk   F attack   B guard   G gather   H deliver   M practice",
        "WASD bergerak   Spasi tunggu   E bicara   F serang   B bertahan   G petik   H sumbang   M latihan").into(),
        -230.0, 14.0, INK);
    label(
        &mut commands,
        tr(
            "U buy   I sell   J join   K brew   L remedy   F5 save   F9 load   Tab Indonesia",
            "U beli   I jual   J gabung   K racik   L minum   F5 simpan   F9 muat   Tab English",
        )
        .into(),
        -257.0,
        14.0,
        INK,
    );
    if world.company().is_some() {
        label(
            &mut commands,
            tr(
                "N invite   O pack herb   R part   C accord   V sponsor",
                "N ajak   O bekali   R pisah   C gencatan   V dukung balai",
            )
            .into(),
            -304.0,
            14.0,
            INK,
        );
    }
    if world.school().is_some() {
        label(
            &mut commands,
            tr(
                "Y shrine duty   Z study   X center   P Reed Strike",
                "Y tugas kuil   Z belajar   X pulihkan Qi   P Jurus Alang-alang",
            )
            .into(),
            -280.0,
            14.0,
            OCHRE,
        );
    }
    if world.politics().is_some() {
        label(
            &mut commands,
            tr(
                "4 pledge   5 supply Stonebank   6 renounce at Hall",
                "4 sumpah   5 bekali Batu Tepi   6 lepas sumpah di balai",
            )
            .into(),
            -318.0,
            13.0,
            OCHRE,
        );
    }
}

fn draw_icon(commands: &mut Commands, at: Vec2, symbol: char) {
    let (art, accent, light) = match symbol {
        '#' => (
            &["11111", "12121", "11111", "21112"][..],
            STONE,
            Color::srgb(0.49, 0.54, 0.55),
        ),
        '*' | '&' => (
            &["00100", "20102", "02220", "00100"][..],
            Color::srgb(0.42, 0.69, 0.36),
            INK,
        ),
        '+' | '=' | '$' | '^' => (
            &["00100", "01110", "11111", "12021", "12221"][..],
            Color::srgb(0.53, 0.30, 0.21),
            OCHRE,
        ),
        '@' => (
            &["01110", "01210", "11111", "01210", "01010"][..],
            OCHRE,
            INK,
        ),
        'x' => (
            &["10001", "01010", "00100", "01010", "10001"][..],
            Color::srgb(0.56, 0.31, 0.27),
            INK,
        ),
        '?' | '.' => return,
        _ => (
            &["01110", "01210", "01110", "11111", "01010"][..],
            Color::srgb(0.42, 0.62, 0.68),
            INK,
        ),
    };
    for (row, pixels) in art.iter().enumerate() {
        for (column, shade) in pixels.bytes().enumerate() {
            let color = match shade {
                b'1' => accent,
                b'2' => light,
                _ => continue,
            };
            let pixel = at + Vec2::new((column as f32 - 2.0) * 4.0, (2.0 - row as f32) * 4.0);
            sprite(commands, pixel, 1.0, Vec2::splat(4.0), color);
        }
    }
}

fn sprite(commands: &mut Commands, at: Vec2, depth: f32, size: Vec2, color: Color) {
    commands.spawn((
        Projection,
        Sprite::from_color(color, size),
        Transform::from_translation(at.extend(depth)),
    ));
}

fn label(commands: &mut Commands, value: String, y: f32, size: f32, color: Color) {
    commands.spawn((
        Projection,
        Text2d::new(value),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
        Transform::from_xyz(0.0, y, 3.0),
    ));
}
