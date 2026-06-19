---
status: draft
packet: 118
task_ids:
  - TASK-253
  - TASK-163b-diagnostic
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-planner-typed-diagnostics

## Goal

Add a typed `Diagnostic` record + `severity-level` enum to `world-prepass.wit`, plumb a guest-side emitter through the prepass output-builder API in `slicer-sdk`, surface diagnostics into the per-stage prepass audit on the host side, and migrate `support-planner`'s three string-prefixed `log(LogLevel::Warn, …)` call sites (`node-clamped-out`, the new `max_branches_per_layer` cap exceeded counter from this packet, and the not-yet-implemented `support_interface_bottom_layers` warning) to emit typed `Diagnostic` values instead.

## Scope Boundaries

Touches one WIT file, the prepass SDK helper crate, the host-side prepass execution code path that owns diagnostic collection, and `support-planner/src/lib.rs`. Every guest WASM is rebuilt (a WIT type was added). No IR shape change; no scheduler rule change; the existing `host-services.log` plumbing is preserved for non-diagnostic uses. The 1024-contact cap retains its current behavior — only the silent drop becomes a typed `Diagnostic` event.

## Prerequisites and Blockers

- Depends on: packet `116_support-modules-doc-honesty-cleanup` should land first because it introduces the `support_interface_bottom_layers` warning as a string log; this packet migrates it. If doc-honesty has not landed, this packet still works but loses one of the three migration targets (the cap exceeded warning and `node-clamped-out` are sufficient to validate the channel).
- Unblocks: `119_support-validation-wedge-harness` packet (whose invariant tests assert that no `LogLevel::Warn` strings prefixed `support-planner.node-clamped-out:` remain).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`, **when** parsed, **then** the file declares `record diagnostic { severity: severity-level, code: u32, layer: option<s32>, object-id: option<string>, message: string }` and `enum severity-level { trace, debug, info, warn, error }`. | `cargo xtask build-guests --check 2>&1 | tee target/test-output.log && rg -q 'record diagnostic' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit && rg -q 'enum severity-level' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`
- **AC-2. Given** the new WIT types, **when** all 20 guest manifests are rebuilt, **then** every guest's bindgen output compiles and the `wit_drift_detection_tdd` test reports the new `diagnostic` and `severity-level` types present and consistent. | `cargo test -p slicer-runtime --test wit_drift_detection_tdd 2>&1 | tee target/test-output.log`
- **AC-3. Given** the prepass SDK, **when** a guest calls `output.push_diagnostic(Diagnostic { severity: Warn, code: 1001, layer: Some(-1), object_id: Some("cube".into()), message: "test".into() })`, **then** the host-side prepass execution audit for that stage contains exactly one `Diagnostic` with the same field values. | `cargo test -p slicer-runtime --test prepass_diagnostic_roundtrip_tdd 2>&1 | tee target/test-output.log`
- **AC-4. Given** `support-planner` running on a fixture that triggers the existing `node-clamped-out` path (a node whose move target lies inside `collision_polys`), **when** the prepass executes, **then** the host's diagnostic audit contains at least one `Diagnostic` with `code` in the `support-planner` range (1000-1999), `severity == Warn`, `message` containing `node-clamped-out`. | `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd -- node_clamped_out_emits_typed_diagnostic --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `support-planner` running on a synthetic overhang fixture that produces > 1024 contact candidates at one layer, **when** the prepass executes, **then** the host's diagnostic audit contains exactly one `Diagnostic` per layer where the cap fires, with `severity == Warn`, `code` in `1000-1999`, `message` containing `max_branches_per_layer cap exceeded`, `dropped_count > 0`, `kept_count == 1024`. | `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd -- max_branches_cap_emits_typed_diagnostic_per_layer --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** `support-planner::on_print_start` called with `support_interface_bottom_layers = Int(3)`, **when** the prepass setup phase completes, **then** the host's diagnostic audit contains exactly one `Diagnostic` with `severity == Warn`, `code` in `1000-1999`, `message` containing `support_interface_bottom_layers is not yet implemented`. | `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd -- interface_bottom_layers_emits_typed_diagnostic --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** the updated `support-planner/src/lib.rs`, **when** searched for the three legacy string prefixes (`support-planner.node-clamped-out:`, `support-planner: max_branches_per_layer cap exceeded`, `support-planner: support_interface_bottom_layers is not yet implemented`), **then** none of them appear (they have all been migrated to typed `Diagnostic` emissions). | `! rg -q 'support-planner\.node-clamped-out:' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'support-planner: max_branches_per_layer' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'support-planner: support_interface_bottom_layers' modules/core-modules/support-planner/src/lib.rs`

## Negative Test Cases

- **AC-N1. Given** `support-planner` running on a fixture whose contact count stays well below 1024 at every layer, **when** the prepass executes, **then** zero `Diagnostic` values are emitted with the `max_branches_per_layer cap exceeded` message. | `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd -- below_cap_emits_no_cap_diagnostic --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** the new WIT types, **when** a guest emits a `Diagnostic` with `code = 99` (outside any module's allocated range), **then** the prepass execution does NOT reject the emission (code-range allocation is convention, not enforced) — but the diagnostic IS captured into the audit with `code = 99`. | `cargo test -p slicer-runtime --test prepass_diagnostic_roundtrip_tdd -- out_of_range_code_still_captured --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p slicer-runtime --test wit_drift_detection_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §B4, §B7, §D10, §D11 — message wording and structured-fields layout.
- `docs/adr/0010-typed-diagnostic-channel.md` — read directly (≤ 100 lines). Authoritative source for the WIT shape, the per-module code-range convention (`support-planner: 1000-1999`), and the variant-vs-record rationale.
- `docs/03_wit_and_manifest.md` — > 300 lines; delegate a SUMMARY of "how to add a new record/enum to a world's deps/* file" (≤ 200 words).
- `CLAUDE.md` — read §"WIT/Type Changes Checklist" directly (≤ 30 lines).

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` §"Diagnostic" — append a section documenting the new typed channel under "Prepass Execution Audit" or a new top-level "IR 12 — Diagnostic" section depending on where the editor decides it fits. Verification: `rg -q 'record diagnostic' docs/02_ir_schemas.md` AND `rg -q 'severity-level' docs/02_ir_schemas.md`.
- `docs/03_wit_and_manifest.md` §"world-prepass deps" — note the new types added by this packet, plus the implicit guest-rebuild ceremony invocation. Verification: `rg -q 'record diagnostic' docs/03_wit_and_manifest.md`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
