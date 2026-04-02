use bevy::prelude::*;

use crate::{SplineCache, SplineCurveKind, SplineDebugDraw, SplinePath};

pub fn draw_debug_gizmos(
    mut gizmos: Gizmos,
    query: Query<(
        &GlobalTransform,
        &SplinePath,
        &SplineCache,
        Option<&SplineDebugDraw>,
    )>,
) {
    for (global, path, cache, debug) in &query {
        let settings = debug.cloned().unwrap_or_default();
        if !settings.enabled {
            continue;
        }

        let affine = global.affine();
        if settings.draw_curve {
            gizmos.linestrip(
                cache
                    .samples()
                    .iter()
                    .map(|sample| affine.transform_point3(sample.position)),
                settings.curve_color,
            );
        }

        if settings.draw_samples {
            for sample in cache.samples() {
                gizmos.sphere(
                    affine.transform_point3(sample.position),
                    0.05,
                    settings.sample_color,
                );
            }
        }

        if settings.draw_control_points {
            for point in &path.curve.points {
                gizmos.cross(
                    affine.transform_point3(point.position),
                    0.14,
                    settings.control_point_color,
                );
            }
        }

        if settings.draw_handles && matches!(path.curve.kind, SplineCurveKind::Bezier) {
            for point in &path.curve.points {
                let origin = affine.transform_point3(point.position);
                if let Some(in_handle) = point.in_handle {
                    let world = affine.transform_point3(in_handle);
                    gizmos.line(origin, world, settings.handle_color);
                    gizmos.sphere(world, 0.04, settings.handle_color);
                }
                if let Some(out_handle) = point.out_handle {
                    let world = affine.transform_point3(out_handle);
                    gizmos.line(origin, world, settings.handle_color);
                    gizmos.sphere(world, 0.04, settings.handle_color);
                }
            }
        }

        if settings.draw_frames {
            let stride = settings.frame_stride.max(1);
            for sample in cache.samples().iter().step_by(stride) {
                let origin = affine.transform_point3(sample.position);
                let normal = affine.transform_vector3(sample.normal).normalize_or_zero();
                let binormal = affine
                    .transform_vector3(sample.binormal)
                    .normalize_or_zero();
                gizmos.arrow(
                    origin,
                    origin + normal * settings.frame_scale,
                    settings.normal_color,
                );
                gizmos.arrow(
                    origin,
                    origin + binormal * settings.frame_scale,
                    settings.binormal_color,
                );
                if settings.draw_frame_tangent {
                    let tangent = affine.transform_vector3(sample.tangent).normalize_or_zero();
                    gizmos.arrow(
                        origin,
                        origin + tangent * settings.frame_scale,
                        settings.tangent_color,
                    );
                }
            }
        }
    }
}
