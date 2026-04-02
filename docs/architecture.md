# Architecture

`saddle-world-spline-tools` is split into four layers:

1. pure curve authoring and evaluation
2. frame generation and baked sampling
3. sweep mesh generation
4. ECS orchestration for runtime editing, cache invalidation, mesh sync, and gizmos

That split keeps the crate useful outside ECS-heavy projects. Consumers can stay entirely in pure Rust when they only need path math, placement queries, or mesh generation for tools and offline preparation.

## Data Flow

```text
SplineCurve + SplineBakeSettings
        |
        v
Curve evaluation
  - Bezier anchor chains
  - Catmull-Rom with selectable parameterization
  - per-anchor roll / width / radius / scale interpolation
        |
        v
Baked sampling
  - per-segment sample tables
  - cumulative arc-length distances
  - frame generation from the sampled tangents
        |
        v
Pure queries
  - sample_normalized
  - sample_distance
  - nearest_point
  - evenly_spaced_transforms
        |
        v
Mesh generation
  - ribbon / tube / custom cross-section sweep
  - UV generation
  - optional flat end caps
  - normals / tangents
        |
        v
ECS runtime
  - apply edit requests
  - mark dirty segments
  - rebuild only dirty spline caches
  - rebuild only the affected spline meshes
  - draw optional debug gizmos
```

## Curve Model

`SplineCurve` stores one list of `SplineControlPoint`s shared by both curve families.

- Bezier uses the control-point anchor positions plus optional `in_handle` and `out_handle`.
- Catmull-Rom ignores the handles and uses the anchor positions only.
- Both families interpolate roll, width, radius, and scale metadata linearly between adjacent anchors.

The crate intentionally treats control points as local-space authoring data. In the ECS runtime that means moving the spline entity via `Transform` moves the whole path and every generated mesh or placement query with it.

## Cache Strategy

`SplineCache` stores two levels of baked data:

- per-segment `CurveEvaluation` sample tables
- one flattened `SplineSample` array with cumulative distance and full orientation frames

The runtime uses segment tables so message-driven point edits can invalidate only the local segments that depend on the edited anchor. After the local segment tables refresh, the flattened table is rebuilt from the current segment data and the frame pass runs again.

This design gives the crate a practical incremental story without making the public API depend on an editor-only diff format.

### Dirty Ranges

Dirty segment rules:

- Bezier point edits affect the previous and next segment around that anchor
- Catmull-Rom point edits affect up to four neighboring segments because each segment depends on four anchors

Direct mutation of the whole `SplinePath` component is supported, but that path conservatively marks every segment dirty because a safe local diff cannot be inferred from an arbitrary external write.

## Frame Generation

Framing is explicit and configurable through `FrameMode`.

- `FixedUp` projects a caller-chosen up vector onto each tangent frame
- `Frenet` derives normals from neighboring tangent deltas and is useful for debugging curvature-driven behavior
- `ParallelTransport` propagates an initial frame by rotating the previous frame between successive tangents
- `RotationMinimizing` uses a double-reflection transport pass and then applies twist correction on closed loops

The default is `RotationMinimizing { up_hint: Vec3::Y }` because roads, tubes, cables, and repeated placement usually need stable low-twist frames more than curvature-pure Frenet normals.

## Closed Loops

Closed loops are handled in three places:

1. segment indexing wraps around the control-point list
2. the flattened cache includes an end sample that lands back on the first point, which creates a stable mesh seam and easy UV continuity
3. the rotation-minimizing and transport frame passes distribute the residual end-to-start twist across the loop so the final frame matches the start frame

That last step matters for sweep meshes and equally spaced placement. Without the twist distribution pass, a closed loop can finish at the right position with the wrong orientation.

## Mesh Generation

Mesh generation is CPU-first and fully deterministic.

- profile points live in the local cross-plane of the spline frame
- `normal` and `binormal` span the sweep plane
- `tangent` points along the path
- UVs are generated from profile distance and traveled curve distance

The sweep layer currently favors reliable geometry over hyper-specialized authoring features:

- one ribbon mode
- one tube mode
- one generic custom cross-section mode
- optional flat end caps

That keeps the public API small while leaving room for later additions such as hard-edge splits, per-segment UV islands, or intersection helpers.

## ECS Runtime

The runtime stages are:

1. `ApplyEdits`
2. `MarkDirty`
3. `RebuildCaches`
4. `RebuildMeshes`
5. `DebugDraw`

`ApplyEdits` owns the buffered edit surface and the helper component insertion for new spline entities.

`MarkDirty` captures direct component edits and changed mesh settings.

`RebuildCaches` updates the pure cache component and emits `SplineRebuilt`.

`RebuildMeshes` updates the target `Mesh` asset and emits `SplineMeshRebuilt`.

`DebugDraw` reads only published state (`SplinePath`, `SplineCache`, `SplineDebugDraw`) so BRP and headless logic tests do not need privileged internal access.
