#[cfg(feature = "e2e")]
mod e2e;

use saddle_world_spline_tools_example_support as support;

use bevy::prelude::*;
#[cfg(feature = "dev")]
use bevy::remote::RemotePlugin;
#[cfg(feature = "dev")]
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_world_spline_tools::{
    RibbonExtrusion, SplineCache, SplineControlPoint, SplineCurve, SplineDebugDraw,
    SplineDiagnostics, SplineEditCommand, SplineEditRequest, SplineExtrusion, SplineExtrusionShape,
    SplineMeshTarget, SplinePath, SplineToolsPlugin, TubeExtrusion,
};

const DEFAULT_BRP_PORT: u16 = 15_736;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum LaneFocus {
    Road,
    Placement,
    Tube,
    Edit,
}

#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct LabControl {
    pub lane_focus: LaneFocus,
    pub pending_add_point: bool,
    pub pending_move_point: bool,
    pub pending_remove_point: bool,
}

impl Default for LabControl {
    fn default() -> Self {
        Self {
            lane_focus: LaneFocus::Road,
            pending_add_point: false,
            pending_move_point: false,
            pending_remove_point: false,
        }
    }
}

#[derive(Resource, Reflect, Clone, Debug, Default)]
#[reflect(Resource)]
pub struct LabDiagnostics {
    pub road_length: f32,
    pub road_vertices: usize,
    pub tube_length: f32,
    pub tube_vertices: usize,
    pub post_count: usize,
    pub edit_control_points: usize,
    pub edit_curve_revision: u64,
    pub edit_mesh_revision: u64,
    pub last_edit_action: Option<String>,
}

#[derive(Component)]
struct LabOverlay;

#[derive(Component)]
struct RoadLane;

#[derive(Component)]
struct PlacementLane;

#[derive(Component)]
struct TubeLane;

#[derive(Component)]
struct EditLane;

#[derive(Component)]
struct PlacementPost;

#[derive(Resource)]
struct LabEntities {
    camera: Entity,
    edit: Entity,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.07)));
    app.insert_resource(LabControl::default());
    app.insert_resource(LabDiagnostics::default());
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "spline_tools crate-local lab".into(),
            resolution: (1560, 940).into(),
            ..default()
        }),
        ..default()
    }));
    support::install_auto_exit(&mut app);
    app.add_plugins(SplineToolsPlugin::default());
    #[cfg(feature = "dev")]
    app.add_plugins(RemotePlugin::default());
    #[cfg(feature = "dev")]
    app.add_plugins(BrpExtrasPlugin::with_port(lab_brp_port()));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::SplineToolsLabE2EPlugin);
    app.register_type::<LabControl>()
        .register_type::<LabDiagnostics>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_keyboard_input,
                emit_edit_requests,
                update_camera_focus,
                refresh_placement_posts,
                sync_diagnostics,
                update_overlay,
            ),
        );
    app.run();
}

