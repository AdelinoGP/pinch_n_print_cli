---
status: draft
packet: infill-angle-and-multiline-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/15-author-packet-p08-strength-infill-infill-modules.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P08; re-authored under the map's Authoring rules 1–6 and split, 210a/210b precedent)
context_cost_estimate: M
---

# Packet Contract: infill-angle-and-multiline-keys

## Goal

Make the four angle/multiline infill keys real decision points inside the fill modules that can act on them: `solid_infill_direction` (a solid-role base angle distinct from the sparse `infill_direction`), `sparse_infill_rotate_template` / `solid_infill_rotate_template` (a per-layer angle cycled from a comma-separated list), and `fill_multiline` (N parallel sparse lines per scan line). Each key is declared only on the module that reads it, and each is proved by an AC that changes geometry at a non-default value.

## Scope Boundaries

The packet edits `modules/core-modules/rectilinear-infill/{rectilinear-infill.toml,src/lib.rs,tests/*}` and `modules/core-modules/gyroid-infill/{gyroid-infill.toml,src/lib.rs,tests/*}`, plus scheduler bounds arms, the runtime CONFIG_BLOCK arm, and the generated `docs/15_config_keys_reference.md`. It declares **nothing** on `lightning-infill` (a tree-based sparse pattern with no scan-line angle and no multiline concept — pinned by AC-N2) and **nothing** on gyroid for `fill_multiline` (offsetting a TPMS curve is a different algorithm, deliberately deferred). It does not touch `ORCA_CONFIG_PADDING` or any CONFIG_BLOCK twin (map Authoring rule 2; AC-N3 asserts a zero-line diff). Pattern selection (`sparse_infill_pattern`, `internal_solid_infill_pattern`) and fill-side gap fill (`gap_fill_target`) moved to packet **262b**; the rotate-template metalanguage beyond the comma-separated list form is out of scope and is rejected rather than silently accepted.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — this directory is the renamed `262-infill-pattern-keys`, split with explicit user approval this session); ticket 105 (infill-angle rename — resolved; the wired keys build on the renamed `infill_direction`); ticket 107 (infill duplicate collapses — resolved).
- Ordering, not gating: packet 262b consumes the same two module manifests. 262a lands first; 262b's new modules are separate crates and do not collide.
- Unblocks: wayfinder ticket 15's resolution (jointly with 262b).
- Activation blockers: none. No `[BLOCK]` in `design.md`.

## Acceptance Criteria

