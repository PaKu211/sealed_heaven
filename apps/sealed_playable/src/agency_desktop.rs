//! Disposable C05 view/input. No position, inventory or NPC/task authority.
use bevy::{
    input::{
        ButtonState,
        keyboard::KeyboardInput,
        mouse::{MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    render::{
        renderer::RenderAdapterInfo,
        view::screenshot::{Screenshot, save_to_disk},
    },
    text::LineBreak,
    window::PrimaryWindow,
};
use sealed_playable::{
    agency::{Action, AgencyPlugin, Input, Session, drive_frame},
    hex,
    locale::{Catalog, Locale},
    view::View,
};
use sealed_sim::player5::{
    Direction, Event, HEIGHT, Intent, Outcome, Position, Rejection, Target, WIDTH,
};
use std::{collections::VecDeque, fmt::Write};

const CELL: f32 = 48.0;
const PANEL: f32 = 440.0;
const TOP: f32 = 64.0;
const FOOTER: f32 = 94.0;
const INK: Color = Color::srgb(0.92, 0.94, 0.89);

#[derive(Resource)]
struct Strings(Catalog);
#[derive(Resource, Default)]
struct Performance {
    frames: u64,
    intervals_ms: VecDeque<f64>,
}
#[derive(Component)]
struct MainCamera;
#[derive(Component)]
struct PlayerMarker;
#[derive(Component)]
struct NpcMarker(u32);
#[derive(Component)]
struct TargetRing;
#[derive(Component)]
struct ScrollPanel;
#[derive(Component, Clone, Copy, Debug)]
enum Control {
    Toggle,
    Step,
    Save,
    Load,
    Reset,
    Language,
    Interact,
    Follow,
}
#[derive(Component)]
enum Label {
    Key(&'static str),
    Toggle,
    Summary,
    Body,
    Help,
    Context,
    Marker(Target),
}

pub fn run() {
    let strings = Catalog::agency().expect("validated C05 localization");
    App::new()
        .insert_resource(Strings(strings))
        .init_resource::<Performance>()
        .insert_resource(ClearColor(Color::srgb(0.035, 0.055, 0.065)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sealed Heaven — C0.5 Qinghe".into(),
                resolution: (1440, 960).into(),
                resize_constraints: bevy::window::WindowResizeConstraints {
                    min_width: 1280.0,
                    min_height: 760.0,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        .add_plugins(AgencyPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (buttons, keyboard, select_target)
                .chain()
                .before(drive_frame),
        )
        .add_systems(Update, (follow_camera, scroll, metrics))
        .add_systems(Update, (paint, capture).chain().after(drive_frame))
        .run();
}

fn at(position: Position) -> Vec2 {
    Vec2::new(
        (position.x as f32 - 5.5) * CELL,
        (4.0 - position.y as f32) * CELL,
    )
}

fn npc_at(id: u32, position: Position) -> Vec2 {
    // Deliberate presentation-only offsets. Canonical occupancy is cell-based.
    let slot = id.saturating_sub(1);
    at(position)
        + Vec2::new(
            (slot % 5) as f32 * 8.0 - 16.0,
            (slot / 5) as f32 * 8.0 - 12.0,
        )
}

fn targets() -> Vec<Target> {
    [Target::Npc(1), Target::HerbPatch, Target::Shrine]
        .into_iter()
        .chain((2..=20).map(Target::Npc))
        .collect()
}

fn target_name(target: Target, catalog: &Catalog, locale: Locale) -> String {
    catalog.text(
        locale,
        match target {
            Target::Npc(1) => "c05.healer",
            Target::HerbPatch => "c05.patch",
            Target::Shrine => "c05.shrine",
            Target::Npc(_) => "inspector.npc",
        },
    ) + &match target {
        Target::Npc(id) if id != 1 => format!(" {id}"),
        _ => String::new(),
    }
}

fn context_intent(target: Target) -> Intent {
    match target {
        Target::Npc(id) => Intent::Talk(id),
        Target::HerbPatch => Intent::Gather,
        Target::Shrine => Intent::Deliver,
    }
}

fn setup(mut commands: Commands, session: Res<Session>, mut fonts: ResMut<Assets<Font>>) {
    fonts
        .insert(
            bevy::asset::AssetId::default(),
            Font::try_from_bytes(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec())
                .expect("bundled font"),
        )
        .expect("default font handle");
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_xyz(PANEL / 2.0, 0.0, 0.0),
    ));
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let pos = Position { x, y };
            let color = if !session.world().is_walkable(pos) {
                Color::srgb(0.15, 0.17, 0.19)
            } else if x == 6 && y == 4 {
                Color::srgb(0.41, 0.32, 0.19)
            } else {
                Color::srgb(if (x + y) % 2 == 0 { 0.12 } else { 0.14 }, 0.22, 0.18)
            };
            commands.spawn((
                Sprite::from_color(color, Vec2::splat(CELL - 2.0)),
                Transform::from_translation(at(pos).extend(0.0)),
            ));
        }
    }
    for (target, color, mark) in [
        (Target::HerbPatch, Color::srgb(0.28, 0.85, 0.43), "*"),
        (Target::Shrine, Color::srgb(0.76, 0.39, 0.34), "+"),
    ] {
        let p = session
            .world()
            .target_position(target)
            .expect("authored object");
        commands.spawn((
            Sprite::from_color(color, Vec2::splat(30.0)),
            Transform::from_translation(at(p).extend(3.0)),
        ));
        commands.spawn((
            Text2d::new(mark),
            TextFont {
                font_size: 23.0,
                ..default()
            },
            TextColor(Color::BLACK),
            Transform::from_translation(at(p).extend(4.0)),
        ));
        commands.spawn((
            Text2d::default(),
            Label::Marker(target),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(INK),
            Transform::from_translation((at(p) + Vec2::new(0.0, -31.0)).extend(7.0)),
        ));
    }
    for id in 1..=20 {
        commands.spawn((
            NpcMarker(id),
            Sprite::from_color(
                if id == 1 {
                    Color::srgb(0.6, 0.85, 1.0)
                } else {
                    Color::srgb(0.35, 0.61, 0.81)
                },
                Vec2::splat(if id == 1 { 15.0 } else { 7.0 }),
            ),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    commands.spawn((
        PlayerMarker,
        Sprite::from_color(Color::srgb(1.0, 0.8, 0.23), Vec2::splat(23.0)),
        Transform::from_translation(at(session.world().player_position()).extend(8.0)),
    ));
    commands.spawn((
        TargetRing,
        Sprite::from_color(Color::srgba(0.9, 0.9, 1.0, 0.3), Vec2::splat(44.0)),
        Transform::default(),
    ));
    commands.spawn((
        Text2d::default(),
        Label::Marker(Target::Npc(1)),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(INK),
        Transform::from_xyz(-130.0, 95.0, 9.0),
    ));
    commands.spawn((
        Text2d::new("21"),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.85, 0.3)),
        Transform::from_translation(
            (at(Position { x: 0, y: 4 }) + Vec2::new(0.0, -21.0)).extend(7.0),
        ),
    ));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                height: px(TOP),
                padding: UiRect::all(px(10)),
                column_gap: px(8),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.1, 0.12)),
        ))
        .with_children(|bar| {
            bar.spawn((
                Text::default(),
                Label::Key("c05.title"),
                TextFont {
                    font_size: 19.0,
                    ..default()
                },
                TextColor(INK),
                Node {
                    width: px(260),
                    ..default()
                },
            ));
            for (control, label) in [
                (Control::Toggle, Label::Toggle),
                (Control::Step, Label::Key("controls.step")),
                (Control::Save, Label::Key("controls.save")),
                (Control::Load, Label::Key("controls.load")),
                (Control::Reset, Label::Key("controls.reset")),
                (Control::Language, Label::Key("controls.language")),
                (Control::Follow, Label::Key("c05.follow")),
            ] {
                bar.spawn((
                    Button,
                    control,
                    Node {
                        padding: UiRect::axes(px(10), px(7)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.23, 0.26)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::default(),
                        label,
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(INK),
                    ));
                });
            }
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(TOP),
                bottom: px(FOOTER),
                width: px(PANEL),
                padding: UiRect::all(px(16)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::srgb(0.065, 0.085, 0.1)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::default(),
                Label::Summary,
                TextFont {
                    font_size: 17.0,
                    ..default()
                },
                TextColor(INK),
            ));
            panel
                .spawn((
                    Button,
                    Control::Interact,
                    Node {
                        padding: UiRect::all(px(12)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.17, 0.32, 0.3)),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::default(),
                        Label::Context,
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(INK),
                    ));
                });
            panel
                .spawn((
                    ScrollPanel,
                    ScrollPosition::default(),
                    Node {
                        flex_grow: 1.0,
                        min_height: px(0),
                        width: percent(100),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|scroll| {
                    scroll.spawn((
                        Text::default(),
                        Label::Body,
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(INK),
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        Node {
                            width: percent(100),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                });
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                height: px(FOOTER),
                padding: UiRect::all(px(14)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.1, 0.12)),
        ))
        .with_children(|f| {
            f.spawn((
                Text::default(),
                Label::Help,
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(INK),
            ));
        });
}

fn toggle(session: &Session) -> Action {
    if session.paused {
        Action::Resume
    } else {
        Action::Pause
    }
}
fn language(locale: Locale) -> Locale {
    if locale == Locale::English {
        Locale::Indonesian
    } else {
        Locale::English
    }
}

fn buttons(
    buttons: Query<(&Interaction, &Control), Changed<Interaction>>,
    session: Res<Session>,
    mut input: ResMut<Input>,
) {
    for (interaction, control) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let action = match control {
            Control::Toggle => toggle(&session),
            Control::Step => Action::Step,
            Control::Save => Action::Save,
            Control::Load => Action::Load,
            Control::Reset => Action::Reset,
            Control::Language => Action::Language(language(session.locale)),
            Control::Interact => Action::Intent(context_intent(session.selected)),
            Control::Follow => continue,
        };
        input.actions.push(action);
    }
}

fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    time: Res<Time>,
    held: Res<ButtonInput<KeyCode>>,
    session: Res<Session>,
    mut input: ResMut<Input>,
    mut repeat: Local<f64>,
) {
    input.elapsed_ns = time.delta().as_nanos();
    let movement = |key| match key {
        KeyCode::KeyW | KeyCode::ArrowUp => Some(Direction::North),
        KeyCode::KeyD | KeyCode::ArrowRight => Some(Direction::East),
        KeyCode::KeyS | KeyCode::ArrowDown => Some(Direction::South),
        KeyCode::KeyA | KeyCode::ArrowLeft => Some(Direction::West),
        _ => None,
    };
    let mut new_move = false;
    for event in keys.read() {
        if event.state != ButtonState::Pressed || event.repeat {
            continue;
        }
        let action = if let Some(direction) = movement(event.key_code) {
            *repeat = 0.0;
            new_move = true;
            Some(Action::Intent(Intent::Move(direction)))
        } else {
            match event.key_code {
                KeyCode::Space => Some(toggle(&session)),
                KeyCode::KeyN => Some(Action::Step),
                KeyCode::F5 => Some(Action::Save),
                KeyCode::F9 => Some(Action::Load),
                KeyCode::KeyR => Some(Action::Reset),
                KeyCode::KeyL => Some(Action::Language(language(session.locale))),
                KeyCode::KeyE => Some(Action::Intent(context_intent(session.selected))),
                KeyCode::Tab => {
                    let all = targets();
                    let index = all.iter().position(|t| *t == session.selected).unwrap_or(0);
                    Some(Action::Select(all[(index + 1) % all.len()]))
                }
                _ => None,
            }
        };
        if let Some(action) = action {
            input.actions.push(action);
        }
    }
    *repeat += time.delta_secs_f64();
    if !new_move && *repeat >= 0.5 && session.queued() == 0 && !session.paused {
        for key in [
            KeyCode::KeyW,
            KeyCode::ArrowUp,
            KeyCode::KeyD,
            KeyCode::ArrowRight,
            KeyCode::KeyS,
            KeyCode::ArrowDown,
            KeyCode::KeyA,
            KeyCode::ArrowLeft,
        ] {
            if held.pressed(key) {
                input
                    .actions
                    .push(Action::Intent(Intent::Move(movement(key).unwrap())));
                *repeat = 0.0;
                break;
            }
        }
    }
}

