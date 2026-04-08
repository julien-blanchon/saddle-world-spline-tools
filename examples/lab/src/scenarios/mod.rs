use bevy::prelude::*;
use saddle_bevy_e2e::{
    action::Action,
    actions::{assertions, inspect},
    scenario::Scenario,
};
use saddle_world_spline_tools::{SplineCache, SplineDiagnostics, SplinePath};

use crate::{
    LabControl, LabDiagnostics, LaneFocus, focus_lane, trigger_add_point, trigger_move_point,
    trigger_remove_point,
};

#[derive(Resource, Default)]
struct BeforeSnapshot {
    edit_points: usize,
    edit_revision: u64,
}

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "spline_tools_extrusion_smoke",
        "spline_tools_closed_loop_tube",
        "spline_tools_runtime_edit_smoke",
        "spline_tools_placement_smoke",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke_launch()),
        "spline_tools_extrusion_smoke" => Some(spline_tools_extrusion_smoke()),
        "spline_tools_closed_loop_tube" => Some(spline_tools_closed_loop_tube()),
        "spline_tools_runtime_edit_smoke" => Some(spline_tools_runtime_edit_smoke()),
        "spline_tools_placement_smoke" => Some(spline_tools_placement_smoke()),
        _ => None,
    }
}

fn smoke_launch() -> Scenario {
    Scenario::builder("smoke_launch")
        .description("Boot the lab and verify that all four demo lanes publish stable diagnostics.")
        .then(Action::WaitFrames(30))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "lab diagnostics populated",
            |diagnostics| {
                diagnostics.road_vertices > 0
                    && diagnostics.tube_vertices > 0
                    && diagnostics.post_count > 0
                    && diagnostics.edit_control_points >= 4
            },
        ))
        .then(Action::Screenshot("smoke_launch".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("smoke_launch"))
        .build()
}

fn spline_tools_extrusion_smoke() -> Scenario {
    Scenario::builder("spline_tools_extrusion_smoke")
        .description(
            "Focus the road and tube lanes, assert mesh output exists, and capture both views.",
        )
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world| {
            focus_lane(world, LaneFocus::Road)
        })))
        .then(Action::WaitFrames(10))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "road mesh generated",
            |diagnostics| diagnostics.road_vertices > 0 && diagnostics.road_length > 5.0,
        ))
        .then(Action::Screenshot("road_lane".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            focus_lane(world, LaneFocus::Tube)
        })))
        .then(Action::WaitFrames(10))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "tube mesh generated",
            |diagnostics| diagnostics.tube_vertices > 0 && diagnostics.tube_length > 5.0,
        ))
        .then(inspect::log_resource::<LabDiagnostics>(
            "extrusion diagnostics",
        ))
        .then(Action::Screenshot("tube_lane".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("spline_tools_extrusion_smoke"))
        .build()
}

fn spline_tools_closed_loop_tube() -> Scenario {
    Scenario::builder("spline_tools_closed_loop_tube")
        .description(
            "Focus the closed-loop tube lane, verify the seam samples stay aligned, and capture the looped sweep.",
        )
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world| focus_lane(world, LaneFocus::Tube))))
        .then(Action::WaitFrames(10))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "tube mesh generated",
            |diagnostics| diagnostics.tube_vertices > 0 && diagnostics.tube_length > 5.0,
        ))
        .then(inspect::log_component_where::<SplinePath, crate::TubeLane>(
            "tube spline path",
        ))
        .then(inspect::log_component_where::<SplineDiagnostics, crate::TubeLane>(
            "tube spline diagnostics",
        ))
        .then(Action::Custom(Box::new(|world| {
            let mut query = world.query_filtered::<
                (&SplinePath, &SplineCache, &SplineDiagnostics),
                With<crate::TubeLane>,
            >();
            let Some((path, cache, diagnostics)) = query.iter(world).next() else {
                panic!("TubeLane entity should exist");
            };

            assert!(path.curve.closed, "tube lane should be configured as a closed loop");
            assert!(diagnostics.sample_count > 0, "closed loop tube should bake samples");

            let first = cache
                .samples()
                .first()
                .expect("closed-loop cache should contain a first sample");
            let last = cache
                .samples()
                .last()
                .expect("closed-loop cache should contain a final seam sample");

            assert!(
                first.position.abs_diff_eq(last.position, 1.0e-2),
                "closed-loop tube seam positions should align"
            );
            assert!(
                first.normal.abs_diff_eq(last.normal, 5.0e-2),
                "closed-loop tube seam normals should align"
            );
            assert!(
                first.binormal.abs_diff_eq(last.binormal, 5.0e-2),
                "closed-loop tube seam binormals should align"
            );
        })))
        .then(Action::Screenshot("closed_loop_tube".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("spline_tools_closed_loop_tube"))
        .build()
}

fn spline_tools_runtime_edit_smoke() -> Scenario {
    Scenario::builder("spline_tools_runtime_edit_smoke")
        .description("Drive add, move, and remove operations through the editable spline lane and verify the revision and point count change.")
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world| focus_lane(world, LaneFocus::Edit))))
        .then(Action::WaitFrames(10))
        .then(Action::Screenshot("edit_before".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            world.insert_resource(BeforeSnapshot {
                edit_points: diagnostics.edit_control_points,
                edit_revision: diagnostics.edit_curve_revision,
            });
        })))
        .then(Action::Custom(Box::new(trigger_add_point)))
        .then(Action::WaitUntil {
            label: "editable point added".into(),
            condition: Box::new(|world| {
                let before = world.resource::<BeforeSnapshot>();
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.edit_control_points > before.edit_points
                    && diagnostics.edit_curve_revision > before.edit_revision
            }),
            max_frames: 120,
        })
        .then(Action::Screenshot("edit_added".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(trigger_move_point)))
        .then(Action::WaitUntil {
            label: "editable spline moved".into(),
            condition: Box::new(|world| {
                let before = world.resource::<BeforeSnapshot>();
                world.resource::<LabDiagnostics>().edit_curve_revision > before.edit_revision + 1
            }),
            max_frames: 120,
        })
        .then(Action::Custom(Box::new(trigger_remove_point)))
        .then(Action::WaitUntil {
            label: "editable point removed".into(),
            condition: Box::new(|world| {
                let before = world.resource::<BeforeSnapshot>();
                world.resource::<LabDiagnostics>().edit_control_points == before.edit_points
            }),
            max_frames: 120,
        })
        .then(Action::Screenshot("edit_after".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "edit mesh updated",
            |diagnostics| diagnostics.edit_mesh_revision > 0,
        ))
        .then(assertions::log_summary("spline_tools_runtime_edit_smoke"))
        .build()
}

fn spline_tools_placement_smoke() -> Scenario {
    Scenario::builder("spline_tools_placement_smoke")
        .description("Focus the placement lane, verify evenly spaced posts exist, and capture the lane view.")
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world| focus_lane(world, LaneFocus::Placement))))
        .then(Action::WaitFrames(10))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "placement posts spawned",
            |diagnostics| diagnostics.post_count >= 8,
        ))
        .then(inspect::log_resource::<LabControl>("placement control"))
        .then(Action::Screenshot("placement_lane".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("spline_tools_placement_smoke"))
        .build()
}
