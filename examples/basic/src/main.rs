use saddle_world_spline_tools_example_support as support;

use bevy::prelude::*;
use saddle_world_spline_tools::{
    SplineCache, SplineControlPoint, SplineCurve, SplineDebugDraw, SplinePath, SplineToolsPlugin,
};

#[derive(Component)]
struct PathFollower {
    spline: Entity,
    speed: f32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "spline_tools basic".into(),
            resolution: (1280, 720).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(SplineToolsPlugin::default());
    support::install_auto_exit(&mut app);
    app.add_systems(Startup, setup);
    app.add_systems(Update, animate_follower);
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
        Vec3::new(0.0, 6.0, 12.0),
        Vec3::new(0.0, 0.5, 0.0),
    );

    let spline = commands
        .spawn((
            Name::new("Basic Spline"),
            SplinePath {
                curve: SplineCurve {
                    points: vec![
                        SplineControlPoint::new(Vec3::new(-5.0, 0.0, -2.0)),
                        SplineControlPoint::new(Vec3::new(-1.5, 2.0, 1.5)),
                        SplineControlPoint::new(Vec3::new(1.5, 0.8, 3.5)),
                        SplineControlPoint::new(Vec3::new(5.0, 0.2, -1.5)),
                    ],
                    ..default()
                },
                ..default()
            },
            SplineDebugDraw::default(),
        ))
        .id();

    let marker = support::spawn_marker(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Spline Sample Marker",
        0.2,
        Color::srgb(0.98, 0.59, 0.16),
    );
    commands.entity(marker).insert(PathFollower {
        spline,
        speed: 0.18,
    });
}

fn animate_follower(
    time: Res<Time>,
    caches: Query<&SplineCache>,
    mut followers: Query<(&PathFollower, &mut Transform)>,
) {
    for (follower, mut transform) in &mut followers {
        let Ok(cache) = caches.get(follower.spline) else {
            continue;
        };
        let progress = (time.elapsed_secs() * follower.speed).fract();
        if let Some(sample) = cache.sample_normalized(progress) {
            transform.translation = sample.position;
            transform.rotation = sample.rotation;
        }
    }
}
