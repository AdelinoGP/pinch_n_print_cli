# Requirements: support-planner-typed-diagnostics

## Packet Metadata

- Grouped task IDs:
  - `TASK-253` — `1024-contact` silent-truncation diagnostic (B4 from `docs/specs/support-modules-orca-port.md`)
  - `TASK-163b-diagnostic` — typed `Diagnostic` channel on `world-prepass` (B7 + ADR-0010)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Three `support-planner` warnings — `node-clamped-out` (line 633), `support_interface_bottom_layers is not yet implemented` (introduced by sibling packet `116_support-modules-doc-honesty-cleanup`), and a new `max_branches_per_layer cap exceeded` event introduced by this packet — currently use `slicer_sdk::host::log(LogLevel::Warn, &format!("..."))` with structured payload encoded in string prefixes. Downstream consumers (the slicer report, CI assertions, GUI surfaces) can read this data only by parsing prefix strings.

The 1024-contact cap (`support-planner/src/lib.rs:326`, `:341`, `:434`) silently truncates contact lists today — drops are not even logged. Any user running a model with a dense overhang loses contact points without signal.

This packet adds a typed `Diagnostic` channel to `world-prepass.wit`, plumbs it through the prepass SDK output builder and the host's prepass execution audit, and migrates the three call sites to emit typed records. The 1024-cap call sites change from silent `continue` / `truncate` to incrementing a per-layer counter and emitting a single `Diagnostic` per layer when the counter is non-zero.

