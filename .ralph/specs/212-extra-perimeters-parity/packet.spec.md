---
status: draft
packet: 212-extra-perimeters-parity
task_ids:
  - TASK-328
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 212-extra-perimeters-parity

## Goal

Make `arachne-perimeters` honour the `extra_perimeters` config key by folding it into the auto-derived `max_bead_count` in `arachne_params_from_config` (`modules/core-modules/arachne-perimeters/src/lib.rs`), so switching `wall_generator` no longer silently discards bonus walls, and close DEV-132's read gap while re-filing its modelling half as its own deviation row.

## Scope Boundaries

In: the arachne read gap (DEV-132 half (a)) — manifest key registration, the `max_bead_count = 2 * (wall_count + extra_perimeters)` derivation, cross-generator equality tests, the `manifest_default_reconcile_tdd` fallback table entry, generated doc tables, and the deviation-ledger split. Out: any per-`Surface`/per-region config plumbing (DEV-132 half (b)); it needs a new per-region field on `SliceRegionView` plus WIT/IR marshalling and is deliberately re-filed as a standalone deviation rather than built here. Out: classic-perimeters behaviour, which already applies the key correctly and must not move.

## Prerequisites and Blockers

- Depends on: nothing. Row #7 of `docs/specs/deviation-remediation-206-212-plan.md` is independent of packets 206-211.
- Unblocks: nothing in this queue. Leaves a newly-filed `DEV-###` row for the per-`Surface` modelling divergence.
- Activation blockers: `TASK-328` is allocated by the approved plan but does not yet exist in `docs/07_implementation_status.md` (verified: the highest `TASK-###` present is `TASK-315`; `TASK-320`/`TASK-321` are claimed by `docs/specs/struct-literal-churn-gate-plan.md`). Step 4 adds the row; this is not a blocker, it is scoped work.

## Acceptance Criteria

