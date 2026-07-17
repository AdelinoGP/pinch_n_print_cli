# Requirements: 176-support-preview-verb

## Packet Metadata

- Grouped task IDs: `TASK-280`
- Backlog source: `docs/07_implementation_status.md` (row minted at closure via `task-map.md`)
- Packet status: `draft`
- Aggregate context cost: `M`
- Plan source: `docs/specs/fork-gaps-wave2-plan.md` (Packet 176 — fork handoff item 13)

## Problem Statement

The OrcaSlicer-fork frontend needs a support overlay while the user paints — it must ask PNP "where would supports land for this model + config" and get geometry back fast, without paying for walls, infill, path generation, or G-code. PNP already has exactly the needed seam: `prepare_prepass_context` runs only the shared prefix through Tier 1 (prepass), and the `PrePass::SupportGeometry` stage commits `SupportGeometryIR` (coarse per-layer outline `ExPolygon`s) to the blackboard — the same slot the visual-debug `PrePass::SupportGeometry` blackboard tap reads. What is missing is a CLI verb that exposes that slot as a stable, documented JSON contract in mm.

## In Scope

- `Cmd::SupportPreview { input, output, config, module_dir, no_default_module_paths }` clap variant in `crates/pnp-cli/src/main.rs` (long flags `--input`, `--output`, `--config`, `--module-dir`, `--no-default-module-paths`), dispatching to a new handler.
- New `crates/pnp-cli/src/support_preview.rs`: `load_model(input)` → config map from optional `--config` via `parse_cli_config_source` → `prepare_prepass_context` → read `ctx.blackboard.support_geometry()` → group `entries` by `SupportGeometryKey.global_support_layer_index` (merging across `object_id`/`region_id`), skip-and-count `u32::MAX` sentinel entries, look up `z_mm` from `ctx.plan.global_layers[index].z`, convert every `Point2` to mm (× 1e-4), serialize with `serde_json`, write to `--output`.
- Output contract (fork-facing, versioned): top-level `{ schema_version: "1.0.0", units: "mm", layer_count, skipped_intermediate_entries, layers: [{ layer_index, z_mm, support: [{ contour: [[x,y],...], holes: [[[x,y],...],...] }] }] }`; layers sorted by `layer_index` ascending; layers with no support geometry are omitted from `layers` (sparse array).
- Missing/disabled support ⇒ `layers: []`, exit 0 (AC-N1); missing input ⇒ nonzero exit, no output file (AC-N2).
- Contract doc `docs/20_support_preview.md` + `.claude/doc-index.md` row.
- New per-file test binary `crates/pnp-cli/tests/support_preview_tdd.rs`.

## Out of Scope

- Support vs interface role split: `SupportGeometryIR` carries a single undifferentiated outline per (layer, object, region); interface/roles exist only later as `ExtrusionPath3D` fields on the per-layer `SupportIR` (`support_paths`/`interface_paths`), which requires Tier 2 execution this verb deliberately never runs. The schema reserves nothing; a future minor bump may add an `interface` array.
- `SupportPlanIR.branch_segments` (organic branch line segments) — polygons only in 1.0.0.
- Raft geometry (packet 124's seam; `raft_paths` are Tier 2 anyway).
- Any change to `run.rs`, `prepass.rs`, `layer_executor.rs`, `postpass.rs`, the blackboard, modules, or WIT.
- JSONL streaming, progress events, stdout output modes, rendering/PNG.
- Object/region attribution in the output (merged per layer; a future field if the fork ever needs per-object overlays).

## Authoritative Docs

- `docs/19_visual_debug.md` — over 300 lines; delegated SUMMARY of the `prepare_prepass_context` + blackboard-read precedent only.
- `docs/08_coordinate_system.md` — ranged read: units table only.
- `docs/20_support_preview.md` — authored here; becomes the fork's normative reference.

## Acceptance Summary

- Positive: `AC-1` through `AC-5`. Refinements: AC-2 is the likeliest silent regression (emitting raw internal units — values 10⁴× too large — still passes AC-1's "finite f64" schema check, so AC-2 pins the ×1e-4 conversion against the committed IR); AC-3 pins the latency contract (no Tier 2/Tier 3 side effects).
- Negative: `AC-N1` (disabled/absent support), `AC-N2` (bad input path).
- Cross-packet impact: none upstream; the fork consumes `docs/20_support_preview.md`. Future raft/interface packets bump `schema_version` minor.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p pnp-cli --test support_preview_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result\|FAILED"` | All ACs 1-4, N1, N2 | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `rg -q 'schema_version' docs/20_support_preview.md && rg -q 'interface' docs/20_support_preview.md && rg -q '20_support_preview' .claude/doc-index.md && echo PASS` | AC-5 doc contract | FACT PASS/absent |
| `cargo check --workspace --all-targets` | Gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Gate | FACT pass/fail |

## Step Completion Expectations

The e2e tests dispatch real core modules through prepass — `cargo xtask build-guests --check` must be clean before attributing any test failure to the new verb (guests are a test input here even though this packet edits no guest-feeding path). Fixture choice is a Step 1 discovery output consumed by Step 3's tests.

## Context Discipline Notes

- `crates/pnp-cli/src/visual_debug.rs` is ~2100 lines — read ONLY the run wiring around lines 1330-1375 (`load_model` → `load_visual_debug_config` → `prepare_prepass_context`) as the pattern; never the renderer/manifest body.
- `crates/slicer-runtime/src/run.rs` is large — read only `PrepassContext` + `prepare_prepass_context` (~lines 694-800).
- Never load `crates/slicer-runtime/src/layer_executor.rs` (tap machinery is precedent, not surface — the verb reads the blackboard slot directly).
