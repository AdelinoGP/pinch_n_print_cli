---
status: draft
packet: top-bottom-surface-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/17-author-packet-p10-strength-top-bottom-shells-infill-modules.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P10)
context_cost_estimate: S
---

# Packet Contract: top-bottom-surface-keys

## Goal

Wire OrcaSlicer's two top/bottom surface density keys (`top_surface_density`, `bottom_surface_density`) into the rectilinear-infill module's existing top/bottom solid spacing decision points (`solid_spacing = line_width / SOLID_DENSITY` in `modules/core-modules/rectilinear-infill/src/lib.rs`, `SOLID_DENSITY = 1.0`) with canonical defaults so the default path is byte-identical, declare the two surface pattern keys (`top_surface_pattern`, `bottom_surface_pattern`) with-gap (pattern selection is module identity in this port), and correct the `top_surface_pattern` CONFIG_BLOCK padding twin to the canonical default (`monotonicline`).

## Scope Boundaries

The packet touches `rectilinear-infill.toml` (4 net-new `[config.schema]` tables), `rectilinear-infill/src/lib.rs` (2 new struct fields + the top/bottom solid spacing wire), the net-new guard test `top_bottom_surface_config_schema_tdd.rs` + its `toml = "0.8"` dev-dependency (add-if-absent), the existing module/scheduler/runtime test suites (`top_bottom_fill_tdd.rs`, `config_bounds_enforcement_tdd.rs`, `gcode_header_thumbnail_config_blocks_tdd.rs`), one `ORCA_CONFIG_PADDING` value correction in `crates/slicer-gcode/src/serialize.rs`, the rebuilt rectilinear guest, and the generated `docs/15_config_keys_reference.md`. It does not implement pattern dispatch (filler selection stays module identity), the gyroid opt-in solid path (ADR-0027 — gyroid's solid emission rides the sparse density; the P10 keys do not reach it), canonical's surface-expansion density gates (`PrintObject.cpp` `detect_surfaces_type`, `PerimeterGenerator.cpp` `top_fill_replaces_inner_walls`), the extra-internal-solid-fill branch (`Fill.cpp` `group_fills`' `top_surface_pattern` read), or the `GCode.cpp` `_needSAFC` / `retract` pattern reads.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — resolved; number 264 derived from disk at authoring time); ticket 05 (packet-list P10 membership); ticket 04 (tier rubric — Tier A membership re-derived in `requirements.md` §Per-Key Canonical Evidence: the two density keys are wired, the two pattern keys re-adjudicated declared-with-gap; the tier-table owner `infill modules` stands).
- Ordering, not gating: packets 253–263 precede this packet in the queue. Packets 262 (P08) and 263 (P09) touch the same `rectilinear-infill.toml` — same-module merge churn (all append `[config.schema]` tables; 262/263 land first per queue order), not a hard gate. P10's guard test is a distinct binary (`top_bottom_surface_config_schema_tdd`) so the three packets' net-new test files never collide; the `toml` dev-dep is add-if-absent (first lander wins).
- Unblocks: wayfinder ticket 17's resolution; nothing downstream gates on this packet specifically.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** `rectilinear-infill.toml` `[config.schema]`, **when** parsed by the guard test, **then** it carries exactly these 4 tables with exactly these fields — `top_surface_density` (`type = "float"`, `default = 100.0`, `min = 0.0`, `max = 100.0`), `bottom_surface_density` (`type = "float"`, `default = 100.0`, `min = 10.0`, `max = 100.0`), `top_surface_pattern` (`type = "enum"`, `values` = the 8 canonical strings `["monotonic", "monotonicline", "rectilinear", "alignedrectilinear", "concentric", "hilbertcurve", "archimedeanchords", "octagramspiral"]`, `default = "monotonicline"`), `bottom_surface_pattern` (`type = "enum"`, `values` = the same 8, `default = "monotonic"`) — each with the canonical-title `display` name, `group = "Infill"`, and a `description` field recording the disposition (wired consumer or decision-point gap). | `cargo test -p rectilinear-infill --test top_bottom_surface_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a rectilinear top/bottom run over the square fixture, **when** the 4 keys are set explicitly to their canonical defaults (`top_surface_density = 100`, `bottom_surface_density = 100`, `top_surface_pattern = "monotonicline"`, `bottom_surface_pattern = "monotonic"`), **then** the emitted `InfillIR` paths are byte-identical to the same run with the 4 keys absent — the wired density keys are default-path identity (100 → fraction 1.0 = `SOLID_DENSITY`) and the declared-with-gap pattern keys are unread at any value. | `cargo test -p rectilinear-infill --test top_bottom_fill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** a rectilinear top/bottom run over the square fixture, **when** `top_surface_density = 50` (or `bottom_surface_density = 50`) is set, **then** the top-solid (resp. bottom-solid) path count is approximately half of the 100-density run (spacing doubles: `line_width / 0.5` vs `line_width / 1.0` — canonical `FillLine.cpp` `FillLine::_fill_surface_single`'s `line_spacing = flow.spacing() / density` shape) — the value reaches the consumer. | `cargo test -p rectilinear-infill --test top_bottom_fill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** the scheduler's config bounds index loaded from the real `rectilinear-infill.toml` manifest, **when** an out-of-bounds or wrong-typed value is resolved, **then** `top_surface_density = 101` and `-1` (outside [0,100]) and `bottom_surface_density = 5` and `101` (outside [10,100]) are each rejected with the numeric `OutOfRange` error, `top_surface_pattern = "bogus"` and `bottom_surface_pattern = "bogus"` (not in the 8-value list) are each rejected with `TypeMismatch` ("unsupported enum value"), and valid `top_surface_density = 0` and `bottom_surface_density = 10` resolve. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a slice run over the square fixture, **when** the G-code CONFIG_BLOCK is emitted, **then** at defaults the block carries `; top_surface_pattern = monotonicline` (padding twin corrected from `"monotonic"` — canonical default is `monotonicline`) and `; bottom_surface_pattern = monotonic` (padding twin unchanged), and carries zero `top_surface_density` / `bottom_surface_density` lines (no padding twins); with an explicit raw-config `top_surface_density = 50.0` the line `; top_surface_density = 50` appears exactly once, and with an explicit `top_surface_pattern = "concentric"` the line `; top_surface_pattern = concentric` appears exactly once (padding twin suppressed by the emitted-key dedup in `serialize_config_block`). | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked, **then** the module-key table carries all 4 keys (owner column `rectilinear-infill` ×4), and the deviations block (`<!-- BEGIN GENERATED: orca-deviations ... -->` … `<!-- END GENERATED: orca-deviations -->`) still contains exactly 26 data rows (pre-packet count, measured 2026-09-01 — the two enum defaults never enter the numeric comparison map in `render_deviations`, and the two percent defaults `100%` fail `parse::<f64>` and never enter it either). | `cargo xtask gen-config-docs --check && for k in top_surface_density bottom_surface_density top_surface_pattern bottom_surface_pattern; do rg -q "$k" docs/15_config_keys_reference.md || exit 9; done && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c "^| \`")" = "26" ]; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** the manifest schema guard, **when** any of the 4 tables is removed from `rectilinear-infill.toml` or its `type`/`default`/`min`/`max`/`values`/`display`/`group` drifts from AC-1's exact table, **then** the guard fails naming the offending key and manifest. | `cargo test -p rectilinear-infill --test top_bottom_surface_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the deliberate omission ruling, **when** `gyroid-infill.toml` and `lightning-infill.toml` are parsed, **then** they do NOT declare any of the 4 keys (gyroid's opt-in solid emission rides the sparse density per ADR-0027 — a pre-existing divergence this packet records, not fixes; lightning holds only `claim:sparse-fill` and has no solid-fill surface); the omission is pinned so a future packet that wires the P10 keys for gyroid/lightning must update the guard. | `cargo test -p rectilinear-infill --test top_bottom_surface_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** a rectilinear top/bottom run over the square fixture, **when** `top_surface_density = 0` is set, **then** the exposed-top region (top_shell_index 0) emits zero `TopSolidInfill` paths (canonical `group_fills`' `density <= 0` skip), while an internal-solid region (top_shell_index ≥ 1) still emits at density 1.0 (canonical `group_fills`' fixed `100.f` for `stInternalSolid`). | `cargo test -p rectilinear-infill --test top_bottom_fill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p rectilinear-infill --test top_bottom_surface_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p rectilinear-infill --test top_bottom_fill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — `rectilinear-infill.toml` and `rectilinear-infill/src/lib.rs` are guest-fingerprint inputs (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated tables regenerate via `cargo xtask gen-config-docs`; verify with `--check` (delegated; the doc is generated, never hand-edited).
- `docs/03_wit_and_manifest.md` — manifest schema shape (`[config.schema]` type table: `bool`/`int`/`float`/`string`/`enum` with `values`; `description` field); delegated SUMMARY if a worker needs the contract.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains 4 rows for the 4 keys (owner column `rectilinear-infill` ×4); the generated deviations block is unchanged (26 data rows — the two enum defaults are outside the numeric comparison, the two percent defaults fail `parse::<f64>`). The doc has no per-module subheadings, so verification is key-presence + row-count, not headings. Verification greps: the AC-6 `rg` loop over all 4 keys plus the AC-6 deviation-block probe. The doc is generated — the edit lands through `cargo xtask gen-config-docs` (Step 5), never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the 4 keys (all on `PrintRegionConfig`): `top_surface_pattern` coEnum default `ipMonotonicLine` with the 8-value `InfillPattern` list; `top_surface_density` coPercent default 100 min 0 max 100; `bottom_surface_pattern` coEnum (same 8 values) default `ipMonotonic`; `bottom_surface_density` coPercent default 100 min 10 max 100. Authoring-time evidence already captured in `requirements.md` §Per-Key Canonical Evidence (dispatched canonical reads, 2026-09-01) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `group_fills` (per-surface-type assignment: `stTop` → `top_surface_pattern` + `top_surface_density` with the `density <= 0` skip; `stBottom` → `bottom_surface_pattern` + `bottom_surface_density`; `stInternalSolid` → `internal_solid_infill_pattern` + fixed `100.f`; the extra-internal-solid-fill branch reading `top_surface_pattern`), `Layer::make_fills` (the `0.01 * density` percent normalization).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLine.cpp` — `FillLine::_fill_surface_single` (the spacing formula this packet's wire mirrors: `line_spacing = flow.spacing() / density`).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `top_fill_replaces_inner_walls` (the `density > 0` gate this packet records, not wires).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `detect_surfaces_type` (the `density > 0` top-surface-expansion gate this packet records, not wires), `invalidate_state_by_config_options` (slice-step invalidation mapping).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `_needSAFC`, `retract` (emission-time pattern reads this packet records, not wires).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
