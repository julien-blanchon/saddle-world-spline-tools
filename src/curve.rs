use bevy::prelude::*;

const POSITION_EPSILON: f32 = 1.0e-4;
const TANGENT_DELTA: f32 = 1.0e-3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum SplineCurveKind {
    Bezier,
    #[default]
    CatmullRom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum CatmullRomParameterization {
    Uniform,
    #[default]
    Centripetal,
    Chordal,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CatmullRomOptions {
    pub parameterization: CatmullRomParameterization,
}

impl Default for CatmullRomOptions {
    fn default() -> Self {
        Self {
            parameterization: CatmullRomParameterization::Centripetal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct SplineControlPoint {
    pub position: Vec3,
    pub in_handle: Option<Vec3>,
    pub out_handle: Option<Vec3>,
    pub roll_radians: f32,
    pub width: f32,
    pub radius: f32,
    pub scale: Vec2,
}

impl Default for SplineControlPoint {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            in_handle: None,
            out_handle: None,
            roll_radians: 0.0,
            width: 1.0,
            radius: 0.5,
            scale: Vec2::ONE,
        }
    }
}

impl SplineControlPoint {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            ..default()
        }
    }

    pub fn with_handles(mut self, in_handle: Option<Vec3>, out_handle: Option<Vec3>) -> Self {
        self.in_handle = in_handle;
        self.out_handle = out_handle;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct SplineCurve {
    pub kind: SplineCurveKind,
    pub points: Vec<SplineControlPoint>,
    pub closed: bool,
    pub catmull_rom: CatmullRomOptions,
}

impl Default for SplineCurve {
    fn default() -> Self {
        Self {
            kind: SplineCurveKind::CatmullRom,
            points: vec![
                SplineControlPoint::new(Vec3::new(-2.0, 0.0, 0.0)),
                SplineControlPoint::new(Vec3::new(2.0, 0.0, 0.0)),
            ],
            closed: false,
            catmull_rom: CatmullRomOptions::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CurveEvaluation {
    pub position: Vec3,
    pub tangent: Vec3,
    pub roll_radians: f32,
    pub width: f32,
    pub radius: f32,
    pub scale: Vec2,
    pub segment_index: usize,
    pub segment_t: f32,
}

impl SplineCurve {
    pub fn segment_count(&self) -> usize {
        match self.points.len() {
            0 | 1 => 0,
            len if self.closed => len,
            len => len - 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn sample(&self, t: f32) -> CurveEvaluation {
        let segment_count = self.segment_count();
        if segment_count == 0 {
            return self
                .points
                .first()
                .map(|point| default_point_evaluation(*point))
                .unwrap_or_default();
        }

        let clamped = if self.closed {
            t.rem_euclid(1.0)
        } else {
            t.clamp(0.0, 1.0)
        };
        let scaled = clamped * segment_count as f32;
        let segment_index = if !self.closed && clamped >= 1.0 {
            segment_count.saturating_sub(1)
        } else {
            scaled.floor().min(segment_count.saturating_sub(1) as f32) as usize
        };
        let segment_t = if !self.closed && clamped >= 1.0 {
            1.0
        } else {
            (scaled - segment_index as f32).clamp(0.0, 1.0)
        };
        self.sample_segment(segment_index, segment_t)
    }

    pub fn sample_segment(&self, segment_index: usize, segment_t: f32) -> CurveEvaluation {
        match self.kind {
            SplineCurveKind::Bezier => self.sample_bezier_segment(segment_index, segment_t),
            SplineCurveKind::CatmullRom => {
                self.sample_catmull_rom_segment(segment_index, segment_t)
            }
        }
    }

    pub fn all_segment_indices(&self) -> Vec<usize> {
        (0..self.segment_count()).collect()
    }

    pub fn affected_segments_for_point(&self, point_index: usize) -> Vec<usize> {
        let segment_count = self.segment_count();
        if segment_count == 0 {
            return Vec::new();
        }

        let offsets: &[isize] = match self.kind {
            SplineCurveKind::Bezier => &[-1, 0],
            SplineCurveKind::CatmullRom => &[-2, -1, 0, 1],
        };

        let mut indices = Vec::new();
        for offset in offsets {
            let candidate = point_index as isize + offset;
            let maybe_index = if self.closed {
                Some(candidate.rem_euclid(segment_count as isize) as usize)
            } else if candidate >= 0 && candidate < segment_count as isize {
                Some(candidate as usize)
            } else {
                None
            };
            if let Some(index) = maybe_index {
                if !indices.contains(&index) {
                    indices.push(index);
                }
            }
        }
        indices.sort_unstable();
        indices
    }

    fn sample_bezier_segment(&self, segment_index: usize, segment_t: f32) -> CurveEvaluation {
        let point_count = self.points.len();
        if point_count == 0 {
            return CurveEvaluation::default();
        }
        if point_count == 1 {
            return default_point_evaluation(self.points[0]);
        }

        let index_a = segment_index.min(point_count - 1);
        let index_b = if self.closed {
            (segment_index + 1) % point_count
        } else {
            (segment_index + 1).min(point_count - 1)
        };

        let a = self.points[index_a];
        let b = self.points[index_b];
        let p0 = a.position;
        let p1 = a.out_handle.unwrap_or(a.position);
        let p2 = b.in_handle.unwrap_or(b.position);
        let p3 = b.position;
        let u = segment_t.clamp(0.0, 1.0);
        let one_minus = 1.0 - u;

        let position = one_minus.powi(3) * p0
            + 3.0 * one_minus.powi(2) * u * p1
            + 3.0 * one_minus * u * u * p2
            + u.powi(3) * p3;

        let tangent = (3.0 * one_minus.powi(2) * (p1 - p0)
            + 6.0 * one_minus * u * (p2 - p1)
            + 3.0 * u.powi(2) * (p3 - p2))
            .normalize_or_zero();

        interpolate_anchor_metadata(a, b, position, tangent, segment_index, u)
    }

    fn sample_catmull_rom_segment(&self, segment_index: usize, segment_t: f32) -> CurveEvaluation {
        let point_count = self.points.len();
        if point_count == 0 {
            return CurveEvaluation::default();
        }
        if point_count == 1 {
            return default_point_evaluation(self.points[0]);
        }

        let anchor_a = self.point_at(segment_index);
        let anchor_b = self.point_at(segment_index + 1);
        let p0 = self.extended_position(segment_index as isize - 1);
        let p1 = anchor_a.position;
        let p2 = anchor_b.position;
        let p3 = self.extended_position(segment_index as isize + 2);

        let position = catmull_rom_position(
            p0,
            p1,
            p2,
            p3,
            segment_t.clamp(0.0, 1.0),
            self.catmull_rom.parameterization,
        );
        let tangent = catmull_rom_tangent(
            p0,
            p1,
            p2,
            p3,
            segment_t.clamp(0.0, 1.0),
            self.catmull_rom.parameterization,
        );

        interpolate_anchor_metadata(
            anchor_a,
            anchor_b,
            position,
            tangent,
            segment_index,
            segment_t.clamp(0.0, 1.0),
        )
    }

    fn point_at(&self, index: usize) -> SplineControlPoint {
        if self.closed {
            self.points[index % self.points.len()]
        } else {
            self.points[index.min(self.points.len() - 1)]
        }
    }

    fn extended_position(&self, index: isize) -> Vec3 {
        if self.closed {
            let len = self.points.len() as isize;
            return self.points[index.rem_euclid(len) as usize].position;
        }

        if index < 0 {
            let first = self.points[0].position;
            let next = self.points.get(1).map_or(first, |point| point.position);
            first + (first - next)
        } else if index as usize >= self.points.len() {
            let last = self
                .points
                .last()
                .map_or(Vec3::ZERO, |point| point.position);
            let previous = self
                .points
                .get(self.points.len().saturating_sub(2))
                .map_or(last, |point| point.position);
            last + (last - previous)
        } else {
            self.points[index as usize].position
        }
    }
}

fn default_point_evaluation(point: SplineControlPoint) -> CurveEvaluation {
    CurveEvaluation {
        position: point.position,
        tangent: Vec3::Z,
        roll_radians: point.roll_radians,
        width: point.width,
        radius: point.radius,
        scale: point.scale,
        segment_index: 0,
        segment_t: 0.0,
    }
}

fn interpolate_anchor_metadata(
    a: SplineControlPoint,
    b: SplineControlPoint,
    position: Vec3,
    tangent: Vec3,
    segment_index: usize,
    segment_t: f32,
) -> CurveEvaluation {
    CurveEvaluation {
        position,
        tangent: tangent.normalize_or_zero(),
        roll_radians: a.roll_radians.lerp(b.roll_radians, segment_t),
        width: a.width.lerp(b.width, segment_t),
        radius: a.radius.lerp(b.radius, segment_t),
        scale: a.scale.lerp(b.scale, segment_t),
        segment_index,
        segment_t,
    }
}

fn catmull_rom_position(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    u: f32,
    parameterization: CatmullRomParameterization,
) -> Vec3 {
    let alpha = parameterization_alpha(parameterization);
    let t0 = 0.0;
    let t1 = next_knot(t0, p0, p1, alpha);
    let t2 = next_knot(t1, p1, p2, alpha);
    let t3 = next_knot(t2, p2, p3, alpha);
    let t = t1 + (t2 - t1) * u;

    let a1 = knot_lerp(p0, p1, t0, t1, t);
    let a2 = knot_lerp(p1, p2, t1, t2, t);
    let a3 = knot_lerp(p2, p3, t2, t3, t);
    let b1 = knot_lerp(a1, a2, t0, t2, t);
    let b2 = knot_lerp(a2, a3, t1, t3, t);
    knot_lerp(b1, b2, t1, t2, t)
}

fn catmull_rom_tangent(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    u: f32,
    parameterization: CatmullRomParameterization,
) -> Vec3 {
    let lower = (u - TANGENT_DELTA).max(0.0);
    let upper = (u + TANGENT_DELTA).min(1.0);
    let before = catmull_rom_position(p0, p1, p2, p3, lower, parameterization);
    let after = catmull_rom_position(p0, p1, p2, p3, upper, parameterization);
    (after - before).normalize_or_zero()
}

fn parameterization_alpha(parameterization: CatmullRomParameterization) -> f32 {
    match parameterization {
        CatmullRomParameterization::Uniform => 0.0,
        CatmullRomParameterization::Centripetal => 0.5,
        CatmullRomParameterization::Chordal => 1.0,
    }
}

fn next_knot(previous: f32, a: Vec3, b: Vec3, alpha: f32) -> f32 {
    if alpha == 0.0 {
        previous + 1.0
    } else {
        previous + a.distance(b).max(POSITION_EPSILON).powf(alpha)
    }
}

fn knot_lerp(a: Vec3, b: Vec3, t0: f32, t1: f32, t: f32) -> Vec3 {
    let span = (t1 - t0).abs();
    if span <= POSITION_EPSILON {
        b
    } else {
        let alpha = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
        a.lerp(b, alpha)
    }
}

#[cfg(test)]
#[path = "curve_tests.rs"]
mod tests;
