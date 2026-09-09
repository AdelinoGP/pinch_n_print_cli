# Requirements: 299-object-level-shell-infill-planning

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/80-author-packet-p73-strength-advanced-strength-object-level-planning.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P73 is the last unowned object-level planning slice: four keys whose canonical readers all converge in `PrintObject.cpp` (`discover_vertical_shells`, `discover_horizontal_shells`, `combine_infill`) and whose behaviour this tree has none of — no vertical-shell guarantee, no extra-solid layer insertion, no multi-layer sparse merge. The tier table sizes it Tier B (new logic in an existing owner) and names the owner object-level planning; claim-time grounding (ticket 80) confirms the owner but narrows it for this tree to the host prepass `commit_shell_classification_builtin` plus the five-way fill partition and the sparse emitters (ticket-36 precedent — the key's decision points span the planner smoothing, both renderers, and the host resolver, so the owner is a seam, not a module). All four keys are zero-occurrence as behaviour, all pass rule 3, and no draft packet owns the decision — so the packet wires all four, sheds none, and returns none.

## In Scope

- `ensure_vertical_shell_thickness` (canonical coEnum: `none` / `ensure_critical_only` / `ensure_moderate` / `ensure_all`, default `ensure_all`): prepass vertical-shell stage behind the mode gate — `ensure_all` projects neighbouring top/bottom shells into internal-solid on slopes, `critical_only`/`moderate` apply the narrowed margin/search branches, `none` is inert. Strict-parse rejection on unknown strings (AC-N1).
- `extra_solid_infills` (canonical coString, default `""`): prepass layer-pattern insertion — non-empty patterns matching `check_layer_id_pattern` (1-based `N`, `N#K`, comma lists) re-type that layer's sparse area to `internal_solid_fill`.
- `infill_combination` (canonical coBool, default `false`): prepass sparse-grouping gate — when true (and density non-zero), consecutive sparse layers merge into one print group; walls keep original layer height (emitter-side exclusion, AC-5).
- `infill_combination_max_layer_height` (canonical coFloatOrPercent, default `100%`-percent-true): grouping cap resolved as percent-against-nozzle (the `ConfigView::get_abs_value` percent-resolution shape, `crates/slicer-ir/src/slice_ir.rs`) — cumulative grouped height stays below `min(cap, nozzle_diameter)`; `0`/`100%` resolve to the nozzle diameter.
- Four `ResolvedConfig` fields at canonical defaults, per-object via the existing overlay (290/291/296 precedent — explicitly not ticket 125's tool axis; canonical scalarity IS held, all four scalar).
- One net-new IR field (`SlicedRegion.combined_infill_height`, `Option<f32>`, serde-defaulted) plus its `SliceRegionView` accessor, carrying the grouped height to the sparse emitters; schema bump computed at activation from the live `CURRENT_SLICE_IR_SCHEMA_VERSION`, never hardcoded (S3).
- Locked-path bypass (ADR-0062/0063 conformance): combination and shell growth neither clip nor merge locked footprints (AC-N2).
- One deviation row (DEV-190) and the `docs/15` regen.

## Out of Scope

- `Print.cpp` reslice-invalidation entries for these keys (invalidation bookkeeping, not slicing behaviour — rides ticket 124, named non-borrow).
- Per-nozzle vector model for any key (canonical declares all four scalar — no ticket-125 arm, unlike packets 276/277/279–289).
- Canonical range enforcement (GUI hints only, ticket-113 rule — below-min saturates, never rejects; the one rejection in this packet is strict-parse of the mode enum, not a numeric bound).
- Gyroid solid-density path (packet-264 omission stands — the combined height drives the sparse role; gyroid's opt-in solid still rides `sparse_infill_density`, named non-borrow).
- `interface_shells` multi-material gating of the vertical-shell pass (P76 scope — this packet gates on the mode alone; the any-region-`evstAll` multi-material early-return is a named non-borrow).
- CONFIG_BLOCK emission for these keys (honest absence — prepass inputs, not emitter inputs; spellings ride ticket 132; no `ORCA_CONFIG_PADDING` edits, rule 2).
- Full `TreeSupport3D.cpp` organic engine (DEV-156 stands), prime-tower body (ticket 122), sequential validator (ticket 124).

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (prepass vs module seam; claim-system rule-4 trigger).
- `docs/08_coordinate_system.md` - direct range read (mm↔unit helpers at shell-margin boundaries).
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest stanza shape; host-prepass keys need no module manifest — `ConfigView::from_declared` whitelist does not apply).
- `docs/DEVIATION_LOG.md` - direct read (DEV-168/169/171 scalar-vs-vector precedents; DEV-190 minting convention).
- `docs/04_host_scheduler.md` - delegated SUMMARY (prepass ordering: shell stages run inside `commit_shell_classification_builtin` before `convert_small_sparse_islands` and the bridge gates).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_vertical_shells` shell-projection + regularization radii (what geometry is added per mode; `evstAll` gate).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_horizontal_shells` margin branches (3× vs 1× solid-width gates; `evstAll` early-continue; `evstCriticalOnly`/`evstNone` search-stop) and `extra_solid_infills` insertion point.
- `OrcaSlicerDocumented/src/libslic3r/utils.cpp` — `check_layer_id_pattern` 1-based / `N` / `N#K` / comma-list matching (exact edge semantics for the pattern parser).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `combine_infill` grouping cap (`get_abs_value(nozzle_diameter)`), pattern-dependent clearance offsets, thickness/thickness_layers write-back, first-layer skip, wall-height exclusion.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical defaults/shapes for all four keys (enum values, empty-string, bool false, 100%-percent).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-6`; AC-2 pins the one intended default output change (vertical shells newly active at `ensure_all`); AC-4 pins the grouping + cap; AC-5 pins the sparse-only emission (walls keep height).
- Negative: `AC-N1` (strict-parse rejection) through `AC-N2` (locked-path bypass).
- Cross-packet impact: none — no draft packet consumes these keys; 234a's `extra_solid_infills` non-borrow stays a non-borrow; gyroid-solid (264), tower-body (122), and validator (124) boundaries are named non-borrows above.
- Disposition table: exactly the 4 P73 keys, all wired, declaration-only keys: 0.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'cli "ensure_vertical_shell_thickness"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "extra_solid_infills"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "infill_combination"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "infill_combination_max_layer_height"' crates/slicer-ir/src/resolved_config.rs 2>&1 \| tee target/test-output.log \| tail -3` | AC-1 field presence | FACT pass/fail |
| `cargo test -p slicer-runtime --lib vertical_shell_mode_drives_solid_fill 2>&1 \| tee target/test-output.log \| tail -5` | AC-2 shell gate | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --lib extra_solid_pattern_inserts_solid_layers 2>&1 \| tee target/test-output.log \| tail -5` | AC-3 pattern insertion | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --lib infill_combination_groups_sparse_layers 2>&1 \| tee target/test-output.log \| tail -5` | AC-4 grouping + cap | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p rectilinear-infill --lib combined_height_drives_sparse_only 2>&1 \| tee target/test-output.log \| tail -5` | AC-5 sparse-only emission | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `rg -q 'declaration-only keys: 0' docs/spec_packets/299-object-level-shell-infill-planning/requirements.md && rg -q 'DEV-190' docs/DEVIATION_LOG.md && rg -q 'ensure_vertical_shell_thickness' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -5` | AC-6 dispositions + docs | FACT pass/fail |
| `cargo test -p slicer-runtime --lib unknown_shell_mode_rejects 2>&1 \| tee target/test-output.log \| tail -5` | AC-N1 rejection | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --lib combination_and_shell_bypass_locked_paths 2>&1 \| tee target/test-output.log \| tail -5` | AC-N2 locks | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | closure gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Steps run in order: ResolvedConfig fields first (later stages read them via `region_map.config_for`), then IR field + view accessor, then the three prepass stages, then the emitter arm, then DEV-190 + docs regen. The emitter step sequences after the grouping step lands (activation-blocked on its metadata, same-packet ordering — not a cross-packet dep).
- Shared scratch state: the sloping-wall + uniform-box fixtures built in Step 3 are reused by Steps 4–6; do not rebuild them per step.

## Context Discipline Notes

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` is over 1100 lines — range-read only (`commit_shell_classification_builtin` lines 116–230, `resolve_shell_counts` lines 1081–1106, `convert_small_sparse_islands` lines 1028–1080); delegate all other facts.
- `OrcaSlicerDocumented/` — delegate every read per the obligations above; never load.
- Heavy dispatches (`cargo check`, `cargo clippy`) return FACT pass/fail only, never full logs.
