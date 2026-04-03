use super::*;

fn sample(position: Vec3, tangent: Vec3, normal: Vec3, binormal: Vec3) -> SplineSample {
    SplineSample {
        position,
        tangent,
        normal,
        binormal,
        rotation: Quat::IDENTITY,
        ..default()
    }
}

#[test]
fn projection_updates_height_and_aligns_frame_to_surface() {
    let samples = vec![sample(
        Vec3::new(1.0, 0.25, -2.0),
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
    )];
    let projected = project_samples_onto_surface(
        &samples,
        &TerrainProjectionSettings {
            vertical_offset: 0.2,
            normal_alignment: 1.0,
        },
        |_| Some(3.5),
        |_| Some(Vec3::new(0.0, 1.0, 0.5).normalize()),
    );

    let projected = projected[0];
    assert!((projected.position.y - 3.7).abs() <= 1.0e-5);
    assert!(projected.tangent.abs_diff_eq(Vec3::X, 1.0e-5));
    assert!(projected.normal.is_normalized());
    assert!(projected.binormal.is_normalized());
    assert!((projected.normal.dot(projected.tangent)).abs() <= 1.0e-5);
    assert!((projected.binormal.dot(projected.tangent)).abs() <= 1.0e-5);
}

#[test]
fn projection_keeps_original_sample_when_height_query_misses() {
    let samples = vec![sample(
        Vec3::new(-1.0, 0.4, 2.0),
        Vec3::Z,
        Vec3::Y,
        -Vec3::X,
    )];

    let projected = project_samples_onto_surface(
        &samples,
        &TerrainProjectionSettings::default(),
        |_| None,
        |_| Some(Vec3::new(0.0, 1.0, 0.2).normalize()),
    );

    assert_eq!(projected, samples);
}
