# Implementation Plan: support-interface-spacing-and-loops

## Execution Rules

- Steps run in order. Each step ends green on its own verification before the next starts.
- A step may edit only the files listed in its "Files allowed to edit". `design.md` §Out-of-Bounds Files binds every step.
- Every cargo run is delegated with a `FACT pass/fail` return and tees to `target/test-output.log`; inspect the log rather than re-running (`CLAUDE.md` § Test output).
- No step may add `support_interface_pattern` to any manifest, and no step may touch `ORCA_CONFIG_PADDING` or a CONFIG_BLOCK twin (Authoring rules 1 and 2).

## Steps

### Step 1: Declare the loop key + align the spacing default in both manifests, with guards

- Task IDs: — (queue packet, `task_ids: []`)
- Objective: both manifests carry the three keys at AC-1's exact contract, and a schema guard in each module fails on drift or on a re-added `support_interface_pattern`.
- Preconditions: both manifests currently declare `support_interface_spacing` (default 0.4), `support_bottom_interface_spacing` (default 0.5, min -1.0), and no loop key — re-verify before editing.
- Postconditions: AC-1 and AC-N1 green; no `src/lib.rs` behaviour change yet, so module tests still pass except any that pin the 0.4 default (Step 2 owns those — if one fails here, record it and carry it into Step 2 rather than patching it now).
- Files allowed to edit: `modules/core-modules/traditional-support/traditional-support.toml`, `modules/core-modules/tree-support/tree-support.toml`, `modules/core-modules/{traditional-support,tree-support}/tests/support_config_schema_tdd.rs` (net-new), and the two `Cargo.toml` files for the `toml` dev-dependency the guards need (verified absent at authoring — both crates' dev-deps are `slicer-sdk` and `slicer-wasm-host` only).
- Out of bounds: both `src/lib.rs`, every other packet directory, the map and tickets.
- Dispatches: `FACT` — list every test expectation in the two modules' test directories pinning `0.4` as the interface spacing default (file + test name). Feeds Step 2.
- Cost: S
- Authorities: `requirements.md` §Per-Key Canonical Evidence (types, defaults, bounds); `docs/03_wit_and_manifest.md` `[config.schema]` contract.
- Verification: `cargo test -p traditional-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition (falsifying): a guard passes while `support_interface_pattern` is present in either manifest, or while either `support_interface_loop_pattern` table is missing or non-bool.

### Step 2: Align the fallback constants, fixture, and every 0.4 expectation

- Task IDs: —
- Objective: the aligned 0.5 default reaches the interface-pitch decision point in both modules, and every expectation that encoded 0.4 is re-measured against the new value.
- Preconditions: Step 1 green; the `FACT` list of 0.4-pinned expectations in hand.
- Postconditions: AC-2 and AC-3 green in both modules, including the `-1.0` mirror witness and the non-default 0.2 / 1.2 / 1.6 arms.
- Files allowed to edit: `modules/core-modules/traditional-support/src/lib.rs` (fallback constant + its comment only), `modules/core-modules/tree-support/src/lib.rs` (same), `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs`, `modules/core-modules/tree-support/tests/tree_support_tdd.rs`, plus the `orca-matched-config.json` fixture and its consumer `support_family_closure.rs`.
- Out of bounds: `crates/slicer-core/src/support_regularize.rs` (the formulas are canonical-exact and do not change), both manifests (Step 1 owns them), `crates/slicer-gcode/**`.
- Dispatches: `FACT` pass/fail for each verification command.
- Cost: M
- Authorities: `requirements.md` §Wiring notes (default alignment; mirror sentinel).
- Verification: `cargo test -p traditional-support --test traditional_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test tree_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition (falsifying): any moved expectation is made to pass by widening a tolerance, deleting an assertion, or reverting the default to 0.4 — all three fail the step regardless of a green run (`CLAUDE.md`: never game verification).

### Step 3: Build the contact-loop helper in `slicer-core`

- Task IDs: —
- Objective: a shared, unit-tested helper that turns one interface `ExPolygon` plus an interface line width and a loop count into (closed loop polyline(s), trimmed fill area), returning no loop when the inward offset empties the area.
- Preconditions: the `SUMMARY` dispatch on canonical `LoopInterfaceProcessor::generate` has returned; Step 2 green.
- Postconditions: the helper is exercised by `slicer-core`'s own tests; no module calls it yet.
- Files allowed to edit: `crates/slicer-core/src/support_regularize.rs` (or a sibling module in the same crate) and its co-located tests.
- Out of bounds: both module `src/lib.rs` files (Step 4 wires them), both manifests, every test file listed for other steps.
- Dispatches: `SUMMARY` (≤200 words) on `LoopInterfaceProcessor::generate`; `FACT` on whether `slicer_sdk::host::offset_polygons` (or its prelude-resolved form) accepts a negative delta and what it returns for an emptied area.
- Cost: M
- Authorities: `design.md` §Data and Contract Notes (interface width, closure); `docs/08_coordinate_system.md` (the offset distance is scaled units, 1 unit = 100 nm).
- Verification: `cargo test -p slicer-core --features host-algos --no-fail-fast 2>&1 | tee target/test-output.log | grep -E "^test result"` — the `--features host-algos` form is mandatory here (`CLAUDE.md`: a bare `-p slicer-core` run silently compiles feature-gated targets to zero tests).
- Exit condition (falsifying): the helper returns an open polyline, or returns a loop for an island narrower than twice the interface line width.

### Step 4: Wire the loop pass into both renderers

- Task IDs: —
- Objective: `support_interface_loop_pattern = true` emits one closed loop per top-interface island in both families and shrinks the scan-filled area; `false` is byte-identical to the pre-step baseline.
- Preconditions: Steps 1–3 green.
- Postconditions: AC-4, AC-5, and AC-N2 green.
- Files allowed to edit: `modules/core-modules/traditional-support/src/lib.rs`, `modules/core-modules/tree-support/src/lib.rs`, `modules/core-modules/{traditional-support,tree-support}/tests/support_contact_loops_tdd.rs` (net-new).
- Out of bounds: `crates/slicer-core/**` (Step 3 froze the helper), both manifests, the scheduler and runtime test files (Step 5).
- Dispatches: `FACT` pass/fail per verification command.
- Cost: M
- Authorities: `requirements.md` §Per-Key Canonical Evidence (loop key row); `design.md` §Data and Contract Notes.
- Verification: `cargo test -p traditional-support --test support_contact_loops_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test support_contact_loops_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition (falsifying): the `false` path's emitted paths differ from the pre-step baseline, or the `true` path's open scan-line count does not strictly decrease.

### Step 5: Integration arms — bounds/type enforcement and CONFIG_BLOCK

- Task IDs: —
- Objective: the three keys are enforced at the scheduler boundary and appear in the CONFIG_BLOCK exactly when explicitly set.
- Preconditions: Steps 1–4 green.
- Postconditions: AC-6 and AC-7 green.
- Files allowed to edit: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`.
- Out of bounds: `crates/slicer-gcode/src/serialize.rs` — asserted against, never edited. Adding a key to `SUPPORT_CONFIG_DEFAULTS` or `ORCA_CONFIG_PADDING` fails the packet (Authoring rule 2).
- Dispatches: `FACT` pass/fail per command.
- Cost: S
- Authorities: `requirements.md` §Wiring notes (CONFIG_BLOCK).
- Verification: `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition (falsifying): `support_bottom_interface_spacing = -1.0` is rejected, or a default-path CONFIG_BLOCK carries any of the three keys.

### Step 6: Regenerate docs and close the gates

- Task IDs: —
- Objective: the generated reference reflects the new key and the removed deviation rows, and every packet gate is green.
- Preconditions: Steps 1–5 green.
- Postconditions: AC-8 green; `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and `cargo xtask build-guests --check` (exit 0) all pass.
- Files allowed to edit: `docs/15_config_keys_reference.md` (generated output only — produced by `cargo xtask gen-config-docs`, never hand-edited).
- Out of bounds: everything else.
- Dispatches: `FACT` exit code per gate.
- Cost: S
- Authorities: `packet.spec.md` §Doc Impact Statement.
- Verification: `cargo xtask gen-config-docs --check && rg -q 'support_interface_loop_pattern' docs/15_config_keys_reference.md && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c 'support_interface_spacing')" = "0" ]; echo "exit=$?"`, then `cargo xtask build-guests --check; echo "exit=$?"`
- Exit condition (falsifying): the deviations block still carries a `support_interface_spacing` row, or the pre/post row-count delta is anything other than -2, or `build-guests --check` returns non-zero (a distinct exit code means `wasm-tools` is missing — an infrastructure error, not freshness).

## Per-Step Budget Roll-Up

| Step | Cost | Notes |
| --- | --- | --- |
| 1 | S | Two manifests, two net-new guards |
| 2 | M | Blast radius of the 0.4 → 0.5 default across two modules, a fixture, and their tests |
| 3 | M | Net-new geometry helper + canonical dispatch |
| 4 | M | Two renderers wired, two net-new test files |
| 5 | S | Two integration arms |
| 6 | S | Doc regen + gates |

Aggregate: **M**. No step is L; no split required.

## Packet Completion Gate

- AC-1 … AC-8, AC-N1, AC-N2 all green by their own commands.
- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals` clean.
- `cargo xtask build-guests --check` returns exit 0 (both manifests and both `src/lib.rs` files are guest-fingerprint inputs, and `slicer-core` sits in both guests' dependency closure — expect a rebuild before the check goes green).
- Map gate (a): the disposition table in `requirements.md` lists zero declaration-only keys. Map gate (b): every kept key has a non-default-value behaviour AC.

## Acceptance Ceremony

Run `cargo xtask test --summary --workspace` once, at closure only, after every narrower command above has passed, and dispatch it to a sub-agent with a `FACT pass/fail` return (`CLAUDE.md` § Test Discipline). Do not absorb the full output.