#[cfg(feature = "dev")]
fn lab_brp_port() -> u16 {
    std::env::var("BRP_EXTRAS_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_BRP_PORT)
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let camera = commands
        .spawn((
            Name::new("Lab Camera"),
            Camera3d::default(),
            Transform::from_xyz(-10.0, 7.5, 14.0).looking_at(Vec3::new(-10.0, 0.5, 0.0), Vec3::Y),
        ))
        .id();
    commands.spawn((
        Name::new("Lab Sun"),
        DirectionalLight {
            illuminance: 20_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.7, 0.0)),
    ));
    commands.spawn((
        Name::new("Lab Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(120.0, 120.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.11, 0.12, 0.15),
            perceptual_roughness: 0.96,
            ..default()
        })),
    ));
    commands.spawn((
        Name::new("Lab Overlay"),
        LabOverlay,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(12.0),
            left: px(12.0),
            ..default()
        },
    ));

    let road_texture = support::stripe_texture(
        &mut images,
        Color::srgb(0.14, 0.16, 0.18),
        Color::srgb(0.94, 0.82, 0.18),
    );

    let road_mesh = support::empty_mesh(&mut meshes);
    commands.spawn((
        Name::new("Road Lane"),
        RoadLane,
        Mesh3d(road_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(road_texture),
            perceptual_roughness: 0.88,
            cull_mode: None,
            ..default()
        })),
        Transform::from_translation(Vec3::new(-10.0, 0.0, 0.0)),
        SplinePath {
            curve: SplineCurve {
                points: vec![
                    SplineControlPoint {
                        position: Vec3::new(-6.5, 0.0, -3.2),
                        width: 1.2,
                        ..default()
                    },
                    SplineControlPoint {
                        position: Vec3::new(-2.0, 0.5, -0.2),
                        width: 1.4,
                        roll_radians: 0.08,
                        ..default()
                    },
                    SplineControlPoint {
                        position: Vec3::new(2.2, 0.0, 2.6),
                        width: 1.55,
                        roll_radians: -0.14,
                        ..default()
                    },
                    SplineControlPoint {
                        position: Vec3::new(6.4, 0.0, -2.4),
                        width: 1.15,
                        ..default()
                    },
                ],
                ..default()
            },
            ..default()
        },
        SplineMeshTarget::new(
            road_mesh,
            SplineExtrusion {
                shape: SplineExtrusionShape::Ribbon(RibbonExtrusion {
                    half_width: 1.0,
                    thickness: 0.0,
                    use_control_point_width: true,
                }),
                uv_tile_length: 1.2,
                ..default()
            },
        ),
        SplineDebugDraw {
            draw_frames: false,
            draw_samples: false,
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Placement Lane"),
        PlacementLane,
        Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
        Visibility::default(),
        SplinePath {
            curve: SplineCurve {
                points: vec![
                    SplineControlPoint::new(Vec3::new(-5.8, 0.0, -3.4)),
                    SplineControlPoint::new(Vec3::new(-2.0, 1.9, -0.2)),
                    SplineControlPoint::new(Vec3::new(1.0, 0.9, 2.4)),
                    SplineControlPoint::new(Vec3::new(5.8, 2.0, -2.0)),
                ],
                ..default()
            },
            ..default()
        },
        SplineDebugDraw {
            draw_samples: false,
            frame_stride: 5,
            ..default()
        },
    ));

    let tube_mesh = support::empty_mesh(&mut meshes);
    commands.spawn((
        Name::new("Tube Lane"),
        TubeLane,
        Mesh3d(tube_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.29, 0.74, 0.93),
            metallic: 0.1,
            perceptual_roughness: 0.45,
            ..default()
        })),
        Transform::from_translation(Vec3::new(-10.0, 0.0, -14.0)),
        SplinePath {
            curve: SplineCurve {
                closed: true,
                points: vec![
                    SplineControlPoint::new(Vec3::new(0.0, 1.4, 5.2)),
                    SplineControlPoint::new(Vec3::new(4.3, 4.2, 2.1)),
                    SplineControlPoint::new(Vec3::new(3.1, 2.1, -4.4)),
                    SplineControlPoint::new(Vec3::new(-3.2, 4.6, -3.2)),
                    SplineControlPoint::new(Vec3::new(-4.9, 1.8, 1.6)),
                ],
                ..default()
            },
            ..default()
        },
        SplineMeshTarget::new(
            tube_mesh,
            SplineExtrusion {
                shape: SplineExtrusionShape::Tube(TubeExtrusion {
                    radius: 0.24,
                    radial_segments: 18,
                    use_control_point_radius: false,
                }),
                ..default()
            },
        ),
        SplineDebugDraw {
            draw_control_points: false,
            draw_handles: false,
            draw_samples: false,
            frame_stride: 8,
            ..default()
        },
    ));

    let edit_mesh = support::empty_mesh(&mut meshes);
    let edit = commands
        .spawn((
            Name::new("Editable Lane"),
            EditLane,
            Mesh3d(edit_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.62, 0.94),
                cull_mode: None,
                ..default()
            })),
            Transform::from_translation(Vec3::new(10.0, 0.0, -14.0)),
            SplinePath {
                curve: SplineCurve {
                    points: vec![
                        SplineControlPoint::new(Vec3::new(-5.4, 0.0, -3.0)),
                        SplineControlPoint::new(Vec3::new(-1.5, 2.2, 0.8)),
                        SplineControlPoint::new(Vec3::new(2.2, 0.4, 2.6)),
                        SplineControlPoint::new(Vec3::new(5.5, 0.0, -0.8)),
                    ],
                    ..default()
                },
                ..default()
            },
            SplineMeshTarget::new(
                edit_mesh,
                SplineExtrusion {
                    shape: SplineExtrusionShape::Ribbon(RibbonExtrusion {
                        half_width: 0.8,
                        thickness: 0.0,
                        use_control_point_width: true,
                    }),
                    ..default()
                },
            ),
            SplineDebugDraw::default(),
        ))
        .id();

    commands.insert_resource(LabEntities { camera, edit });
}

