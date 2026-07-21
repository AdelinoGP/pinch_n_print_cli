# Design: support-modules-doc-honesty-cleanup

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/tree-support/src/lib.rs` contiguous leading `//!` block, including its opening line and `BASE_SPEED` consumer documentation.
  - `modules/core-modules/traditional-support/src/lib.rs` contiguous leading `//!` block, including its opening line and `BASE_SPEED` consumer documentation.
  - `modules/core-modules/support-planner/src/lib.rs` contiguous leading `//!` block, `SupportPlanner`, `PrepassModule::on_print_start`, and the existing private `default_planner` test fixture.
  - `modules/core-modules/rectilinear-infill/src/lib.rs` contiguous leading `//!` block and `BASE_SPEED` consumer documentation.
  - `modules/core-modules/support-planner/support-planner.toml` `[config.schema.support_interface_bottom_layers]` entry.
- Neighboring tests/fixtures: none added. Packet 118 owns the typed warning tests; this packet only compiles the dead-state removal and documentation changes.
- OrcaSlicer comparison: none; this packet removes unsupported parity claims and does not assert borrowed Orca behavior.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- The config key `support_interface_bottom_layers` remains in the TOML schema. Packet 116 removes its unused Rust field and parse branch and does not inspect it for a warning; packet 118 owns the typed D11 diagnostic.
- AC-1, AC-2, and AC-3 extract only the contiguous leading `//!` block from a maximum 80-line prefix, then assert the required opening line and honesty wording inside that block; a later matching comment cannot satisfy them.
- AC-4 rejects every `support_interface_bottom_layers:` or `support_interface_bottom_layers =` field/struct-literal assignment and separately rejects the `config.get("support_interface_bottom_layers")` parse-and-store lookup.
- AC-7 checks the snake_case schema section and an immediately adjacent deferred-status comment in a maximum 200-line TOML prefix.
- Packet 116 must not emit the pre-typed string warning. Packet 118's current dependency on a packet-116 warning path is an activation blocker that must be reconciled there, not satisfied by this packet.
- `BASE_SPEED` documentation is limited to current in-scope consumers: `tree-support`, `traditional-support`, and `rectilinear-infill`. AC-6 extracts each consumer's contiguous leading `//!` block and asserts `# Speed normalization`, the normalization formula, and `BASE_SPEED = 50.0`; `support-planner` has no `BASE_SPEED` constant in the current tree.

## Code Change Surface

- Selected approach: edit the existing contiguous leading documentation blocks in place; delete only the dead bottom-interface field/state; preserve the TOML key and add its immediately adjacent deferred-status comment. No warning branch or warning test is added here.
- Exact functions, structs, and fixtures:
  - `SupportPlanner` in `support-planner/src/lib.rs` - remove `support_interface_bottom_layers` from the struct and its `default_planner` literal.
  - `PrepassModule::on_print_start` in the same file - remove the parse-and-store branch without adding a string warning; packet 118 owns the typed lookup/emission.
  - The four named module `//!` blocks - update only contiguous leading documentation; add the speed section to the three actual consumers.
  - `[config.schema.support_interface_bottom_layers]` in `support-planner.toml` - add the adjacent status comment without changing schema fields.
- Rejected alternatives and reasons:
  - Remove the TOML key - rejected because it breaks the existing user-facing schema.
  - Add a speed section to `support-planner` - rejected because the current module has no speed-factor consumer or `BASE_SPEED` symbol.
  - Emit a typed diagnostic now - rejected because packet 118 owns the WIT/channel contract.

## Files in Scope (read + edit)

The five files are a uniform documentation/dead-state slice; they cover the four comments, the preserved schema signal, and D8's dead-state cleanup without taking D11's diagnostic channel.

