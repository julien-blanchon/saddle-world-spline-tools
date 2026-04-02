# Saddle World Spline Tools

Reusable spline and path toolkit for Bevy. The crate is built for 3D authoring workflows such as roads, rivers, rails, cables, fences, camera rails, placement guides, and procedural sweep meshes.

The runtime stays project-agnostic. It depends on `bevy` only, exposes injectable schedules, and keeps the math layer available outside ECS so consumers can evaluate curves, sample by arc length, query nearest points, generate stable frames, and build meshes without depending on a specific game architecture.

For examples and tools that should stay active for the whole app lifetime, `SplineToolsPlugin::always_on(Update)` is the simplest entrypoint. For game integration, prefer `SplineToolsPlugin::new(...)` and wire it to your own activate and deactivate schedules.

## Quick Start

```toml
[dependencies]
bevy = "0.18"
saddle-world-spline-tools = { git = "https://github.com/julien-blanchon/saddle-world-spline-tools" }
```

```rust,no_run
use bevy::prelude::*;
use saddle_world_spline_tools::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Running,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .init_state::<DemoState>()
        .add_plugins(SplineToolsPlugin::new(
            OnEnter(DemoState::Running),
            OnExit(DemoState::Running),
            Update,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let curve = SplineCurve {
        kind: SplineCurveKind::CatmullRom,
        points: vec![
            SplineControlPoint::new(Vec3::new(-4.0, 0.0, -3.0)),
            SplineControlPoint::new(Vec3::new(-1.0, 0.5, 1.0)),
            SplineControlPoint::new(Vec3::new(2.0, 0.0, 2.5)),
            SplineControlPoint::new(Vec3::new(4.0, 0.0, -1.0)),
        ],
        closed: false,
        ..default()
    };

    let mesh = meshes.add(Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::MAIN_WORLD
            | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ));

    commands.spawn((
        Name::new("Road Spline"),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.20, 0.24))),
        SplinePath {
            curve,
            bake: SplineBakeSettings::default(),
        },
        SplineMeshTarget::new(
            mesh,
            SplineExtrusion {
                shape: SplineExtrusionShape::Ribbon(RibbonExtrusion {
                    half_width: 1.2,
                    thickness: 0.0,
                    use_control_point_width: true,
                }),
                uv_mode: SplineUvMode::TileByWorldDistance,
                uv_tile_length: 2.0,
                cap_mode: SplineCapMode::None,
            },
        ),
        SplineDebugDraw::default(),
    ));
}
```

## Public API

| Type | Purpose |
| --- | --- |
| `SplineToolsPlugin` | Injects the ECS runtime with activate, deactivate, and update schedules |
| `SplineToolsSystems` | Public ordering hooks: `ApplyEdits`, `MarkDirty`, `RebuildCaches`, `RebuildMeshes`, `DebugDraw` |
| `SplinePath` | ECS authoring component: spline definition plus bake settings |
| `SplineMeshTarget` | ECS mesh-output component: target mesh handle plus sweep config |
| `SplineDebugDraw` | Per-entity gizmo settings for control points, frames, handles, and samples |
| `SplineDiagnostics` | Per-entity runtime diagnostics for BRP, tests, and overlays |
| `SplineEditRequest` / `SplineEditCommand` | Buffered runtime editing surface |
| `SplineRebuilt` / `SplineMeshRebuilt` | Buffered notifications after cache or mesh work completes |
| `SplineCurve`, `SplineControlPoint`, `SplineCurveKind` | Pure authoring layer for Bezier and Catmull-Rom curves |
| `SplineBakeSettings`, `SplineCache`, `SplineSample`, `SplineNearestPoint` | Arc-length cache, sample results, and pure query API |
| `FrameMode`, `SplineFrame` | Explicit framing strategy for fixed-up, Frenet, transport, and RMF-style sweeps |
| `SplineExtrusion`, `SplineExtrusionShape`, `RibbonExtrusion`, `TubeExtrusion`, `CustomExtrusion`, `CrossSection` | Mesh sweep configuration |
| `build_extrusion_buffers`, `extrusion_buffers_to_mesh` | Pure mesh-generation entrypoints |

## Supported Curve Types

- Cubic Bezier chains authored as anchor points with optional per-anchor `in_handle` and `out_handle`
- Catmull-Rom splines with `Uniform`, `Centripetal`, and `Chordal` parameterizations
- Open and closed paths
- Per-control-point roll, width, radius, and scale metadata

## Sampling

`SplineCurve` exposes raw parametric evaluation through `sample(t)`. That path is curve-domain sampling.

`SplineCache` exposes:

- `sample_normalized(...)` for approximate arc-length normalized sampling
- `sample_distance(...)` for approximate world-distance sampling
- `nearest_point(...)` for polyline-backed nearest-point queries
- `sample_evenly_spaced(...)` and `sample_evenly_spaced_transforms(...)` for repeated placement workflows

`SplineSample` contains:

- `position`
- `tangent`
- `normal`
- `binormal`
- `rotation`
- `distance`
- `normalized`
- interpolated roll, width, radius, and scale metadata

## Extrusion

`SplineExtrusionShape` supports:

- `Ribbon` for roads, rivers, rails, belts, and strips
- `Tube` for cables, pipes, hoses, and loops
- `Custom` for arbitrary 2D cross-sections swept in the path frame

Current UV modes:

- `Stretch`
- `TileByWorldDistance`
- `TilePerSegment`

Current cap modes:

- `None`
- `Fill`

The default framing strategy is `FrameMode::RotationMinimizing { up_hint: Vec3::Y }`. Frenet framing is still available for debugging or for cases where curvature-derived normals are the explicit goal.

## Runtime Editing

The ECS runtime is built around local entity updates instead of world-wide rebuilds.

- Edits go through `SplineEditRequest`
- the runtime marks only affected segments dirty for message-driven control-point edits
- curve caches rebuild from dirty segments
- mesh output is rebuilt only for the spline entity whose cache changed

Direct external mutation of `SplinePath` still works, but that path conservatively marks the whole spline dirty because the runtime cannot infer a narrow diff safely from an arbitrary component write.

## Examples

Set `SPLINE_TOOLS_EXIT_AFTER_SECONDS=3` to make long-running examples auto-exit during batch verification.

| Example | Purpose | Run |
| --- | --- | --- |
| `basic` | Minimal cache, gizmo, and moving-sample preview | `cargo run -p saddle-world-spline-tools-example-basic` |
| `extrusion_road` | Ribbon road with world-length UV tiling and roll | `cargo run -p saddle-world-spline-tools-example-extrusion-road` |
| `placement_along_path` | Equal-distance object placement with stable orientation | `cargo run -p saddle-world-spline-tools-example-placement-along-path` |
| `closed_loop_tube` | Closed tubular sweep stressing seam closure and RMF continuity | `cargo run -p saddle-world-spline-tools-example-closed-loop-tube` |
| `runtime_editing` | Add, move, and remove control points while the runtime rebuilds the path | `cargo run -p saddle-world-spline-tools-example-runtime-editing` |
| `lab` | Crate-local BRP and E2E verification app | `cargo run -p saddle-world-spline-tools-lab` |

## Documentation

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [Math Notes](docs/math-notes.md)

## Current Tradeoffs

- The runtime cache invalidates only the edited spline entity, not the whole world, but mesh rebuilds still regenerate the full mesh for that spline entity in v0.1.
- Custom cross-section caps use a simple center-fan fill and therefore fit convex or star-convex profiles best.
- Nearest-point queries use the baked polyline cache rather than iterative root finding; they are fast and robust but approximate.
