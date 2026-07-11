# Design: 131_per-region-config-delivery

## Controlling Code Paths

- Primary code path: `crates/slicer-wasm-host/src/dispatch.rs:1629-1650` —
  `effective_config_view` derivation: today `map.entries.keys().find(|key|
  key.global_layer_index == layer.index)` (line 1640) picks an arbitrary entry; replaced by
  per-region resolution keyed on the full `RegionKey` of the region being iterated, sourced
  from the `RegionMapIR` interned pool via `RegionMapIR::config_for(&RegionKey)`
  (`crates/slicer-ir/src/slice_ir.rs:1194-1204`, `:1232`). Two other sites match the looser
  `global_layer_index == layer` substring (`:1378`, `:1680`) — both are unrelated lookups,
  out of scope; do not touch them.
- Secondary: `crates/slicer-schema/wit/deps/ir-types.wit` region-view resources gain the
  config accessor; `crates/slicer-sdk/src/views.rs` + `crates/slicer-macros/src/lib.rs` glue.
- Neighboring tests or fixtures: `crates/slicer-runtime/tests/contract/` (new
  `per_region_config_*` tests; reuse the 130 echo-guest pattern), e2e wedge SHA tests
  (`crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs` — guard, not edit).
- OrcaSlicer comparison surface: none — PnP delivery mechanics.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Additive-only WIT change: existing guests that never call the accessor must remain valid
  after rebuild; no existing view method changes shape.
- Config keys are snake_case everywhere (CLAUDE.md §Config Key Naming Convention).
- The baseline-before-edit ordering is a hard constraint (see requirements.md §Step
  Completion Expectations).
- FORWARD-DEP: packet 130 is `status: draft` at authoring time — this packet's world-version
  bump target is computed at activation (+0.1 from whatever 130 lands as), never hardcoded.

## Code Change Surface

- Selected approach: the host constructs a per-region `ConfigView` lazily when the guest
  calls the region-view accessor, resolving the region's `RegionKey` → interned
  `ResolvedConfig` once per region per dispatch. The module-level `ConfigView` parameter
  stays (module-scoped keys); the accessor is the per-region path. For layers whose region
  has no `RegionMapIR` entry, the accessor falls back to the object-level config (same value
  the old first-match produced on single-region layers — this is what makes AC-N1/AC-N2
  hold).
- Exact changes: `dispatch.rs` derivation + accessor host impl; `ir-types.wit` gains
  `use slicer:config/config-types.{config-view};` in the `ir-handles` interface plus a
  `config: func() -> config-view` method on both `slice-region-view` and
  `perimeter-region-view`; world-layer bumped by +0.1 from whatever version 130 lands at
  (+ any other world exposing these views — discover via rg, don't assume); SDK accessors;
  macros glue; contract tests; carve survey artifacts; the pinned-SHA byte-identity test
  (AC-N2).
- Rejected alternatives: (a) replacing the module `ConfigView` param with a per-region one —
  breaks every module signature for no gain; (b) passing a config *list* keyed by region into
  each dispatch — pushes the join onto every module author; (c) fixing only the first-match
  bug without an accessor — leaves modifier densities (ADR-0030) undeliverable.

## Files in Scope (read + edit)

- `crates/slicer-wasm-host/src/dispatch.rs` — role: derivation fix + accessor impl; expected
  change: replace first-match block, add per-region resolution helper.
- `crates/slicer-schema/wit/deps/ir-types.wit` (+ world version files) — role: accessor
  contract; expected change: one accessor on two views + version bump.
- `crates/slicer-sdk/src/views.rs` + `crates/slicer-macros/src/lib.rs` — role: SDK surface;
  expected change: accessor plumbing.
- `crates/slicer-runtime/tests/contract/per_region_config_tdd.rs` (new) — role: AC-1/AC-N1.
- `.ralph/specs/131_per-region-config-delivery/carve-list.md` (new) — role: AC-4 artifact.
- Carved test files — role: add `#[ignore = "carved: infill-parity D6; restored in packet
  136"]` markers only where the survey says so.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` — lines 1194-1232 only — `RegionMapIR` pool shape
  (struct at 1194-1204; `config_for(&RegionKey) -> &ResolvedConfig` at 1232 is the resolution
  idiom to reuse — `extensions` lives on `ResolvedConfig`, not on `RegionMapIR` itself).
- `crates/slicer-wasm-host/src/dispatch.rs` — lines 1600-1730 only (the derivation block to
  edit is 1629-1650; line 1378 is outside this range and irrelevant; line 1680 falls inside
  this read range but is a separate `held_claims_map` resolution — read-only, do not edit).
- One 130-era contract test + echo guest — idiom reuse.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — never load.
- `crates/slicer-ir/src/resolved_config.rs` in full — not needed; delegate any key-name FACT.
- e2e/golden test file bodies — the survey is delegated; the implementer edits only the
  `#[ignore]` lines the survey returns.
- `target/`, `Cargo.lock`, generated code — never load.

## Expected Sub-Agent Dispatches

- "BEFORE any edit: run the e2e + executor + contract suites' SHA/golden subset (wedge +
  cube_4color + cube_fuzzy fixtures); return FACT: fixture → SHA/assertion baseline" — Step 1
  baseline.
