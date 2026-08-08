# Requirements: 200-batched-host-bridge-wasm-arms

## Packet Metadata

- Grouped task IDs: `DEV-094` (deviation-log anchor; `docs/07_implementation_status.md` has no TASK rows for the distribution program — see the plan's Backlog anchoring [FWD])
- Backlog source: `docs/DEVIATION_LOG.md` (DEV-094) + `docs/specs/multi-edition-distribution-plan.md` (queue row 1)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

DEV-094 ("phantom host bridge", filed 2026-07-25): the `slicer:common/host-services` WIT interface declares mesh queries, polygon ops, and `now-us`, the host implements all of them in `crates/slicer-wasm-host/src/host.rs`, but the guest-callable SDK wrappers in `crates/slicer-sdk/src/host.rs` never call the imports. Only `log` (2026-07-25 partial remediation), `medial_axis`, and `generate_arachne_walls` carry the ADR-0033 `#[cfg(target_arch = "wasm32")]` arm. Seven wrappers remain unbridged: `raycast_z_down`, `surface_normal_at`, `object_bounds` (phantom — thread-local `MeshSource` has no production installer, so guests always get `None`/`Err`), `clip_polygons`, `offset_polygons`, `simplify_polygon` (correct-but-misplaced — clipper2 runs inside the sandbox, single-threaded), and `now_us` (phantom — `std::time::Instant` is unavailable on `wasm32-unknown-unknown`).

Two adjacent gaps make this one coherent slice rather than three: (a) `classic-perimeters` bypasses the wrappers entirely for its hot loops (`use slicer_core::polygon_ops::{difference_ex, offset, offset2_ex, opening_ex, remove_small_and_small_holes}`), so bridging the wrappers alone moves none of the measured-hot work; (b) ADR-0055 records the in-guest-vs-host-native question as OPEN because wall-clock noise swallowed the 2026-07-25 measurement — the fuel profiler this packet uses as its evidence instrument was built precisely to make this packet's question decidable.

**Plan-assumption correction (supersedes queue-row wording, no packet superseded):** the plan lists `support-planner` as a hot consumer to migrate. That migration already landed — commit `088a7a74` (2026-07-25) moved its collision-cache loop onto `slicer_sdk::host_batch::batch_offset`, and the same commit shipped `host_batch.rs` (all five batch wrappers, wasm32 arms included) and the host-side `map_batch` fan-out. What that commit explicitly did NOT deliver (its own message: "NOT verified: that adoption is behaviour-preserving") is verification and evidence; this packet supplies those.

## In Scope

- `#[cfg(target_arch = "wasm32")]` bridge arms for the seven singular wrappers in `crates/slicer-sdk/src/host.rs`, via one shared inline import-only `wit_bindgen::generate!` world following the existing `log`/`medial_axis` mini-world pattern. Native arms unchanged.
- WIT contract extension for parity-preserving migration: `arc-tolerance-mm: f32` added to the singular `offset-polygons` func and the `offset-request` record in `crates/slicer-schema/wit/deps/common.wit`; host impls (`offset_polygons`, `offset_polygons_batch`) pass it into `slicer_core::polygon_ops::offset`; `slicer_sdk::host::offset_polygons` gains an `arc_tolerance_mm: f32` parameter; `slicer_sdk::host_batch::OffsetRequest` gains an `arc_tolerance_mm: f32` field. Full struct-literal and call-site blast radius (enumerated in `implementation-plan.md` Step 2) is owned by the same step.
- Migration of `classic-perimeters`' direct calls where a WIT-equivalent service exists: 8 `offset` call sites → `slicer_sdk::host::offset_polygons` (carrying `self.perimeter_arc_tolerance`), 4 `difference_ex` call sites → `slicer_sdk::host::clip_polygons(..., ClipOperation::Difference)` (`difference_ex` delegates to `clip_polygons(…, Difference)` inside `slicer_core::polygon_ops`, so this is the identical native code path).
- End-to-end bridge proof: new test guest `crates/slicer-wasm-host/test-guests/sdk-host-bridge-guest/` (auto-discovered by `discover_guests` in `xtask/src/build_guests.rs`; no registration list) + new integration test `crates/slicer-runtime/tests/integration/host_bridge_roundtrip_tdd.rs` registered in `crates/slicer-runtime/tests/integration/main.rs`, modeled on `prepass_diagnostic_roundtrip_tdd.rs`'s dispatch harness.
- A/B evidence per ADR-0055: baseline capture before any code change, post-migration capture after; fuel primary, profiling-off wall-clock secondary; recorded in an ADR-0055 amendment + the DEV-094 closure row. ACs require the measurement, not a win (ADR-0049's amendment flags marshalling cost for large-polygon/small-compute calls; a measured regression is a valid, recordable outcome with a pre-declared revert rule — see `design.md` §Risks).
- Verification of `support-planner`'s already-landed batch adoption: the e2e profile run shows the module executing through the batch path with a successful slice, and the existing forced-serial/forced-parallel equality test in `crates/slicer-wasm-host/src/batch.rs` is cited as the determinism gate. Byte-level G-code attribution stays blocked on DEV-093 and is not claimed.
- Doc edits per the Doc Impact Statement: `docs/05_module_sdk.md` §Host Service Wrappers, `docs/DEVIATION_LOG.md` DEV-094 row, `docs/adr/0055-fuel-based-module-profiling.md` amendment.

## Out of Scope

- New WIT host services: `offset2_ex`, `opening_ex` (fused two-pass offsets) have no WIT form; they stay in-guest. Adding them is a new-service decision to be priced by this packet's evidence, not taken inside it.
- The through-slicer-core clipper2 path: `classic-perimeters` reaches clipper2 via `slicer_core::top_surface_split::split_top_surfaces`, which a wrapper layer cannot see (ADR-0055 §Decision records exactly this). It stays in-guest; the fuel scope marks at the three `slicer-core` primitives (`clip_polygons`, `offset`, `offset2_ex`) attribute it in the evidence, so the residual in-guest share is quantified, not migrated. Moving it host-side would be a new ADR-0033 four-layer bridge — follow-up.
- `remove_small_and_small_holes` migration: it is a pure area-retain filter with no clipper2 call; nothing to bridge.
- DEV-094's "stronger fix" (making a missing bridge a compile error on wasm32 instead of a silent native fallback): with all wrappers bridged, no current wrapper needs it; the guard remains available to a future packet and the DEV-094 closure text says so.
- DEV-093 (medial-axis nondeterminism) and any G-code byte-diff acceptance.
- Re-migrating `support-planner` or reconstructing a pre-`088a7a74` baseline for it.
- Integrated modules, editions, dispatch routing, `docs/07_implementation_status.md` edits (parallel-session freeze per the plan), and packets 194–199 files.
- Behavior changes to `perimeter_arc_tolerance` semantics (the value is carried, never reinterpreted).

## Authoritative Docs

- `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — 42 lines; direct.
- `docs/adr/0049-batched-host-services-over-threaded-guests.md` — 186 lines; direct (Decision/Consequences/Amendment sections).
- `docs/adr/0055-fuel-based-module-profiling.md` — 127 lines; direct.
- `docs/adr/0056-integrated-modules-native-dispatch.md` — 122 lines; direct read of §Context only (why DEV-094 is the perf lever).
- `docs/DEVIATION_LOG.md` — large; delegate; only the `^\| DEV-094` row.
- `docs/05_module_sdk.md` — large; ranged read of §Host Service Wrappers only.
- `docs/08_coordinate_system.md` — delegate if unit questions arise; the wrappers take mm-valued deltas/tolerances and 100 nm lattice points unchanged.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9`. Refinements: AC-3/AC-4 are the load-bearing falsifiers (both guest behaviors are impossible on the pre-fix tree: mesh queries return `None`, `now_us` traps); AC-6's zero-fixture-re-record claim holds because the native arms call the identical `slicer_core::polygon_ops` functions with identical arguments — any fixture movement falsifies the migration, and per Test Discipline the fix is the migration, never the fixture.
- Negative: `AC-N1` (unknown-object mesh query fails loudly through the bridge; the pre-fix path silently returned `None`).
- Cross-packet impact: `arachne-perimeters`' single `slicer_sdk::host::clip_polygons` call site and `layer-planner-default`'s `host::object_bounds` fallback switch from inert in-guest fallbacks to live host calls without those modules being edited — both are named consumers in `design.md` and covered by the module suites in the verification matrix. Packet 204 (hybrid pilot) consumes this packet's evidence and the bridged wrappers.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `sh -c 'rg -U -q "offset-polygons:[^;]*arc-tolerance-mm: f32" crates/slicer-schema/wit/deps/common.wit && rg -U -q "record offset-request \{[^}]*arc-tolerance-mm: f32" crates/slicer-schema/wit/deps/common.wit && echo PASS'` | AC-1 WIT contract | FACT PASS/fail |
| `sh -c 'for f in raycast-z-down surface-normal-at object-bounds clip-polygons "offset-polygons:" simplify-polygon now-us; do rg -q -- "$f" crates/slicer-sdk/src/host.rs || rg -q -- "$f" crates/slicer-sdk/src/host_wit.rs || { echo "MISSING: $f"; exit 1; }; done; echo PASS'` | AC-2 bridge-arm tripwire | FACT PASS/fail + missing name |
| `cargo xtask build-guests --check` | guest freshness after WIT/SDK/module edits | FACT clean/STALE list |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- host_bridge 2>&1 \| tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` | AC-3/AC-4/AC-N1 e2e bridge proof | FACT pass/fail; failing-test SNIPPETS ≤20 lines from the log |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- perimeter_parity gap_fill_emission 2>&1 \| tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` | AC-6 native parity stability | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-sdk --features test 2>&1 \| tee target/test-output.log; rg -q "^test result: ok" target/test-output.log && ! rg -q "test result: FAILED" target/test-output.log && echo PASS \|\| echo FAIL` | SDK wrapper suites (host_wrappers_tdd, host_batch tests, smoke) still green after signature change; compile failure = FAIL (no ok-summary line) | FACT PASS/fail |
| `mkdir -p target && cargo test -p slicer-wasm-host --test contract host_services 2>&1 \| tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` | host-side singular/batch impls green after the arc-tolerance pass-through | FACT pass/fail |
| AC-7 chained command from `packet.spec.md` (slice `--profile` + `profile --from … --json` + module-id greps) | evidence capture exists and names both hot modules | FACT PASS/fail |
| `sh -c 'rg -q "^\| DEV-094 \|.*\| Closed" docs/DEVIATION_LOG.md && rg -q "^## Amendment" docs/adr/0055-fuel-based-module-profiling.md && rg -q "in-guest" docs/adr/0055-fuel-based-module-profiling.md && echo PASS'` | AC-8 evidence recorded | FACT PASS/fail |
| `sh -c '! rg -q "the host bridge is not wired" docs/05_module_sdk.md && rg -q "bridged to the host" docs/05_module_sdk.md && echo PASS'` | AC-9 docs/05 caveat retired | FACT PASS/fail |
| `cargo check --workspace --all-targets` | whole-tree compile incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation; every `cargo test` tees to `target/test-output.log` per `CLAUDE.md` and results are read from the log, never re-run for output.

## Step Completion Expectations

- **Baseline before code:** Step 1's baseline artifacts (`target/p200-profile-before.jsonl`, profiling-off wall-clock medians, per-scope fuel rows for `com.core.classic-perimeters` and `com.core.support-planner`) must be captured from the unmodified tree and the numbers copied into the step's completion note *before* any Step 2+ edit lands. `target/` artifacts are disposable; the quoted numbers in the note are the durable record the Step 7 amendment consumes.
- **WIT before arms:** Step 2 (arc-tolerance field) must land before Step 3 writes the SDK inline import world, so the inline `offset-polygons` declaration is written once with the final wire shape (a mismatched inline record fails typed instantiation for every guest).
- **Rebuild between every guest-feeding edit and any test run:** `cargo xtask build-guests` (then `--check` clean) after Steps 2, 3, 4, and 5 — WIT, `slicer-sdk`, and `modules/core-modules` edits all invalidate guests.
- **One consumer at a time (ADR-0049):** Step 5's classic-perimeters migration lands only after Step 4's bridge-proof tests are green, and Step 5's own parity suites run before Step 6 measures.

## Context Discipline Notes

- `crates/slicer-sdk/src/host.rs` is 899 lines and `crates/slicer-wasm-host/src/host.rs` is ~5,000: ranged reads only (wrapper bodies at the line anchors given in `design.md`; host trait impls in the `impl hs::Host for HostExecutionContext` block).
- `modules/core-modules/classic-perimeters/src/lib.rs` is 1,466 lines: open only the call-site windows enumerated in `implementation-plan.md` Step 5; never load the file whole.
- `modules/core-modules/support-planner/src/lib.rs` (2,058 lines) is read-only context: only the `batch_offset` window (~lines 268–320) and the three `#[cfg(test)]` `host::offset_polygons` literals matter.
- The evidence slices are release builds of a real model — always dispatch `cargo run --release -- slice` invocations to a worker returning FACT + the extracted summary rows; never absorb slice stderr (the JSONL stream is thousands of lines).
