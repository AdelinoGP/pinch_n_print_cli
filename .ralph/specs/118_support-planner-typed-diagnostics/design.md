# Design: support-planner-typed-diagnostics

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — add `record diagnostic` + `enum severity-level` + `push-diagnostic: func(d: diagnostic)` on the prepass output-builder interface.
  - `crates/slicer-sdk/src/lib.rs` (or its `prepass_builders` submodule — confirm exact path via dispatch) — `Diagnostic` Rust struct + `push_diagnostic(&mut self, d: Diagnostic)` impl on the prepass output-builder type.
  - `crates/slicer-runtime/src/prepass.rs` — per-stage audit struct gains a `diagnostics: Vec<Diagnostic>` field; the commit/drain path collects guest-emitted diagnostics into it.
  - `modules/core-modules/support-planner/src/lib.rs` — three call sites migrated; one new counter + emission for the 1024 cap path.
- Neighboring tests/fixtures:
  - `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` — extended to assert the new `diagnostic` record and `severity-level` enum.
  - `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs` — new file.
  - `crates/slicer-runtime/tests/integration/support_planner_diagnostic_emission_tdd.rs` — new file.
- OrcaSlicer comparison surface: not consulted by this packet. No Orca behavior is being ported.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- The WIT change is **additive** (new record + new enum + new method on the output-builder interface). Existing guests that don't call `push-diagnostic` continue to work; only `support-planner` adopts it in this packet.
- Per ADR-0010 §"Why a record + enum, not a variant": `Diagnostic` is a record (open code-space via `code: u32`), not a variant. Adding a new diagnostic class is a module-internal change, not a WIT change. The implementer MUST NOT introduce a `variant diagnostic { ... }` form.
- Per ADR-0010 §"Field semantics": `layer: option<s32>` (NOT `option<u32>`) so future raft-default-module work can emit diagnostics with negative raft layer indices without a WIT-schema bump.
- Per ADR-0010 §"code allocation": `support-planner` uses range `1000-1999`. This packet allocates `1001` (max-branches-cap), `1002` (node-clamped-out), `1003` (interface-bottom-layers).
- Diagnostic emission is recoverable. `ModuleError::fatal(...)` remains the abort path for unrecoverable errors. The implementer MUST NOT use `Diagnostic` as a substitute for `ModuleError`.

## Code Change Surface

- Selected approach:
  - WIT additions are minimal and additive (avoid the temptation to also add `tracing`-style fields like `span` or `target` — those are future evolutions, not B7 scope).
  - The host audit struct gains one new vector field with a stable ordering guarantee.
  - `support-planner`'s three migration sites use a small helper closure or inline calls; no abstraction layer is introduced (overkill for three call sites).
  - The 1024-cap path retains its `continue` / `truncate` behavior at the data-flow level; only the *counter increment + post-loop emission* is added.
- Exact functions/structs/manifests/tests to change:
  - `world-prepass.wit` — top-level deps file gains `record diagnostic`, `enum severity-level`; the world's output interface gains `push-diagnostic`.
  - `slicer-sdk::prepass_builders::SupportGeometryOutput` (or the relevant output-builder type) — `push_diagnostic` impl + a re-exported `Diagnostic` struct.
  - `crates/slicer-runtime/src/prepass.rs::PrepassStageAudit` (or whatever the audit struct is named) — new `diagnostics: Vec<Diagnostic>` field.
  - `crates/slicer-runtime/src/prepass.rs` commit/drain path — collect diagnostics from the guest's audit return.
  - `support_planner::plan_for_object` — three migration sites + one new per-layer counter + one new emission point at the end of each layer's contact-collection loop.
- Rejected alternatives:
  - **Variant `diagnostic` with per-class arms** — rejected per ADR-0010 §"Why a record + enum, not a variant". Adding a new diagnostic class would require a WIT change.
  - **Bundling B4's cap diagnostic on the old string channel and migrating later** — rejected: doubles the migration work and ships a known-temporary string for one packet.
  - **A central `code: u32` registry crate** — rejected per ADR-0010 §"code allocation is module-allocated, not centrally registered". If collisions become a problem in practice, a registry can land later.
  - **Adding `Diagnostic` emission to `tree-support` and `traditional-support` in this packet** — rejected: scope creep. The channel works; other modules adopt later.

## Files in Scope (read + edit)

The packet edits 5 source files plus 2 new test files (7 total). This exceeds the soft `≤ 3` ceiling; the work is justified because the channel is structurally end-to-end (WIT → SDK → host audit → guest call sites → tests) and cannot be partially landed without leaving the workspace in a half-typed state.

- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — role: new types + method; expected change: ≈10 lines added.
- `crates/slicer-sdk/src/lib.rs` (or `src/prepass_builders/...`) — role: SDK helper API; expected change: `Diagnostic` struct + `push_diagnostic` impl (≈40 lines).
- `crates/slicer-runtime/src/prepass.rs` — role: host-side collection + audit; expected change: audit struct field + drain logic (≈20 lines).
- `modules/core-modules/support-planner/src/lib.rs` — role: three migration sites + per-layer counter; expected change: counter, emission points, and three replaced `log(...)` calls (≈30 lines net).
- `docs/02_ir_schemas.md` — Doc Impact: new `Diagnostic` section.
- `docs/03_wit_and_manifest.md` — Doc Impact: note the new WIT types.
- `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs` — new file.
- `crates/slicer-runtime/tests/integration/support_planner_diagnostic_emission_tdd.rs` — new file.

## Read-Only Context

