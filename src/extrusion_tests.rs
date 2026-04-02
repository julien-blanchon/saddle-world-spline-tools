use super::*;
use crate::{SplineBakeSettings, SplineCache, SplineControlPoint, SplineCurve, SplineCurveKind};

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
fn tube_buffers_have_expected_counts() {
    let curve = SplineCurve {
        points: vec![
            SplineControlPoint::new(Vec3::new(0.0, 0.0, 0.0)),
            SplineControlPoint::new(Vec3::new(1.0, 0.5, 0.0)),
            SplineControlPoint::new(Vec3::new(2.0, 0.0, 1.0)),
        ],
        ..default()
    };
    let cache = bake_cache(curve);
    let extrusion = SplineExtrusion {
        shape: SplineExtrusionShape::Tube(TubeExtrusion {
            radius: 0.4,
            radial_segments: 8,
            use_control_point_radius: false,
        }),
        ..default()
    };

    let buffers = build_extrusion_buffers(&cache, &extrusion);
    assert_eq!(buffers.positions.len(), cache.sample_count * 8);
    assert_eq!(buffers.indices.len(), (cache.sample_count - 1) * 8 * 6);
}

#[test]
fn caps_add_extra_vertices_and_indices() {
    let curve = SplineCurve {
        kind: SplineCurveKind::Bezier,
        points: vec![
            SplineControlPoint::new(Vec3::new(0.0, 0.0, 0.0)),
            SplineControlPoint::new(Vec3::new(3.0, 0.0, 0.0)),
        ],
        ..default()
    };
    let cache = bake_cache(curve);
    let tube = SplineExtrusion {
        shape: SplineExtrusionShape::Tube(TubeExtrusion {
            radius: 0.3,
            radial_segments: 6,
            use_control_point_radius: false,
        }),
        cap_mode: SplineCapMode::Fill,
        ..default()
    };

    let buffers = build_extrusion_buffers(&cache, &tube);
    let expected_extra_vertices = 2 * (1 + 6);
    assert_eq!(
        buffers.positions.len(),
        cache.sample_count * 6 + expected_extra_vertices
    );
    assert!(!buffers.indices.is_empty());
}

#[test]
fn closed_loop_cache_duplicates_the_seam_sample() {
    let curve = SplineCurve {
        closed: true,
        points: vec![
            SplineControlPoint::new(Vec3::new(0.0, 0.0, 2.0)),
            SplineControlPoint::new(Vec3::new(2.0, 0.0, 0.0)),
            SplineControlPoint::new(Vec3::new(0.0, 0.0, -2.0)),
            SplineControlPoint::new(Vec3::new(-2.0, 0.0, 0.0)),
        ],
        ..default()
    };
    let cache = bake_cache(curve);

    let first = cache.samples().first().unwrap();
    let last = cache.samples().last().unwrap();
    assert!(first.position.abs_diff_eq(last.position, 1.0e-2));
}

#[test]
fn generated_buffers_do_not_contain_nans_and_uvs_advance_along_length() {
    let curve = SplineCurve {
        points: vec![
            SplineControlPoint::new(Vec3::new(-1.0, 0.0, -1.0)),
            SplineControlPoint::new(Vec3::new(0.0, 1.0, 0.0)),
            SplineControlPoint::new(Vec3::new(1.0, 0.0, 1.0)),
        ],
        ..default()
    };
    let cache = bake_cache(curve);
    let extrusion = SplineExtrusion {
        shape: SplineExtrusionShape::Tube(TubeExtrusion {
            radius: 0.25,
            radial_segments: 5,
            use_control_point_radius: false,
        }),
        ..default()
    };
    let buffers = build_extrusion_buffers(&cache, &extrusion);

    assert!(
        buffers
            .positions
            .iter()
            .all(|position| position.iter().all(|value| value.is_finite()))
    );
    assert!(
        buffers
            .normals
            .iter()
            .all(|normal| normal.iter().all(|value| value.is_finite()))
    );
    assert!(
        buffers
            .uvs
            .iter()
            .all(|uv| uv.iter().all(|value| value.is_finite()))
    );

    let ring_size = 5usize;
    let mut previous_v = f32::NEG_INFINITY;
    for ring in 0..cache.sample_count {
        let uv = buffers.uvs[ring * ring_size];
        assert!(uv[1] >= previous_v - 1.0e-5);
        previous_v = uv[1];
    }
}

