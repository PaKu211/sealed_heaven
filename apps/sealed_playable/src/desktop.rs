//! C0.4 native surface. Coordinates, colors and ID offsets have no gameplay authority.
use std::fmt::Write;

use bevy::{
    input::{
        ButtonState,
        keyboard::KeyboardInput,
        mouse::{MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    text::LineBreak,
    window::PrimaryWindow,
};
use sealed_playable::{
    Action, FrameInput, Session, SimulationPlugin, drive_frame, hex,
    locale::{Catalog, Locale},
    view::{ActorView, StockView, View},
};

const PANEL_WIDTH: f32 = 500.0;
const TOOLBAR_HEIGHT: f32 = 80.0;
const HELP_HEIGHT: f32 = 108.0;
const INK: Color = Color::srgb(0.91, 0.94, 0.98);
const MUTED: Color = Color::srgb(0.65, 0.73, 0.82);

#[derive(Resource)]
struct Strings(Catalog);

#[derive(Resource, Default)]
struct PaintState {
    dirty: bool,
    tick: Option<u64>,
    paused: Option<bool>,
    status: String,
    diagnostic: String,
}

#[derive(Component)]
struct MapCamera;

#[derive(Component)]
struct NpcMarker(u32);

#[derive(Component)]
struct MapVisual;

#[derive(Resource, Default)]
struct MapState {
    sites: Vec<u64>,
    edges: Vec<(u64, u64, u64, u64)>,
    grid: Option<sealed_playable::view::GridView>,
}

#[derive(Component)]
struct SelectionRing;

#[derive(Component)]
struct InspectorScroll;

#[derive(Component, Clone, Copy, Debug)]
enum Control {
    Toggle,
    Step,
    Save,
    Load,
    Reset,
    English,
    Indonesian,
}

#[derive(Component)]
enum Label {
    Key(&'static str),
    Toggle,
    Summary,
    Inspector,
    Help,
}

pub fn run() {
    let catalog = match Catalog::runtime() {
        Ok(catalog) => catalog,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    let title = catalog.text(Locale::English, "app.title");
    App::new()
        .insert_resource(Strings(catalog))
        .init_resource::<PaintState>()
        .init_resource::<MapState>()
        .insert_resource(ClearColor(Color::srgb(0.035, 0.055, 0.085)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title,
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
        .add_plugins(SimulationPlugin)
        .add_systems(Startup, setup)
        // The same FrameInput/drive_frame path is used by the adapter tests.
        .add_systems(
            Update,
            (button_input, keyboard_input, mouse_selection)
                .chain()
                .before(drive_frame),
        )
        .add_systems(Update, (camera_controls, scroll_inspector))
        .add_systems(Update, (sync_scene, paint).chain().after(drive_frame))
        .add_systems(Update, capture_evidence.after(paint))
        .run();
}

/// Opt-in native QA capture from Bevy's actual framebuffer, not the OS desktop.
/// It never captures other applications or modifies the simulation.
#[derive(bevy::ecs::system::SystemParam)]
struct CaptureView<'w, 's> {
    adapter: Option<Res<'w, bevy::render::renderer::RenderAdapterInfo>>,
    buttons: Query<
        'w,
        's,
        (
            &'static Control,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    camera: Single<'w, 's, (&'static Camera, &'static GlobalTransform), With<MapCamera>>,
}

fn capture_evidence(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    session: Res<Session>,
    strings: Res<Strings>,
    mut index: Local<u64>,
    surface: CaptureView,
) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }
    let Some(directory) = std::env::var_os("SEALED_HEAVEN_EVIDENCE_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    if let Err(error) = std::fs::create_dir_all(&directory) {
        eprintln!("{error}");
        return;
    }
    *index += 1;
    let name = format!("native-{:03}", *index);
    let buttons: Vec<_> = surface
        .buttons
        .iter()
        .map(|(control, node, transform)| {
            serde_json::json!({"control": format!("{control:?}"), "x": transform.translation.x,
            "y": transform.translation.y, "width": node.size().x, "height": node.size().y })
        })
        .collect();
    let (camera, global) = *surface.camera;
    let actors: Vec<_> = session
        .view
        .actors
        .iter()
        .filter_map(|actor| {
            camera
                .world_to_viewport(global, actor_position(actor, &session.view).extend(5.0))
                .ok()
                .map(|p| serde_json::json!({"id": actor.id, "x": p.x, "y": p.y}))
        })
        .collect();
    let report = serde_json::json!({ "tick": session.tick(), "paused": session.paused,
        "selected": session.selected, "locale": format!("{:?}", session.locale),
        "status": session.status, "diagnostic": session.diagnostic,
        "hash": session.hash(), "digest": session.digest(), "next_event": session.next_event(),
        "actors": session.view.actors.len(), "inspector": inspector_text(&session, &strings.0),
        "renderer": surface.adapter.map(|a| format!("{} / {:?}", a.name, a.backend)),
        "buttons": buttons, "marker_positions": actors,
        "save_path": session.save_path.display().to_string() });
    if let Err(error) = std::fs::write(directory.join(format!("{name}.json")), report.to_string()) {
        eprintln!("{error}");
        return;
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(directory.join(format!("{name}.png"))));
}

fn setup(mut commands: Commands, session: Res<Session>, mut fonts: ResMut<Assets<Font>>) {
    // OFL-licensed Unicode UI font, embedded to work from any launch directory.
    fonts
        .insert(
            bevy::asset::AssetId::default(),
            Font::try_from_bytes(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec())
                .expect("bundled UI font must validate"),
        )
        .expect("default font handle must exist");
    commands.spawn((
        Camera2d,
        MapCamera,
        Transform::from_xyz(PANEL_WIDTH / 2.0, 0.0, 0.0),
    ));
    for actor in &session.view.actors {
        commands
            .spawn((
                NpcMarker(actor.id),
                Sprite::from_color(marker_color(actor.id), Vec2::splat(26.0)),
                Transform::from_translation(actor_position(actor, &session.view).extend(5.0)),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text2d::new(actor.id.to_string()),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::BLACK),
                    Transform::from_xyz(0.0, 0.0, 1.0),
                ));
            });
    }
    commands.spawn((
        SelectionRing,
        Sprite::from_color(Color::WHITE, Vec2::splat(34.0)),
        Transform::from_xyz(0.0, 0.0, 4.0),
    ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                height: px(TOOLBAR_HEIGHT),
                padding: UiRect::all(px(10)),
                column_gap: px(8),
                row_gap: px(6),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.10, 0.15)),
        ))
        .with_children(|bar| {
            bar.spawn((
                Text::default(),
                Label::Key("app.title"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(INK),
                Node {
                    width: px(240),
                    ..default()
                },
            ));
            for (control, label) in [
                (Control::Toggle, Label::Toggle),
                (Control::Step, Label::Key("controls.step")),
                (Control::Save, Label::Key("controls.save")),
                (Control::Load, Label::Key("controls.load")),
                (Control::Reset, Label::Key("controls.reset")),
            ] {
                bar.spawn((
                    Button,
                    control,
                    Node {
                        min_height: px(34),
                        padding: UiRect::axes(px(12), px(7)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.23, 0.32)),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::default(),
                        label,
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(INK),
                    ));
                });
            }
            bar.spawn((
                Text::default(),
                Label::Key("controls.language"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(MUTED),
            ));
            for (control, key) in [
                (Control::English, "locale.en"),
                (Control::Indonesian, "locale.id"),
            ] {
                bar.spawn((
                    Button,
                    control,
                    Node {
                        min_height: px(34),
                        padding: UiRect::axes(px(10), px(7)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.23, 0.32)),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::default(),
                        Label::Key(key),
                        TextFont {
                            font_size: 15.0,
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
                top: px(TOOLBAR_HEIGHT),
                bottom: px(HELP_HEIGHT),
                width: px(PANEL_WIDTH),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(Color::srgb(0.065, 0.085, 0.12)),
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
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            panel.spawn((
                Text::default(),
                Label::Key("help.inspector"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(MUTED),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            panel
                .spawn((
                    InspectorScroll,
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
                        Label::Inspector,
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
                left: px(0),
                right: px(0),
                bottom: px(0),
                height: px(HELP_HEIGHT),
                padding: UiRect::axes(px(14), px(9)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.10, 0.15)),
        ))
        .with_children(|footer| {
            footer.spawn((
                Text::default(),
                Label::Help,
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(MUTED),
            ));
        });
}

fn spawn_map(commands: &mut Commands, view: &View) {
    // Render a bounded grid corner for foreign checkpoints; the entire authored
    // 3x3 grid fits. This draws stored passability, never searches a path.
    for y in 0..view.grid.height.min(32) {
        for x in 0..view.grid.width.min(32) {
            let p = grid_position(view.grid.site, x, y);
            let color = if view.grid.passable[(y * view.grid.width + x) as usize] == 0 {
                Color::srgb(0.07, 0.09, 0.12)
            } else {
                Color::srgb(0.12, 0.19, 0.23)
            };
            commands.spawn((
                MapVisual,
                Sprite::from_color(color, Vec2::splat(56.0)),
                Transform::from_translation(p.extend(0.0)),
            ));
            world_label(
                commands,
                format!("({x},{y})"),
                p + Vec2::new(0.0, -18.0),
                11.0,
            );
        }
    }
    for &(from, to, id, _) in &view.edges {
        let a = site_position(from);
        let b = site_position(to);
        let delta = b - a;
        let side = Vec2::new(-delta.y, delta.x).normalize_or_zero();
        let middle = (a + b) / 2.0 + side * 7.0;
        commands.spawn((
            MapVisual,
            Sprite::from_color(
                Color::srgb(0.26, 0.34, 0.39),
                Vec2::new(delta.length(), 5.0),
            ),
            Transform::from_translation(middle.extend(-1.0))
                .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
        ));
        world_label(
            commands,
            format!("{from} -> {to} [{id}]"),
            middle + side * 14.0,
            13.0,
        );
    }
    for &id in &view.sites {
        let p = site_position(id);
        commands.spawn((
            MapVisual,
            Sprite::from_color(Color::srgb(0.11, 0.18, 0.23), Vec2::new(190.0, 160.0)),
            Transform::from_translation(p.extend(0.0)),
        ));
        world_label(commands, id.to_string(), p + Vec2::new(0.0, 96.0), 26.0);
    }
}

fn world_label(commands: &mut Commands, value: String, at: Vec2, size: f32) {
    commands.spawn((
        MapVisual,
        Text2d::new(value),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(MUTED),
        Transform::from_translation(at.extend(2.0)),
    ));
}

fn sync_scene(
    mut commands: Commands,
    session: Res<Session>,
    mut map: ResMut<MapState>,
    visuals: Query<Entity, With<MapVisual>>,
    actors: Query<(Entity, &NpcMarker)>,
) {
    for (entity, marker) in &actors {
        if !session.view.actors.iter().any(|a| a.id == marker.0) {
            commands.entity(entity).despawn();
        }
    }
    for actor in &session.view.actors {
        if !actors.iter().any(|(_, marker)| marker.0 == actor.id) {
            commands
                .spawn((
                    NpcMarker(actor.id),
                    Sprite::from_color(marker_color(actor.id), Vec2::splat(26.0)),
                    Transform::from_translation(actor_position(actor, &session.view).extend(5.0)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text2d::new(actor.id.to_string()),
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(Color::BLACK),
                        Transform::from_xyz(0.0, 0.0, 1.0),
                    ));
                });
        }
    }
    if map.sites != session.view.sites
        || map.edges != session.view.edges
        || map.grid.as_ref() != Some(&session.view.grid)
    {
        for entity in &visuals {
            commands.entity(entity).despawn();
        }
        spawn_map(&mut commands, &session.view);
        map.sites.clone_from(&session.view.sites);
        map.edges.clone_from(&session.view.edges);
        map.grid = Some(session.view.grid.clone());
    }
}

fn site_position(id: u64) -> Vec2 {
    match id {
        1 => Vec2::new(-220.0, 120.0),
        2 => Vec2::new(220.0, 120.0),
        9 => Vec2::new(220.0, -210.0),
        // Diagnostic placement for a valid but unauthored site ID, never simulation.
        _ => Vec2::new((id % 7) as f32 * 65.0 - 195.0, -330.0),
    }
}

fn grid_position(site: u64, x: u32, y: u32) -> Vec2 {
    site_position(site) + Vec2::new(x as f32 * 60.0 - 60.0, -160.0 - y as f32 * 60.0)
}

fn actor_position(actor: &ActorView, view: &View) -> Vec2 {
    let canonical = match actor.locus {
        2 => grid_position(actor.site, actor.x, actor.y),
        3 => view
            .edges
            .iter()
            .find(|edge| edge.2 == actor.edge)
            .map_or(Vec2::ZERO, |edge| {
                let ratio = if edge.3 == 0 {
                    0.0
                } else {
                    actor.traversed as f32 / edge.3 as f32
                };
                site_position(edge.0).lerp(site_position(edge.1), ratio)
            }),
        _ => site_position(actor.site),
    };
    // Stable visual slots: no interpolation, path state, or frame-time motion.
    let slot = actor.id.saturating_sub(1) % 20;
    let offset = Vec2::new(
        (slot % 5) as f32 * 32.0 - 64.0,
        (slot / 5) as f32 * 32.0 - 48.0,
    );
    canonical
        + if actor.locus == 2 {
            offset * 0.3
        } else {
            offset
        }
}

fn marker_color(id: u32) -> Color {
    Color::hsl((id as f32 * 137.508) % 360.0, 0.65, 0.64)
}

fn queued_paused(session: &Session, input: &FrameInput) -> bool {
    input
        .actions
        .iter()
        .fold(session.paused, |paused, action| match action {
            Action::Pause | Action::Load | Action::Reset => true,
            Action::Resume => false,
            _ => paused,
        })
}

fn button_input(
    buttons: Query<(&Interaction, &Control), Changed<Interaction>>,
    session: Res<Session>,
    mut input: ResMut<FrameInput>,
    mut paint: ResMut<PaintState>,
) {
    for (interaction, control) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let action = match control {
            Control::Toggle => {
                if queued_paused(&session, &input) {
                    Action::Resume
                } else {
                    Action::Pause
                }
            }
            Control::Step if !queued_paused(&session, &input) => continue,
            Control::Step => Action::Step,
            Control::Save => Action::Save,
            Control::Load => Action::Load,
            Control::Reset => Action::Reset,
            Control::English => Action::Language(Locale::English),
            Control::Indonesian => Action::Language(Locale::Indonesian),
        };
        input.actions.push(action);
        paint.dirty = true;
    }
}

fn keyboard_input(
    mut keys: MessageReader<KeyboardInput>,
    time: Res<Time<Real>>,
    session: Res<Session>,
    mut input: ResMut<FrameInput>,
    mut paint: ResMut<PaintState>,
) {
    // Real time avoids the virtual clock's maximum-delta clamp discarding debt.
    input.elapsed_ns = time.delta().as_nanos();
    for key in keys.read() {
        if key.state != ButtonState::Pressed || key.repeat {
            continue;
        }
        let action = match key.key_code {
            KeyCode::Space => {
                if queued_paused(&session, &input) {
                    Action::Resume
                } else {
                    Action::Pause
                }
            }
            KeyCode::KeyN if queued_paused(&session, &input) => Action::Step,
            KeyCode::F5 => Action::Save,
            KeyCode::F9 => Action::Load,
            KeyCode::KeyR => Action::Reset,
            KeyCode::KeyL => {
                let locale = input
                    .actions
                    .iter()
                    .rev()
                    .find_map(|a| match a {
                        Action::Language(locale) => Some(*locale),
                        _ => None,
                    })
                    .unwrap_or(session.locale);
                Action::Language(if locale == Locale::English {
                    Locale::Indonesian
                } else {
                    Locale::English
                })
            }
            KeyCode::Tab => {
                let selected = input
                    .actions
                    .iter()
                    .rev()
                    .find_map(|a| match a {
                        Action::Select(id) => Some(*id),
                        _ => None,
                    })
                    .unwrap_or(session.selected);
                let actors = &session.view.actors;
                if actors.is_empty() {
                    continue;
                }
                let index = actors
                    .iter()
                    .position(|a| a.id == selected)
                    .map_or(0, |i| (i + 1) % actors.len());
                Action::Select(actors[index].id)
            }
            _ => continue,
        };
        input.actions.push(action);
        paint.dirty = true;
    }
}

fn mouse_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform, &Projection), With<MapCamera>>,
    session: Res<Session>,
    mut input: ResMut<FrameInput>,
    mut paint: ResMut<PaintState>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if cursor.x >= window.width() - PANEL_WIDTH
        || cursor.y <= TOOLBAR_HEIGHT
        || cursor.y >= window.height() - HELP_HEIGHT
    {
        return;
    }
    let (camera, global, projection) = *camera;
    let Ok(world) = camera.viewport_to_world_2d(global, cursor) else {
        return;
    };
    let scale = match projection {
        Projection::Orthographic(p) => p.scale,
        _ => 1.0,
    };
    let closest = session
        .view
        .actors
        .iter()
        .map(|actor| {
            (
                actor.id,
                actor_position(actor, &session.view).distance_squared(world),
            )
        })
        .filter(|(_, distance)| *distance <= (18.0 * scale.max(1.0)).powi(2))
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if let Some((id, _)) = closest {
        input.actions.push(Action::Select(id));
        paint.dirty = true;
    }
}

fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    camera: Single<(&mut Transform, &mut Projection), With<MapCamera>>,
) {
    let (mut transform, mut projection) = camera.into_inner();
    let Projection::Orthographic(p) = &mut *projection else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyC) {
        transform.translation = Vec3::new(PANEL_WIDTH / 2.0, 0.0, 0.0);
        p.scale = 1.0;
        return;
    }
    let dt = time.delta_secs().min(0.1);
    let mut direction = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        direction.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        direction.x += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        direction.y += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        direction.y -= 1.0;
    }
    transform.translation += (direction.normalize_or_zero() * 420.0 * dt * p.scale).extend(0.0);
    let zoom = f32::from(keys.pressed(KeyCode::KeyQ)) - f32::from(keys.pressed(KeyCode::KeyE));
    p.scale = (p.scale * (zoom * dt * 1.5).exp()).clamp(0.35, 3.0);
}

fn scroll_inspector(
    mut wheel: MessageReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<&Window, With<PrimaryWindow>>,
    panel: Single<(&mut ScrollPosition, &ComputedNode), With<InspectorScroll>>,
) {
    let (mut scroll, computed) = panel.into_inner();
    let over_panel = window.cursor_position().is_some_and(|p| {
        p.x >= window.width() - PANEL_WIDTH
            && p.y > TOOLBAR_HEIGHT
            && p.y < window.height() - HELP_HEIGHT
    });
    let mut delta = 0.0;
    for event in wheel.read() {
        if over_panel {
            delta -= event.y
                * if event.unit == MouseScrollUnit::Line {
                    48.0
                } else {
                    1.0
                };
        }
    }
    if keys.just_pressed(KeyCode::PageDown) {
        delta += 320.0;
    }
    if keys.just_pressed(KeyCode::PageUp) {
        delta -= 320.0;
    }
    let max = ((computed.content_size().y - computed.size().y) * computed.inverse_scale_factor())
        .max(0.0);
    scroll.y = (scroll.y + delta).clamp(0.0, max);
    if keys.just_pressed(KeyCode::Home) {
        scroll.y = 0.0;
    }
    if keys.just_pressed(KeyCode::End) {
        scroll.y = max;
    }
}

// Splitting the queries keeps all mutable component access demonstrably disjoint.
#[derive(bevy::ecs::system::SystemParam)]
struct PaintQueries<'w, 's> {
    labels: Query<'w, 's, (&'static Label, &'static mut Text)>,
    markers: Query<
        'w,
        's,
        (
            &'static NpcMarker,
            &'static mut Transform,
            &'static mut Sprite,
        ),
        Without<SelectionRing>,
    >,
    ring: Single<'w, 's, &'static mut Transform, (With<SelectionRing>, Without<NpcMarker>)>,
    buttons: Query<
        'w,
        's,
        (
            &'static Control,
            &'static Interaction,
            &'static mut BackgroundColor,
        ),
    >,
    window: Single<'w, 's, &'static mut Window, With<PrimaryWindow>>,
}

fn paint(
    session: Res<Session>,
    strings: Res<Strings>,
    mut cache: ResMut<PaintState>,
    mut queries: PaintQueries,
) {
    // Hover feedback remains responsive without rehashing/reformatting the world.
    for (control, interaction, mut color) in &mut queries.buttons {
        let disabled = matches!(control, Control::Step) && !session.paused;
        let selected = matches!(
            (control, session.locale),
            (Control::English, Locale::English) | (Control::Indonesian, Locale::Indonesian)
        );
        let value = if disabled {
            Color::srgb(0.09, 0.11, 0.14)
        } else if *interaction == Interaction::Pressed {
            Color::srgb(0.22, 0.48, 0.53)
        } else if *interaction == Interaction::Hovered {
            Color::srgb(0.23, 0.34, 0.45)
        } else if selected {
            Color::srgb(0.16, 0.35, 0.35)
        } else {
            Color::srgb(0.15, 0.23, 0.32)
        };
        if color.0 != value {
            color.0 = value;
        }
    }
    if !cache.dirty
        && cache.tick == Some(session.tick())
        && cache.paused == Some(session.paused)
        && cache.status == session.status
        && cache.diagnostic == session.diagnostic
    {
        return;
    }
    cache.dirty = false;
    cache.tick = Some(session.tick());
    cache.paused = Some(session.paused);
    cache.status.clone_from(&session.status);
    cache.diagnostic.clone_from(&session.diagnostic);
    let text = |key: &str| strings.0.text(session.locale, key);
    queries.window.title = text("app.title");
    for (label, mut content) in &mut queries.labels {
        **content = match label {
            Label::Key(key) => text(key),
            Label::Toggle => text(if session.paused {
                "controls.resume"
            } else {
                "controls.pause"
            }),
            Label::Summary => format!(
                "{}\n{}: {} · {}\n{}{}",
                text("inspector.title"),
                text("inspector.tick"),
                session.tick(),
                text(if session.paused {
                    "status.paused"
                } else {
                    "status.running"
                }),
                text(&session.status),
                if session.diagnostic.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", session.diagnostic)
                }
            ),
            Label::Inspector => inspector_text(&session, &strings.0),
            Label::Help => format!(
                "{}\n{}\n{}",
                text("help.controls"),
                text("help.needs"),
                text("help.travel")
            ),
        };
    }
    for (marker, mut transform, mut sprite) in &mut queries.markers {
        if let Some(actor) = session.view.actors.iter().find(|a| a.id == marker.0) {
            let p = actor_position(actor, &session.view);
            transform.translation = p.extend(5.0);
            sprite.color = if actor.enabled == 0 {
                Color::srgb(0.32, 0.34, 0.38)
            } else {
                marker_color(actor.id)
            };
            if actor.id == session.selected {
                queries.ring.translation = p.extend(4.0);
            }
        }
    }
}