fn select_target(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    session: Res<Session>,
    mut input: ResMut<Input>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if cursor.x >= window.width() - PANEL || cursor.y < TOP || cursor.y > window.height() - FOOTER {
        return;
    }
    let (camera, global) = *camera;
    let Ok(world) = camera.viewport_to_world_2d(global, cursor) else {
        return;
    };
    let nearest = targets()
        .into_iter()
        .filter_map(|target| {
            let p = session.world().target_position(target)?;
            let point = match target {
                Target::Npc(id) => npc_at(id, p),
                _ => at(p),
            };
            Some((target, point.distance_squared(world)))
        })
        .filter(|(_, distance)| *distance < 28.0 * 28.0)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((target, _)) = nearest {
        input.actions.push(Action::Select(target));
    }
}

fn follow_camera(
    session: Res<Session>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Query<(&Interaction, &Control), Changed<Interaction>>,
) {
    // Clamped follow keeps the whole compact map visible while centering on the
    // player within its safe range. UI/camera positions never enter checkpoints.
    let p = at(session.world().player_position());
    let half = ((window.width() - PANEL) / 2.0 - (WIDTH as f32 * CELL) / 2.0).max(0.0);
    let desired = p.x.clamp(-half, half) + PANEL / 2.0;
    camera.translation.x = desired;
    camera.translation.y = 0.0;
    if keys.just_pressed(KeyCode::KeyC)
        || buttons
            .iter()
            .any(|(i, c)| *i == Interaction::Pressed && matches!(c, Control::Follow))
    {
        camera.translation.x = PANEL / 2.0;
    }
}

fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut panel: Single<&mut ScrollPosition, With<ScrollPanel>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let over = window
        .cursor_position()
        .is_some_and(|p| p.x > window.width() - PANEL);
    for event in wheel.read() {
        if over {
            panel.y = (panel.y
                - event.y
                    * if event.unit == MouseScrollUnit::Line {
                        34.0
                    } else {
                        1.0
                    })
            .max(0.0);
        }
    }
    if keys.just_pressed(KeyCode::PageDown) {
        panel.y += 220.0;
    }
    if keys.just_pressed(KeyCode::PageUp) {
        panel.y = (panel.y - 220.0).max(0.0);
    }
    if keys.just_pressed(KeyCode::Home) {
        panel.y = 0.0;
    }
    if keys.just_pressed(KeyCode::End) {
        panel.y = 10000.0;
    }
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
    }
}

fn event_text(event: &Event, cat: &Catalog, locale: Locale) -> String {
    let label = match event.outcome {
        Outcome::Moved => cat.text(locale, "c05.moved"),
        Outcome::Requested => cat.text(locale, "c05.requested"),
        Outcome::Gathered => cat.text(locale, "c05.gathered"),
        Outcome::Delivered => cat.text(locale, "c05.delivered_ok"),
        Outcome::Rejected(reason) => format!(
            "{}: {}",
            cat.text(locale, "c05.rejected"),
            cat.text(locale, rejection_key(reason))
        ),
    };
    format!("#{} · t{} · {label}", event.id, event.tick)
}