- **AC-1. Given** a `ConfigView` with `wall_count = 2`, `extra_perimeters = 2`, no `max_bead_count` key, `inner_wall_line_width = outer_wall_line_width = 1.0`, and a 20 mm square region, **when** `ArachnePerimeters::run_perimeters` is driven at `layer_index = 0`, **then** `output.wall_loops().len()` is exactly `4` (auto-derived `max_bead_count = 2 * (2 + 2) = 8`; `LimitedBeadingStrategy`'s sentinel pair makes the emitted count `max_bead_count / 2` for an even cap). | `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd::arachne_extra_perimeters_bonus_adds_to_wall_count -- --exact 2>&1 | tee target/test-output.log | rg -q '^test result: ok\. 1 passed'`
- **AC-2. Given** the same fixture with `extra_perimeters = 0`, **when** `ArachnePerimeters::run_perimeters` is driven at `layer_index = 0`, **then** `output.wall_loops().len()` is exactly `2` (auto-derived `max_bead_count = 2 * 2 = 4`), proving the bonus is a strict no-op at zero and the pre-packet baseline is unchanged. | `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd::arachne_extra_perimeters_zero_is_noop -- --exact 2>&1 | tee target/test-output.log | rg -q '^test result: ok\. 1 passed'`
- **AC-3. Given** one shared `ConfigView` carrying `wall_count = 2`, `extra_perimeters = 2`, `inner_wall_line_width = outer_wall_line_width = 1.0` and no `max_bead_count`, **when** the same 20 mm square region is run through `ClassicPerimeters::run_perimeters` and `ArachnePerimeters::run_perimeters` at `layer_index = 0`, **then** both emit the same `Outer`/`Inner` wall-loop count (`4`), i.e. switching `wall_generator` no longer drops the bonus — this is the DEV-132 half-(a) assertion, and the assertion is generator-symmetric (`classic_count == arachne_count`) rather than two independent literals. | `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd::extra_perimeters_survives_wall_generator_switch -- --exact 2>&1 | tee target/test-output.log | rg -q '^test result: ok\. 1 passed'`
- **AC-4. Given** `arachne-perimeters.toml`, **when** its `[config.schema.extra_perimeters]` block is inspected, **then** the block exists and carries exactly the four value lines `type = "int"`, `default = 0`, `min = 0`, `max = 10` — the same four the classic manifest's block carries (the classic side is asserted too, so the command fails if either drifts). | `test "$(rg -A6 -N '^\[config\.schema\.extra_perimeters\]$' modules/core-modules/arachne-perimeters/arachne-perimeters.toml | rg -c '^\s*(type\s*=\s*"int"|default\s*=\s*0|min\s*=\s*0|max\s*=\s*10)\s*$')" = "4" && test "$(rg -A6 -N '^\[config\.schema\.extra_perimeters\]$' modules/core-modules/classic-perimeters/classic-perimeters.toml | rg -c '^\s*(type\s*=\s*"int"|default\s*=\s*0|min\s*=\s*0|max\s*=\s*10)\s*$')" = "4" && echo PASS`
- **AC-5. Given** the exhaustive-by-enumeration guard `assert_exhaustive_reconcile` in `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`, **when** `extra_perimeters` is added to `arachne-perimeters.toml`, **then** `ARACHNE_FALLBACKS` contains the row `("extra_perimeters", Int(0))` and the set-equality check passes in both directions. | `mkdir -p target && cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd 2>&1 | tee target/test-output.log | rg -q '^test result: ok\.'`
- **AC-6. Given** the new manifest key, **when** `cargo xtask check-deviations --check` runs, **then** it exits `0`, proving `docs/15_config_keys_reference.md`'s generated table carries an `extra_perimeters` row owned by `arachne-perimeters` and `docs/07_implementation_status.md`'s Open Deviation Map view is in sync with `docs/DEVIATION_LOG.md`. | `cargo xtask check-deviations --check && rg -q '^\| .extra_perimeters. \| int \|.*arachne-perimeters' docs/15_config_keys_reference.md && echo PASS`
- **AC-7. Given** `docs/DEVIATION_LOG.md`, **when** the packet closes, **then** DEV-132's `Status` column names the read gap as closed by this packet AND cites the newly-allocated `DEV-###` row that carries the surviving per-`Surface` modelling divergence forward; the new row's `Rationale` states that canonical's `Surface::extra_perimeters` (`Surface.hpp`) is written only by `PrintObject::make_perimeters` (`PrintObject.cpp`), whose loop body the BBS patch short-circuits with a bare `continue`, so the field is in practice always `0` upstream and per-`Surface` plumbing would port a dead field. | `rg -q 'DEV-132.*[Cc]losed' docs/DEVIATION_LOG.md && rg -q 'Surface::extra_perimeters' docs/DEVIATION_LOG.md && rg -q 'PrintObject::make_perimeters' docs/DEVIATION_LOG.md && echo PASS`
- **AC-8. Given** `docs/07_implementation_status.md`, **when** the packet closes, **then** a `TASK-328` line exists naming the arachne `extra_perimeters` read and is checked off. | `rg -q '^- \[x\] TASK-328' docs/07_implementation_status.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a `ConfigView` with an EXPLICIT `max_bead_count = 4` and `extra_perimeters = 2` on the 20 mm / 1.0 mm-bead fixture, **when** `ArachnePerimeters::run_perimeters` is driven at `layer_index = 0`, **then** `output.wall_loops().len()` is exactly `2` — an explicit positive `max_bead_count` is an advanced override honoured verbatim (the documented contract in `arachne-perimeters.toml`'s `[config.schema.max_bead_count].description`) and `extra_perimeters` is folded ONLY into the auto-derive branch. | `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd::arachne_explicit_max_bead_count_override_ignores_extra_perimeters -- --exact 2>&1 | tee target/test-output.log | rg -q '^test result: ok\. 1 passed'`
- **AC-N2. Given** `alternate_extra_wall = true`, `extra_perimeters = 2`, `wall_count = 2`, no `max_bead_count`, `spiral_vase = false`, `sparse_infill_density = 20.0`, **when** `ArachnePerimeters::run_perimeters` is driven at `layer_index = 1` (odd), **then** `output.wall_loops().len()` is exactly `5` — the `extra_perimeters` fold happens inside `arachne_params_from_config` (before the `params.max_bead_count += 2` bump in `run_perimeters`) and the two compose additively, mirroring classic's ordering where DEV-125's `base_wall_count + 1` guard sits AFTER the `extra_perimeters` addition. | `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd::arachne_extra_perimeters_composes_with_alternate_extra_wall -- --exact 2>&1 | tee target/test-output.log | rg -q '^test result: ok\. 1 passed'`
- **AC-N3. Given** the pre-existing arachne fixture in `modules/core-modules/arachne-perimeters/tests/alternate_extra_wall_tdd.rs`, which pins `max_bead_count = 4` explicitly and sets no `extra_perimeters`, **when** the suite runs after this packet, **then** both of its tests still pass unchanged — this packet must not move any behaviour on the explicit-cap path. | `mkdir -p target && cargo test -p arachne-perimeters --test alternate_extra_wall_tdd 2>&1 | tee target/test-output.log | rg -q '^test result: ok\. 2 passed'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd 2>&1 | tee target/test-output.log | rg -q '^test result: ok\.'`

## Authoritative Docs

- `docs/15_config_keys_reference.md` - GENERATED tables; never hand-edit. Regenerate via `cargo xtask check-deviations`. Read only the `arachne-perimeters` / `classic-perimeters` owner rows around `extra_perimeters`; do not read in full.
- `docs/03_wit_and_manifest.md` - delegated SUMMARY only, and only if the `[config.schema.<key>]` field set for an `int` key is in doubt; `classic-perimeters.toml`'s own block is the working template.
- `docs/DEVIATION_LOG.md` - read row `DEV-132` only (ranged read); it is the source of truth for this packet.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/15_config_keys_reference.md` generated config-key table, `arachne-perimeters` owner rows - `rg -q '^\| .extra_perimeters. \| int \|.*arachne-perimeters' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` DEV-132 row + one newly-allocated `DEV-###` row for the surviving modelling divergence - `rg -q 'DEV-132.*[Cc]losed' docs/DEVIATION_LOG.md`
- `docs/07_implementation_status.md` backlog line for `TASK-328` and the regenerated Open Deviation Map view - `rg -q '^- \[x\] TASK-328' docs/07_implementation_status.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — the `int loop_number = this->config->wall_loops + surface.extra_perimeters - 1;` fold, present in BOTH `PerimeterGenerator::process_classic` and `PerimeterGenerator::process_arachne` (they differ only in whitespace before the trailing comment); this is the behaviour being mirrored into arachne.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` — `WallToolPaths::generate`'s `max_bead_count = 2 * inset_count` relation, and `process_arachne`'s `WallToolPaths(..., coord_t(loop_number + 1), ...)` constructor call: together they give `max_bead_count = 2 * (wall_loops + extra_perimeters)`, the unit relation this packet ports.
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` and `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `Surface::extra_perimeters` (`unsigned short`, zero-initialised in every ctor) and its sole in-code writer `PrintObject::make_perimeters`; deliberately NOT borrowed, because that writer's loop body is short-circuited by the BBS patch, making the field effectively always `0` upstream.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — evidence of ABSENCE: `PrintConfigDef::init_fff_params` has no `add("extra_perimeters", ...)`; only `add("extra_perimeters_on_overhangs", coBool)`. PnP's `extra_perimeters` config key has no canonical config-option counterpart.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
