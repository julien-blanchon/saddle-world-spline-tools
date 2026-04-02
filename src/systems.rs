use std::collections::BTreeSet;

use bevy::prelude::*;

use crate::{
    SplineCache, SplineCurveKind, SplineDiagnostics, SplineEditCommand, SplineEditRequest,
    SplineMeshRebuilt, SplineMeshTarget, SplinePath, SplineRebuilt, build_extrusion_buffers,
    extrusion_buffers_to_mesh,
};

#[derive(Default, Resource)]
pub(crate) struct SplineToolsRuntimeState {
    pub active: bool,
}

#[derive(Component, Debug, Default)]
pub(crate) struct SplineDirtyState {
    pub dirty_segments: BTreeSet<usize>,
    pub cache_dirty: bool,
    pub mesh_dirty: bool,
    pub had_message_edit: bool,
}

impl SplineDirtyState {
    fn mark_all(&mut self, path: &SplinePath) {
        self.dirty_segments = path.curve.all_segment_indices().into_iter().collect();
        self.cache_dirty = true;
        self.mesh_dirty = true;
    }
}

pub(crate) fn activate_runtime(mut state: ResMut<SplineToolsRuntimeState>) {
    state.active = true;
}

pub(crate) fn deactivate_runtime(mut state: ResMut<SplineToolsRuntimeState>) {
    state.active = false;
}

pub(crate) fn runtime_is_active(state: Option<Res<SplineToolsRuntimeState>>) -> bool {
    state.is_some_and(|state| state.active)
}

pub(crate) fn ensure_runtime_components(
    mut commands: Commands,
    query: Query<(Entity, &SplinePath), Added<SplinePath>>,
) {
    for (entity, path) in &query {
        let mut dirty = SplineDirtyState::default();
        dirty.mark_all(path);
        commands.entity(entity).insert((
            SplineCache::default(),
            SplineDiagnostics {
                control_point_count: path.curve.points.len(),
                segment_count: path.curve.segment_count(),
                dirty_segment_count: path.curve.segment_count(),
                ..default()
            },
            dirty,
        ));
    }
}

pub(crate) fn apply_edit_requests(
    mut requests: MessageReader<SplineEditRequest>,
    mut query: Query<(
        &mut SplinePath,
        &mut SplineDirtyState,
        &mut SplineDiagnostics,
    )>,
) {
    for request in requests.read() {
        let Ok((mut path, mut dirty, mut diagnostics)) = query.get_mut(request.entity) else {
            continue;
        };

        let affected = apply_edit_command(&mut path, &request.command);
        if !affected.is_empty() {
            dirty.dirty_segments.extend(affected);
            dirty.cache_dirty = true;
            dirty.mesh_dirty = true;
            dirty.had_message_edit = true;
            diagnostics.control_point_count = path.curve.points.len();
            diagnostics.dirty_segment_count = dirty.dirty_segments.len();
        }
    }
}

pub(crate) fn mark_paths_dirty_from_changes(
    mut query: Query<
        (&SplinePath, &mut SplineDirtyState, &mut SplineDiagnostics),
        Changed<SplinePath>,
    >,
) {
    for (path, mut dirty, mut diagnostics) in &mut query {
        if dirty.had_message_edit {
            dirty.had_message_edit = false;
            diagnostics.control_point_count = path.curve.points.len();
            diagnostics.segment_count = path.curve.segment_count();
            diagnostics.dirty_segment_count = dirty.dirty_segments.len();
            continue;
        }
        dirty.mark_all(path);
        diagnostics.control_point_count = path.curve.points.len();
        diagnostics.segment_count = path.curve.segment_count();
        diagnostics.dirty_segment_count = dirty.dirty_segments.len();
    }
}

pub(crate) fn mark_mesh_targets_dirty_from_changes(
    mut query: Query<&mut SplineDirtyState, (Changed<SplineMeshTarget>, With<SplinePath>)>,
) {
    for mut dirty in &mut query {
        dirty.mesh_dirty = true;
    }
}

pub(crate) fn rebuild_dirty_caches(
    mut query: Query<(
        Entity,
        &SplinePath,
        &mut SplineCache,
        &mut SplineDirtyState,
        &mut SplineDiagnostics,
    )>,
    mut rebuilt: MessageWriter<SplineRebuilt>,
) {
    for (entity, path, mut cache, mut dirty, mut diagnostics) in &mut query {
        if !dirty.cache_dirty {
            continue;
        }

        cache.rebuild(&path.curve, &path.bake, &dirty.dirty_segments);
        diagnostics.curve_revision = cache.revision;
        diagnostics.control_point_count = path.curve.points.len();
        diagnostics.segment_count = cache.segment_count;
        diagnostics.sample_count = cache.sample_count;
        diagnostics.total_length = cache.total_length;
        diagnostics.dirty_segment_count = dirty.dirty_segments.len();
        dirty.cache_dirty = false;
        dirty.mesh_dirty = true;
        dirty.dirty_segments.clear();

        rebuilt.write(SplineRebuilt {
            entity,
            curve_revision: cache.revision,
            total_length: cache.total_length,
            sample_count: cache.sample_count,
        });
    }
}

