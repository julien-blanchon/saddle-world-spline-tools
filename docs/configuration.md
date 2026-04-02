# Configuration

This is the tuning reference for `saddle-world-spline-tools`.

## `SplinePath`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `curve` | `SplineCurve` | two-point Catmull-Rom | Authoring data for anchors, handles, and metadata |
| `bake` | `SplineBakeSettings` | `samples_per_segment = 24`, RMF | Controls cache density and frame strategy |

## `SplineCurve`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `kind` | `SplineCurveKind` | `CatmullRom` | `Bezier` or `CatmullRom` |
| `points` | `Vec<SplineControlPoint>` | two anchors | At least two points are useful |
| `closed` | `bool` | `false` | Wraps the spline back to its first anchor |
| `catmull_rom` | `CatmullRomOptions` | centripetal | Used only when `kind = CatmullRom` |

## `SplineControlPoint`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `position` | `Vec3` | `Vec3::ZERO` | Local-space anchor position |
| `in_handle` | `Option<Vec3>` | `None` | Used only by Bezier |
| `out_handle` | `Option<Vec3>` | `None` | Used only by Bezier |
| `roll_radians` | `f32` | `0.0` | Additional roll around the tangent |
| `width` | `f32` | `1.0` | Ribbon-scale metadata |
| `radius` | `f32` | `0.5` | Tube-scale metadata |
| `scale` | `Vec2` | `Vec2::ONE` | Custom cross-section scale metadata |

## `CatmullRomOptions`

| Field | Type | Default | Valid values | Notes |
| --- | --- | --- | --- | --- |
| `parameterization` | `CatmullRomParameterization` | `Centripetal` | `Uniform`, `Centripetal`, `Chordal` | `Centripetal` is the recommended production default |

## `SplineBakeSettings`

| Field | Type | Default | Typical range | Performance notes |
| --- | --- | --- | --- | --- |
| `samples_per_segment` | `usize` | `24` | `8 ..= 64` | Higher values improve arc-length accuracy, nearest-point stability, and sweep smoothness, but scale linearly with rebuild cost |
| `frame_mode` | `FrameMode` | RMF with `Vec3::Y` hint | mode-specific | Frame generation runs over every baked sample |

### Frame-mode guidance

| Mode | Use when | Cost / tradeoff |
| --- | --- | --- |
| `FixedUp` | You need a stable authored up axis and do not want twist transport | Cheapest |
| `Frenet` | You want curvature-driven normals for debugging | Can flip or become unstable on straight sections |
| `ParallelTransport` | You want low-twist placement or sweeps with simpler math | Robust and fast |
| `RotationMinimizing` | You want the best production default for roads, tubes, and loops | Slightly more work than transport, but still CPU-cheap |

## `SplineMeshTarget`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `mesh` | `Handle<Mesh>` | required | Target mesh asset to overwrite |
| `extrusion` | `SplineExtrusion` | ribbon, tiled UVs | Mesh sweep config |

## `SplineExtrusion`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `shape` | `SplineExtrusionShape` | `Ribbon` | Ribbon, tube, or custom cross-section |
| `uv_mode` | `SplineUvMode` | `TileByWorldDistance` | UV progression along the path |
| `uv_tile_length` | `f32` | `1.0` | World units per V tile when `TileByWorldDistance` is active |
| `cap_mode` | `SplineCapMode` | `None` | Flat center-fan caps for closed profiles |

## `RibbonExtrusion`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `half_width` | `f32` | `1.0` | Base half-width in local cross-plane units |
| `thickness` | `f32` | `0.0` | `0.0` gives a flat strip; positive values turn the ribbon into a closed rectangle |
| `use_control_point_width` | `bool` | `true` | Multiplies `half_width` by sampled `SplineControlPoint.width` |

## `TubeExtrusion`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `radius` | `f32` | `0.5` | Base tube radius |
| `radial_segments` | `usize` | `12` | Polygonal tube resolution; minimum practical value is `3` |
| `use_control_point_radius` | `bool` | `true` | Multiplies the base radius by sampled `SplineControlPoint.radius` |

## `CustomExtrusion`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `cross_section` | `CrossSection` | rectangle | Arbitrary 2D profile |
| `scale` | `Vec2` | `Vec2::ONE` | Global 2D profile scale |
| `use_control_point_scale` | `bool` | `true` | Multiplies the global scale by sampled `SplineControlPoint.scale` |

## `CrossSection`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `points` | `Vec<Vec2>` | empty | Profile points in local cross-plane space |
| `closed` | `bool` | `false` | `true` means the last point connects back to the first |

Helpers:

- `CrossSection::line()`
- `CrossSection::rectangle(width, height)`
- `CrossSection::regular_polygon(radius, sides)`

## `SplineDebugDraw`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `enabled` | `bool` | `true` | Master debug toggle |
| `draw_curve` | `bool` | `true` | Draws the baked spline line |
| `draw_control_points` | `bool` | `true` | Draws anchor crosses |
| `draw_handles` | `bool` | `true` | Draws Bezier handles when present |
| `draw_samples` | `bool` | `false` | Draws every baked sample point |
| `draw_frames` | `bool` | `true` | Draws normal and binormal vectors |
| `draw_frame_tangent` | `bool` | `true` | Adds tangent arrows to the frame debug |
| `frame_stride` | `usize` | `4` | Draw every Nth sample frame |
| `frame_scale` | `f32` | `0.45` | World-space gizmo length |

## Performance Notes

- The dominant cost is `samples_per_segment * segment_count`.
- Tube and dense custom sweeps scale with `path_sample_count * cross_section_point_count`.
- `TileByWorldDistance` is effectively free compared with sweep geometry.
- RMF and transport frames are inexpensive compared with generating a dense mesh.
- The runtime rebuilds only the affected spline entity, but each dirty mesh rebuild still regenerates the full mesh for that entity in v0.1.
