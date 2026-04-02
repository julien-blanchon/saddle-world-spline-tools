use bevy::prelude::*;

use crate::{
    SplineBakeSettings, SplineControlPoint, SplineCurve, SplineCurveKind, SplineExtrusion,
};

#[derive(Component, Clone, Debug, Default, Reflect, PartialEq)]
#[reflect(Component)]
pub struct SplinePath {
    pub curve: SplineCurve,
    pub bake: SplineBakeSettings,
}

#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct SplineMeshTarget {
    pub mesh: Handle<Mesh>,
    pub extrusion: SplineExtrusion,
}

impl SplineMeshTarget {
    pub fn new(mesh: Handle<Mesh>, extrusion: SplineExtrusion) -> Self {
        Self { mesh, extrusion }
    }
}

#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct SplineDebugDraw {
    pub enabled: bool,
    pub draw_curve: bool,
    pub draw_control_points: bool,
    pub draw_handles: bool,
    pub draw_samples: bool,
    pub draw_frames: bool,
    pub draw_frame_tangent: bool,
    pub frame_stride: usize,
    pub curve_color: Color,
    pub control_point_color: Color,
    pub handle_color: Color,
    pub sample_color: Color,
    pub tangent_color: Color,
    pub normal_color: Color,
    pub binormal_color: Color,
    pub frame_scale: f32,
}

impl Default for SplineDebugDraw {
    fn default() -> Self {
        Self {
            enabled: true,
            draw_curve: true,
            draw_control_points: true,
            draw_handles: true,
            draw_samples: false,
            draw_frames: true,
            draw_frame_tangent: true,
            frame_stride: 4,
            curve_color: Color::srgb(0.95, 0.94, 0.88),
            control_point_color: Color::srgb(0.27, 0.84, 0.56),
            handle_color: Color::srgb(0.98, 0.64, 0.19),
            sample_color: Color::srgb(0.27, 0.62, 0.98),
            tangent_color: Color::srgb(0.98, 0.51, 0.16),
            normal_color: Color::srgb(0.36, 0.86, 0.62),
            binormal_color: Color::srgb(0.28, 0.52, 0.97),
            frame_scale: 0.45,
        }
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct SplineDiagnostics {
    pub curve_revision: u64,
    pub mesh_revision: u64,
    pub control_point_count: usize,
    pub segment_count: usize,
    pub sample_count: usize,
    pub total_length: f32,
    pub dirty_segment_count: usize,
    pub last_vertex_count: usize,
    pub last_index_count: usize,
}

#[derive(Message, Clone, Debug, Reflect)]
pub struct SplineEditRequest {
    pub entity: Entity,
    pub command: SplineEditCommand,
}

#[derive(Clone, Debug, Reflect)]
pub enum SplineEditCommand {
    AddPoint {
        index: usize,
        point: SplineControlPoint,
    },
    RemovePoint {
        index: usize,
    },
    SetPoint {
        index: usize,
        point: SplineControlPoint,
    },
    MovePoint {
        index: usize,
        position: Vec3,
    },
    SetBezierHandles {
        index: usize,
        in_handle: Option<Vec3>,
        out_handle: Option<Vec3>,
    },
    SetRoll {
        index: usize,
        roll_radians: f32,
    },
    SetWidth {
        index: usize,
        width: f32,
    },
    SetRadius {
        index: usize,
        radius: f32,
    },
    SetScale {
        index: usize,
        scale: Vec2,
    },
    SetClosed {
        closed: bool,
    },
    SetCurveKind {
        kind: SplineCurveKind,
    },
}

#[derive(Message, Clone, Debug, Reflect)]
pub struct SplineRebuilt {
    pub entity: Entity,
    pub curve_revision: u64,
    pub total_length: f32,
    pub sample_count: usize,
}

#[derive(Message, Clone, Debug, Reflect)]
pub struct SplineMeshRebuilt {
    pub entity: Entity,
    pub mesh_revision: u64,
    pub vertex_count: usize,
    pub index_count: usize,
}