- "rg for SHA-pinned or infill-output-shape assertions across
  `crates/slicer-runtime/tests/{e2e,executor,integration}` and
  `crates/pnp-cli/tests`; return LOCATIONS ≤25 (test fn + pinned value)" — Step 1 survey.
- "Run `cargo check --workspace --all-targets`; FACT or LOCATIONS ≤30" — sweep gate.
- "Run `cargo test -p slicer-runtime --test contract -- per_region_config 2>&1 | tee
  target/test-output.log | grep '^test result'`; FACT + counts" — AC-1/AC-N1.
- "Run `cargo xtask build-guests --check`; FACT clean or STALE; rebuild if stale".

## Data and Contract Notes

- IR contracts: `RegionMapIR` is read, not changed.
- WIT boundary: additive accessor; version bump; full guest rebuild.
- Determinism: per-region resolution must be pure (RegionKey → config); no iterator-order
  dependence remains — that is the point of the packet.

## Locked Assumptions and Invariants

- Single-region layers produce byte-identical output (AC-N2 wedge SHA) — this invariant is
  what separates "delivery fix" from "behavior change"; if it breaks, the implementation is
  wrong, not the test.
- The carve marker string is exactly `carved: infill-parity D6` — packet 136 greps for it.
- The module-level `ConfigView` parameter is not removed or re-pointed.
- Fallback for regions without a `RegionMapIR` entry: object-level config (preserves current
  single-region behavior).

## Risks and Tradeoffs

- The survey may under-enumerate: a multi-region-affected test not carved here shows up red
  in 132–135. Mitigation: the carve-list is append-able by later packets (each append is a
  recorded deviation in that packet), and AC-N2 keeps the single-region floor solid.
- Two WIT bumps in two adjacent packets (130 then here) is deliberate — the schema-churn
  phase is front-loaded by roadmap decision D1; do not merge the packets to "save a bump".
- Per-region `ConfigView` construction cost: lazy construction + per-dispatch memoization by
  `RegionKey` keeps it O(regions), matching current behavior asymptotically.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (WIT + SDK + host derivation)
- Highest-risk dispatch: the golden survey — must return a bounded LOCATIONS inventory, not
  test-file dumps.

## Open Questions

- `FWD-RESOLVED` Accessor WIT shape: `config: func() -> config-view` on both
  `slice-region-view` and `perimeter-region-view`, reusing the existing
  `slicer:config/config-types.config-view` resource (`crates/slicer-schema/wit/deps/
  config.wit:8`) rather than duplicating its six `get*`/`keys` methods per-key on two
  resources. `ir-types.wit`'s `ir-handles` interface gains
  `use slicer:config/config-types.{config-view};` alongside its existing `use
  slicer:types/geometry...` import. AC-3 pins this shape.
- `[FWD]` Which worlds beyond world-layer expose the two views (rg at implementation);
  version-bump all that do.
