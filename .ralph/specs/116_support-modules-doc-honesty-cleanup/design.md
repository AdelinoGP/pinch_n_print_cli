# Design: support-modules-doc-honesty-cleanup

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/tree-support/src/lib.rs` — lead `//!` block (current lines 1-12); add `# Speed normalization` section.
  - `modules/core-modules/traditional-support/src/lib.rs` — lead `//!` block (current lines 1-16); add `# Speed normalization` section.
  - `modules/core-modules/support-planner/src/lib.rs` — lead `//!` block (current lines 1-35); delete `support_interface_bottom_layers` field on `SupportPlanner` struct; rewrite the parse block in `on_print_start` (current lines 156-160 and 178); add `# Speed normalization` section.
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — lead `//!` block; add `# Speed normalization` section only (B3).
  - `modules/core-modules/support-planner/support-planner.toml` — add `# Not yet implemented` comment next to the `support_interface_bottom_layers` schema entry.
- Neighboring tests/fixtures:
  - `modules/core-modules/support-planner/tests/` — add three new tests (AC-5, AC-N1, AC-N2). Likely new file `tests/interface_bottom_layers_warning_tdd.rs`.
- OrcaSlicer comparison surface: not consulted by this packet. The new doc-comments declare what these modules are *not*; no Orca behavior is being ported.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- The deletion of `support_interface_bottom_layers` from the Rust struct preserves the TOML config key. The implementer MUST NOT remove the key from `[config.schema]` — that would be a user-facing API change beyond this packet's scope.

- The `LogLevel::Warn` diagnostic added in B2 ships as a string-prefixed log call (`log(LogLevel::Warn, "support-planner: support_interface_bottom_layers is not yet implemented ...")`), NOT a typed `Diagnostic`. The typed Diagnostic channel lands in sibling packet `118_support-planner-typed-diagnostics`; this packet ships the string form deliberately so the diagnostic exists before its typed migration.

## Code Change Surface

- Selected approach: edit-in-place. Each of the four module files receives a targeted edit to its lead `//!` block; `support-planner/src/lib.rs` additionally receives field deletion + parse replacement; `support-planner.toml` receives a one-line comment addition.
- Exact functions/structs/manifests/tests to change:
  - `tree_support::TreeSupport` — module doc-comment only (B1 + B3 section).
  - `traditional_support::TraditionalSupport` — module doc-comment only (B1 + B3 section).
  - `support_planner::SupportPlanner` — module doc-comment (B1 + B3 section); struct field deleted; `on_print_start` parse block replaced (B2).
  - `rectilinear_infill::RectilinearInfill` — module doc-comment only (B3 section).
  - `support_planner.toml [config.schema.support_interface_bottom_layers]` — TOML comment added (B2).
  - New test file `modules/core-modules/support-planner/tests/interface_bottom_layers_warning_tdd.rs` — three tests for AC-5, AC-N1, AC-N2.
- Rejected alternatives:
  - **Removing `support_interface_bottom_layers` from `support-planner.toml`** — rejected: user-facing config breakage exceeds the scope of an "honesty + cleanup" packet.
  - **Migrating B2's warning straight to a typed `Diagnostic`** — rejected: introduces WIT changes (`world-prepass.wit`) that belong to the sibling typed-diagnostics packet. Bundling them grows this packet beyond `S` cost.
  - **Replacing the `BASE_SPEED = 50.0` constant with config-driven speeds** — rejected: would be a real behavior change with downstream gcode-emit consequences; this packet only documents the existing convention.

## Files in Scope (read + edit)

The packet edits 5 source files plus one new test file (6 total). This exceeds the soft `≤ 3` ceiling and is justified inline below — the work is doc-shaped and uniform across modules; splitting would multiply ralph ceremony for no reduction in implementer work.

