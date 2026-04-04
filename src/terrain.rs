use bevy::prelude::*;

use crate::SplineSample;

const PROJECTION_EPSILON: f32 = 1.0e-5;

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct TerrainProjectionSettings {
    pub vertical_offset: f32,
    pub normal_alignment: f32,
}

impl Default for TerrainProjectionSettings {
    fn default() -> Self {
        Self {
            vertical_offset: 0.05,
            normal_alignment: 1.0,
        }
    }
}

pub fn project_samples_onto_surface<Height, Normal>(
    samples: &[SplineSample],
    settings: &TerrainProjectionSettings,
    mut sample_height: Height,
    mut sample_normal: Normal,
) -> Vec<SplineSample>
where
    Height: FnMut(Vec2) -> Option<f32>,
    Normal: FnMut(Vec2) -> Option<Vec3>,
{
    let settings = settings.clone();
    samples
        .iter()
        .copied()
        .map(|sample| {
            let xz = Vec2::new(sample.position.x, sample.position.z);
            let Some(surface_height) = sample_height(xz) else {
                return sample;
            };

            let tangent = sample.tangent.normalize_or_zero();
            let surface_normal = sample_normal(xz)
                .unwrap_or(sample.normal)
                .normalize_or_zero();
            let projected_surface_normal = projected_perpendicular(surface_normal, tangent)
                .unwrap_or_else(|| {
                    projected_perpendicular(sample.normal, tangent).unwrap_or(Vec3::Y)
                });
            let blended_normal = sample
                .normal
                .normalize_or_zero()
                .lerp(
                    projected_surface_normal,
                    settings.normal_alignment.clamp(0.0, 1.0),
                )
                .normalize_or_zero();
            let binormal = tangent.cross(blended_normal).normalize_or_zero();
            let corrected_normal = binormal.cross(tangent).normalize_or_zero();
            let rotation = Quat::from_mat3(&Mat3::from_cols(corrected_normal, binormal, tangent));

            SplineSample {
                position: Vec3::new(
                    sample.position.x,
                    surface_height + settings.vertical_offset,
                    sample.position.z,
                ),
                normal: corrected_normal,
                binormal,
                rotation,
                ..sample
            }
        })
        .collect()
}

fn projected_perpendicular(vector: Vec3, tangent: Vec3) -> Option<Vec3> {
    let projected = vector - tangent * vector.dot(tangent);
    (projected.length_squared() > PROJECTION_EPSILON).then_some(projected.normalize())
}

#[cfg(test)]
#[path = "terrain_tests.rs"]
mod tests;
