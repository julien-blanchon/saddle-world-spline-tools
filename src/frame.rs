use bevy::prelude::*;

use crate::curve::CurveEvaluation;

const FRAME_EPSILON: f32 = 1.0e-5;

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum FrameMode {
    FixedUp { up: Vec3 },
    Frenet,
    ParallelTransport { up_hint: Vec3 },
    RotationMinimizing { up_hint: Vec3 },
}

impl Default for FrameMode {
    fn default() -> Self {
        Self::RotationMinimizing { up_hint: Vec3::Y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SplineFrame {
    pub normal: Vec3,
    pub binormal: Vec3,
    pub rotation: Quat,
}

pub fn generate_frames(
    samples: &[CurveEvaluation],
    frame_mode: &FrameMode,
    closed: bool,
) -> Vec<SplineFrame> {
    if samples.is_empty() {
        return Vec::new();
    }

    let base = match frame_mode {
        FrameMode::FixedUp { up } => fixed_up_frames(samples, *up),
        FrameMode::Frenet => frenet_frames(samples),
        FrameMode::ParallelTransport { up_hint } => transport_frames(samples, *up_hint, closed),
        FrameMode::RotationMinimizing { up_hint } => rmf_frames(samples, *up_hint, closed),
    };

    base.into_iter()
        .zip(samples.iter())
        .map(|(frame, sample)| apply_roll(frame, sample.tangent, sample.roll_radians))
        .collect()
}

fn fixed_up_frames(samples: &[CurveEvaluation], up: Vec3) -> Vec<SplineFrame> {
    let hint = safe_axis(up);
    samples
        .iter()
        .map(|sample| {
            let tangent = safe_tangent(sample.tangent);
            let normal = projected_perpendicular(hint, tangent).unwrap_or_else(|| {
                projected_perpendicular(fallback_axis(tangent), tangent).unwrap()
            });
            frame_from_tangent_normal(tangent, normal)
        })
        .collect()
}

fn frenet_frames(samples: &[CurveEvaluation]) -> Vec<SplineFrame> {
    let mut frames = Vec::with_capacity(samples.len());
    let mut previous_normal = projected_perpendicular(Vec3::Y, safe_tangent(samples[0].tangent))
        .or_else(|| projected_perpendicular(Vec3::X, safe_tangent(samples[0].tangent)))
        .unwrap_or(Vec3::X);

    for (index, sample) in samples.iter().enumerate() {
        let tangent = safe_tangent(sample.tangent);
        let previous_tangent = safe_tangent(samples[index.saturating_sub(1)].tangent);
        let next_tangent = safe_tangent(samples[(index + 1).min(samples.len() - 1)].tangent);
        let derivative = (next_tangent - previous_tangent).normalize_or_zero();
        let normal = if derivative.length_squared() > FRAME_EPSILON {
            derivative
        } else {
            projected_perpendicular(previous_normal, tangent).unwrap_or_else(|| {
                projected_perpendicular(fallback_axis(tangent), tangent).unwrap()
            })
        };
        let frame = frame_from_tangent_normal(tangent, normal);
        previous_normal = frame.normal;
        frames.push(frame);
    }

    frames
}

fn transport_frames(samples: &[CurveEvaluation], up_hint: Vec3, closed: bool) -> Vec<SplineFrame> {
    let mut frames = Vec::with_capacity(samples.len());
    let mut current_normal = initial_normal(samples[0].tangent, up_hint);
    frames.push(frame_from_tangent_normal(
        safe_tangent(samples[0].tangent),
        current_normal,
    ));

    for pair in samples.windows(2) {
        let tangent_a = safe_tangent(pair[0].tangent);
        let tangent_b = safe_tangent(pair[1].tangent);
        let rotation = Quat::from_rotation_arc(tangent_a, tangent_b);
        current_normal = (rotation * current_normal).normalize_or_zero();
        current_normal = projected_perpendicular(current_normal, tangent_b)
            .unwrap_or_else(|| initial_normal(tangent_b, up_hint));
        frames.push(frame_from_tangent_normal(tangent_b, current_normal));
    }

    if closed {
        distribute_closed_loop_twist(samples, &mut frames);
    }

    frames
}

fn rmf_frames(samples: &[CurveEvaluation], up_hint: Vec3, closed: bool) -> Vec<SplineFrame> {
    let mut normals = Vec::with_capacity(samples.len());
    let tangents: Vec<Vec3> = samples
        .iter()
        .map(|sample| safe_tangent(sample.tangent))
        .collect();

    normals.push(initial_normal(tangents[0], up_hint));
    for index in 0..samples.len().saturating_sub(1) {
        let position_a = samples[index].position;
        let position_b = samples[index + 1].position;
        let tangent_a = tangents[index];
        let tangent_b = tangents[index + 1];
        let propagated =
            double_reflection(position_a, position_b, tangent_a, tangent_b, normals[index]);
        normals.push(
            projected_perpendicular(propagated, tangent_b)
                .unwrap_or_else(|| initial_normal(tangent_b, up_hint)),
        );
    }

    let mut frames: Vec<SplineFrame> = tangents
        .into_iter()
        .zip(normals)
        .map(|(tangent, normal)| frame_from_tangent_normal(tangent, normal))
        .collect();

    if closed {
        distribute_closed_loop_twist(samples, &mut frames);
    }

    frames
}

fn distribute_closed_loop_twist(samples: &[CurveEvaluation], frames: &mut [SplineFrame]) {
    if frames.len() < 2 {
        return;
    }

    let start = frames[0];
    let end = frames[frames.len() - 1];
    let tangent = safe_tangent(samples[0].tangent);
    let signed_angle = signed_angle_around_axis(end.normal, start.normal, tangent);
    if signed_angle.abs() <= 1.0e-4 {
        return;
    }

    let total_distance = total_length(samples);
    if total_distance <= FRAME_EPSILON {
        return;
    }

    let mut accumulated = 0.0;
    for index in 0..frames.len() {
        if index > 0 {
            accumulated += samples[index]
                .position
                .distance(samples[index - 1].position);
        }
        let ratio = (accumulated / total_distance).clamp(0.0, 1.0);
        let correction =
            Quat::from_axis_angle(safe_tangent(samples[index].tangent), signed_angle * ratio);
        let normal = (correction * frames[index].normal).normalize_or_zero();
        frames[index] = frame_from_tangent_normal(safe_tangent(samples[index].tangent), normal);
    }

    frames[frames.len() - 1] = frame_from_tangent_normal(tangent, frames[0].normal);
}

fn double_reflection(
    point_a: Vec3,
    point_b: Vec3,
    tangent_a: Vec3,
    tangent_b: Vec3,
    normal_a: Vec3,
) -> Vec3 {
    let v1 = point_b - point_a;
    let c1 = v1.length_squared();
    if c1 <= FRAME_EPSILON {
        let rotation = Quat::from_rotation_arc(tangent_a, tangent_b);
        return (rotation * normal_a).normalize_or_zero();
    }

    let reflected_normal = normal_a - (2.0 * v1.dot(normal_a) / c1) * v1;
    let reflected_tangent = tangent_a - (2.0 * v1.dot(tangent_a) / c1) * v1;
    let v2 = tangent_b - reflected_tangent;
    let c2 = v2.length_squared();
    if c2 <= FRAME_EPSILON {
        return reflected_normal.normalize_or_zero();
    }

    (reflected_normal - (2.0 * v2.dot(reflected_normal) / c2) * v2).normalize_or_zero()
}

fn apply_roll(frame: SplineFrame, tangent: Vec3, roll_radians: f32) -> SplineFrame {
    if roll_radians.abs() <= FRAME_EPSILON {
        return frame;
    }
    let rotation = Quat::from_axis_angle(safe_tangent(tangent), roll_radians);
    frame_from_tangent_normal(safe_tangent(tangent), rotation * frame.normal)
}

fn frame_from_tangent_normal(tangent: Vec3, normal: Vec3) -> SplineFrame {
    let tangent = safe_tangent(tangent);
    let normal = projected_perpendicular(normal, tangent)
        .unwrap_or_else(|| projected_perpendicular(fallback_axis(tangent), tangent).unwrap());
    let binormal = tangent.cross(normal).normalize_or_zero();
    let rotation = Quat::from_mat3(&Mat3::from_cols(normal, binormal, tangent));
    SplineFrame {
        normal,
        binormal,
        rotation,
    }
}

fn initial_normal(tangent: Vec3, up_hint: Vec3) -> Vec3 {
    projected_perpendicular(up_hint, safe_tangent(tangent))
        .or_else(|| projected_perpendicular(Vec3::X, safe_tangent(tangent)))
        .or_else(|| projected_perpendicular(Vec3::Z, safe_tangent(tangent)))
        .unwrap_or(Vec3::Y)
}

fn projected_perpendicular(vector: Vec3, tangent: Vec3) -> Option<Vec3> {
    let projected = vector - tangent * vector.dot(tangent);
    (projected.length_squared() > FRAME_EPSILON).then_some(projected.normalize())
}

fn safe_tangent(tangent: Vec3) -> Vec3 {
    if tangent.length_squared() <= FRAME_EPSILON {
        Vec3::Z
    } else {
        tangent.normalize()
    }
}

fn safe_axis(axis: Vec3) -> Vec3 {
    if axis.length_squared() <= FRAME_EPSILON {
        Vec3::Y
    } else {
        axis.normalize()
    }
}

fn fallback_axis(tangent: Vec3) -> Vec3 {
    if tangent.abs_diff_eq(Vec3::Y, 1.0e-2) || tangent.abs_diff_eq(Vec3::NEG_Y, 1.0e-2) {
        Vec3::X
    } else {
        Vec3::Y
    }
}

fn signed_angle_around_axis(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
    let from = projected_perpendicular(from, axis).unwrap_or(from.normalize_or_zero());
    let to = projected_perpendicular(to, axis).unwrap_or(to.normalize_or_zero());
    let cross = from.cross(to);
    cross.dot(axis).atan2(from.dot(to).clamp(-1.0, 1.0))
}

fn total_length(samples: &[CurveEvaluation]) -> f32 {
    samples
        .windows(2)
        .map(|pair| pair[0].position.distance(pair[1].position))
        .sum()
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