This packet closes both gaps in one slice because they share the same code surface (`support-planner/src/lib.rs`'s warning paths) and the same infrastructure (the new `Diagnostic` channel). Doing them separately would require shipping the cap diagnostic on the old string channel, then re-migrating it — wasted work.

## In Scope

- Add `record diagnostic` and `enum severity-level` to `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` per ADR-0010.
- Add a `push-diagnostic: func(d: diagnostic)` method to the prepass output-builder WIT interface.
- Update `slicer-sdk`'s prepass output-builder Rust API to expose `push_diagnostic(Diagnostic { ... })` with a `Diagnostic` struct mirroring the WIT record.
- Update the host-side prepass execution code (in `crates/slicer-runtime/src/prepass.rs` or its diagnostic-collection helper) to collect emitted `Diagnostic` values into a per-stage `Vec<Diagnostic>` exposed on the prepass execution audit.
- Add a per-layer counter in `support-planner::plan_for_object` that tracks how many contact candidates were dropped by the 1024-cap path at line 326, 341, and 434.
- Emit a single `Diagnostic { severity: Warn, code: 1001 (allocated), layer: Some(...), object_id: Some(...), message: "support-planner: max_branches_per_layer cap exceeded — dropped {dropped_count}, kept {kept_count}" }` per layer where the counter is non-zero, NOT per drop.
- Migrate the `support-planner.node-clamped-out` `log(LogLevel::Warn, ...)` call site at line 633 to emit a `Diagnostic { code: 1002 (allocated), severity: Warn, ... }`.
- Migrate the `support_interface_bottom_layers` warning from sibling packet `116_support-modules-doc-honesty-cleanup` to emit a `Diagnostic { code: 1003 (allocated), severity: Warn, ... }`.
- Add `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs` covering AC-3 and AC-N2.
- Add `crates/slicer-runtime/tests/integration/support_planner_diagnostic_emission_tdd.rs` covering AC-4, AC-5, AC-6, AC-N1.
- Update `docs/02_ir_schemas.md` with the new `Diagnostic` section per Doc Impact Statement.
- Update `docs/03_wit_and_manifest.md` with the new types per Doc Impact Statement.

## Out of Scope

- Migrating all `log(...)` calls workspace-wide to `Diagnostic`. The channel is for *diagnostic events with structured payload*; routine trace/debug logging stays on `host-services.log` (per ADR-0010 §"Future-Reviewer Notes").
- A central registry for `code: u32` allocation. The per-module range convention (`support-planner: 1000-1999`) is documented in ADR-0010 but not enforced by host code.
- Diagnostic emission from other modules (`tree-support`, `traditional-support`, `raft-default`, infill modules, etc.). They can adopt the channel later without WIT changes.
- Diagnostic propagation into a real-time GUI surface — only the per-stage audit collection lands in this packet.
- Changing the 1024 cap value to a higher number, or making it configurable. The cap itself stays exactly as is; only the silent-drop becomes a typed event.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` — §B4 (≈10 lines), §B7 (≈30 lines), §D10 (≈5 lines), §D11 (≈5 lines). Read directly.
- `docs/adr/0010-typed-diagnostic-channel.md` — ≈90 lines; read directly. Source of WIT shape + code-range convention.
- `docs/03_wit_and_manifest.md` — > 300 lines; delegate SUMMARY of "how a new record/enum is added to a world's deps/* file" (≤ 200 words) and how the canonical-WIT validation tests are organized.
- `CLAUDE.md` — read §"WIT/Type Changes Checklist" + §"Guest WASM Staleness" (≤ 60 lines combined).
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — read fully (≤ 200 lines). This is the file being edited.
- `crates/slicer-sdk/src/lib.rs` — locate the prepass output-builder; range-read the relevant impl (delegate `LOCATIONS` if > 300 lines).
- `crates/slicer-runtime/src/prepass.rs` — locate the per-stage audit struct + commit path; range-read (≤ 100 lines around the relevant impl).

## Acceptance Summary

- Positive cases: `AC-1` through `AC-7` from `packet.spec.md`.
  - `AC-1` and `AC-2` gate the WIT additions and bindgen consistency.
  - `AC-3` is the SDK round-trip: guest → host audit.
  - `AC-4`, `AC-5`, `AC-6` exercise the three migrated call sites with synthetic fixtures.
  - `AC-7` is a static grep: legacy string prefixes are gone from `support-planner/src/lib.rs`.
- Negative cases: `AC-N1` (no diagnostic when below cap) and `AC-N2` (out-of-range code accepted at the channel boundary; range convention is convention, not enforcement).
- Cross-packet impact: this packet's typed channel becomes the canonical recoverable-warning surface for `support-planner`. Sibling packet `119_support-validation-wedge-harness` asserts via its invariant tests that the planner emits the right `Diagnostic` for `node-clamped-out` events on the wedge fixture (the harness reads from the per-stage audit).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check 2>&1 \| tail -5` | All 20 guests rebuilt and current after WIT change. | FACT pass/fail |
| `cargo build --workspace` | Workspace compiles end-to-end after WIT + SDK + host changes. | FACT pass/fail |
| `cargo test -p slicer-runtime --test wit_drift_detection_tdd 2>&1 \| tee target/test-output.log` | AC-2: bindgen consistency for the new types. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test prepass_diagnostic_roundtrip_tdd 2>&1 \| tee target/test-output.log` | AC-3, AC-N2. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd 2>&1 \| tee target/test-output.log` | AC-4, AC-5, AC-6, AC-N1. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `! rg -q 'support-planner\.node-clamped-out:' modules/core-modules/support-planner/src/lib.rs` | AC-7: legacy `node-clamped-out` string gone. | FACT pass/fail |
| `! rg -q 'support-planner: max_branches_per_layer' modules/core-modules/support-planner/src/lib.rs` | AC-7: legacy cap-exceeded string gone. | FACT pass/fail |
| `! rg -q 'support-planner: support_interface_bottom_layers' modules/core-modules/support-planner/src/lib.rs` | AC-7: legacy bottom-layers string gone. | FACT pass/fail |
| `rg -q 'record diagnostic' docs/02_ir_schemas.md && rg -q 'severity-level' docs/02_ir_schemas.md` | Doc Impact: docs/02 updated. | FACT pass/fail |
| `rg -q 'record diagnostic' docs/03_wit_and_manifest.md` | Doc Impact: docs/03 updated. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint gate. | FACT pass/fail |

## Step Completion Expectations

- The WIT change (Step 2) triggers a full `cargo xtask build-guests` rebuild of all 20 guests. Steps 3-6 MUST NOT be attempted until Step 2's guest rebuild completes successfully — Rust compile of any guest using the new types depends on the bindgen output being fresh.
- The host-side audit collection (Step 4) MUST preserve diagnostic *order* (emissions appear in `Vec<Diagnostic>` in the order the guest emitted them). AC-3 specifically asserts a one-to-one round-trip; an unordered collection (e.g., `HashSet`) would break this implicitly.
- The new tests in Step 7 (`support_planner_diagnostic_emission_tdd`) require synthetic fixtures that hit the cap (AC-5) and stay below it (AC-N1). The implementer authors a small fixture builder that produces a configurable number of overhang facets; both the positive cap fixture (>1024 facets at one layer) and the below-cap fixture (<100 facets per layer) reuse it.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `docs/03_wit_and_manifest.md` — delegate SUMMARY of the "add new type to world deps" section. Do NOT read end-to-end.
  - `crates/slicer-sdk/src/lib.rs` — delegate `LOCATIONS` for the prepass output-builder impl; range-read the located impl body.
  - `crates/slicer-runtime/src/prepass.rs` — range-read the per-stage audit struct + commit path (≤ 100 lines around).
  - `modules/core-modules/support-planner/src/lib.rs` — 1,000+ lines; range-read around the three migration sites (lines 326-341 cap path, line 434 cap path, line 633 node-clamped-out, plus wherever the doc-honesty packet inserted the bottom-layers warning).
- Likely temptation reads (skip these):
  - Other modules that use `host::log` — out of scope; the channel migration is `support-planner`-local in this packet.
  - All 20 guest manifests — bindgen handles the regeneration; the implementer only needs to confirm `cargo xtask build-guests --check` returns clean.
  - `OrcaSlicerDocumented/**` — no Orca behavior is being ported here.
- Sub-agent return-format hints for heaviest dispatches:
  - `cargo xtask build-guests --check` — FACT (`up to date` or `STALE: <list>`); the dispatch MUST NOT paste the full guest-rebuild log (can be thousands of lines).
  - `cargo build --workspace` (post WIT change) — FACT pass/fail; on fail, SNIPPETS ≤ 30 lines with the first error.
  - WIT-bindgen diffs — FACT (number of changed bindings) + SNIPPETS ≤ 30 lines of the most relevant generated-code change.
