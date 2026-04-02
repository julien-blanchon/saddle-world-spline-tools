use saddle_world_spline_tools_example_support as support;

use bevy::prelude::*;
use saddle_world_spline_tools::{
    RibbonExtrusion, SplineControlPoint, SplineCurve, SplineDebugDraw, SplineExtrusion,
    SplineExtrusionShape, SplineMeshTarget, SplinePath, SplineToolsPlugin, SplineUvMode,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "spline_tools extrusion road".into(),
            resolution: (1440, 860).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(SplineToolsPlugin::default());
    support::install_auto_exit(&mut app);
    app.add_systems(Startup, setup);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    support::spawn_scene_basics(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(-5.0, 8.5, 14.0),
        Vec3::new(0.0, 0.5, 0.0),
    );

    let texture = support::stripe_texture(
        &mut images,
        Color::srgb(0.16, 0.18, 0.2),
        Color::srgb(0.9, 0.8, 0.25),
    );
    let mesh = support::empty_mesh(&mut meshes);

    commands.spawn((
        Name::new("Road Ribbon"),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            perceptual_roughness: 0.88,
            cull_mode: None,
            ..default()
        })),
        SplinePath {
            curve: SplineCurve {
                points: vec![
                    SplineControlPoint {
                        position: Vec3::new(-8.0, 0.0, -5.5),
                        width: 1.2,
                        roll_radians: 0.0,
                        ..default()
                    },
                    SplineControlPoint {
                        position: Vec3::new(-3.0, 0.2, -1.5),
                        width: 1.35,
                        roll_radians: 0.08,
                        ..default()
                    },
                    SplineControlPoint {
                        position: Vec3::new(1.5, 0.0, 1.0),
                        width: 1.55,
                        roll_radians: -0.18,
                        ..default()
                    },
                    SplineControlPoint {
                        position: Vec3::new(5.5, 0.1, 3.8),
                        width: 1.3,
                        roll_radians: 0.14,
                        ..default()
                    },
                    SplineControlPoint {
                        position: Vec3::new(8.0, 0.0, -2.2),
                        width: 1.1,
                        roll_radians: -0.05,
                        ..default()
                    },
                ],
                ..default()
            },
            ..default()
        },
        SplineMeshTarget::new(
            mesh,
            SplineExtrusion {
                shape: SplineExtrusionShape::Ribbon(RibbonExtrusion {
                    half_width: 1.0,
                    thickness: 0.0,
                    use_control_point_width: true,
                }),
                uv_mode: SplineUvMode::TileByWorldDistance,
                uv_tile_length: 1.25,
                ..default()
            },
        ),
        SplineDebugDraw {
            draw_frames: false,
            draw_samples: false,
            ..default()
        },
    ));
}