fn handle_keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut control: ResMut<LabControl>) {
    if keys.just_pressed(KeyCode::Digit1) {
        control.lane_focus = LaneFocus::Road;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        control.lane_focus = LaneFocus::Placement;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        control.lane_focus = LaneFocus::Tube;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        control.lane_focus = LaneFocus::Edit;
    }
    if keys.just_pressed(KeyCode::KeyA) {
        control.pending_add_point = true;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        control.pending_move_point = true;
    }
    if keys.just_pressed(KeyCode::KeyD) {
        control.pending_remove_point = true;
    }
}

fn emit_edit_requests(
    mut control: ResMut<LabControl>,
    entities: Res<LabEntities>,
    mut edits: MessageWriter<SplineEditRequest>,
    mut diagnostics: ResMut<LabDiagnostics>,
) {
    if control.pending_add_point {
        control.pending_add_point = false;
        diagnostics.last_edit_action = Some("add point".into());
        edits.write(SplineEditRequest {
            entity: entities.edit,
            command: SplineEditCommand::AddPoint {
                index: 3,
                point: SplineControlPoint {
                    position: Vec3::new(3.8, 1.2, 2.0),
                    width: 1.2,
                    ..default()
                },
            },
        });
    }
    if control.pending_move_point {
        control.pending_move_point = false;
        diagnostics.last_edit_action = Some("move point".into());
        edits.write(SplineEditRequest {
            entity: entities.edit,
            command: SplineEditCommand::MovePoint {
                index: 1,
                position: Vec3::new(-1.6, 2.8, 1.4),
            },
        });
    }
    if control.pending_remove_point {
        control.pending_remove_point = false;
        diagnostics.last_edit_action = Some("remove point".into());
        edits.write(SplineEditRequest {
            entity: entities.edit,
            command: SplineEditCommand::RemovePoint { index: 3 },
        });
    }
}

fn update_camera_focus(
    control: Res<LabControl>,
    entities: Res<LabEntities>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    if !control.is_changed() {
        return;
    }
    let Ok(mut camera) = cameras.get_mut(entities.camera) else {
        return;
    };
    let (translation, look_at) = match control.lane_focus {
        LaneFocus::Road => (Vec3::new(-10.0, 7.5, 14.0), Vec3::new(-10.0, 0.5, 0.0)),
        LaneFocus::Placement => (Vec3::new(10.0, 8.0, 14.0), Vec3::new(10.0, 1.0, 0.0)),
        LaneFocus::Tube => (Vec3::new(-10.0, 10.0, 0.0), Vec3::new(-10.0, 2.0, -14.0)),
        LaneFocus::Edit => (Vec3::new(10.0, 8.0, 0.0), Vec3::new(10.0, 0.8, -14.0)),
    };
    *camera = Transform::from_translation(translation).looking_at(look_at, Vec3::Y);
}

