use saddle_world_spline_tools_example_support as support;

use bevy::prelude::*;
use saddle_world_spline_tools::{
    RibbonExtrusion, SplineControlPoint, SplineCurve, SplineEditCommand, SplineEditRequest,
    SplineExtrusion, SplineExtrusionShape, SplineMeshTarget, SplinePath, SplineToolsPlugin,
};

#[derive(Resource)]
struct EditDemo {
    entity: Entity,
    timer: Timer,
    stage: usize,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "spline_tools runtime editing".into(),
            resolution: (1440, 860).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(SplineToolsPlugin::default());
    support::install_auto_exit(&mut app);
    app.add_systems(Startup, setup);
    app.add_systems(Update, drive_edit_demo);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    support::spawn_scene_basics(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(0.0, 7.5, 13.0),
        Vec3::new(0.0, 0.5, 0.0),
    );

    let mesh = support::empty_mesh(&mut meshes);
    let entity = commands
        .spawn((
            Name::new("Editable Spline"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.21, 0.61, 0.91),
                cull_mode: None,
                ..default()
            })),
            SplinePath {
                curve: SplineCurve {
                    points: vec![
                        SplineControlPoint::new(Vec3::new(-5.0, 0.0, -3.5)),
                        SplineControlPoint::new(Vec3::new(-1.5, 2.0, 0.8)),
                        SplineControlPoint::new(Vec3::new(2.4, 0.4, 2.8)),
                        SplineControlPoint::new(Vec3::new(5.5, 0.0, -1.2)),
                    ],
                    ..default()
                },
                ..default()
            },
            SplineMeshTarget::new(
                mesh,
                SplineExtrusion {
                    shape: SplineExtrusionShape::Ribbon(RibbonExtrusion {
                        half_width: 0.8,
                        thickness: 0.0,
                        use_control_point_width: true,
                    }),
                    uv_tile_length: 1.0,
                    ..default()
                },
            ),
        ))
        .id();

    commands.insert_resource(EditDemo {
        entity,
        timer: Timer::from_seconds(1.25, TimerMode::Repeating),
        stage: 0,
    });
}

fn drive_edit_demo(
    time: Res<Time>,
    mut demo: ResMut<EditDemo>,
    mut edits: MessageWriter<SplineEditRequest>,
) {
    if !demo.timer.tick(time.delta()).just_finished() {
        return;
    }

    let command = match demo.stage % 4 {
        0 => SplineEditCommand::MovePoint {
            index: 1,
            position: Vec3::new(-1.8, 2.6, 1.4),
        },
        1 => SplineEditCommand::AddPoint {
            index: 3,
            point: SplineControlPoint {
                position: Vec3::new(4.2, 1.1, 1.8),
                width: 1.25,
                ..default()
            },
        },
        2 => SplineEditCommand::SetRoll {
            index: 2,
            roll_radians: -0.28,
        },
        _ => SplineEditCommand::RemovePoint { index: 3 },
    };

    edits.write(SplineEditRequest {
        entity: demo.entity,
        command,
    });
    demo.stage = demo.stage.saturating_add(1);
}
