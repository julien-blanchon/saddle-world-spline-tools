# `spline_tools_lab`

Crate-local showcase and verification app for `spline_tools`.

## Purpose

- keep a multi-lane scene that exercises ribbon extrusion, tube sweeps, equal-distance placement, and runtime editing in one app
- expose BRP-queryable `LabControl` and `LabDiagnostics` resources for live inspection
- host crate-local E2E scenarios for shared-crate verification instead of relying on project-level sandboxes

## Status

Working.

## Run

```bash
cargo run -p spline_tools_lab
```

Keyboard shortcuts:

- `1` / `2` / `3` / `4`: focus the road, placement, tube, or runtime-editing lane
- `A`: add a new control point to the editable spline
- `M`: move the second control point on the editable spline
- `D`: remove the newest editable point

## E2E

```bash
cargo run -p spline_tools_lab --features e2e -- smoke_launch
cargo run -p spline_tools_lab --features e2e -- spline_tools_extrusion_smoke
cargo run -p spline_tools_lab --features e2e -- spline_tools_closed_loop_tube
cargo run -p spline_tools_lab --features e2e -- spline_tools_runtime_edit_smoke
cargo run -p spline_tools_lab --features e2e -- spline_tools_placement_smoke
```

## BRP

The lab uses port `15736` by default. Override it with `BRP_EXTRAS_PORT`.

```bash
BRP_PORT=15736 uv run --active --project .codex/skills/bevy-brp/script brp app launch spline_tools_lab
BRP_PORT=15736 uv run --active --project .codex/skills/bevy-brp/script brp resource get spline_tools_lab::LabDiagnostics
BRP_PORT=15736 uv run --active --project .codex/skills/bevy-brp/script brp resource get spline_tools_lab::LabControl
BRP_PORT=15736 uv run --active --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
BRP_PORT=15736 uv run --active --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/spline_tools_lab.png
BRP_PORT=15736 uv run --active --project .codex/skills/bevy-brp/script brp extras shutdown
```
