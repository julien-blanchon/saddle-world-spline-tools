use saddle_world_spline_tools_example_support as support;

use bevy::prelude::*;
use saddle_world_spline_tools::{
    SplineCache, SplineControlPoint, SplineCurve, SplineDebugDraw, SplinePath, SplineToolsPlugin,
};

#[derive(Component)]
struct PlacementSpline;

#[derive(Component)]
struct PlacementPost;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "spline_tools placement".into(),
            resolution: (1440, 860).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(SplineToolsPlugin::default());
    support::install_auto_exit(&mut app);
    app.add_systems(Startup, setup);
    app.add_systems(Update, refresh_posts);
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
        Vec3::new(0.0, 9.0, 15.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    commands.spawn((
        Name::new("Placement Guide"),
        PlacementSpline,
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        SplinePath {
            curve: SplineCurve {
                points: vec![
                    SplineControlPoint::new(Vec3::new(-8.0, 0.0, -4.0)),
                    SplineControlPoint::new(Vec3::new(-4.0, 1.8, -1.0)),
                    SplineControlPoint::new(Vec3::new(0.0, 1.1, 2.0)),
                    SplineControlPoint::new(Vec3::new(4.0, 2.4, 0.5)),
                    SplineControlPoint::new(Vec3::new(8.0, 0.6, -3.5)),
                ],
                ..default()
            },
            ..default()
        },
        SplineDebugDraw {
            draw_frames: true,
            draw_samples: false,
            ..default()
        },
    ));
}

fn refresh_posts(
    mut commands: Commands,
    spline_query: Query<(Entity, &SplineCache), (With<PlacementSpline>, Changed<SplineCache>)>,
    posts: Query<Entity, With<PlacementPost>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((spline_entity, cache)) = spline_query.single() else {
        return;
    };

    for entity in &posts {
        commands.entity(entity).despawn();
    }

    let post_mesh = meshes.add(Cuboid::new(0.12, 1.1, 0.12));
    let post_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.43, 0.28),
        perceptual_roughness: 0.92,
        ..default()
    });
    let rail_mesh = meshes.add(Cuboid::new(0.1, 0.08, 0.8));
    let rail_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.52, 0.34),
        perceptual_roughness: 0.86,
        ..default()
    });

    let transforms = cache.sample_evenly_spaced_transforms(1.2, true);
    commands.entity(spline_entity).with_children(|parent| {
        for (index, transform) in transforms.into_iter().enumerate() {
            parent.spawn((
                Name::new(format!("Fence Post {index:02}")),
                PlacementPost,
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(post_material.clone()),
                Transform::from_translation(
                    transform.translation + transform.rotation * Vec3::Y * 0.55,
                ),
            ));
            parent.spawn((
                Name::new(format!("Fence Rail {index:02}")),
                PlacementPost,
                Mesh3d(rail_mesh.clone()),
                MeshMaterial3d(rail_material.clone()),
                Transform {
                    translation: transform.translation
                        + transform.rotation * Vec3::new(0.0, 0.55, 0.0),
                    rotation: transform.rotation,
                    ..default()
                },
            ));
        }
    });
}
