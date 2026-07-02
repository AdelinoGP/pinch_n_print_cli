# Implementation Plan: 131_per-region-config-delivery

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Baseline capture + golden survey (BEFORE any code edit)

- Task IDs:
  - `TASK-256`
- Objective: capture pre-change baselines (wedge, cube_4color, cube_fuzzy SHAs/assertions)
  and enumerate every SHA-pinned or infill-output-shape test that a multi-region config
  correction can affect; author `carve-list.md` (entries: test path, pinned value, carve
  yes/no + reason).
- Precondition: clean tree; packet 130 closed; NO edits from this packet yet.
- Postcondition: `carve-list.md` exists with per-test entries and baselines; no code changed.
- Files allowed to read: none directly (pure-dispatch step).
- Files allowed to edit (≤ 3):
  - `.ralph/specs/131_per-region-config-delivery/carve-list.md` (new)
- Files explicitly out-of-bounds for this step:
  - all source and test bodies (survey is delegated)
- Expected sub-agent dispatches:
  - "Run the wedge/cube SHA-bearing e2e tests; return FACT: fixture → SHA" — baseline
  - "rg SHA-pinned / infill-shape assertions across runtime+pnp-cli test trees; LOCATIONS ≤25"
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/modifier-region-infill.md` §Phase M2
- OrcaSlicer refs: none.
- Verification:
  - `test -s .ralph/specs/131_per-region-config-delivery/carve-list.md && echo OK` — FACT
- Exit condition: carve-list authored; expected inventory: ≥ the cube_4color/cube_fuzzy SHA
  tests; wedge explicitly listed as NOT carved (single-region guard).

### Step 2: WIT accessor + SDK surface + macros glue

- Task IDs:
  - `TASK-256`
- Objective: add the config accessor to `slice-region-view` and `perimeter-region-view` in
  `ir-types.wit`; bump world-layer 1.1.0 → 1.2.0 (+ any other exposing world found by rg);
  SDK accessors on both view types; macros glue.
- Precondition: Step 1 exit condition.
- Postcondition: `cargo build --tests` compiles schema/sdk/macros; host may be red until
  Step 3.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-schema/wit/deps/ir-types.wit` — view resources + existing config-view
    modeling
  - `crates/slicer-sdk/src/views.rs` — both view regions only
- Files allowed to edit (≤ 3):
  - `crates/slicer-schema/wit/deps/ir-types.wit` (+ world version file(s) — counted as one
    WIT wave), `crates/slicer-sdk/src/views.rs`, `crates/slicer-macros/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - `dispatch.rs` (Step 3)
- Expected sub-agent dispatches:
  - "rg which worlds expose slice-region-view/perimeter-region-view; LOCATIONS" — bump scope
  - "Run `cargo build --tests 2>&1 | tail -40`; FACT or LOCATIONS ≤30"
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §WIT/Type Changes Checklist
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p slicer-schema -p slicer-sdk -p slicer-macros --all-targets` — FACT
- Exit condition: contract compiles at the SDK layer.

### Step 3: Host derivation fix + accessor implementation

- Task IDs:
  - `TASK-256`
- Objective: replace the first-match `effective_config_view` block
  (`dispatch.rs:1629-1645`) with `RegionKey`-matched per-region resolution (lazy, memoized
  per dispatch); implement the accessor host-side; object-level fallback for regions without
  a pool entry.
- Precondition: Step 2 exit condition.
- Postcondition: `cargo check --workspace --all-targets` green; AC-2 structural grep returns
  0.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-wasm-host/src/dispatch.rs` — lines 1600-1730 only
  - `crates/slicer-ir/src/slice_ir.rs` — lines 1176-1200 only
- Files allowed to edit (≤ 3):
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-wasm-host/src/host.rs` (only if the accessor resource impl lives there)
- Files explicitly out-of-bounds for this step:
  - module bodies; test files
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; FACT or LOCATIONS ≤30"
  - "Run `rg -n 'global_layer_index == layer' crates/slicer-wasm-host/src/dispatch.rs | wc
    -l`; FACT (expect 0)"
- Context cost: `M`
- Authoritative docs: `docs/adr/0030-…` Decision point 3.
- OrcaSlicer refs: none.
- Verification:
  - both dispatches above — FACT
- Exit condition: workspace compiles; first-match derivation gone.

### Step 4: Contract tests + guards + carve marking (RED→GREEN)

- Task IDs:
  - `TASK-256`
- Objective: write `per_region_config_two_densities` (AC-1) and
  `per_region_config_single_region_unchanged` (AC-N1) against the echo-guest pattern; rebuild
  guests; run the wedge guard (AC-N2); apply `#[ignore = "carved: infill-parity D6; restored
  in packet 136"]` to exactly the carve-list entries and confirm the carved suites otherwise
  pass.
- Precondition: Step 3 exit condition.
- Postcondition: contract suite green incl. new tests; wedge SHA test green un-carved; carved
  tests ignored with the exact marker string.
- Files allowed to read (with line-range hints when > 300 lines):
  - the 130 echo guest + one contract test (idiom); carve-list.md
- Files allowed to edit (≤ 3 per wave):
  - `crates/slicer-runtime/tests/contract/per_region_config_tdd.rs` (new) + harness mod line
  - carved test files (marker lines only; one wave per file batch)
- Files explicitly out-of-bounds for this step:
  - any test not on the carve list
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE"
  - "Run `cargo test -p slicer-runtime --test contract -- per_region_config 2>&1 | tee
    target/test-output.log | grep '^test result'`; FACT + counts"
  - "Run `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 | tee target/test-output.log
    | grep '^test result'`; FACT"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - the three dispatches above — FACT each
  - `rg -c 'carved: infill-parity D6' --type rust | delegate` — FACT count == carve-list count
- Exit condition: AC-1, AC-N1, AC-N2 green; carve markers exactly match the list.

### Step 5: Doc Impact + gate ceremony

- Task IDs:
  - `TASK-256`
- Objective: land `docs/03_wit_and_manifest.md` (accessor + version) and
  `docs/05_module_sdk.md` (per-region config usage) sections; run packet gates.
- Precondition: Step 4 exit condition.
- Postcondition: Doc Impact greps hit; the three gates in `packet.spec.md` §Verification
  green.
- Files allowed to read (with line-range hints when > 300 lines):
  - the two docs — rg-located sections only
- Files allowed to edit (≤ 3):
  - `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`
- Files explicitly out-of-bounds for this step: code.
- Expected sub-agent dispatches:
  - "Run the two Doc Impact greps + the three gates; FACT each"
- Context cost: `S`
- Authoritative docs: the two being edited.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'per-region config' docs/05_module_sdk.md && echo HIT` — FACT
- Exit condition: greps hit; gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | pure-dispatch survey + baselines |
| Step 2 | M | WIT + SDK + macros |
| Step 3 | M | dispatch derivation |
| Step 4 | M | tests + carve application |
| Step 5 | S | docs + gates |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-256 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
