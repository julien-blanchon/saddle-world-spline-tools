use std::collections::BTreeSet;

use bevy::prelude::*;

use crate::{
    SplineCurve,
    curve::CurveEvaluation,
    frame::{FrameMode, generate_frames},
};

const DISTANCE_EPSILON: f32 = 1.0e-5;

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct SplineBakeSettings {
    pub samples_per_segment: usize,
    pub frame_mode: FrameMode,
}

impl Default for SplineBakeSettings {
    fn default() -> Self {
        Self {
            samples_per_segment: 24,
            frame_mode: FrameMode::default(),
        }
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct SplineCache {
    pub total_length: f32,
    pub segment_count: usize,
    pub sample_count: usize,
    pub revision: u64,
    #[reflect(ignore)]
    segment_samples: Vec<Vec<CurveEvaluation>>,
    #[reflect(ignore)]
    baked_samples: Vec<SplineSample>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub struct SplineSample {
    pub position: Vec3,
    pub tangent: Vec3,
    pub normal: Vec3,
    pub binormal: Vec3,
    pub rotation: Quat,
    pub distance: f32,
    pub normalized: f32,
    pub roll_radians: f32,
    pub width: f32,
    pub radius: f32,
    pub scale: Vec2,
    pub segment_index: usize,
    pub segment_t: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub struct SplineNearestPoint {
    pub sample: SplineSample,
    pub distance_to_curve: f32,
}

impl SplineCache {
    pub fn rebuild(
        &mut self,
        curve: &SplineCurve,
        settings: &SplineBakeSettings,
        dirty_segments: &BTreeSet<usize>,
    ) {
        let segment_count = curve.segment_count();
        let samples_per_segment = settings.samples_per_segment.max(2);
        let rebuild_all = self.segment_samples.len() != segment_count || dirty_segments.is_empty();

        if rebuild_all {
            self.segment_samples = (0..segment_count)
                .map(|segment_index| sample_segment(curve, segment_index, samples_per_segment))
                .collect();
        } else {
            for &segment_index in dirty_segments {
                if segment_index < segment_count {
                    self.segment_samples[segment_index] =
                        sample_segment(curve, segment_index, samples_per_segment);
                }
            }
        }

        self.segment_count = segment_count;
        self.baked_samples =
            flatten_and_frame_samples(&self.segment_samples, &settings.frame_mode, curve.closed);
        self.total_length = self
            .baked_samples
            .last()
            .map_or(0.0, |sample| sample.distance);
        if self.total_length > DISTANCE_EPSILON {
            for sample in &mut self.baked_samples {
                sample.normalized = sample.distance / self.total_length;
            }
        }
        self.sample_count = self.baked_samples.len();
        self.revision = self.revision.saturating_add(1);
    }

    pub fn samples(&self) -> &[SplineSample] {
        &self.baked_samples
    }

    pub fn sample_normalized(&self, normalized: f32) -> Option<SplineSample> {
        if self.baked_samples.is_empty() {
            return None;
        }
        if self.total_length <= DISTANCE_EPSILON {
            return self.baked_samples.first().copied();
        }
        self.sample_distance(normalized.clamp(0.0, 1.0) * self.total_length)
    }

    pub fn sample_distance(&self, distance: f32) -> Option<SplineSample> {
        if self.baked_samples.is_empty() {
            return None;
        }
        if self.baked_samples.len() == 1 || self.total_length <= DISTANCE_EPSILON {
            return self.baked_samples.first().copied();
        }

        let clamped = distance.clamp(0.0, self.total_length);
        match self
            .baked_samples
            .binary_search_by(|sample| sample.distance.total_cmp(&clamped))
        {
            Ok(index) => Some(self.baked_samples[index]),
            Err(index) => {
                let upper = index.min(self.baked_samples.len() - 1);
                let lower = upper.saturating_sub(1);
                Some(lerp_samples(
                    self.baked_samples[lower],
                    self.baked_samples[upper],
                    clamped,
                    self.total_length,
                ))
            }
        }
    }

    pub fn nearest_point(&self, query: Vec3) -> Option<SplineNearestPoint> {
        let mut best: Option<SplineNearestPoint> = None;
        for pair in self.baked_samples.windows(2) {
            let a = pair[0];
            let b = pair[1];
            let segment = b.position - a.position;
            let length_sq = segment.length_squared();
            let alpha = if length_sq <= DISTANCE_EPSILON {
                0.0
            } else {
                ((query - a.position).dot(segment) / length_sq).clamp(0.0, 1.0)
            };
            let sample = lerp_samples(a, b, a.distance.lerp(b.distance, alpha), self.total_length);
            let distance_to_curve = sample.position.distance(query);
            match best {
                Some(current) if current.distance_to_curve <= distance_to_curve => {}
                _ => {
                    best = Some(SplineNearestPoint {
                        sample,
                        distance_to_curve,
                    });
                }
            }
        }
        best.or_else(|| {
            self.baked_samples
                .first()
                .copied()
                .map(|sample| SplineNearestPoint {
                    sample,
                    distance_to_curve: sample.position.distance(query),
                })
        })
    }

    pub fn evenly_spaced_distances(&self, spacing: f32, include_end: bool) -> Vec<f32> {
        if self.baked_samples.is_empty() {
            return Vec::new();
        }
        if spacing <= DISTANCE_EPSILON || self.total_length <= DISTANCE_EPSILON {
            return vec![0.0];
        }

        let mut distances = Vec::new();
        let mut current = 0.0;
        while current < self.total_length {
            distances.push(current);
            current += spacing;
        }
        if include_end && distances.last().copied() != Some(self.total_length) {
            distances.push(self.total_length);
        }
        distances
    }

    pub fn sample_evenly_spaced(&self, spacing: f32, include_end: bool) -> Vec<SplineSample> {
        self.evenly_spaced_distances(spacing, include_end)
            .into_iter()
            .filter_map(|distance| self.sample_distance(distance))
            .collect()
    }

    pub fn sample_evenly_spaced_transforms(
        &self,
        spacing: f32,
        include_end: bool,
    ) -> Vec<Transform> {
        self.sample_evenly_spaced(spacing, include_end)
            .into_iter()
            .map(|sample| Transform {
                translation: sample.position,
                rotation: sample.rotation,
                scale: Vec3::new(sample.scale.x, sample.scale.y, 1.0),
            })
            .collect()
    }
}

fn sample_segment(
    curve: &SplineCurve,
    segment_index: usize,
    samples_per_segment: usize,
) -> Vec<CurveEvaluation> {
    (0..=samples_per_segment)
        .map(|step| step as f32 / samples_per_segment as f32)
        .map(|segment_t| curve.sample_segment(segment_index, segment_t))
        .collect()
}

fn flatten_and_frame_samples(
    segments: &[Vec<CurveEvaluation>],
    frame_mode: &FrameMode,
    closed: bool,
) -> Vec<SplineSample> {
    let evaluations: Vec<CurveEvaluation> = segments
        .iter()
        .enumerate()
        .flat_map(|(segment_index, segment)| {
            segment
                .iter()
                .copied()
                .enumerate()
                .filter(move |(sample_index, _)| segment_index == 0 || *sample_index > 0)
                .map(|(_, evaluation)| evaluation)
        })
        .collect();

    if evaluations.is_empty() {
        return Vec::new();
    }

    let frames = generate_frames(&evaluations, frame_mode, closed);
    let total_length = evaluations
        .windows(2)
        .map(|pair| pair[0].position.distance(pair[1].position))
        .sum::<f32>();

    let mut distance = 0.0;
    let mut samples = Vec::with_capacity(evaluations.len());
    for index in 0..evaluations.len() {
        if index > 0 {
            distance += evaluations[index - 1]
                .position
                .distance(evaluations[index].position);
        }
        let frame = frames[index];
        let evaluation = evaluations[index];
        samples.push(SplineSample {
            position: evaluation.position,
            tangent: evaluation.tangent,
            normal: frame.normal,
            binormal: frame.binormal,
            rotation: frame.rotation,
            distance,
            normalized: if total_length <= DISTANCE_EPSILON {
                0.0
            } else {
                distance / total_length
            },
            roll_radians: evaluation.roll_radians,
            width: evaluation.width,
            radius: evaluation.radius,
            scale: evaluation.scale,
            segment_index: evaluation.segment_index,
            segment_t: evaluation.segment_t,
        });
    }

    samples
}

fn lerp_samples(
    a: SplineSample,
    b: SplineSample,
    distance: f32,
    total_length: f32,
) -> SplineSample {
    let span = (b.distance - a.distance).abs();
    let alpha = if span <= DISTANCE_EPSILON {
        0.0
    } else {
        ((distance - a.distance) / (b.distance - a.distance)).clamp(0.0, 1.0)
    };
    let rotation = a.rotation.slerp(b.rotation, alpha);
    let tangent = (rotation * Vec3::Z).normalize_or_zero();
    let normal = (rotation * Vec3::X).normalize_or_zero();
    let binormal = (rotation * Vec3::Y).normalize_or_zero();
    SplineSample {
        position: a.position.lerp(b.position, alpha),
        tangent,
        normal,
        binormal,
        rotation,
        distance,
        normalized: if total_length <= DISTANCE_EPSILON {
            0.0
        } else {
            distance / total_length
        },
        roll_radians: a.roll_radians.lerp(b.roll_radians, alpha),
        width: a.width.lerp(b.width, alpha),
        radius: a.radius.lerp(b.radius, alpha),
        scale: a.scale.lerp(b.scale, alpha),
        segment_index: a.segment_index,
        segment_t: a.segment_t.lerp(b.segment_t, alpha),
    }
}

#[cfg(test)]
#[path = "sampling_tests.rs"]
mod tests;
