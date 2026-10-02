# Requirements: scope-eligibility

## Packet Metadata

- Grouped task IDs: `TASK-568`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 01 provides validated, unioned `denied_scopes`, but broad host rows remain empty except `wall_generator`, module declarations do not yet carry the initial machine/emitter policy, and packet 05's resolver has no admission gate. Meanwhile two mandatory module-level allow-list sections remain in every core manifest even though 23 are empty and production never consumes either list. Eligibility therefore exists as metadata without complete authorship or enforcement.

## In Scope

- Reconcile packet 05's final `resolve_scope_stack` and `ResolutionError` shapes.
- Add `denied_scopes` to `HostConfigKey` and the host DSL output while preserving `HostRuntimeKey`'s existing const-safe field.
- Carry explicit denials for all 26 `SPEED_KEYS`/`SPEED_META` entries without breaking their positional alignment.
- Hand-author the exact AC-1 roster on every host and module schema declarer; use the union rule, but do not rely on one declarer to conceal another's missing authorship.
- Treat `use_relative_e_distances`, `thumbnail_path`, `wall_generator`, the 26 speed keys, `bed_shape`, `gcode_xy_decimals`, `disable_m73`, and ten named `machine_max_*` keys as whole-print-only.
- Treat `retract_length`, `filament_diameter`, `filament_density`, and `nozzle_diameter` as tool-capable but denied at object/layer-range/modifier/paint-semantic scopes.
- Add `ConfigSchemaRegistry::admission_set(scope)` and use registry output as the only per-object and other scope admission source.
- Extend packet 05's resolver error with `ResolutionError::ScopeDenied { key, scope }` and reject atomically before merge/expansion.
- Build a test-only mechanical proposal from registry declarations, host consumer channels, module stage/granularity metadata, and the resolver delivery matrix; compare it to hand-authored denials as a drift test.
- Delete both legacy section tables from all 24 manifests, `LoadedModule`, its builder, `ingest_manifest_text`, and parse tests.
- Update manifest and scheduler docs.

## Out of Scope

- Runtime auto-generation or mutation of `denied_scopes` from the mechanical proposal.
- Denying ordinary geometry/quality keys merely because current code does not yet consume an override.
- Layer-range file ingestion, overlap geometry, catch-up behavior, or XML parsing (packet 09).
- Modifier-kind typing (packet 08) and resolved-view/fallback/emission work (packet 06).
- Config aliases, key renames, schema-version bumps, or WIT changes.
- Changing packet 01's union rule or permissive absent-means-allowed default.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — bounded Eligibility, Resolution, Selector, and RC-9 sections.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — direct short read; normative policy and retirement.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` — ranged config-scope/manifest/resolver sections.
- `docs/22_test_quality.md` — independent-oracle and drift-test constraints.

## Acceptance Summary

- Positive: `AC-1` through `AC-6` in `packet.spec.md` cover the exact roster, independent derivation, registry admission sets, allowed values, inert-surface deletion, and docs.
- Negative: `AC-N1` and `AC-N2` cover atomic denied-scope rejection and order-independent union enforcement.
- Cross-packet impact: packet 08 routes modifier deltas through this gate; packet 09 uses it for layer-range selector/machine rejection.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_eligibility_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Roster, derivation, admission, and rejection | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-scheduler --all-targets --test scheduler_integration manifest_ingestion_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Manifest retirement/parser behavior | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo xtask build-guests --check` | Artifact-verified guest freshness after slicer-ir/manifest edits | FACT exit code and stale names only |
| `cargo check --workspace --all-targets` | All-target type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Host declaration struct-literal gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Touched-test quality | FACT touched findings only |

## Step Completion Expectations

- The mechanical derivation proposes; authored declarations remain the production source of truth.
- Extend host carriers/DSL and update their complete struct-literal blast radius before authoring the roster.
- Delete parser/model surfaces and all 48 manifest section headers in one coherent retirement sequence; do not leave optional dead aliases.

## Context Discipline Notes

The roster spans declaration channels and 24 manifests. Use bounded generated location lists and ≤3-file edit batches; do not read every manifest in full.