pub(crate) fn rebuild_dirty_meshes(
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(
        Entity,
        &SplineCache,
        &SplineMeshTarget,
        &mut SplineDirtyState,
        &mut SplineDiagnostics,
    )>,
    mut rebuilt: MessageWriter<SplineMeshRebuilt>,
) {
    for (entity, cache, mesh_target, mut dirty, mut diagnostics) in &mut query {
        if !dirty.mesh_dirty {
            continue;
        }

        let buffers = build_extrusion_buffers(cache, &mesh_target.extrusion);
        let vertex_count = buffers.positions.len();
        let index_count = buffers.indices.len();
        let mesh = extrusion_buffers_to_mesh(&buffers);
        if let Some(existing) = meshes.get_mut(&mesh_target.mesh) {
            *existing = mesh;
        } else {
            let _ = meshes.insert(mesh_target.mesh.id(), mesh);
        }

        diagnostics.mesh_revision = diagnostics.mesh_revision.saturating_add(1);
        diagnostics.last_vertex_count = vertex_count;
        diagnostics.last_index_count = index_count;
        dirty.mesh_dirty = false;

        rebuilt.write(SplineMeshRebuilt {
            entity,
            mesh_revision: diagnostics.mesh_revision,
            vertex_count,
            index_count,
        });
    }
}

fn apply_edit_command(path: &mut SplinePath, command: &SplineEditCommand) -> Vec<usize> {
    match command {
        SplineEditCommand::AddPoint { index, point } => {
            let insert_index = (*index).min(path.curve.points.len());
            path.curve.points.insert(insert_index, *point);
            path.curve.affected_segments_for_point(insert_index)
        }
        SplineEditCommand::RemovePoint { index } => {
            if *index < path.curve.points.len() && path.curve.points.len() > 2 {
                path.curve.points.remove(*index);
                path.curve
                    .affected_segments_for_point(index.saturating_sub(1))
            } else {
                Vec::new()
            }
        }
        SplineEditCommand::SetPoint { index, point } => {
            if let Some(target) = path.curve.points.get_mut(*index) {
                *target = *point;
                path.curve.affected_segments_for_point(*index)
            } else {
                Vec::new()
            }
        }
        SplineEditCommand::MovePoint { index, position } => {
            if let Some(target) = path.curve.points.get_mut(*index) {
                target.position = *position;
                path.curve.affected_segments_for_point(*index)
            } else {
                Vec::new()
            }
        }
        SplineEditCommand::SetBezierHandles {
            index,
            in_handle,
            out_handle,
        } => {
            if let Some(target) = path.curve.points.get_mut(*index) {
                target.in_handle = *in_handle;
                target.out_handle = *out_handle;
                path.curve.affected_segments_for_point(*index)
            } else {
                Vec::new()
            }
        }
        SplineEditCommand::SetRoll {
            index,
            roll_radians,
        } => {
            if let Some(target) = path.curve.points.get_mut(*index) {
                target.roll_radians = *roll_radians;
                path.curve.affected_segments_for_point(*index)
            } else {
                Vec::new()
            }
        }
        SplineEditCommand::SetWidth { index, width } => {
            if let Some(target) = path.curve.points.get_mut(*index) {
                target.width = *width;
                path.curve.affected_segments_for_point(*index)
            } else {
                Vec::new()
            }
        }
        SplineEditCommand::SetRadius { index, radius } => {
            if let Some(target) = path.curve.points.get_mut(*index) {
                target.radius = *radius;
                path.curve.affected_segments_for_point(*index)
            } else {
                Vec::new()
            }
        }
        SplineEditCommand::SetScale { index, scale } => {
            if let Some(target) = path.curve.points.get_mut(*index) {
                target.scale = *scale;
                path.curve.affected_segments_for_point(*index)
            } else {
                Vec::new()
            }
        }
        SplineEditCommand::SetClosed { closed } => {
            if path.curve.closed != *closed {
                path.curve.closed = *closed;
                path.curve.all_segment_indices()
            } else {
                Vec::new()
            }
        }
        SplineEditCommand::SetCurveKind { kind } => {
            if path.curve.kind != *kind {
                path.curve.kind = *kind;
                if *kind == SplineCurveKind::CatmullRom {
                    for point in &mut path.curve.points {
                        point.in_handle = None;
                        point.out_handle = None;
                    }
                }
                path.curve.all_segment_indices()
            } else {
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