fn inspector_text(session: &Session, catalog: &Catalog) -> String {
    let t = |key: &str| catalog.text(session.locale, key);
    let mut out = String::new();
    if let Some(actor) = session
        .view
        .actors
        .iter()
        .find(|a| a.id == session.selected)
    {
        let _ = writeln!(
            out,
            "{} {}\n{}: {} [{}] · {}: {} [{}]",
            t("inspector.npc"),
            actor.id,
            t("inspector.enabled"),
            t(if actor.enabled != 0 {
                "value.yes"
            } else {
                "value.no"
            }),
            actor.enabled,
            t("inspector.mode"),
            t(&format!("mode.{}", actor.mode)),
            actor.mode
        );
        let _ = writeln!(
            out,
            "{} [{}]\n  {}: {} · {}: ({},{})\n  {}: {} · {}: {}\n{}: {}",
            t("inspector.position"),
            actor.locus,
            t("inspector.site"),
            actor.site,
            t("inspector.grid"),
            actor.x,
            actor.y,
            t("inspector.edge"),
            actor.edge,
            t("inspector.progress"),
            actor.traversed,
            t("inspector.home"),
            actor.home
        );
        let _ = writeln!(out, "\n{}", t("inspector.needs"));
        for (channel, value) in actor.needs.iter().enumerate() {
            let _ = writeln!(
                out,
                "  [{channel}] {}: {value}",
                t(&format!("need.{channel}"))
            );
        }
        let _ = writeln!(out, "\n{}: {}", t("inspector.task"), actor.task);
        if let Some(task) = session
            .view
            .tasks
            .iter()
            .find(|task| task.id == actor.task && task.actor == actor.id)
        {
            let _ = writeln!(
                out,
                "{}: {} [{}]\n{}: {} [{}]",
                t("inspector.goal"),
                t(&format!("goal.{}", task.goal)),
                task.goal,
                t("inspector.stage"),
                t(&format!("stage.{}", task.stage)),
                task.stage
            );
        } else {
            let _ = writeln!(
                out,
                "{}: {}\n{}: {}",
                t("inspector.goal"),
                t("inspector.none"),
                t("inspector.stage"),
                t("inspector.none")
            );
        }
        let inventory: Vec<_> = session
            .view
            .stocks
            .iter()
            .filter(|stock| stock.owner_kind == 1 && stock.owner == u64::from(actor.id))
            .collect();
        let _ = writeln!(out, "\n{} ({})", t("inspector.inventory"), inventory.len());
        if inventory.is_empty() {
            let _ = writeln!(out, "  {}", t("inspector.none"));
        }
        for stock in inventory {
            append_stock(&mut out, stock, catalog, session.locale);
        }
    }
    // All 64 hex digits are visible, wrapped at a known boundary rather than truncated.
    for (key, value) in [
        ("inspector.checkpoint", session.hash()),
        ("inspector.digest", session.digest()),
    ] {
        let _ = writeln!(out, "\n{}\n{}\n{}", t(key), &value[..32], &value[32..]);
    }
    let _ = writeln!(
        out,
        "\n{}\n{}\n{}",
        t("controls.save"),
        session.save_path.display(),
        t("help.save")
    );
    let _ = writeln!(
        out,
        "\n{} ({})",
        t("inspector.stocks"),
        session.view.stocks.len()
    );
    for stock in &session.view.stocks {
        append_stock(&mut out, stock, catalog, session.locale);
    }
    let _ = writeln!(
        out,
        "\n{} ({})",
        t("inspector.events"),
        session.recent.len()
    );
    if session.recent.is_empty() {
        let _ = writeln!(out, "{}", t("inspector.none"));
    }
    for event in &session.recent {
        let _ = writeln!(
            out,
            "\n{} {} · {} {} · {} {}\n{}: {} [{}]\n{}: {}",
            t("inspector.id"),
            event.id,
            t("inspector.tick"),
            event.tick,
            t("inspector.npc"),
            event.actor_id,
            t("inspector.kind"),
            t(&format!("event.{}", event.kind)),
            event.kind,
            t("inspector.payload"),
            hex(&event.payload)
        );
    }
    out
}

