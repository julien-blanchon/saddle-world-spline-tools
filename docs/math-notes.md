# Math Notes

This document records the formulas and numerical choices used in `saddle-world-spline-tools`.

## Bezier Chains

Bezier segments are authored as anchor pairs with optional per-anchor handles.

For a segment with:

- `P0` = start anchor
- `P1` = start out-handle
- `P2` = end in-handle
- `P3` = end anchor

the position is:

```text
B(u) =
  (1 - u)^3 P0
  + 3 (1 - u)^2 u P1
  + 3 (1 - u) u^2 P2
  + u^3 P3
```

and the tangent is:

```text
B'(u) =
  3 (1 - u)^2 (P1 - P0)
  + 6 (1 - u) u (P2 - P1)
  + 3 u^2 (P3 - P2)
```

## Catmull-Rom

The crate uses the non-uniform Catmull-Rom form driven by knot parameters:

```text
t(i+1) = t(i) + |P(i+1) - P(i)|^alpha
```

where:

- `alpha = 0.0` for uniform
- `alpha = 0.5` for centripetal
- `alpha = 1.0` for chordal

Evaluation follows the recursive knot-space interpolation form:

```text
A1 = lerp(P0, P1, t0, t1, t)
A2 = lerp(P1, P2, t1, t2, t)
A3 = lerp(P2, P3, t2, t3, t)
B1 = lerp(A1, A2, t0, t2, t)
B2 = lerp(A2, A3, t1, t3, t)
C  = lerp(B1, B2, t1, t2, t)
```

with `t = t1 + u (t2 - t1)`.

The default is centripetal Catmull-Rom because it is the safest general-purpose choice for unevenly spaced points and sharp turns.

## Tangents

Bezier tangents use the analytic derivative.

Catmull-Rom tangents use a small symmetric finite difference in local segment space. This keeps the implementation small and reliable across all parameterization modes without maintaining separate symbolic derivatives for each knot-space form.

## Frame Construction

Every baked sample needs:

- tangent
- normal
- binormal
- rotation

The local frame convention is:

- local `X` = `normal`
- local `Y` = `binormal`
- local `Z` = `tangent`

That is the same convention used when sweeping cross-sections:

```text
world_position =
  sample.position
  + sample.normal   * local_x
  + sample.binormal * local_y
```

## Fixed-Up Frames

`FixedUp` projects the requested up vector onto the plane perpendicular to the tangent:

```text
N = normalize(U - dot(U, T) T)
B = normalize(T x N)
```

If the up vector becomes parallel to the tangent, the implementation falls back to a safe axis (`X` or `Y`) before projection.

## Frenet Frames

The Frenet path uses neighboring tangent differences as the curvature-derived normal estimate:

```text
N ~= normalize(T(i+1) - T(i-1))
B  = normalize(T x N)
```

If the local derivative is too small, the implementation falls back to the previous usable normal. This keeps the mode usable for debugging without pretending it is robust on near-straight sections.

## Parallel Transport

The transport mode propagates a frame by rotating the previous normal between consecutive tangents:

```text
R = rotation_arc(T(i), T(i+1))
N(i+1) = normalize(project_perpendicular(R * N(i), T(i+1)))
```

This keeps twist low and avoids the instability that pure Frenet frames show on straight or nearly straight sections.

## Rotation-Minimizing Frames

The default RMF mode uses the double-reflection update:

```text
v1 = P(i+1) - P(i)
rL = r(i) - 2 dot(v1, r(i)) / |v1|^2 * v1
tL = t(i) - 2 dot(v1, t(i)) / |v1|^2 * v1
v2 = t(i+1) - tL
r(i+1) = rL - 2 dot(v2, rL) / |v2|^2 * v2
```

where `r(i)` is the transported reference normal.

This keeps twist low and gives better sweep behavior than naive Frenet framing. Closed loops then apply one more pass: the remaining end-to-start twist is measured around the tangent axis and distributed across the loop by traveled distance.

## Arc-Length Approximation

`SplineCache` approximates arc length by summing straight-line distances between baked samples:

```text
L ~= sum |P(i+1) - P(i)|
```

`sample_distance` and `sample_normalized` then interpolate between the bracketing baked samples. This is a deliberate CPU-first approximation:

- deterministic
- cheap to update
- robust for runtime editing

Increasing `samples_per_segment` improves the approximation.

## Nearest-Point Approximation

Nearest-point queries operate on the baked polyline. Each cached segment between adjacent baked samples is tested by projecting the query point onto the segment and keeping the closest projection.

This is not a continuous root-solve on the analytic curve. The tradeoff is intentional: the query is fast, deterministic, and naturally shares the same approximation space as arc-length sampling and mesh extrusion.

## Numerical Edge Cases

- coincident control points clamp knot spacing to a small epsilon so Catmull-Rom evaluation stays finite
- zero-length tangents fall back to `Vec3::Z`
- near-parallel up vectors are reprojected with fallback axes
- closed-loop RMF correction is skipped when the residual twist is already negligible
- profile UV tiling clamps tile length to a small epsilon to avoid division by zero
