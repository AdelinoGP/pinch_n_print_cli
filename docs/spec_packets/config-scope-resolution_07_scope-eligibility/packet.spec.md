---
status: draft
packet: config-scope-resolution_07_scope-eligibility
task_ids:
  - TASK-568
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: scope-eligibility

## Goal

Make per-key `denied_scopes` the sole scope-admission authority, hand-author the initial machine/emitter denial roster across host and module declarations, pin it against a mechanical reachability derivation, and remove the two inert manifest allow-list sections.

## Scope Boundaries

This packet owns eligibility declaration, registry-derived admission sets, loud resolver rejection, the author-confirmed initial roster, its derivation/drift test, and removal of `[config.overridable-per-region]` plus `[config.overridable-per-layer]` from all 24 core manifests and manifest ingestion. It does not add layer-range ingestion or silently generate denials at runtime.

## Prerequisites and Blockers

- Depends on: packet 05, `config-scope-resolution_05_scope-resolution-module`, for `resolve_scope_stack` and `ResolutionError`.
- Also consumes landed packet 01's `RegistryEntry.denied_scopes` union/validation and `HostRuntimeKey.denied_scopes` contract.
- Unblocks: queue row 8 modifier enforcement and queue row 9 layer-range selector/machine rejection.
- Activation blockers: packet 05 must land and its resolver/error shape must be reconciled.

## Compatibility Checklist

- IR schema versions: unchanged; declaration metadata is not serialized IR.
- WIT packages: unchanged; no WIT edit.
- CLI schema wire: unchanged; `denied_scopes` remains registry/manifest metadata under packet 01's policy.
- Manifest schema: removes the two required section tables; per-field optional snake_case `denied_scopes` is the only eligibility declaration.
- Runtime behavior: a denied statement becomes a load/resolution error rather than accepted-and-ignored.

## Acceptance Criteria

- **AC-1. Given** the four declaration channels and all core manifests, **when** the initial eligibility roster is assembled, **then** every declarer carries the author-confirmed denials and registry union preserves them: (a) whole-print-only keys deny exactly `object`, `layer_range`, `modifier`, `paint_semantic`, `tool`: all 26 `SPEED_KEYS`; `bed_shape`; `gcode_xy_decimals`; `disable_m73`; `machine_max_acceleration_extruding`; `machine_max_acceleration_travel`; `machine_max_speed_x`; `machine_max_speed_y`; `machine_max_speed_z`; `machine_max_speed_e`; `machine_max_jerk_x`; `machine_max_jerk_y`; `machine_max_jerk_z`; `machine_max_jerk_e`; `use_relative_e_distances`; `thumbnail_path`; and existing `wall_generator`; (b) tool-capable machine/filament keys `retract_length`, `filament_diameter`, `filament_density`, and `nozzle_diameter` deny exactly `object`, `layer_range`, `modifier`, `paint_semantic` and allow `tool`; all other keys retain authored/default-empty denials. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_eligibility_tdd authored_denial_roster_is_exact_across_all_declarers -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test authored_denial_roster_is_exact_across_all_declarers .* ok" target/test-output.log'`
- **AC-2. Given** the live registry, host channels, module manifests, stage/granularity metadata, and packet-05 delivery matrix, **when** the test-only mechanical derivation computes every `(key, sub-print scope)` pair no consumer can reach, **then** its proposed set equals the hand-authored registry denials for the AC-1 roster; the implementation never copies the proposal into runtime declarations and an added/removed proposal fails the drift test until an author explicitly updates or rejects the declaration. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_eligibility_tdd mechanical_derivation_matches_author_confirmed_denials -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test mechanical_derivation_matches_author_confirmed_denials .* ok" target/test-output.log'`
- **AC-3. Given** the assembled registry and any `ConfigScope`, **when** `ConfigSchemaRegistry::admission_set(scope)` is queried, **then** it returns exactly the registry keys whose `denied_scopes` omit that scope; in particular object admission excludes every AC-1 key while tool admission includes exactly the four tool-capable keys from that roster and excludes its whole-print-only keys. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_eligibility_tdd admission_sets_are_derived_only_from_registry_denials -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test admission_sets_are_derived_only_from_registry_denials .* ok" target/test-output.log'`
- **AC-4. Given** a declared key allowed at a scope, **when** `resolve_scope_stack` processes the delta, **then** it applies that value normally; `retract_length` and `nozzle_diameter` are accepted at tool scope and an ordinary key with no denials is accepted at object, modifier, paint-semantic, and tool scopes. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_eligibility_tdd allowed_scope_values_resolve_normally -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test allowed_scope_values_resolve_normally .* ok" target/test-output.log'`
- **AC-5. Given** all 24 core-module manifests, **when** manifest ingestion runs after retirement, **then** none contains `[config.overridable-per-region]` or `[config.overridable-per-layer]`, `ingest_manifest_text` no longer requires/parses either path, `LoadedModule` and its builder expose neither list, and the former parse round-trip assertions are replaced by per-field `denied_scopes` parsing/round-trip coverage. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-scheduler --all-targets --test scheduler_integration manifest_ingestion_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log; ! rg -n "config\.overridable-per-(region|layer)|overridable_per_(region|layer)" modules/core-modules crates/slicer-scheduler/src crates/slicer-scheduler/tests --glob "*.toml" --glob "*.rs"'`
- **AC-6. Given** eligibility implementation is complete, **when** docs are inspected, **then** `docs/03_wit_and_manifest.md` documents per-field `denied_scopes`, absent-means-all-scopes, and contains neither retired table name, while `docs/04_host_scheduler.md` names `ConfigSchemaRegistry::admission_set` and `ResolutionError::ScopeDenied`. | `python3 -c "from pathlib import Path; a=Path('docs/03_wit_and_manifest.md').read_text(); b=Path('docs/04_host_scheduler.md').read_text(); req=('denied_scopes','absent','every scope'); missing=[x for x in req if x not in a]; forbidden=[x for x in ('overridable-per-region','overridable-per-layer') if x in a]; assert not missing and not forbidden and 'ConfigSchemaRegistry::admission_set' in b and 'ResolutionError::ScopeDenied' in b,(missing,forbidden)"`

## Negative Test Cases

- **AC-N1. Given** `bed_shape` is stated at object scope or `nozzle_diameter` is stated at modifier scope, **when** `resolve_scope_stack` validates the applicable delta, **then** it returns `ResolutionError::ScopeDenied { key, scope }` naming the exact pair and produces no partially resolved config. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_eligibility_tdd denied_scope_is_rejected_atomically -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test denied_scope_is_rejected_atomically .* ok" target/test-output.log'`
- **AC-N2. Given** a module declares a denial and a second declarer omits it, **when** the registry reconciles and admission is queried, **then** the union still denies that scope and resolution rejects the value; discovery/declaration order cannot make it admissible. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_eligibility_tdd multi_declarer_denial_union_is_order_independent -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test multi_declarer_denial_union_is_order_independent .* ok" target/test-output.log'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_eligibility_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'`

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — Eligibility, Resolution rejection, Selector keys, RC-9, and queue row 7.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — deny-list semantics, permissive default, union, and retired allow lists.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, and `docs/04_host_scheduler.md` — config scope, manifest schema, and resolver ownership.
- `docs/22_test_quality.md` — independent derivation and drift-test quality.

## Doc Impact Statement (Required)

- `docs/03_wit_and_manifest.md` — remove the two inert sections and document sole per-key eligibility; verified by `AC-6`.
- `docs/04_host_scheduler.md` — document registry-derived admission and loud rejection; verified by `AC-6`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
