use super::*;
use crate::curve::CurveEvaluation;

fn sample_evaluations() -> Vec<CurveEvaluation> {
    vec![
        CurveEvaluation {
            position: Vec3::new(0.0, 0.0, 0.0),
            tangent: Vec3::new(1.0, 0.1, 0.0).normalize(),
            ..default()
        },
        CurveEvaluation {
            position: Vec3::new(1.0, 0.2, 0.1),
            tangent: Vec3::new(1.0, 0.2, 0.4).normalize(),
            ..default()
        },
        CurveEvaluation {
            position: Vec3::new(2.0, 0.5, 0.8),
            tangent: Vec3::new(0.8, 0.2, 0.6).normalize(),
            ..default()
        },
        CurveEvaluation {
            position: Vec3::new(3.0, 0.9, 1.3),
            tangent: Vec3::new(0.5, 0.0, 0.8).normalize(),
            ..default()
        },
    ]
}

#[test]
fn rmf_frames_are_orthonormal() {
    let frames = generate_frames(&sample_evaluations(), &FrameMode::default(), false);
    assert_eq!(frames.len(), 4);

    for frame in frames {
        assert!(frame.normal.length() > 0.99);
        assert!(frame.binormal.length() > 0.99);
        assert!(frame.normal.dot(frame.binormal).abs() < 1.0e-3);
        let tangent = frame.rotation * Vec3::Z;
        assert!(tangent.dot(frame.normal).abs() < 1.0e-3);
        assert!(tangent.dot(frame.binormal).abs() < 1.0e-3);
    }
}

#[test]
fn closed_loop_correction_aligns_first_and_last_normals() {
    let samples = vec![
        CurveEvaluation {
            position: Vec3::new(1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, 1.0),
            ..default()
        },
        CurveEvaluation {
            position: Vec3::new(0.0, 1.0, 1.0),
            tangent: Vec3::new(-1.0, 1.0, 0.0).normalize(),
            ..default()
        },
        CurveEvaluation {
            position: Vec3::new(-1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, -1.0, -1.0).normalize(),
            ..default()
        },
        CurveEvaluation {
            position: Vec3::new(0.0, -1.0, -1.0),
            tangent: Vec3::new(1.0, -1.0, 0.0).normalize(),
            ..default()
        },
        CurveEvaluation {
            position: Vec3::new(1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, 1.0),
            ..default()
        },
    ];

    let frames = generate_frames(&samples, &FrameMode::default(), true);
    let first = frames.first().unwrap();
    let last = frames.last().unwrap();
    assert!(first.normal.abs_diff_eq(last.normal, 5.0e-2));
    assert!(first.binormal.abs_diff_eq(last.binormal, 5.0e-2));
}

#[test]
fn fixed_up_handles_vertical_tangent_without_nans() {
    let samples = vec![
        CurveEvaluation {
            position: Vec3::ZERO,
            tangent: Vec3::Y,
            ..default()
        },
        CurveEvaluation {
            position: Vec3::new(0.0, 1.0, 0.0),
            tangent: Vec3::Y,
            ..default()
        },
    ];

    let frames = generate_frames(&samples, &FrameMode::FixedUp { up: Vec3::Y }, false);
    for frame in frames {
        assert!(frame.normal.is_finite());
        assert!(frame.binormal.is_finite());
    }
}