fn body_text(session: &Session, cat: &Catalog) -> String {
    let world = session.world();
    let locale = session.locale;
    let t = |key| cat.text(locale, key);
    let mut text = format!(
        "{}\n{}: {}\n{}: {}\n{}: {}\n\n{}\n{}\n\n{}\n{}\n",
        t("c05.inventory"),
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
        t(&session.status),
        t("c05.target"),
        target_name(session.selected, cat, locale)
    );
    let _ = writeln!(
        text,
        "{}",
        t(if world.in_range(session.selected) {
            "c05.range"
        } else {
            "c05.far"
        })
    );
    if let Some(p) = world.target_position(session.selected) {
        let _ = writeln!(text, "{}: ({},{})", t("c05.cell"), p.x, p.y);
    }
    if let Target::Npc(id) = session.selected
        && let Ok(view) = View::from_sim(world.npcs())
        && let Some(actor) = view.actors.iter().find(|a| a.id == id)
    {
        let _ = writeln!(
            text,
            "{}: {:?}\n{}: {}",
            t("inspector.needs"),
            actor.needs,
            t("inspector.task"),
            actor.task
        );
    }
    let _ = writeln!(
        text,
        "\n{}: {} / {}\n{}: {}\n",
        t("c05.npc_alive"),
        world.npcs().tick(),
        world.npcs().next_event_id(),
        t("c05.queue"),
        session.queued()
    );
    let _ = writeln!(text, "{}", t("c05.pending"));
    for event in session.recent.iter().rev().take(6) {
        let _ = writeln!(text, "{}", event_text(event, cat, locale));
    }
    if !session.diagnostic.is_empty() {
        let _ = writeln!(text, "\n{}", session.diagnostic);
    }
    let hash = hex(&world.hash());
    let _ = write!(
        text,
        "\n{}\n{}\n\n{}\n{}",
        t("c05.saved_prefix"),
        hash,
        session.save_path.display(),
        t("help.save")
    );
    text
}

#[allow(clippy::type_complexity)]
fn paint(
    session: Res<Session>,
    strings: Res<Strings>,
    mut texts: Query<(&Label, Option<&mut Text>, Option<&mut Text2d>)>,
    mut player: Single<
        &mut Transform,
        (With<PlayerMarker>, Without<NpcMarker>, Without<TargetRing>),
    >,
    mut npcs: Query<
        (&NpcMarker, &mut Transform, &mut Visibility),
        (Without<PlayerMarker>, Without<TargetRing>),
    >,
    mut ring: Single<
        (&mut Transform, &mut Visibility),
        (With<TargetRing>, Without<PlayerMarker>, Without<NpcMarker>),
    >,
    mut stamp: Local<String>,
) {
    let world = session.world();
    player.translation = at(world.player_position()).extend(8.0);
    for (marker, mut transform, mut visible) in &mut npcs {
        if let Some(pos) = world.npc_position(marker.0) {
            transform.translation =
                npc_at(marker.0, pos).extend(if marker.0 == 1 { 6.0 } else { 5.0 });
            *visible = Visibility::Visible;
        } else {
            *visible = Visibility::Hidden;
        }
    }
    if let Some(pos) = world.target_position(session.selected) {
        ring.0.translation = at(pos).extend(1.0);
        *ring.1 = Visibility::Visible;
    } else {
        *ring.1 = Visibility::Hidden;
    }
    let state = format!(
        "{}|{}|{:?}|{:?}|{}|{}|{}",
        world.tick(),
        session.paused,
        session.locale,
        session.selected,
        session.queued(),
        session.status,
        session.diagnostic
    );
    if *stamp == state {
        return;
    }
    *stamp = state;
    let cat = &strings.0;
    let locale = session.locale;
    let t = |key| cat.text(locale, key);
    for (label, ui, world_text) in &mut texts {
        let value = match label {
            Label::Key(key) => t(key),
            Label::Toggle => t(if session.paused {
                "controls.resume"
            } else {
                "controls.pause"
            }),
            Label::Help => t("c05.controls"),
            Label::Summary => format!(
                "{}\n{}: ({},{}) · {} {}\n{}",
                t("c05.player"),
                t("c05.cell"),
                world.player_position().x,
                world.player_position().y,
                t("inspector.tick"),
                world.tick(),
                t(if session.paused {
                    "status.paused"
                } else {
                    "status.running"
                })
            ),
            Label::Body => body_text(&session, cat),
            Label::Context => t(match session.selected {
                Target::Npc(_) => "c05.talk",
                Target::HerbPatch => "c05.gather",
                Target::Shrine => "c05.deliver",
            }),
            Label::Marker(target) => target_name(*target, cat, locale),
        };
        if let Some(mut text) = ui {
            text.0 = value;
        } else if let Some(mut text) = world_text {
            text.0 = value;
        }
    }
}

fn metrics(time: Res<Time>, mut perf: ResMut<Performance>) {
    perf.frames += 1;
    if perf.frames > 60 {
        perf.intervals_ms.push_back(time.delta_secs_f64() * 1000.0);
        while perf.intervals_ms.len() > 600 {
            perf.intervals_ms.pop_front();
        }
    }
}