fn refresh_placement_posts(
    mut commands: Commands,
    spline_query: Query<(Entity, &SplineCache), (With<PlacementLane>, Changed<SplineCache>)>,
    posts: Query<Entity, With<PlacementPost>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((entity, cache)) = spline_query.single() else {
        return;
    };

    for post in &posts {
        commands.entity(post).despawn();
    }

    let post_mesh = meshes.add(Cuboid::new(0.12, 1.0, 0.12));
    let post_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.54, 0.42, 0.28),
        perceptual_roughness: 0.9,
        ..default()
    });
    let rail_mesh = meshes.add(Cuboid::new(0.08, 0.08, 0.8));
    let rail_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.64, 0.54, 0.34),
        perceptual_roughness: 0.86,
        ..default()
    });

    let transforms = cache.sample_evenly_spaced_transforms(1.1, true);
    commands.entity(entity).with_children(|parent| {
        for (index, transform) in transforms.into_iter().enumerate() {
            parent.spawn((
                Name::new(format!("Placement Post {index:02}")),
                PlacementPost,
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(post_material.clone()),
                Transform::from_translation(
                    transform.translation + transform.rotation * Vec3::Y * 0.5,
                ),
            ));
            parent.spawn((
                Name::new(format!("Placement Rail {index:02}")),
                PlacementPost,
                Mesh3d(rail_mesh.clone()),
                MeshMaterial3d(rail_material.clone()),
                Transform {
                    translation: transform.translation
                        + transform.rotation * Vec3::new(0.0, 0.5, 0.0),
                    rotation: transform.rotation,
                    ..default()
                },
            ));
        }
    });
}

fn sync_diagnostics(
    road_query: Query<(&SplineCache, &SplineDiagnostics), With<RoadLane>>,
    placement_query: Query<&Children, With<PlacementLane>>,
    tube_query: Query<(&SplineCache, &SplineDiagnostics), With<TubeLane>>,
    edit_query: Query<(&SplinePath, &SplineDiagnostics), With<EditLane>>,
    posts: Query<(), With<PlacementPost>>,
    mut diagnostics: ResMut<LabDiagnostics>,
) {
    if let Ok((cache, spline_diagnostics)) = road_query.single() {
        diagnostics.road_length = cache.total_length;
        diagnostics.road_vertices = spline_diagnostics.last_vertex_count;
    }
    if let Ok((cache, spline_diagnostics)) = tube_query.single() {
        diagnostics.tube_length = cache.total_length;
        diagnostics.tube_vertices = spline_diagnostics.last_vertex_count;
    }
    if let Ok((path, spline_diagnostics)) = edit_query.single() {
        diagnostics.edit_control_points = path.curve.points.len();
        diagnostics.edit_curve_revision = spline_diagnostics.curve_revision;
        diagnostics.edit_mesh_revision = spline_diagnostics.mesh_revision;
    }
    if placement_query.single().is_ok() {
        diagnostics.post_count = posts.iter().count();
    }
}

fn update_overlay(
    control: Res<LabControl>,
    diagnostics: Res<LabDiagnostics>,
    mut overlays: Query<&mut Text, With<LabOverlay>>,
) {
    if !control.is_changed() && !diagnostics.is_changed() {
        return;
    }
    let Ok(mut overlay) = overlays.single_mut() else {
        return;
    };
    overlay.0 = format!(
        "Spline Tools Lab\n\
         Focus: {:?}\n\
         Road: {:.2}m, {} vertices\n\
         Tube: {:.2}m, {} vertices\n\
         Placement posts: {}\n\
         Editable spline: {} points, curve rev {}, mesh rev {}\n\
         Last edit: {}\n\
         Keys: 1/2/3/4 focus lanes, A add point, M move point, D delete point",
        control.lane_focus,
        diagnostics.road_length,
        diagnostics.road_vertices,
        diagnostics.tube_length,
        diagnostics.tube_vertices,
        diagnostics.post_count,
        diagnostics.edit_control_points,
        diagnostics.edit_curve_revision,
        diagnostics.edit_mesh_revision,
        diagnostics
            .last_edit_action
            .clone()
            .unwrap_or_else(|| "none".into()),
    );
}

pub fn focus_lane(world: &mut World, lane: LaneFocus) {
    world.resource_mut::<LabControl>().lane_focus = lane;
}

pub fn trigger_add_point(world: &mut World) {
    world.resource_mut::<LabControl>().pending_add_point = true;
}

pub fn trigger_move_point(world: &mut World) {
    world.resource_mut::<LabControl>().pending_move_point = true;
}

pub fn trigger_remove_point(world: &mut World) {
    world.resource_mut::<LabControl>().pending_remove_point = true;
}
