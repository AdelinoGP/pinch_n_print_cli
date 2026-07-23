# Design: support-plan-raft-plan-and-raftinfill-role

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-ir/src/slice_ir.rs` `pub enum ExtrusionRole` (line 1659) — new `RaftInfill` variant.
  - `crates/slicer-ir/src/slice_ir.rs` `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` (lines 251-255) — bump from 1.2.0 to 1.3.0.
  - `crates/slicer-sdk/src/views.rs::should_emit` (line 498) — new `RaftInfill => "claim:raft-fill"` arm.
  - `crates/slicer-schema/wit/deps/types.wit` `variant extrusion-role` (lines 13-18) — new `raft-infill` member.
  - 14 workspace `match role` sites (see `requirements.md` §Authoritative Docs) — audited; non-wildcard sites get the new arm.
- Neighboring tests/fixtures:
  - `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs` — new (AC-4, AC-N1, AC-N3).
  - `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs` — existing; AC-N3's WIT round-trip is exercised here, not edited.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations. No OrcaSlicer code is ported in this packet; the raft geometry comparison surface is owned by `raft-default-module`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- The `ExtrusionRole::RaftInfill` variant is additive but extends an exhaustive `match` surface in every `should_emit` consumer and every per-role dispatch. The `should_emit` at `views.rs:504` already has the `_ => return true` fallback, so a missing `RaftInfill` arm at `should_emit` means modules fall into `_` and emit `true` unconditionally — the silent-true failure mode. The implementer's Step 4 audit must guard against this. At `gcode::emit`, `gcode::serialize`, `overhang-classifier-default`, `path-optimization-default`, and the `marshal::leaf` sites, every variant is listed explicitly with no `_ =>`, so a missing `RaftInfill` arm at any of those sites escalates to a non-exhaustive-match compile error (caught by AC-N2's `cargo build --workspace`).
- The `claim:raft-fill` string is a NEW claim. Existing `should_emit` consumers that don't hold it return `false` (per the existing `held_claims.iter().any` semantics) — this is correct per AC-N1.
- The schema_version bump is minor (additive). The implementer MUST NOT bump major; doing so breaks existing host compatibility checks per `docs/02_ir_schemas.md` semver convention. The implementer MUST also update any test that hard-asserts on the old `1.2.0` value (search for `1.2.0` in `crates/slicer-ir` and `crates/slicer-runtime` tests before bumping).
- The WIT mirror (at `crates/slicer-schema/wit/deps/types.wit`) is NOT 1:1 with the Rust enum (WIT has 12 named + `custom(string)`; Rust has 18 named + `Custom(String)`; the packet adds one to each). The packet maintains this asymmetry: WIT gets `raft-infill`; Rust gets `RaftInfill`. The two names are linked by the bindgen mapping convention (`top-solid-infill` → `TopSolidInfill`).
- The packet does NOT modify `SupportPlanIR`, `RaftPlan`, or `support-planner`. The §C6 contract is canonical; the packet must not corrupt it.

## Code Change Surface

- Selected approach: additive role/claim extension + schema minor bump + workspace-wide `match role` audit. No new module. No IR shape change.
- Exact functions/structs/manifests/tests to change:
  - `slicer_ir::ExtrusionRole` (line 1659) — new variant `RaftInfill`.
  - `slicer_ir::CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` (lines 251-255) — bump to `SemVer { major: 1, minor: 3, patch: 0 }`.
  - `slicer_sdk::views::should_emit` (line 498) — new arm.
  - `slicer_schema::wit::deps::types::geometry::extrusion_role` (`types.wit:13-18`) — new `raft-infill` member.
  - 14 `match role` sites — non-wildcard sites gain the new arm; wildcard sites are exempt. Specifically the non-wildcard sites are `gcode::emit`, `gcode::serialize`, `overhang-classifier-default`, `path-optimization-default`, and the two `marshal::leaf` sites. (The `view.rs::should_emit` site is exempt because it already has `_ =>`; the `gcode` test site and the two `runtime` test sites are test code and may be touched only to fix a compile error, not preemptively.)
  - `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs` — new.
- Rejected alternatives:
  - **Make `RaftPlan` carry geometry (footprint, layers, Z gap)** — rejected: violates §C6; the synthesizer (raft-default) owns geometry, not the planner.
  - **Add a separate `Layer::Raft` stage with its own renderer claim** — rejected per ADR-0009 §Future-Reviewer Notes; would proliferate stages without solving the duplication problem.
  - **Skip the schema bump** — rejected: ADR-0009 §Consequences explicitly calls for the semver-minor bump. The bump is the marker that the `ExtrusionRole` enum gained an additive variant.
  - **Add `RaftInfill` to `RaftPlan` instead of `ExtrusionRole`** — rejected: conflates planner-side config with role-side dispatch. The two are different seams; conflating them would break the per-writer single-owner rule for `SupportPlanIR`.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` — role: enum variant + schema bump; expected change: 2 lines added (one enum variant, one literal field) + a few comment lines.