#[test]
fn ribbon_width_metadata_changes_ring_span() {
    let curve = SplineCurve {
        kind: SplineCurveKind::Bezier,
        points: vec![
            SplineControlPoint {
                position: Vec3::new(0.0, 0.0, 0.0),
                width: 1.0,
                ..default()
            },
            SplineControlPoint {
                position: Vec3::new(4.0, 0.0, 0.0),
                width: 2.0,
                ..default()
            },
        ],
        ..default()
    };
    let cache = bake_cache(curve);
    let extrusion = SplineExtrusion {
        shape: SplineExtrusionShape::Ribbon(RibbonExtrusion {
            half_width: 0.75,
            thickness: 0.0,
            use_control_point_width: true,
        }),
        ..default()
    };

    let buffers = build_extrusion_buffers(&cache, &extrusion);
    let start_span =
        Vec3::from_array(buffers.positions[0]).distance(Vec3::from_array(buffers.positions[1]));
    let end_ring = (cache.sample_count - 1) * 2;
    let end_span = Vec3::from_array(buffers.positions[end_ring])
        .distance(Vec3::from_array(buffers.positions[end_ring + 1]));

    assert!((start_span - 1.5).abs() <= 1.0e-3);
    assert!((end_span - 3.0).abs() <= 1.0e-3);
}

#[test]
fn tube_radius_metadata_changes_ring_radius_without_first_sample_bias() {
    let curve = SplineCurve {
        kind: SplineCurveKind::Bezier,
        points: vec![
            SplineControlPoint {
                position: Vec3::new(0.0, 0.0, 0.0),
                radius: 0.5,
                ..default()
            },
            SplineControlPoint {
                position: Vec3::new(0.0, 0.0, 4.0),
                radius: 2.0,
                ..default()
            },
        ],
        ..default()
    };
    let cache = bake_cache(curve);
    let extrusion = SplineExtrusion {
        shape: SplineExtrusionShape::Tube(TubeExtrusion {
            radius: 0.3,
            radial_segments: 8,
            use_control_point_radius: true,
        }),
        ..default()
    };

    let buffers = build_extrusion_buffers(&cache, &extrusion);
    let start_center = cache.samples()[0].position;
    let start_radius = ring_max_radius(&buffers, 0, 8, start_center);
    let end_ring = cache.sample_count - 1;
    let end_center = cache.samples()[end_ring].position;
    let end_radius = ring_max_radius(&buffers, end_ring, 8, end_center);

    assert!((start_radius - 0.15).abs() <= 1.0e-3);
    assert!((end_radius - 0.6).abs() <= 1.0e-3);
}

fn ring_max_radius(
    buffers: &ExtrusionBuffers,
    ring_index: usize,
    ring_size: usize,
    center: Vec3,
) -> f32 {
    (0..ring_size)
        .map(|point_index| {
            Vec3::from_array(buffers.positions[ring_index * ring_size + point_index])
                .distance(center)
        })
        .fold(0.0, f32::max)
}

#[test]
#[ignore = "manual benchmark-style probe for dense sweeps"]
fn benchmark_dense_tube_rebuild() {
    let points: Vec<SplineControlPoint> = (0..300)
        .map(|index| {
            let x = index as f32 * 0.4;
            let z = (index as f32 * 0.18).sin() * 3.0;
            SplineControlPoint::new(Vec3::new(x, 0.0, z))
        })
        .collect();
    let curve = SplineCurve {
        points,
        ..default()
    };

    let mut cache = SplineCache::default();
    let bake = SplineBakeSettings {
        samples_per_segment: 32,
        ..default()
    };
    let dirty = curve.all_segment_indices().into_iter().collect();
    cache.rebuild(&curve, &bake, &dirty);
    let extrusion = SplineExtrusion {
        shape: SplineExtrusionShape::Tube(TubeExtrusion {
            radius: 0.18,
            radial_segments: 20,
            use_control_point_radius: false,
        }),
        ..default()
    };
    let buffers = build_extrusion_buffers(&cache, &extrusion);
    assert!(buffers.positions.len() > 10_000);
}
