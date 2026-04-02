use bevy::prelude::*;

use crate::{SplineCache, SplineSample};

const UV_EPSILON: f32 = 1.0e-5;

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct CrossSection {
    pub points: Vec<Vec2>,
    pub closed: bool,
}

impl CrossSection {
    pub fn line() -> Self {
        Self {
            points: vec![Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0)],
            closed: false,
        }
    }

    pub fn rectangle(width: f32, height: f32) -> Self {
        let half_width = width * 0.5;
        let half_height = height * 0.5;
        Self {
            points: vec![
                Vec2::new(-half_width, -half_height),
                Vec2::new(half_width, -half_height),
                Vec2::new(half_width, half_height),
                Vec2::new(-half_width, half_height),
            ],
            closed: true,
        }
    }

    pub fn regular_polygon(radius: f32, sides: usize) -> Self {
        let sides = sides.max(3);
        let points = (0..sides)
            .map(|index| {
                let angle = index as f32 / sides as f32 * std::f32::consts::TAU;
                Vec2::new(angle.cos() * radius, angle.sin() * radius)
            })
            .collect();
        Self {
            points,
            closed: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct RibbonExtrusion {
    pub half_width: f32,
    pub thickness: f32,
    pub use_control_point_width: bool,
}

impl Default for RibbonExtrusion {
    fn default() -> Self {
        Self {
            half_width: 1.0,
            thickness: 0.0,
            use_control_point_width: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct TubeExtrusion {
    pub radius: f32,
    pub radial_segments: usize,
    pub use_control_point_radius: bool,
}

impl Default for TubeExtrusion {
    fn default() -> Self {
        Self {
            radius: 0.5,
            radial_segments: 12,
            use_control_point_radius: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct CustomExtrusion {
    pub cross_section: CrossSection,
    pub scale: Vec2,
    pub use_control_point_scale: bool,
}

impl Default for CustomExtrusion {
    fn default() -> Self {
        Self {
            cross_section: CrossSection::rectangle(1.0, 0.4),
            scale: Vec2::ONE,
            use_control_point_scale: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum SplineUvMode {
    Stretch,
    #[default]
    TileByWorldDistance,
    TilePerSegment,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum SplineCapMode {
    #[default]
    None,
    Fill,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum SplineExtrusionShape {
    Ribbon(RibbonExtrusion),
    Tube(TubeExtrusion),
    Custom(CustomExtrusion),
}

impl Default for SplineExtrusionShape {
    fn default() -> Self {
        Self::Ribbon(RibbonExtrusion::default())
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct SplineExtrusion {
    pub shape: SplineExtrusionShape,
    pub uv_mode: SplineUvMode,
    pub uv_tile_length: f32,
    pub cap_mode: SplineCapMode,
}

impl Default for SplineExtrusion {
    fn default() -> Self {
        Self {
            shape: SplineExtrusionShape::default(),
            uv_mode: SplineUvMode::TileByWorldDistance,
            uv_tile_length: 1.0,
            cap_mode: SplineCapMode::None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtrusionBuffers {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

pub fn build_extrusion_buffers(
    cache: &SplineCache,
    extrusion: &SplineExtrusion,
) -> ExtrusionBuffers {
    if cache.samples().len() < 2 {
        return ExtrusionBuffers::default();
    }

    let (cross_section, close_profile) = cross_section_for_shape(&extrusion.shape);
    if cross_section.points.len() < 2 {
        return ExtrusionBuffers::default();
    }

    build_sweep_buffers(cache.samples(), extrusion, &cross_section, close_profile)
}

fn cross_section_for_shape(shape: &SplineExtrusionShape) -> (CrossSection, bool) {
    match shape {
        SplineExtrusionShape::Ribbon(config) => {
            if config.thickness <= UV_EPSILON {
                (
                    CrossSection {
                        points: vec![Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0)],
                        closed: false,
                    },
                    false,
                )
            } else {
                (unit_rectangle_cross_section(), true)
            }
        }
        SplineExtrusionShape::Tube(config) => (
            CrossSection::regular_polygon(1.0, config.radial_segments),
            true,
        ),
        SplineExtrusionShape::Custom(config) => {
            (config.cross_section.clone(), config.cross_section.closed)
        }
    }
}

fn build_sweep_buffers(
    samples: &[SplineSample],
    extrusion: &SplineExtrusion,
    cross_section: &CrossSection,
    close_profile: bool,
) -> ExtrusionBuffers {
    let ring_size = cross_section.points.len();
    let ring_count = samples.len();
    let profile_distances = cross_section_distances(cross_section);
    let max_profile_distance = profile_distances
        .last()
        .copied()
        .unwrap_or(1.0)
        .max(UV_EPSILON);

    let mut positions = Vec::with_capacity(ring_count * ring_size);
    let mut uvs = Vec::with_capacity(ring_count * ring_size);
    for sample in samples {
        let (profile_scale, profile_radius) = profile_scale_for_sample(*sample, &extrusion.shape);
        for (profile_index, point) in cross_section.points.iter().enumerate() {
            let local = Vec2::new(
                point.x * profile_scale.x * profile_radius,
                point.y * profile_scale.y * profile_radius,
            );
            let world = sample.position + sample.normal * local.x + sample.binormal * local.y;
            positions.push(world.to_array());
            let v = match extrusion.uv_mode {
                SplineUvMode::Stretch => sample.normalized,
                SplineUvMode::TileByWorldDistance => {
                    sample.distance / extrusion.uv_tile_length.max(UV_EPSILON)
                }
                SplineUvMode::TilePerSegment => sample.segment_index as f32 + sample.segment_t,
            };
            let u = profile_distances[profile_index] / max_profile_distance;
            uvs.push([u, v]);
        }
    }

    let mut indices = Vec::new();
    let profile_edges = if close_profile {
        ring_size
    } else {
        ring_size.saturating_sub(1)
    };
    let ring_spans = ring_count.saturating_sub(1);
    for ring_index in 0..ring_spans {
        let base_a = ring_index * ring_size;
        let base_b = (ring_index + 1) * ring_size;
        for edge_index in 0..profile_edges {
            let next_edge = if close_profile {
                (edge_index + 1) % ring_size
            } else {
                edge_index + 1
            };
            if next_edge >= ring_size {
                continue;
            }
            let a0 = (base_a + edge_index) as u32;
            let a1 = (base_a + next_edge) as u32;
            let b0 = (base_b + edge_index) as u32;
            let b1 = (base_b + next_edge) as u32;
            indices.extend_from_slice(&[a0, b0, a1, a1, b0, b1]);
        }
    }

    if matches!(extrusion.cap_mode, SplineCapMode::Fill) && close_profile && ring_size >= 3 {
        add_cap(
            &mut positions,
            &mut uvs,
            &mut indices,
            &samples[0],
            cross_section,
            true,
            extrusion,
        );
        add_cap(
            &mut positions,
            &mut uvs,
            &mut indices,
            samples.last().expect("samples should have an end"),
            cross_section,
            false,
            extrusion,
        );
    }

    let normals = accumulate_normals(&positions, &indices);
    ExtrusionBuffers {
        positions,
        normals,
        uvs,
        indices,
    }
}

fn add_cap(
    positions: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    sample: &SplineSample,
    cross_section: &CrossSection,
    is_start: bool,
    extrusion: &SplineExtrusion,
) {
    let base_index = positions.len() as u32;
    let (profile_scale, profile_radius) = profile_scale_for_sample(*sample, &extrusion.shape);
    positions.push(sample.position.to_array());
    uvs.push([0.5, if is_start { 0.0 } else { 1.0 }]);

    for point in &cross_section.points {
        let local = Vec2::new(
            point.x * profile_scale.x * profile_radius,
            point.y * profile_scale.y * profile_radius,
        );
        let world = sample.position + sample.normal * local.x + sample.binormal * local.y;
        positions.push(world.to_array());
        uvs.push([0.5 + point.x * 0.5, 0.5 + point.y * 0.5]);
    }

    for edge_index in 0..cross_section.points.len() {
        let next = (edge_index + 1) % cross_section.points.len();
        let current_index = base_index + edge_index as u32 + 1;
        let next_index = base_index + next as u32 + 1;
        if is_start {
            indices.extend_from_slice(&[base_index, next_index, current_index]);
        } else {
            indices.extend_from_slice(&[base_index, current_index, next_index]);
        }
    }
}

fn cross_section_distances(cross_section: &CrossSection) -> Vec<f32> {
    let mut distances = Vec::with_capacity(cross_section.points.len());
    let mut accumulated = 0.0;
    for (index, point) in cross_section.points.iter().enumerate() {
        if index > 0 {
            accumulated += point.distance(cross_section.points[index - 1]);
        }
        distances.push(accumulated);
    }
    distances
}

fn profile_scale_for_sample(sample: SplineSample, shape: &SplineExtrusionShape) -> (Vec2, f32) {
    match shape {
        SplineExtrusionShape::Ribbon(config) => {
            let width_scale = if config.use_control_point_width {
                sample.width
            } else {
                1.0
            };
            let height = if config.thickness <= UV_EPSILON {
                1.0
            } else {
                config.thickness
            };
            (Vec2::new(config.half_width * width_scale, height), 1.0)
        }
        SplineExtrusionShape::Tube(config) => {
            let radius = if config.use_control_point_radius {
                config.radius * sample.radius
            } else {
                config.radius
            };
            (Vec2::ONE, radius)
        }
        SplineExtrusionShape::Custom(config) => {
            let scale = if config.use_control_point_scale {
                sample.scale * config.scale
            } else {
                config.scale
            };
            (scale, 1.0)
        }
    }
}

fn unit_rectangle_cross_section() -> CrossSection {
    CrossSection {
        points: vec![
            Vec2::new(-1.0, -0.5),
            Vec2::new(1.0, -0.5),
            Vec2::new(1.0, 0.5),
            Vec2::new(-1.0, 0.5),
        ],
        closed: true,
    }
}

fn accumulate_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let [ia, ib, ic] = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let a = Vec3::from_array(positions[ia]);
        let b = Vec3::from_array(positions[ib]);
        let c = Vec3::from_array(positions[ic]);
        let face = (b - a).cross(c - a).normalize_or_zero();
        normals[ia] += face;
        normals[ib] += face;
        normals[ic] += face;
    }
    normals
        .into_iter()
        .map(|normal| normal.normalize_or_zero().to_array())
        .collect()
}

#[cfg(test)]
#[path = "extrusion_tests.rs"]
mod tests;