fn summary(values: impl IntoIterator<Item = f64>) -> serde_json::Value {
    let mut values: Vec<_> = values.into_iter().collect();
    if values.is_empty() {
        return serde_json::Value::Null;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    serde_json::json!({"samples":n,"mean_ms":values.iter().sum::<f64>()/n as f64,"p50_ms":values[(n-1)/2],"p95_ms":values[(n-1)*95/100],"max_ms":values[n-1]})
}

#[derive(bevy::ecs::system::SystemParam)]
struct CaptureSurface<'w, 's> {
    adapter: Option<Res<'w, RenderAdapterInfo>>,
    camera: Single<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>,
    buttons: Query<
        'w,
        's,
        (
            &'static Control,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    entities: Query<'w, 's, Entity>,
}

fn capture(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    session: Res<Session>,
    strings: Res<Strings>,
    performance: Res<Performance>,
    surface: CaptureSurface,
    mut index: Local<u64>,
) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }
    let Some(dir) = std::env::var_os("SEALED_HEAVEN_EVIDENCE_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!("{error}");
        return;
    }
    *index += 1;
    let name = format!("c05-{:03}", *index);
    let world = session.world();
    let (camera, transform) = *surface.camera;
    let positions: Vec<_> = targets()
        .into_iter()
        .filter_map(|target| {
            let p = world.target_position(target)?;
            let p = match target {
                Target::Npc(id) => npc_at(id, p),
                _ => at(p),
            };
            camera
                .world_to_viewport(transform, p.extend(5.0))
                .ok()
                .map(|p| serde_json::json!({"target":format!("{target:?}"),"x":p.x,"y":p.y}))
        })
        .collect();
    let buttons:Vec<_>=surface.buttons.iter().map(|(control,node,transform)|serde_json::json!({"control":format!("{control:?}"),"x":transform.translation.x,"y":transform.translation.y,"width":node.size().x,"height":node.size().y})).collect();
    let report = serde_json::json!({"tick":world.tick(),"player_id":21,"player":{"x":world.player_position().x,"y":world.player_position().y,"herbs":world.player_herbs()},"source":world.source_herbs(),"delivered":world.delivered_herbs(),"requested":world.requested(),"completed":world.completed(),
        "paused":session.paused,"queued":session.queued(),"selected":format!("{:?}",session.selected),"locale":format!("{:?}",session.locale),"status":session.status,"diagnostic":session.diagnostic,"hash":hex(&world.hash()),"npc_hash":hex(&world.npcs().domain_checkpoint_sha256()),"npc_next_event":world.npcs().next_event_id(),"npc_count":20,"event_digest":hex(&world.event_digest()),"buttons":buttons,"targets":positions,"body":body_text(&session,&strings.0),
        "entities":surface.entities.iter().count(),"renderer":surface.adapter.map(|a|format!("{} / {:?}",a.name,a.backend)),"frame_time":summary(performance.intervals_ms.iter().copied()),"tick_compute":summary(session.tick_samples_ns.iter().map(|ns|*ns as f64/1_000_000.0)),"timing_note":"frame_time is Bevy wall frame interval, not isolated GPU time; tick_compute measures canonical drive, excludes rendering and save/capture","save_path":session.save_path.display().to_string()});
    if let Err(error) = std::fs::write(dir.join(format!("{name}.json")), report.to_string()) {
        eprintln!("{error}");
        return;
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(dir.join(format!("{name}.png"))));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_reason_and_target_has_a_translated_label() {
        let catalog = Catalog::agency().unwrap();
        for locale in [Locale::English, Locale::Indonesian] {
            for rejection in [
                Rejection::OutOfBounds,
                Rejection::Terrain,
                Rejection::Occupied,
                Rejection::OutOfRange,
                Rejection::UnknownTarget,
                Rejection::RequestNotAccepted,
                Rejection::Empty,
                Rejection::AlreadyCompleted,
                Rejection::Overflow,
            ] {
                let key = rejection_key(rejection);
                assert_ne!(catalog.text(locale, key), key);
            }
            for target in targets() {
                assert!(!target_name(target, &catalog, locale).is_empty());
            }
        }
    }
}
