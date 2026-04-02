use saddle_world_spline_tools_example_support as support;

use bevy::prelude::*;
use saddle_world_spline_tools::{
    SplineCache, SplineControlPoint, SplineCurve, SplineDebugDraw, SplineExtrusion,
    SplineExtrusionShape, SplineMeshTarget, SplinePath, SplineToolsPlugin, TubeExtrusion,
};

#[derive(Component)]
struct LoopFollower {
    spline: Entity,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "spline_tools closed loop tube".into(),
            resolution: (1440, 860).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(SplineToolsPlugin::default());
    support::install_auto_exit(&mut app);
    app.add_systems(Startup, setup);
    app.add_systems(Update, animate_loop_follower);
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
        Vec3::new(0.0, 10.0, 16.0),
        Vec3::new(0.0, 1.6, 0.0),
    );

    let mesh = support::empty_mesh(&mut meshes);
    let spline = commands
        .spawn((
            Name::new("Tube Loop"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.28, 0.72, 0.92),
                metallic: 0.1,
                perceptual_roughness: 0.42,
                ..default()
            })),
            SplinePath {
                curve: SplineCurve {
                    closed: true,
                    points: vec![
                        SplineControlPoint::new(Vec3::new(0.0, 1.4, 6.0)),
                        SplineControlPoint::new(Vec3::new(4.5, 4.2, 2.5)),
                        SplineControlPoint::new(Vec3::new(3.2, 2.2, -4.5)),
                        SplineControlPoint::new(Vec3::new(-3.4, 5.0, -3.5)),
                        SplineControlPoint::new(Vec3::new(-5.0, 2.0, 2.2)),
                    ],
                    ..default()
                },
                ..default()
            },
            SplineMeshTarget::new(
                mesh,
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
        ))
        .id();

    let follower = support::spawn_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Loop Follower",
        0.18,
        Color::srgb(0.98, 0.4, 0.18),
    );
    commands.entity(follower).insert(LoopFollower { spline });
}

fn animate_loop_follower(
    time: Res<Time>,
    caches: Query<&SplineCache>,
    mut followers: Query<(&LoopFollower, &mut Transform)>,
) {
    for (follower, mut transform) in &mut followers {
        let Ok(cache) = caches.get(follower.spline) else {
            continue;
        };
        let progress = (time.elapsed_secs() * 0.1).fract();
        if let Some(sample) = cache.sample_normalized(progress) {
            transform.translation = sample.position;
            transform.rotation = sample.rotation;
        }
    }
}