- **AC-1. Given** the fill-module manifests, **when** their `[config.schema]` tables are parsed, **then** `rectilinear-infill.toml` carries exactly these four net-new tables — `solid_infill_direction` (`type = "float"`, `default = 45.0`, `min = 0.0`, `max = 360.0`), `sparse_infill_rotate_template` (`type = "string"`, `default = ""`), `solid_infill_rotate_template` (`string`, `""`), `fill_multiline` (`type = "int"`, `default = 1`, `min = 1`, `max = 10`) — and `gyroid-infill.toml` carries exactly the first three, every table with `group = "Infill"`, the canonical title as `display`, and a `description` naming the in-module consumer; `lightning-infill.toml` gains none of the four. | `cargo test -p rectilinear-infill --test infill_angle_multiline_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a rectilinear run over the square fixture with `infill_direction = 45` and `solid_infill_direction = 90`, **when** the region carries both a sparse area and a solid area, **then** every emitted `SparseInfill` path's line direction is 45° and every `TopSolidInfill` / `BottomSolidInfill` / internal-solid path's direction is 90° (± 0.01°) — the solid role reads its own base angle, the sparse role keeps `infill_direction`. | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** the same configuration on `gyroid-infill` (which holds all four fill claims per ADR-0027), **when** a solid role is routed to gyroid, **then** its solid wave orientation follows `solid_infill_direction = 90` while the sparse wave follows `infill_direction = 45`. | `cargo test -p gyroid-infill --test gyroid_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a rectilinear run with `sparse_infill_rotate_template = "0,90"` and an empty `solid_infill_rotate_template`, **when** `run_infill` is invoked for `layer_index` 0, 1, 2, **then** the sparse path directions are 0°, 90°, 0° (the list cycled by layer index, canonical `Fill.cpp::calculate_infill_rotation_angle`'s list form) and the solid path direction stays at `solid_infill_direction` on every layer. | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a rectilinear run with `solid_infill_rotate_template = "0,90"` and an empty `sparse_infill_rotate_template`, **when** `run_infill` is invoked for `layer_index` 0, 1, 2, **then** the solid path directions are 0°, 90°, 0° and the sparse path direction stays at `infill_direction` on every layer; the same two assertions hold for `gyroid-infill`. | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p gyroid-infill --test gyroid_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** a rectilinear run with `fill_multiline = 3` over the square fixture, **when** the sparse area is scanned, **then** the sparse path count is exactly 3× the `fill_multiline = 1` count, the three copies of each scan line are offset by one line width from one another, the overall line spacing (period between line groups) is unchanged, and the solid path count is identical to the `fill_multiline = 1` run (canonical applies multiline to the sparse role only). | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** a rotate template that is not a comma-separated list of numbers (for example `"0..90:3"`, canonical's unported metalanguage), **when** it is resolved, **then** the module logs one warn naming the key and the unsupported template and falls back to the base angle for every layer — it never silently emits a wrong angle and never panics. | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** the scheduler's config bounds index built from the real manifests, **when** values are resolved, **then** `fill_multiline = 0` and `fill_multiline = 11` are rejected with the numeric `OutOfRange` error, `solid_infill_direction = -1` and `= 361` are rejected with `OutOfRange`, `fill_multiline = "abc"` is rejected with `TypeMismatch`, and `sparse_infill_rotate_template = "0,90"` resolves as a string. | `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** a slice run over the square fixture, **when** the G-code CONFIG_BLOCK is emitted, **then** with an explicit raw-config `fill_multiline = 3` the line `; fill_multiline = 3` appears exactly once (explicit values reach the block through the raw-config sorted dump), and at defaults the block carries zero `fill_multiline` / `solid_infill_direction` / `sparse_infill_rotate_template` / `solid_infill_rotate_template` lines. | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-10. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md` is checked, **then** its generated module-key table carries `solid_infill_direction`, `sparse_infill_rotate_template`, `solid_infill_rotate_template` with owners `rectilinear-infill` and `gyroid-infill`, and `fill_multiline` with owner `rectilinear-infill` only, and the generated deviations block has the same number of data rows as immediately before the packet's manifest edits (re-derive that number from disk at implementation time; do not freeze it). | `cargo xtask gen-config-docs --check && for k in solid_infill_direction sparse_infill_rotate_template solid_infill_rotate_template fill_multiline; do rg -q "$k" docs/15_config_keys_reference.md || exit 9; done; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** the manifest schema guard, **when** any of the seven tables is removed or its `type`/`default`/`min`/`max`/`display`/`group` drifts from AC-1's exact table, **then** the guard fails naming the offending key and manifest. | `cargo test -p rectilinear-infill --test infill_angle_multiline_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the deliberate-omission ruling, **when** `lightning-infill.toml` is parsed, **then** it declares none of the four keys (lightning is a tree-based sparse generator with no scan-line angle and no multiline concept, and holds only `claim:sparse-fill`), and `gyroid-infill.toml` does not declare `fill_multiline`; the omissions are pinned so a future packet that implements either must update the guard. | `cargo test -p rectilinear-infill --test infill_angle_multiline_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** the `ORCA_CONFIG_PADDING` table (`crates/slicer-gcode/src/serialize.rs`), **when** the packet's diff is inspected, **then** it contains zero added, removed, or edited lines — this packet corrects no padding twin and adds none (map Authoring rule 2). | `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs | grep -cE "^[+-][^+-]"` (expect `0`)
- **AC-N4. Given** default configuration (no key supplied), **when** rectilinear and gyroid runs are compared against the pre-packet baseline, **then** the emitted `InfillIR` paths are byte-identical — the four keys change nothing at their canonical defaults. This is an *additional* criterion; it is never the sole evidence for any key. | `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd` and `cargo test -p gyroid-infill --test gyroid_infill_tdd` (primary behaviour contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — both module manifests and sources are guest-fingerprint inputs, so this must return exit 0 before closure.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — `[config.schema]` table shape and the fill-role claim vocabulary.
- `docs/adr/0027-gyroid-multi-role-fill-holder.md` — why gyroid holds four claims and what a solid-role angle means for it.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; angle→offset conversions cross the mm boundary once.
- `docs/15_config_keys_reference.md` — generated; `cargo xtask gen-config-docs` / `--check`, never hand-edited.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — the generated module-key table gains seven rows (`rectilinear-infill` ×4, `gyroid-infill` ×3). The generated deviations block must not change row count; capture the pre-edit count from disk and diff it. The edit lands through `cargo xtask gen-config-docs`. Verification: the AC-10 command.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` and `group_fills` (which role receives `solid_infill_direction` vs `infill_direction`, and that multiline applies to the `erInternalInfill` branch only); `calculate_infill_rotation_angle` (the rotate-template list form and the metalanguage this packet does not port).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `multiline_fill` (the offset-list construction: base spacing × N with N copies at line-width offsets).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `fill_surface_by_multilines` (how the multiline copies are clipped and de-overlapped).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the four key declarations (`solid_infill_direction` coFloat 45; `fill_multiline` coInt 1 min 1 max 10; the two rotate templates coString "").

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
