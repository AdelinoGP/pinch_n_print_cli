---
status: draft
packet: infill-pattern-specific-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/16-author-packet-p09-strength-infill-pattern-specific-infill-modules.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P09)
context_cost_estimate: S
---

# Packet Contract: infill-pattern-specific-keys

## Goal

Declare OrcaSlicer's 10 P09 pattern-specific infill keys in the `rectilinear-infill` owner manifest with canonical types, defaults, and bounds, each with a `description` recording its honest disposition. Authoring-time grounding re-derives every key's decision point from canonical: **all 10 are re-adjudicated declared-with-gap** — six are consumed only by the locked-zag pattern (`FillLockedZag`), two only by the lateral-lattice pattern (`FillLateralLattice`), one only by the lateral-honeycomb pattern (`FillLateralHoneycomb`), and `symmetric_infill_y_axis` is activated in canonical only when the sparse pattern is zigzag/crosszag/lockedzag — and this port ships exactly three pattern families (rectilinear, gyroid, lightning), so no key has a reachable consumer. The packet therefore writes manifests, guard, inertness, bounds, CONFIG_BLOCK, and docs-15 arms only — zero module-source reads, zero behavior change at any value (defaults and explicit values are byte-identical to absent).

## Scope Boundaries

The packet touches `rectilinear-infill.toml` (10 net-new `[config.schema]` tables), the net-new guard test `infill_pattern_specific_config_schema_tdd.rs` + its `toml = "0.8"` dev-dependency (add-if-absent), the existing module/scheduler/runtime test suites (`rectilinear_raw_emit_tdd.rs`, `config_bounds_enforcement_tdd.rs`, `gcode_header_thumbnail_config_blocks_tdd.rs`), the rebuilt rectilinear guest, and the generated `docs/15_config_keys_reference.md`. It does not implement the locked-zag, lateral-lattice, or lateral-honeycomb pattern classes, pattern dispatch (module identity), the symmetric-Y double mirror (canonical-activation-gated to unshipped patterns), or any module-source read of the 10 keys.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — resolved; number 263 derived from disk at authoring time); ticket 05 (packet-list P09 membership); ticket 04 (tier rubric — Tier A membership re-derived in `requirements.md` §Per-Key Canonical Evidence: all 10 re-adjudicated declared-with-gap; the tier-table owner `infill modules` stands).
- Ordering, not gating: packets 253–262 precede this packet in the queue. Packet 262 (P08) touches the same `rectilinear-infill.toml` — same-module merge churn (both append `[config.schema]` tables; 262 lands first per queue order), not a hard gate. P09's guard test is a distinct binary (`infill_pattern_specific_config_schema_tdd`) so the two packets' net-new test files never collide. P10 (ticket 17) touches the same infill modules — different keys, no dependency.
- Unblocks: wayfinder ticket 16's resolution; nothing downstream gates on this packet specifically.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** `rectilinear-infill.toml` `[config.schema]`, **when** parsed by the guard test, **then** it carries exactly these 10 tables with exactly these fields — `infill_lock_depth` (`type = "float"`, `default = 1.0`, `min = 0.0`, `max = 100.0`), `infill_overhang_angle` (`float`, `60.0`, `15.0`, `75.0`), `lateral_lattice_angle_1` (`float`, `-45.0`, `-75.0`, `75.0`), `lateral_lattice_angle_2` (`float`, `45.0`, `-75.0`, `75.0`), `skeleton_infill_density` (`float`, `25.0`, `0.0`, `100.0`), `skeleton_infill_line_width` (`float`, `0.0`, `0.0`, `2.0`), `skin_infill_density` (`float`, `25.0`, `0.0`, `100.0`), `skin_infill_depth` (`float`, `2.0`, `0.0`, `100.0`), `skin_infill_line_width` (`float`, `0.0`, `0.0`, `2.0`), `symmetric_infill_y_axis` (`type = "bool"`, `default = false`, no bounds) — each with the canonical-title `display` name, `group = "Infill"`, and a `description` field recording the disposition (canonical consumer function + the unshipped pattern class). | `cargo test -p rectilinear-infill --test infill_pattern_specific_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a rectilinear run over the square fixture, **when** the 10 keys are set explicitly to their canonical defaults (`infill_lock_depth = 1`, `infill_overhang_angle = 60`, `lateral_lattice_angle_1 = -45`, `lateral_lattice_angle_2 = 45`, `skeleton_infill_density = 25`, `skeleton_infill_line_width = 0`, `skin_infill_density = 25`, `skin_infill_depth = 2`, `skin_infill_line_width = 0`, `symmetric_infill_y_axis = false`), **then** the emitted `InfillIR` paths are byte-identical to the same run with the 10 keys absent — every key is unread at any value (default-path identity). | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** the scheduler's config bounds index loaded from the real `rectilinear-infill.toml` manifest, **when** an out-of-bounds or wrong-typed value is resolved, **then** `lateral_lattice_angle_1 = -80` (below min −75), `lateral_lattice_angle_2 = 80` (above max 75), `infill_overhang_angle = 10` (below min 15), `skin_infill_density = 101` (above max 100), and `skin_infill_depth = -1` (below min 0) are each rejected with the numeric `OutOfRange` error, and `symmetric_infill_y_axis = "abc"` is rejected with `TypeMismatch`; a valid `symmetric_infill_y_axis = true` resolves. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a slice run over the square fixture, **when** the G-code CONFIG_BLOCK is emitted, **then** at defaults the block carries zero `infill_lock_depth` / `infill_overhang_angle` / `lateral_lattice_angle_1` / `lateral_lattice_angle_2` / `skeleton_infill_density` / `skeleton_infill_line_width` / `skin_infill_density` / `skin_infill_depth` / `skin_infill_line_width` / `symmetric_infill_y_axis` lines (none of the 10 keys has an `ORCA_CONFIG_PADDING` twin — verified zero occurrences in `crates/` at authoring); with an explicit raw-config `skin_infill_density = 30.0`, the line `; skin_infill_density = 30` appears exactly once (explicit values reach the block via the raw-config sorted dump, packet-257 AC-5 form). | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked, **then** the module-key table carries all 10 keys (owner column `rectilinear-infill` ×10), and the deviations block (`<!-- BEGIN GENERATED: orca-deviations ... -->` … `<!-- END GENERATED: orca-deviations -->`) still contains exactly 26 data rows (pre-packet count, measured 2026-09-01 — the 5 parseable float defaults 1.0/60.0/−45.0/45.0/2.0 match canonical, the percent defaults `25%`/`100%` fail `parse::<f64>` and never enter the numeric comparison map, and the bool `false` matches canonical's `0` under the ticket-100 bool comparison). | `cargo xtask gen-config-docs --check && for k in infill_lock_depth infill_overhang_angle lateral_lattice_angle_1 lateral_lattice_angle_2 skeleton_infill_density skeleton_infill_line_width skin_infill_density skin_infill_depth skin_infill_line_width symmetric_infill_y_axis; do rg -q "$k" docs/15_config_keys_reference.md || exit 9; done && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| \`')" = "26" ]; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** the manifest schema guard, **when** any of the 10 tables is removed from `rectilinear-infill.toml` or its `type`/`default`/`min`/`max`/`display`/`group` drifts from AC-1's exact table, **then** the guard fails naming the offending key and manifest. | `cargo test -p rectilinear-infill --test infill_pattern_specific_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the deliberate omission ruling, **when** `gyroid-infill.toml` and `lightning-infill.toml` are parsed, **then** they do NOT declare any of the 10 keys (every canonical consumer is a `FillRectilinear` subclass — locked-zag/lateral-lattice/lateral-honeycomb are rectilinear-family patterns; the line-width and density tables already present in gyroid/lightning are different keys with different defaults); the omission is pinned so a future packet that implements any of the three pattern classes must update the guard. | `cargo test -p rectilinear-infill --test infill_pattern_specific_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p rectilinear-infill --test infill_pattern_specific_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — `rectilinear-infill.toml` is a guest-fingerprint input (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated tables regenerate via `cargo xtask gen-config-docs`; verify with `--check` (delegated; the doc is generated, never hand-edited).
- `docs/03_wit_and_manifest.md` — manifest schema shape (`[config.schema]` type table: `bool`/`int`/`float`/`string`/`enum` with `values`; `description` field); delegated SUMMARY if a worker needs the contract.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains 10 rows for the 10 keys (owner column `rectilinear-infill` ×10); the generated deviations block is unchanged (26 data rows — the five parseable declared float defaults match canonical, percent defaults are outside the numeric comparison, the bool matches under the ticket-100 comparison). The doc has no per-module subheadings, so verification is key-presence + row-count, not headings. Verification greps: the AC-5 `rg` loop over all 10 keys plus the AC-5 deviation-block probe. The doc is generated — the edit lands through `cargo xtask gen-config-docs` (Step 5), never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the 10 keys (all on `PrintObjectConfig`): `infill_lock_depth` coFloat default 1 min 0 max 100; `infill_overhang_angle` coFloat default 60 min 15 max 75; `lateral_lattice_angle_1` coFloat default −45 min −75 max 75; `lateral_lattice_angle_2` coFloat default 45 min −75 max 75; `skeleton_infill_density` coPercent default 25 min 0 max 100; `skeleton_infill_line_width` coFloatOrPercent default 100% min 0; `skin_infill_density` coPercent default 25 min 0 max 100; `skin_infill_depth` coFloat default 2 min 0 max 100; `skin_infill_line_width` coFloatOrPercent default 100% min 0; `symmetric_infill_y_axis` coBool default false. Authoring-time evidence already captured in `requirements.md` §Per-Key Canonical Evidence (dispatched canonical reads, 2026-09-01) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` / `group_fills` (per-surface param plumbing of all 10 keys; the `symmetric_infill_y_axis` activation gate: the flag is set only when `params.pattern` is `ipCrossZag`/`ipLockedZag`/`ipZigZag`, read from the region's sparse pattern, role-independent).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `FillLockedZag::fill_surface_locked_zag` (consumer of `infill_lock_depth`/`skin_infill_depth`/`skin_infill_density`/`skin_infill_line_width`/`skeleton_infill_density`/`skeleton_infill_line_width`), `FillLateralHoneycomb::fill_surface` (consumer of `infill_overhang_angle`), `FillLateralLattice::fill_surface` (consumer of `lateral_lattice_angle_1`/`lateral_lattice_angle_2`), `fill_surface_by_lines` (the `if (params.symmetric_infill_y_axis)` rotate-back mirror branch inherited by the zigzag family).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Flow::new_from_config_width` (resolves the two coFloatOrPercent width keys' `100%` ratio over `nozzle_diameter`).
- `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp` — `MultiPoint::symmetric_y` (the exact mirror arithmetic: `pt(0) = 2 * x_axis - pt(0)`, axis = `extended_object_bounding_box().center().x()`).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
