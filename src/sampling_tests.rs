use super::*;
use crate::{SplineControlPoint, SplineCurve, SplineCurveKind};

fn bake_cache(curve: SplineCurve) -> SplineCache {
    let mut cache = SplineCache::default();
    cache.rebuild(
        &curve,
        &SplineBakeSettings::default(),
        &curve.all_segment_indices().into_iter().collect(),
    );
    cache
}

#[test]
fn sample_distances_are_monotonic_and_tangents_stay_normalized() {
    let curve = SplineCurve {
        points: vec![
            SplineControlPoint::new(Vec3::new(-3.0, 0.0, -1.0)),
            SplineControlPoint::new(Vec3::new(-1.0, 1.5, 0.5)),
            SplineControlPoint::new(Vec3::new(2.0, 0.3, 2.0)),
            SplineControlPoint::new(Vec3::new(4.0, 0.0, -0.5)),
        ],
        ..default()
    };
    let cache = bake_cache(curve);

    let mut previous_distance = -1.0;
    for sample in cache.samples() {
        assert!(sample.distance >= previous_distance);
        assert!((sample.tangent.length() - 1.0).abs() <= 1.0e-3);
        previous_distance = sample.distance;
    }
}

#[test]
fn nearest_point_matches_expected_projection_on_straight_curve() {
    let curve = SplineCurve {
        kind: SplineCurveKind::Bezier,
        points: vec![
            SplineControlPoint::new(Vec3::new(0.0, 0.0, 0.0)),
            SplineControlPoint::new(Vec3::new(6.0, 0.0, 0.0)),
        ],
        ..default()
    };
    let cache = bake_cache(curve);
    let nearest = cache.nearest_point(Vec3::new(2.4, 1.5, 0.0)).unwrap();

    assert!(
        nearest
            .sample
            .position
            .abs_diff_eq(Vec3::new(2.4, 0.0, 0.0), 1.0e-3)
    );
    assert!((nearest.distance_to_curve - 1.5).abs() <= 1.0e-3);
}

#[test]
fn duplicate_control_points_keep_cache_finite() {
    let curve = SplineCurve {
        points: vec![
            SplineControlPoint::new(Vec3::new(0.0, 0.0, 0.0)),
            SplineControlPoint::new(Vec3::new(0.0, 0.0, 0.0)),
            SplineControlPoint::new(Vec3::new(2.0, 0.0, 1.0)),
            SplineControlPoint::new(Vec3::new(4.0, 0.0, 1.0)),
        ],
        ..default()
    };
    let cache = bake_cache(curve);

    assert!(cache.total_length.is_finite());
    assert!(cache.samples().iter().all(|sample| {
        sample.position.is_finite()
            && sample.tangent.is_finite()
            && sample.normal.is_finite()
            && sample.binormal.is_finite()
    }));
}

#[test]
fn closed_loop_with_roll_keeps_first_and_last_frames_aligned() {
    let curve = SplineCurve {
        closed: true,
        points: vec![
            SplineControlPoint {
                position: Vec3::new(0.0, 0.0, 2.0),
                roll_radians: 0.0,
                ..default()
            },
            SplineControlPoint {
                position: Vec3::new(2.0, 1.0, 0.0),
                roll_radians: 0.35,
                ..default()
            },
            SplineControlPoint {
                position: Vec3::new(0.0, 0.0, -2.0),
                roll_radians: 0.55,
                ..default()
            },
            SplineControlPoint {
                position: Vec3::new(-2.0, 1.0, 0.0),
                roll_radians: 0.15,
                ..default()
            },
        ],
        ..default()
    };
    let cache = bake_cache(curve);
    let first = cache.samples().first().unwrap();
    let last = cache.samples().last().unwrap();

    assert!(first.position.abs_diff_eq(last.position, 1.0e-2));
    assert!(first.normal.abs_diff_eq(last.normal, 5.0e-2));
    assert!(first.binormal.abs_diff_eq(last.binormal, 5.0e-2));
}
