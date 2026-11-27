//! Desktop projection of the regional campaign. Keyboard actions enter the
//! same session as terminal commands, and rendering never advances the clock.

use bevy::{prelude::*, window::PrimaryWindow};
use sealed_sim::living::{ActorId, BREATH_MASTERY, MAX_QI, Position, Region};

use crate::{
    clock,
    living_ascii::{Session, campaign_session, outcome_label},
    locale::Locale,
    locale_living::{living_format, living_text},
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
            session: campaign_session(false, 26),
            target: None,
        })
        .insert_resource(ClearColor(Color::srgb(0.048, 0.062, 0.065)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: living_text(Locale::English, "living.pixel.window.title").into(),
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

fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        let command = if game.session.locale == Locale::English {
            "lang id"
        } else {
            "lang en"
        };
        game.session.input(command);
        if let Ok(mut window) = windows.single_mut() {
            window.title = living_text(game.session.locale, "living.pixel.window.title").to_owned();
        }
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
    let tr = |key| living_text(session.locale, key);
    let sites = Region::sites();
    let distance =
        |site: Position| player.position.x.abs_diff(site.x) + player.position.y.abs_diff(site.y);
    let near_hall = world.player_sees(sites.hall);
    let near_market = world.player_sees(sites.market);

    label(
        &mut commands,
        living_format(
            session.locale,
            "living.pixel.draw.template.scenario_name",
            &[("scenario_name", scenario.name)],
        ),
        257.0,
        27.0,
        INK,
    );
    let (hour, minute) = clock::clock_hhmm(clock::minute_of_day(world.calendar_tick()));
    let mut stats = living_format(
        session.locale,
        "living.pixel.draw.template.day_clock_day_number_world_calendar",
        &[
            (
                "clock_day_number_world_calendar_tick",
                &format!("{}", clock::day_number(world.calendar_tick())),
            ),
            ("hour", &format!("{:02}", hour)),
            ("minute", &format!("{:02}", minute)),
            ("player_hp", &format!("{}", player.hp)),
            ("player_herbs", &format!("{}", player.herbs)),
            (
                "world_breath_progress",
                &format!("{}", world.breath_progress()),
            ),
            ("breath_mastery", &format!("{}", BREATH_MASTERY)),
        ],
    );
    if let Some(region) = world.region() {
        stats.push_str(&living_format(
            session.locale,
            "living.pixel.draw.template.coins_region_coins_remedies_region_remedies",
            &[
                ("region_coins", &format!("{}", region.coins)),
                ("region_remedies", &format!("{}", region.remedies)),
            ],
        ));
    }
    if let Some(school) = world.school().filter(|school| school.form_learned) {
        stats.push_str(&living_format(
            session.locale,
            "living.pixel.draw.template.school_qi_max_qi",
            &[
                ("school_qi", &format!("{}", school.qi)),
                ("max_qi", &format!("{}", MAX_QI)),
            ],
        ));
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
        living_format(
            session.locale,
            "living.pixel.draw.template.shrine_herbs_world_supply_shrine_scenario",
            &[
                ("world_supply_shrine", &format!("{}", world.supply().shrine)),
                (
                    "scenario_shrine_target",
                    &format!("{}", scenario.shrine_target),
                ),
            ],
        )
    } else if world.region().is_some()
        && near_hall
        && (!near_market || distance(sites.hall) < distance(sites.market))
    {
        session.hall_hint()
    } else if let Some(region) = world.region().filter(|_| near_market) {
        if world.is_night() {
            tr("living.pixel.draw.willowford_market_opens_at_dawn").into()
        } else {
            let quote = world.market_quote().expect("regional market");
            living_format(
                session.locale,
                "living.pixel.draw.template.market_herbs_region_market_herbs_buy",
                &[
                    ("region_market_herbs", &format!("{}", region.market_herbs)),
                    ("quote_buy", &format!("{}", quote.buy)),
                    ("quote_sell", &format!("{}", quote.sell)),
                ],
            )
        }
    } else {
        tr("living.pixel.draw.willowford_lies_east_of_qinghe").into()
    };
    label(&mut commands, local, -132.0, 16.0, OCHRE);
    let message = if session.notice.is_empty() {
        world.latest_outcome().map_or_else(
            || tr("living.pixel.draw.choose_a_background_1_wanderer_2_herbalist").into(),
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
            living_format(
                session.locale,
                "living.pixel.draw.template.next_target_npc_definition_name",
                &[("npc_definition_name", npc.definition.name)],
            ),
            -198.0,
            14.0,
            OCHRE,
        );
    }
    label(
        &mut commands,
        tr("living.pixel.draw.wasd_move_space_wait_e_talk_f").into(),
        -230.0,
        14.0,
        INK,
    );
    label(
        &mut commands,
        tr("living.pixel.draw.u_buy_i_sell_j_join_k").into(),
        -257.0,
        14.0,
        INK,
    );
    if world.company().is_some() {
        label(
            &mut commands,
            tr("living.pixel.draw.n_invite_o_pack_herb_r_part").into(),
            -304.0,
            14.0,
            INK,
        );
    }
    if world.school().is_some() {
        label(
            &mut commands,
            tr("living.pixel.draw.y_shrine_duty_z_study_x_center").into(),
            -280.0,
            14.0,
            OCHRE,
        );
    }
    if world.politics().is_some() {
        label(
            &mut commands,
            tr("living.pixel.draw.4_pledge_5_supply_stonebank_6_renounce").into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_tab_refreshes_real_projection_without_advancing_gameplay() {
        let mut app = App::new();
        app.insert_resource(Game {
            session: campaign_session(false, 26),
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((
            Window {
                title: living_text(Locale::English, "living.pixel.window.title").into(),
                ..default()
            },
            PrimaryWindow,
        ));
        app.update();
        let before = app.world().resource::<Game>().session.world.checkpoint();
        let snapshot = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|x| x.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(snapshot(&mut app).contains("Day"));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        assert_eq!(
            app.world().resource::<Game>().session.locale,
            Locale::Indonesian
        );
        assert!(snapshot(&mut app).contains("Hari"));
        assert!(snapshot(&mut app).contains("Spasi tunggu"));
        let mut windows = app
            .world_mut()
            .query_filtered::<&Window, With<PrimaryWindow>>();
        assert_eq!(
            windows.single(app.world()).unwrap().title,
            living_text(Locale::Indonesian, "living.pixel.window.title")
        );
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }
}