fn append_stock(out: &mut String, stock: &StockView, catalog: &Catalog, locale: Locale) {
    let t = |key: &str| catalog.text(locale, key);
    let _ = writeln!(
        out,
        "{} {} · {} {} · {} {}\n  {}: {} [{}]\n  {}: {} [{}]: {}",
        t("inspector.id"),
        stock.id,
        t("inspector.definition"),
        stock.definition,
        t("inspector.quantity"),
        stock.quantity,
        t("inspector.class"),
        t(&format!("class.{}", stock.class)),
        stock.class,
        t("inspector.owner"),
        t(&format!("owner.{}", stock.owner_kind)),
        stock.owner_kind,
        stock.owner
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_a_different_valid_actor_set_removes_ghost_markers() {
        let mut app = App::new();
        app.add_plugins(SimulationPlugin)
            .init_resource::<MapState>()
            .add_systems(Update, sync_scene.after(drive_frame));
        let original = app.world().resource::<Session>().checkpoint();
        app.update();
        let mut markers = app.world_mut().query::<&NpcMarker>();
        assert_eq!(markers.iter(app.world()).count(), 20);
        assert_eq!(app.world().resource::<Session>().checkpoint(), original);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("other.c03");
        let two = sealed_sim::Simulation::two_claimants().domain_checkpoint_bytes();
        sealed_playable::persistence::save(&path, &two).unwrap();
        app.world_mut().resource_mut::<Session>().save_path = path;
        app.world_mut()
            .resource_mut::<FrameInput>()
            .actions
            .push(Action::Load);
        app.update();
        let mut ids: Vec<_> = markers.iter(app.world()).map(|m| m.0).collect();
        ids.sort_unstable();
        assert_eq!(ids, [1, 2]);
        assert_eq!(app.world().resource::<Session>().checkpoint(), two);
    }

    fn actor(id: u32) -> ActorView {
        ActorView {
            id,
            enabled: 1,
            mode: 1,
            next_wake: 1,
            needs: [0; 5],
            home: 2,
            task: 0,
            locus: 1,
            site: 1,
            x: 0,
            y: 0,
            edge: 0,
            traversed: 0,
        }
    }

    fn view() -> View {
        View {
            actors: Vec::new(),
            sites: vec![1, 2, 9],
            edges: vec![(1, 2, 1, 4)],
            grid: sealed_playable::view::GridView {
                site: 1,
                width: 0,
                height: 0,
                passable: vec![],
            },
            stocks: Vec::new(),
            tasks: Vec::new(),
        }
    }

    #[test]
    fn twenty_site_slots_are_distinct_and_stable() {
        let view = view();
        let positions: Vec<_> = (1..=20)
            .map(|id| actor_position(&actor(id), &view))
            .collect();
        for (i, position) in positions.iter().enumerate() {
            assert!(!positions[..i].contains(position));
            assert_eq!(*position, actor_position(&actor(i as u32 + 1), &view));
        }
    }

    #[test]
    fn edge_and_grid_positions_use_checkpoint_coordinates_only() {
        let view = view();
        let mut a = actor(1);
        let slot = actor_position(&a, &view) - site_position(1);
        a.locus = 3;
        a.site = 0;
        a.edge = 1;
        a.traversed = 2;
        assert_eq!(
            actor_position(&a, &view),
            site_position(1).lerp(site_position(2), 0.5) + slot
        );
        a.locus = 2;
        a.site = 1;
        a.x = 2;
        a.y = 1;
        assert_eq!(
            actor_position(&a, &view),
            grid_position(1, 2, 1) + slot * 0.3
        );
    }
}