- `modules/core-modules/tree-support/src/lib.rs` — role: B1 doc-comment + B3 speed-normalization section; expected change: lead `//!` block replaced/extended.
- `modules/core-modules/traditional-support/src/lib.rs` — role: B1 doc-comment + B3 speed-normalization section; expected change: lead `//!` block replaced/extended.
- `modules/core-modules/support-planner/src/lib.rs` — role: B1 doc-comment + B2 field deletion + B2 parse replacement + B3 speed-normalization section; expected change: lead `//!` block replaced/extended, `SupportPlanner` struct field removed, `on_print_start` parse block replaced.
- `modules/core-modules/rectilinear-infill/src/lib.rs` — role: B3 only (consistency across all `BASE_SPEED` consumers); expected change: lead `//!` block extended with `# Speed normalization` section.
- `modules/core-modules/support-planner/support-planner.toml` — role: B2 user-facing signal; expected change: one TOML comment line added next to the config schema entry.
- `modules/core-modules/support-planner/tests/interface_bottom_layers_warning_tdd.rs` — role: three new unit tests for AC-5, AC-N1, AC-N2; expected change: file created.

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` — read §B1, §B2, §B3, §D8, §D9 only. Contains the exact doc-comment text the implementer copies.
- `crates/slicer-sdk/src/host.rs` — confirm the `log(LogLevel::Warn, ...)` call shape used elsewhere in core modules so AC-5 / AC-N1 / AC-N2 tests assert the right call surface. Read only the `log` fn signature + nearest existing caller (≤ 30 lines around the use site).
- `modules/core-modules/support-planner/src/lib.rs` current lead `//!` block (lines 1-35) — implementer reads to confirm the replacement target before editing.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted (no Orca behavior being ported).
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/**`, `crates/slicer-scheduler/**`, `crates/slicer-host/**` — out of scope; this packet edits only `modules/core-modules/` and one TOML.
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — historical packet 31b test; unrelated to this packet, do not open.
- Other infill modules (`gyroid-infill`, `lightning-infill`) — `BASE_SPEED` is consumed by them too but they were not in the original Block B scope and are deferred. Do not extend the B3 edit to them in this packet.

## Expected Sub-Agent Dispatches

- "Run `cargo build -p tree-support -p traditional-support -p support-planner -p rectilinear-infill`; return FACT pass/fail" — purpose: validate compile after each module's doc-comment + struct edit.
- "Run `cargo test -p support-planner --test interface_bottom_layers_warning_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure with the assertion + offending line" — purpose: gate AC-5, AC-N1, AC-N2.
- "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <which>`)" — purpose: confirm guest WASM artifacts caught up after src/lib.rs edits.
- "Find all callers of `slicer_sdk::host::log` inside `modules/core-modules/`; return LOCATIONS (max 20)" — purpose: confirm the `LogLevel::Warn` call shape for AC-5 matches existing convention.
- "Run `rg -c 'BASE_SPEED' modules/core-modules/`; return FACT (number per module)" — purpose: confirm B3 doc-comment scope covers every module that uses the constant.

## Data and Contract Notes

- IR or manifest contracts touched: none. The TOML `[config.schema.support_interface_bottom_layers]` entry retains its existing `type`, `default`, `min`, `max`, `display`, `group` keys. Only a TOML comment is added.
- WIT boundary considerations: none in this packet. Guest WASM rebuild is required because `modules/core-modules/*/src/lib.rs` are guest sources; that is operational, not contractual.
- Determinism: `LogLevel::Warn` is recoverable, does not affect IR commit semantics. The warning fires deterministically once per `on_print_start` invocation when the trigger condition is met.

## Locked Assumptions and Invariants

- `slicer_sdk::host::log(LogLevel::Warn, &str)` is the canonical recoverable-warning API used by core modules at print-start. New diagnostic stays on this API until sibling packet `118_support-planner-typed-diagnostics` migrates it.
- `support-planner.toml [config.schema.support_interface_bottom_layers]` user-facing key is preserved; downstream user profile files referencing it continue to load without warning unless they set it to a non-`-1` value.
- The four-module `BASE_SPEED = 50.0` convention is documented but unchanged. Any future change to the normalization base must update all four doc-comments in lockstep.

## Risks and Tradeoffs

- **Risk**: A future contributor sees the doc-comment "honesty downgrade" (Orca-port → "not a port of") and erroneously decides the algorithmic-shape work in `121_support-planner-smooth-nodes` and `122_support-planner-multi-neighbour-mst` is non-binding. **Mitigation**: each new doc-comment cross-references the path forward (`docs/specs/support-modules-orca-port.md` Block C) where Orca-shape work continues.
- **Tradeoff**: keeping the TOML config key after deleting the Rust field means a future C-block implementation of bottom-interface bands will need to re-introduce the field. Acceptable: user-facing surface stability > field-symmetry purity.
- **Risk**: `cargo xtask build-guests --check` failing post-edit. **Mitigation**: explicitly listed as a verification step.

## Context Cost Estimate

- Aggregate (sum across all steps): `S`
- Largest single step: `S`
- Highest-risk dispatch: `cargo test -p support-planner` after the AC-5/N1/N2 tests land. Required return format: FACT pass/fail, SNIPPETS ≤ 20 lines on failure with assertion text + offending line.

## Open Questions

None.
