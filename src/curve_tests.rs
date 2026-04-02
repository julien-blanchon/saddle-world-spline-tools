use super::*;

fn approx_vec3(a: Vec3, b: Vec3) {
    assert!(a.distance(b) <= 1.0e-3, "expected {a:?} ~= {b:?}");
}

#[test]
fn bezier_segment_hits_anchor_endpoints() {
    let curve = SplineCurve {
        kind: SplineCurveKind::Bezier,
        points: vec![
            SplineControlPoint::new(Vec3::new(0.0, 0.0, 0.0))
                .with_handles(None, Some(Vec3::new(1.0, 0.0, 0.0))),
            SplineControlPoint::new(Vec3::new(3.0, 1.0, 0.0))
                .with_handles(Some(Vec3::new(2.0, 1.0, 0.0)), None),
        ],
        ..default()
    };

    let start = curve.sample_segment(0, 0.0);
    let end = curve.sample_segment(0, 1.0);

    approx_vec3(start.position, Vec3::ZERO);
    approx_vec3(end.position, Vec3::new(3.0, 1.0, 0.0));
    assert!(start.tangent.length() > 0.99);
    assert!(end.tangent.length() > 0.99);
}

#[test]
fn default_catmull_rom_is_centripetal() {
    assert_eq!(
        SplineCurve::default().catmull_rom.parameterization,
        CatmullRomParameterization::Centripetal
    );
}

#[test]
fn centripetal_catmull_rom_differs_from_uniform_on_uneven_points() {
    let points = vec![
        SplineControlPoint::new(Vec3::new(0.0, 0.0, 0.0)),
        SplineControlPoint::new(Vec3::new(0.05, 0.0, 0.0)),
        SplineControlPoint::new(Vec3::new(3.0, 1.0, 0.0)),
        SplineControlPoint::new(Vec3::new(4.0, 1.2, 0.0)),
    ];
    let centripetal = SplineCurve {
        points: points.clone(),
        catmull_rom: CatmullRomOptions {
            parameterization: CatmullRomParameterization::Centripetal,
        },
        ..default()
    };
    let uniform = SplineCurve {
        points,
        catmull_rom: CatmullRomOptions {
            parameterization: CatmullRomParameterization::Uniform,
        },
        ..default()
    };

    let cent = centripetal.sample_segment(1, 0.5);
    let uni = uniform.sample_segment(1, 0.5);

    assert!(cent.position.x > 0.05 && cent.position.x < 3.0);
    assert!(cent.position.is_finite());
    assert!(!cent.position.abs_diff_eq(uni.position, 1.0e-4));
}

#[test]
fn affected_segments_cover_local_edit_neighborhood() {
    let curve = SplineCurve {
        points: vec![
            SplineControlPoint::new(Vec3::new(-2.0, 0.0, 0.0)),
            SplineControlPoint::new(Vec3::new(-1.0, 1.0, 0.0)),
            SplineControlPoint::new(Vec3::new(1.0, 1.0, 0.0)),
            SplineControlPoint::new(Vec3::new(2.0, 0.0, 0.0)),
            SplineControlPoint::new(Vec3::new(3.0, -1.0, 0.0)),
        ],
        ..default()
    };

    assert_eq!(curve.affected_segments_for_point(2), vec![0, 1, 2, 3]);
}