- `modules/core-modules/tree-support/src/lib.rs` - role: B1/B3 documentation; expected change: leading comment block only.
- `modules/core-modules/traditional-support/src/lib.rs` - role: B1/B3 documentation; expected change: leading comment block only.
- `modules/core-modules/support-planner/src/lib.rs` - role: B1/B2 documentation and D8 state; expected change: comment rewrite, field/parse/fixture deletion, no warning branch.
- `modules/core-modules/rectilinear-infill/src/lib.rs` - role: B3 documentation; expected change: leading comment block only.
- `modules/core-modules/support-planner/support-planner.toml` - role: B2 user-facing signal; expected change: preserve the snake_case entry and add one immediately adjacent comment.

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` - §B1-B3 and §D8-D9 only - source wording and dead-state boundary.
- `docs/adr/0010-typed-diagnostic-channel.md` - typed D11 contract owned by packet 118; packet 116 must not create its string predecessor.
- `docs/specs/support-modules-orca-port-plan.md` - packet 116 queue row only - source-plan labels and dependency order.
- `docs/07_implementation_status.md` - targeted support/task-ID searches only - resolve the blocked crosswalk; never edit it here.

## Out-of-Bounds Files

- `docs/07_implementation_status.md` - mutable backlog ownership; maintainer-only mapping, not a packet edit.
- `modules/core-modules/{gyroid-infill,lightning-infill,classic-perimeters,top-surface-ironing,support-surface-ironing}/**` - other `BASE_SPEED` consumers are outside the source-plan slice.
- `crates/slicer-schema/wit/**`, `crates/slicer-ir/**`, `crates/slicer-runtime/**`, `crates/slicer-scheduler/**` - no contract or host pipeline change.
- `OrcaSlicerDocumented/**` - no parity assertion in this packet.
- `target/`, `Cargo.lock`, generated code, and every other packet directory - never load or edit.

## Expected Sub-Agent Dispatches

- Question: Which current backlog rows own source-plan B1, B2, and B3 after checking all collisions for `TASK-250`, `TASK-251`, and `TASK-252`? Scope: `docs/07_implementation_status.md` targeted searches. Return: `LOCATIONS` with at most 20 entries; purpose: unblock activation without inventing IDs.
- Question: Run the targeted module compile, clippy, planner test, and guest freshness checks. Scope: commands in `requirements.md`. Return: `FACT` pass/fail; on test failure, at most 20 relevant lines; purpose: packet gate.

## Data and Contract Notes

- IR/manifest contracts: none; the TOML schema key and its type/default/range remain unchanged.
- WIT boundary: none; packet 118 owns the D11 typed channel and its WIT change.
- Determinism/scheduler constraints: this packet emits no diagnostic; comments and dead-state removal do not affect stage ordering or geometry output.

## Locked Assumptions and Invariants

- `support_interface_bottom_layers` remains a snake_case config key with default `-1`.
- The TOML key keeps default `-1`; packet 116 makes no runtime warning claim for any value. Packet 118 must own the typed behavior for non-default, default, and absent-key cases.
- `BASE_SPEED = 50.0` and each existing speed calculation remain unchanged; only explanatory comments are added.
- No canonical backlog ID is assigned by this packet. Until the blocker is resolved, status remains `draft`.

## Risks and Tradeoffs

- The typed-warning packet currently references a packet-116 warning path that this narrowed packet deliberately does not create; that cross-packet mismatch is an explicit activation blocker.
- Removing the dead field changes no output because the field was never consumed, but struct literals in the module's private test fixture must be updated in the same edit.
- Restricting B3 to current consumers corrects the old packet's false `support-planner` claim but leaves other modules' comments for separate work.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: backlog crosswalk survey; `LOCATIONS` at most 20 entries, with exact collision context.

## Open Questions

- *Resolved at activation.* Backlog crosswalk for source-plan B1, B2, B3: `docs/07_implementation_status.md` support rows `TASK-163`/`TASK-163b-diagnostic` are the closest current ownership; `TASK-250`/`TASK-252` are unrelated closed work, and `TASK-251` does not exist as a support row. Packet 116 intentionally maps no replacement `TASK-###`; the maintainer crosswalk is the source-plan label, not a canonical backlog ID.
- *Resolved at activation.* Packet 118's dependency/AC wording was reconciled: it creates the typed `support_interface_bottom_layers` diagnostic itself, with no packet-116 string-warning prerequisite. Packet 116 intentionally owns only D8 dead-state cleanup and does not emit the untyped predecessor.
