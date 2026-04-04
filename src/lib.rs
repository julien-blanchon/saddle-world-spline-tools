#![doc = include_str!("../README.md")]

pub mod components;
pub mod curve;
pub mod extrusion;
pub mod frame;
pub mod gizmos;
pub mod mesh;
pub mod sampling;
mod systems;
pub mod terrain;

pub use components::{
    SplineDebugDraw, SplineDiagnostics, SplineEditCommand, SplineEditRequest, SplineMeshRebuilt,
    SplineMeshTarget, SplinePath, SplineRebuilt,
};
pub use curve::{
    CatmullRomOptions, CatmullRomParameterization, CurveEvaluation, SplineControlPoint,
    SplineCurve, SplineCurveKind,
};
pub use extrusion::{
    CrossSection, CustomExtrusion, ExtrusionBuffers, RibbonExtrusion, SplineCapMode,
    SplineExtrusion, SplineExtrusionShape, SplineUvMode, TubeExtrusion, build_extrusion_buffers,
    build_extrusion_buffers_from_samples,
};
pub use frame::{FrameMode, SplineFrame};
pub use mesh::extrusion_buffers_to_mesh;
pub use sampling::{SplineBakeSettings, SplineCache, SplineNearestPoint, SplineSample};
pub use terrain::{TerrainProjectionSettings, project_samples_onto_surface};

use bevy::{
    app::PostStartup,
    asset::AssetApp,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SplineToolsSystems {
    ApplyEdits,
    MarkDirty,
    RebuildCaches,
    RebuildMeshes,
    DebugDraw,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct SplineToolsPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl SplineToolsPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Default for SplineToolsPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for SplineToolsPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        app.init_asset::<Mesh>()
            .init_resource::<systems::SplineToolsRuntimeState>()
            .add_message::<SplineEditRequest>()
            .add_message::<SplineRebuilt>()
            .add_message::<SplineMeshRebuilt>()
            .register_type::<CatmullRomOptions>()
            .register_type::<CatmullRomParameterization>()
            .register_type::<CrossSection>()
            .register_type::<CustomExtrusion>()
            .register_type::<FrameMode>()
            .register_type::<RibbonExtrusion>()
            .register_type::<SplineBakeSettings>()
            .register_type::<SplineCapMode>()
            .register_type::<SplineControlPoint>()
            .register_type::<SplineCurve>()
            .register_type::<SplineCurveKind>()
            .register_type::<SplineDebugDraw>()
            .register_type::<SplineDiagnostics>()
            .register_type::<SplineEditCommand>()
            .register_type::<SplineEditRequest>()
            .register_type::<SplineExtrusion>()
            .register_type::<SplineExtrusionShape>()
            .register_type::<SplineMeshRebuilt>()
            .register_type::<SplineMeshTarget>()
            .register_type::<SplinePath>()
            .register_type::<SplineRebuilt>()
            .register_type::<SplineSample>()
            .register_type::<SplineNearestPoint>()
            .register_type::<TerrainProjectionSettings>()
            .register_type::<SplineUvMode>()
            .register_type::<TubeExtrusion>()
            .add_systems(self.activate_schedule, systems::activate_runtime)
            .add_systems(self.deactivate_schedule, systems::deactivate_runtime)
            .configure_sets(
                self.update_schedule,
                (
                    SplineToolsSystems::ApplyEdits,
                    SplineToolsSystems::MarkDirty,
                    SplineToolsSystems::RebuildCaches,
                    SplineToolsSystems::RebuildMeshes,
                    SplineToolsSystems::DebugDraw,
                )
                    .chain(),
            )
            .add_systems(
                self.update_schedule,
                (
                    (
                        systems::ensure_runtime_components,
                        systems::apply_edit_requests,
                    )
                        .chain()
                        .in_set(SplineToolsSystems::ApplyEdits),
                    (
                        systems::mark_paths_dirty_from_changes,
                        systems::mark_mesh_targets_dirty_from_changes,
                    )
                        .chain()
                        .in_set(SplineToolsSystems::MarkDirty),
                    systems::rebuild_dirty_caches.in_set(SplineToolsSystems::RebuildCaches),
                    systems::rebuild_dirty_meshes.in_set(SplineToolsSystems::RebuildMeshes),
                )
                    .run_if(systems::runtime_is_active),
            );

        if app.is_plugin_added::<bevy::gizmos::GizmoPlugin>() {
            app.add_systems(
                self.update_schedule,
                gizmos::draw_debug_gizmos
                    .in_set(SplineToolsSystems::DebugDraw)
                    .run_if(systems::runtime_is_active),
            );
        }
    }
}