- `crates/slicer-sdk/src/views.rs` — role: claim arm; expected change: 1 line.
- `crates/slicer-schema/wit/deps/types.wit` — role: WIT mirror update; expected change: 1 line added to the `extrusion-role` variant.
- `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs` — role: behavioral test; expected change: new file.
- 14 `match role` sites — role: exhaustive-match audit; expected change: 1 line per non-wildcard site (5-6 sites).
- `crates/slicer-ir/tests/` (any test that hard-asserts `1.2.0`) — role: bump fallout; expected change: 1 line per test (Step 1 audit must enumerate these before the bump).

## Read-Only Context

- `docs/adr/0009-raft-as-layer-infill-role.md` — full read (94 lines).
- `docs/specs/support-modules-orca-port.md` §C6 (lines 380-410) and §C7 (lines 412-418) — range reads.
- `docs/specs/raft-default-module.md` — read for consumer alignment; not edited.
- `crates/slicer-ir/src/slice_ir.rs` — range-read `ExtrusionRole` (lines 1655-1700) and `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` (lines 245-260).
- `crates/slicer-sdk/src/views.rs` — range-read `should_emit` (lines 480-520).
- `crates/slicer-schema/wit/deps/types.wit` — full read (small).
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit:163` — `push-raft-plan` interface; full read.
- `modules/core-modules/traditional-support/src/lib.rs:1-30` — lead `//!` block.
- `modules/core-modules/traditional-support/traditional-support.toml` — full read.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load.
- `modules/core-modules/support-planner/**` — out of bounds. The planner is §C6-conformant; this packet does not touch it.
- `crates/slicer-ir/src/slice_ir.rs::SupportPlanIR`, `RaftPlan`, `SupportPlanEntry` — out of bounds. The IR shape is §C6-canonical.
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — out of bounds. The `push-raft-plan` interface signature is unchanged.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `docs/02_ir_schemas.md`, `docs/01_system_architecture.md` — no edits; the role/claim pattern is already documented.
- `docs/specs/raft-default-module.md` — read-only; the synthesizer is a separate spec.
- Post-74710fa fill-partition test files — out of scope; the new dispatch they exercise is separate from this packet's role/claim addition.

## Expected Sub-Agent Dispatches

- Question: "Locate every workspace `match role` site that switches on `ExtrusionRole` and identify whether each uses an explicit per-variant arm list (no `_ =>`) or a wildcard fallback"; scope: `crates/ modules/ --type rust`; return: `LOCATIONS` (file:line + 1-line context, ≤ 20 entries, with each entry tagged `[explicit]` or `[wildcard]`); purpose: AC-6 audit and Step 4 arm-addition plan.
- Question: "Locate every test in `crates/slicer-ir/tests/` and `crates/slicer-runtime/tests/` that hard-asserts on the literal value `1.2.0` (the `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` before the bump)"; scope: `crates/slicer-ir/tests/ crates/slicer-runtime/tests/`; return: `LOCATIONS` (file:line, ≤ 20 entries); purpose: Step 1 fallout enumeration for the schema bump.
- Question: "Confirm `ExtrusionRole` is mirrored in `crates/slicer-schema/wit/deps/types.wit` (NOT `ir-types.wit`) and report the current number of named members + the bindgen name for each"; scope: `crates/slicer-schema/wit/deps/types.wit`; return: `SNIPPETS` (≤ 20 lines); purpose: Step 3 WIT edit site confirmation.
- Question: "Run `cargo xtask build-guests` after the WIT enum addition"; scope: workspace; return: `FACT` pass/fail; purpose: 20-guest rebuild.
- Question: "Run `cargo xtask build-guests --check`"; scope: workspace; return: `FACT` clean / STALE; purpose: WASM freshness gate.
- Question: "Run AC-1 through AC-9 + AC-N1 through AC-N3 verification commands"; scope: workspace; return: `FACT` (PASS / FAIL list); purpose: packet-level gate.
- Question: "Run `cargo build --workspace --all-targets` after the variant addition and every non-wildcard `match role` arm addition"; scope: workspace; return: `FACT` pass/fail; `SNIPPETS` ≤ 30 lines FIRST error on fail; purpose: AC-N2 exhaustive-match gate.
- Question: "Run `cargo clippy --workspace --all-targets -- -D warnings`"; scope: workspace; return: `FACT` pass/fail; `SNIPPETS` ≤ 20 lines FIRST error on fail; purpose: lint gate.