- `docs/adr/0010-typed-diagnostic-channel.md` — read fully (≈90 lines). Source of WIT shape + code allocation rationale.
- `CLAUDE.md` — read §"WIT/Type Changes Checklist" + §"Guest WASM Staleness" only.
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — read fully before editing (≤ 200 lines).
- `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` — confirm the assertion pattern used for existing types; the new types follow the same pattern.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted by this packet.
- `target/`, `Cargo.lock`, generated bindgen output — never load directly; rely on `cargo xtask build-guests --check` for staleness verification.
- All 20 guest crates outside `support-planner` — out of scope; do not browse to "see how their bindgen will react." Rely on `cargo build --workspace` post-WIT to surface any breakage.
- `OrcaSlicerDocumented/**` — no Orca behavior being ported.
- The full body of `modules/core-modules/support-planner/src/lib.rs` outside the three migration sites (line 326-434 region; line 633 region; and the doc-honesty packet's `on_print_start` insertion region) — range-read only.

## Expected Sub-Agent Dispatches

- "Summarize `docs/03_wit_and_manifest.md` §'how to add a new type to a world's deps/* file'; return SUMMARY ≤ 200 words. No code unless asked." — purpose: confirm the conventional shape of the WIT addition.
- "Locate the prepass output-builder impl in `crates/slicer-sdk/src/`; return LOCATIONS (file:line + 1-line context, ≤ 10 entries) for the type definition and impl block." — purpose: find the right edit target in the SDK.
- "Locate `PrepassStageAudit` (or equivalent) in `crates/slicer-runtime/src/prepass.rs`; return SNIPPETS ≤ 30 lines showing the struct definition + the commit path." — purpose: identify where the `diagnostics: Vec<Diagnostic>` field lands.
- "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <list of guests>`). Do NOT paste the rebuild log." — purpose: guest-rebuild ceremony.
- "Run `cargo build --workspace`; return FACT pass/fail; on fail SNIPPETS ≤ 30 lines with the FIRST error only." — purpose: post-WIT compile gate.
- "Run `cargo test -p slicer-runtime --test wit_drift_detection_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: bindgen drift gate.
- "Run `cargo test -p slicer-runtime --test prepass_diagnostic_roundtrip_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: AC-3, AC-N2.
- "Run `cargo test -p slicer-runtime --test support_planner_diagnostic_emission_tdd`; return FACT pass/fail; SNIPPETS ≤ 30 lines on failure." — purpose: AC-4, AC-5, AC-6, AC-N1.

## Data and Contract Notes

- IR or manifest contracts touched: the prepass output-builder WIT interface gains one method. No IR struct changes.
- WIT boundary considerations: `s32` for `layer` (signed; not `u32`) is required for ADR-0010 forward-compatibility with negative raft layer indices.
- Determinism: emission order MUST be preserved in the host audit (FIFO). The collection is `Vec<Diagnostic>`, not a set.
- The `code: u32` field is not validated at the WIT boundary; AC-N2 asserts the channel accepts any value. Convention is documented in ADR-0010 + the migrated call sites.

## Locked Assumptions and Invariants

- `Diagnostic` is recoverable. `ModuleError::fatal(...)` remains the abort path. This invariant MUST be preserved.
- `severity-level` enum is exactly `{ trace, debug, info, warn, error }` — no `critical`/`fatal` variant.
- The per-stage audit `Vec<Diagnostic>` preserves emission order.
- `support-planner` allocates `code` values in `1000-1999`. This packet uses `1001`, `1002`, `1003`. Future diagnostic classes in `support-planner` allocate within range.
- The 1024-cap behavior is unchanged at the data-flow level: drops still happen via `continue`/`truncate`. Only the *signal* of the drop changes (silent → typed Diagnostic).

## Risks and Tradeoffs

- **Risk**: bindgen drift — adding a WIT record can change the generated host-side `bindgen!` output in subtle ways (e.g., `option<s32>` may map differently than expected). **Mitigation**: AC-2 gates `wit_drift_detection_tdd`; AC-3 gates the SDK round-trip; the implementer MUST run these before considering Step 2 complete.
- **Risk**: existing guests that don't adopt the channel are silently rebuilt with no behavior change, BUT a typo in the WIT change could break their bindgen. **Mitigation**: `cargo xtask build-guests --check` after the WIT edit is mandatory; FAIL means rebuild and re-check before moving on.
- **Tradeoff**: per-layer per-object emission for the cap diagnostic produces O(layers × objects-with-cap-fires) diagnostics on a worst-case dense fixture. Acceptable: the channel is unbounded; the count is bounded by total layers × objects (modest); if a future user complains about diagnostic flood, deduplication can be added without WIT change.
- **Risk**: the `support-planner.node-clamped-out` migration changes the message format slightly (typed structured fields instead of string prefix). Downstream tooling that grepped the old string is broken. **Mitigation**: AC-4 asserts the new shape; downstream tooling migrates to the typed channel separately.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — WIT change + guest rebuild).
- Highest-risk dispatch: `cargo xtask build-guests --check`. Required return format: FACT (`up to date` or `STALE: <list>`); MUST NOT paste rebuild log.

## Open Questions

- `[FWD]` Diagnostic emission from inside `on_print_start` (which is called before the per-stage audit context exists in the current host plumbing) may require a transitional buffering mechanism: the guest's `on_print_start` calls `push_diagnostic` against a per-instance buffer that the host drains on first stage invocation. This is forward-looking — Step 4 of the implementation plan inspects the current `on_print_start` plumbing and decides between (a) plumbing the output-builder into `on_print_start` directly, (b) buffering the call until the first stage invocation, or (c) deferring the AC-6 (`interface_bottom_layers`) migration to a follow-up packet. The decision lands as a packet-author note in `requirements.md` Step Completion Expectations before Step 5 begins; it does not block packet activation because options (a) and (b) are both within scope and the decision is local to Step 4.
