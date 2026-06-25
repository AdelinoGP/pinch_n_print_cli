# Implementation Plan: 125_voronoi-oom-hardening

> Implementation-complete. Steps record the **shipped** sequence (all green). A reviewer re-runs each
> step's verification to confirm. Order keeps the build green between steps; the full executor bucket is
> re-run after each Part (the trap the original packet fell into).

## Execution Rules

- One atomic step; never leave the bucket red between Parts. After any guest-dep edit, run
  `cargo xtask build-guests --check` (rebuild if `STALE:`) before trusting a guest/executor result.
- Delegate every cargo run with a FACT pass/fail return. Tee to `target/test-output.log`; read the file.
- Files allowed to edit per step ≤ 3 primary (supporting plumbing files threaded in the same step).

## Steps

### Step A1 — add `PrintEntity.tool_index` + schema bump
- Objective: first-class tool field; LayerCollection schema 1.0.0→1.1.0.
- Precondition: packet-125 floor/guard/tripwire in place. Postcondition: field present, `#[serde(default)]`, no struct `Default`.
- Read: `slice_ir.rs` §IR 10. Edit: `crates/slicer-ir/src/slice_ir.rs`.
- Cost: S. Verify: `cargo test -p slicer-ir --test ir_tests slice_ir_schema_version_is_one_one_zero` (AC-6).
- Exit: `cargo check -p slicer-ir` passes; schema test green.

### Step A2 — fix all `PrintEntity` construction sites (compiler-guided)
- Objective: every literal sets `tool_index` (transitional = the value then in `region_id`).
- Read: compiler errors. Edit: ~43 sites across production + tests (delegable to a Sonnet sub-agent with the rule "tool_index = that site's region_id value, else 0").
- Cost: M. Verify: `cargo check --workspace --all-targets` clean.
- Exit: `--all-targets` compiles.

### Step A3 — carry tool to the path-opt guest (3 layers)
- Objective: `ordered-entity-view.tool-index` + host `dispatch::OrderedEntityView` + SDK `OrderedEntityView`.
- Edit: `ir-types.wit`, `crates/slicer-wasm-host/src/{dispatch.rs,host.rs}`, `crates/slicer-sdk/src/views.rs`, macro conversion.
- Cost: M. Dispatch: `cargo xtask build-guests` FACT. Verify: AC-5 once A6 lands.

### Step A4 — flip emit + guest tool readers to `tool_index`
- Objective: `emit.rs` ~13 reads + path-opt `tool_index_of` read the field; keep the guard.
- Edit: `crates/slicer-gcode/src/emit.rs`, `modules/core-modules/path-optimization-default/src/lib.rs`.
- Cost: S. Verify: AC-N1 `cargo test -p slicer-gcode emit_rejects_out_of_range_tool_id`.

### Step A5 — finalization explicit tool param
- Objective: `tool-index` on push/insert WIT methods + `print-entity-view.tool-index`; host reconstruction, SDK builder, macro drain, skirt/wipe pass the tool.
- Edit: `world-finalization.wit`, `host.rs` finalization, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs`, skirt-brim/wipe-tower guests.
- Cost: M. Dispatch: rebuild guests. Verify: executor bucket finalization tests stay green.

### Step A6 — assembly sets real tool, restores identity
- Objective: `tool_index = resolved_tool` (`.unwrap_or(DEFAULT_TOOL)`); `region_key.region_id = region.region_id`.
- Edit: `crates/slicer-runtime/src/layer_executor.rs`.
- Cost: S. Verify: AC-1, AC-2, AC-3, AC-4, AC-5; then **full bucket** `cargo test -p slicer-runtime --test executor`.
- Exit: red tests #1/#2 pass; `extruder_synthetic_t0_t1_emission` + `cross_object_ordering` updated to read `tool_index`; bucket green except red #3 (Part B).

### Step B — D14 fuzzy via `slice-region-view.variant-chain`
- Objective: drop segment_annotations synthesis; expose `variant-chain()` over WIT; thread `variant_fuzzy` into `build_wall_flags`.
- Edit: `paint_segmentation/mod.rs`, `ir-types.wit` + host `SliceRegionData`/`marshal/in_.rs` + SDK `SliceRegionView`, `perimeter_utils.rs` + arachne/classic guests (+ ~6 `inner_wall_*` test callers).
- Cost: M. Dispatch: rebuild guests. Verify: AC-7, AC-8; full bucket → **167/0**.

### Step C-emit — per-tool config resolution + emit consumer
- Objective: `resolve_per_tool_configs` (+ exports); `DefaultGCodeEmitter` per-tool `retract_length`, wired in `run.rs`.
- Edit: `crates/slicer-scheduler/src/config_resolution.rs`, `crates/slicer-gcode/src/emit.rs`, `crates/slicer-runtime/src/run.rs` (+ lib re-exports).
- Cost: S. Verify: AC-9, AC-10.

### Step C-geometry — painted per-tool overlay at RegionMapping
- Objective: value-aware overlay in `execute_region_mapping_inner` (per-tool highest precedence), threaded prepass → producer.
- Edit: `crates/slicer-core/src/algos/region_mapping.rs`, `crates/slicer-runtime/src/{prepass.rs,builtins/region_mapping_producer.rs}`.
- Cost: M. Dispatch: rebuild guests (slicer-core). Verify: AC-11, AC-12; region-mapping bucket behavior-neutral with empty tool_configs.

### Step D/E — voronoi hardening
- Objective: `MAX_VORONOI_SEGMENTS` input cap + typed `InputTooLarge`; `catch_unwind` backstop + `PredicatePanic` in `from_colored_lines`.
- Edit: `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`.
- Cost: S. Verify: AC-13, AC-14; re-confirm AC-8 jitter (Part E must not break paint segmentation).

### Step Docs — contract docs + deviation
- Edit: `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/DEVIATION_LOG.md` (`D-125-TOOL-IDENTITY-SPLIT`).
- Cost: S. Verify: doc edits present (grep).

## Per-Step Budget Roll-Up

S×6 + M×4 = aggregate **L**. No single step is L (A2 is the largest, M, and delegable).

## Packet Completion Gate

- All AC commands (AC-1…AC-14, AC-N1, AC-N2) green.
- `cargo test -p slicer-runtime --test executor` → **167 passed / 0 failed**.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` clean.
- Workspace `cargo test --workspace --no-fail-fast` → **2307 passed / 0 failed** (one-time closure sweep
  — required because two of this packet's regressions lived in targets invisible to the executor bucket).

## Acceptance Ceremony

Run the Completion Gate as a single FACT dispatch. Do NOT declare done on a subset of tests — the
subset-green / bucket-red gap is the specific failure mode that produced the original deferral.