## Data and Contract Notes

- IR/manifest contracts: `ExtrusionRole` gains one additive variant. `SupportPlanIR` is unchanged. The `push-raft-plan` WIT interface is unchanged. `should_emit`'s contract gains one entry in the per-role mapping.
- WIT boundary: the new `extrusion-role` variant in `types.wit` MUST match the bindgen name. The `crates/slicer-schema/wit` package's bindgen produces the Rust enum; the WIT variant name becomes a Rust variant (likely `RaftInfill` from `raft-infill`, matching the convention `top-solid-infill` → `TopSolidInfill`).
- Determinism: the role/claim extension is deterministic — no timing, no scheduler state, no module-dispatch reordering.
- Locked events: none. The packet does not bump `PROGRESS_EVENT_SCHEMA_VERSION` or any other event-locked constant.

## Locked Assumptions and Invariants

- The `RaftPlan` config-only contract (§C6) is locked. The packet MUST NOT extend `RaftPlan` to carry geometry; that work is `raft-default-module`'s.
- The `support-planner` is the sole writer of `SupportPlanIR`. The packet does not change that.
- The `RaftInfill` variant is documented as "rendered by whichever `Layer::Infill` module declares `claim:raft-fill`" per ADR-0009. The packet does not introduce the renderer; the renderer is a future packet.
- The schema minor bump (1.2.0 → 1.3.0) is the marker that `ExtrusionRole` gained an additive variant. The bump is locked at `1.3.0`; the implementer MUST NOT bump to `1.2.1` (patch) or `2.0.0` (major).
- The `should_emit` arm placement is locked at "after the existing `TopSolidInfill` arm" to match the role/claim grouping pattern. The implementer MUST NOT insert it elsewhere.

## Risks and Tradeoffs

- **Risk**: a missing `RaftInfill` arm at a non-wildcard `match role` site escalates to a compile error (AC-N2 catches it). **Mitigation**: Step 1 audit identifies all 14 sites and tags them `[explicit]` or `[wildcard]`; Step 4 adds the arm at every `[explicit]` site before the variant is added to the enum. The order is critical: audit first, then add variant.
- **Risk**: the WIT enum addition triggers 20-guest rebuild. **Mitigation**: this is a one-time cost; `cargo xtask build-guests` handles it. AC-7's WIT round-trip exercises the new variant.
- **Risk**: the schema bump to `1.3.0` breaks any test that hard-asserts on `1.2.0`. **Mitigation**: Step 1 dispatch enumerates every such test; Step 2 updates them in the same step as the bump.
- **Risk**: silent-true-fallback at `views.rs:504` if the `RaftInfill` arm is missing from `should_emit`. **Mitigation**: AC-2 grep is the structural gate; AC-4 and AC-N1 are the behavioral gates. Both are required.
- **Tradeoff**: the packet touches 5-6 non-wildcard `match role` sites for a single additive variant. The alternative (mass wildcard conversion) would lose exhaustiveness checking at those sites. The per-site arm addition is the right tradeoff.

## Context Cost Estimate

- Aggregate: `M`.
- Largest step: `M` (Step 2 — IR + schema bump, with fallout enumeration).
- Highest-risk dispatch: the 14-site `match role` audit (Step 1) — return MUST be `LOCATIONS` ≤ 20 entries with each entry tagged `[explicit]` or `[wildcard]`; never paste source.

## Open Questions

- None. The scope is fully determined by ADR-0009 + §C6 + §C7. The packet does not introduce design choices; it only implements the additive role/claim extension that ADR-0009 commits.
