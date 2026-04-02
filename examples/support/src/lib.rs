use std::{thread, time::Duration};

use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
};

pub const EXIT_AFTER_ENV: &str = "SPLINE_TOOLS_EXIT_AFTER_SECONDS";

#[derive(Resource)]
struct ExitAfterTimer(Timer);

pub fn install_auto_exit(app: &mut App) {
    let timer = std::env::var(EXIT_AFTER_ENV)
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .map(|seconds| seconds.max(0.1));

    if let Some(seconds) = timer {
        app.insert_resource(bevy::winit::WinitSettings::continuous());
        app.insert_resource(ExitAfterTimer(Timer::from_seconds(
            seconds,
            TimerMode::Once,
        )));
        app.add_systems(Update, exit_after_timer);

        // Windowed examples can stop polling updates in agent-driven runs, so keep a
        // process-level fallback only for explicit batch-verification launches.
        thread::spawn(move || {
            thread::sleep(Duration::from_secs_f32(seconds + 0.25));
            std::process::exit(0);
        });
    }
}

pub fn spawn_scene_basics(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    camera_position: Vec3,
    look_at: Vec3,
) {
    commands.spawn((
        Name::new("Example Camera"),
        Camera3d::default(),
        Transform::from_translation(camera_position).looking_at(look_at, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Example Sun"),
        DirectionalLight {
            illuminance: 18_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.75, 0.0)),
    ));
    commands.spawn((
        Name::new("Ground Plane"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(80.0, 80.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.16, 0.18),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));
}

#[allow(dead_code)]
pub fn spawn_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    radius: f32,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Name::new(name.to_owned()),
            Mesh3d(meshes.add(Sphere::new(radius).mesh().uv(24, 16))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: color.into(),
                ..default()
            })),
            Transform::default(),
        ))
        .id()
}

#[allow(dead_code)]
pub fn stripe_texture(
    images: &mut Assets<Image>,
    primary: Color,
    secondary: Color,
) -> Handle<Image> {
    let width = 128u32;
    let height = 16u32;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    let primary = primary.to_srgba();
    let secondary = secondary.to_srgba();
    for _y in 0..height {
        for x in 0..width {
            let source = if (x / 8) % 2 == 0 { primary } else { secondary };
            pixels.extend_from_slice(&[
                (source.red * 255.0) as u8,
                (source.green * 255.0) as u8,
                (source.blue * 255.0) as u8,
                (source.alpha * 255.0) as u8,
            ]);
        }
    }
    images.add(Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

#[allow(dead_code)]
pub fn empty_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    meshes.add(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

fn exit_after_timer(
    time: Res<Time>,
    mut timer: ResMut<ExitAfterTimer>,
    mut exit: MessageWriter<AppExit>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        exit.write(AppExit::Success);
    }
}
