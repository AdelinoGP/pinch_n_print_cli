# Design: 200-batched-host-bridge-wasm-arms

## Controlling Code Paths

- Primary code path: `crates/slicer-sdk/src/host.rs` (the seven unbridged wrappers: `raycast_z_down`, `surface_normal_at`, `object_bounds`, `clip_polygons`, `offset_polygons`, `simplify_polygon`, `now_us`) → WIT imports declared in `crates/slicer-schema/wit/deps/common.wit` `host-services` → host trait impls in the `impl hs::Host for HostExecutionContext` block of `crates/slicer-wasm-host/src/host.rs` (all seven already implemented, plus the five `*_batch` forms via `crate::batch::map_batch`).
- Consumer path: `modules/core-modules/classic-perimeters/src/lib.rs` `run_perimeters`/thin-wall/gap-fill regions (direct `slicer_core::polygon_ops` calls); `modules/core-modules/support-planner/src/lib.rs` collision cache (already on `slicer_sdk::host_batch::batch_offset` since commit `088a7a74`).
- Neighboring tests/fixtures: `crates/slicer-sdk/tests/host_wrappers_tdd.rs`, `crates/slicer-sdk/tests/smoke.rs`, `crates/slicer-sdk/src/host_batch.rs` `#[cfg(test)]` block, `crates/slicer-wasm-host/tests/contract/host_services_tdd.rs`, `crates/slicer-wasm-host/tests/unit/batch_call_audit_tdd.rs`, `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs` (harness pattern to mirror), `crates/slicer-runtime/tests/integration/main.rs` (aggregator; S7).
- OrcaSlicer comparison: none — this packet relocates where identical code executes; no canonical behavior is ported. (No §OrcaSlicer Reference Obligations anywhere in this packet, deliberately.)

## Architecture Constraints

- **ADR-0033 four-layer shape is mandatory for every arm**: WIT decl (exists) → host impl delegating to native code (exists) → `cfg`-split SDK wrapper whose wasm32 arm marshals the import (this packet) → guests call only the wrapper. The wasm32 arms use the established inline import-only `wit_bindgen::generate!` mini-world pattern (`log` / `medial_axis` / `generate_arachne_walls` in `crates/slicer-sdk/src/host.rs`, `mod wit` in `host_batch.rs`): one new shared world declaring the seven singular funcs and their types. Component imports resolve structurally, so the mini-world's package name is irrelevant — only the wire shape must match canonical `common.wit`. The inline copies MUST be updated in the same edit as any canonical WIT change; a drifted record shape fails typed instantiation for every guest.
- `slicer-sdk` enables `host-algos` only under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` and takes `wit-bindgen` only under `[target.'cfg(target_arch = "wasm32")'.dependencies]` (verified in `crates/slicer-sdk/Cargo.toml`) — the new arms must not disturb either gate.
- **ADR-0049 constraints**: batch results stay input-ordered; singular forms stay; fan-out is the host's estimated-work decision (`crate::batch::map_batch`) and callers leave it alone; adoption is one module at a time with the parity suites green in between; marshalling cost is the first suspect if an adoption measures slower.
- **ADR-0055 evidence discipline**: fuel is the primary signal (deterministic; host calls burn no fuel, so moving work host-side shows as a guest-fuel drop by construction — that alone proves routing, not speed); wall-clock is secondary and MUST come from profiling-off runs (wall-clock under `--profile` is inflated by mark host calls); DEV-093 makes whole-slice fuel totals drift slightly, so compare per-(module, scope) rows, and never use G-code byte diffs as evidence.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Config keys are snake_case everywhere: the test guest's probe key is `bridge_probe_object`; `perimeter_arc_tolerance` is read as-is and its semantics are untouched.

## Code Change Surface

- Selected approach — four moves:
  1. **Arc-tolerance contract** (parity prerequisite): every `offset` call in classic-perimeters passes `self.perimeter_arc_tolerance` (default 0.0125, non-zero), but WIT `offset-polygons`, the host impl, and the SDK wrapper all pin arc tolerance to `0.0`. Migrating onto the wrapper as-is would silently change an argument. Fix the carrier: add `arc-tolerance-mm: f32` to the WIT singular func + `offset-request` record; host `offset_polygons`/`offset_polygons_batch` pass it through to `slicer_core::polygon_ops::offset` (whose 4th parameter it already is); `slicer_sdk::host::offset_polygons(polygons, delta_mm, join, arc_tolerance_mm)` and `host_batch::OffsetRequest { …, arc_tolerance_mm }` carry it. Existing callers pass `0.0` (behavior-identical).
  2. **Seven wasm32 arms** in `crates/slicer-sdk/src/host.rs`: one shared inline world (working name `__sdk_host_services_import`, or a sibling `crates/slicer-sdk/src/host_wit.rs` module if rustfmt/readability demands — AC-2 accepts either file) declaring `point2`, `point3`, `bounding-box3`, `polygon`, `ex-polygon`, `object-id`, `clip-operation`, `offset-join-type`, and the seven funcs with post-move-1 shapes. Each wrapper body gains `#[cfg(target_arch = "wasm32")] { …marshal… }`; native arms byte-for-byte unchanged. `object_bounds`'s wasm32 arm wraps the import result in `Ok(...)` (host raises on unknown ids → trap, so `HostUnavailable` is never constructed on wasm32).
  3. **classic-perimeters migration**: 8 `offset` sites → `host::offset_polygons` (tolerance carried), 4 `difference_ex` sites → `host::clip_polygons(…, ClipOperation::Difference)` — identical native path since `difference_ex` → `difference` → `clip_polygons(…, Difference)` inside `slicer_core::polygon_ops`. 1:1 singular-call migration, no loop restructuring into batches: the inset loop is serially dependent (each inset consumes the previous result), classic-perimeters already receives host layer fan-out, and ADR-0049 keeps singular forms exactly for this. Dual-enum hazard: `slicer_sdk::host::{OffsetJoinType, ClipOperation}` vs `slicer_core::polygon_ops::OffsetJoinType` (still needed by `offset2_ex`/`opening_ex`) — alias one side at the import (`as CoreJoin` / `as HostJoin`).
  4. **Evidence + docs**: baseline (pre-change) and post-migration `--profile` captures + profiling-off wall-clock; ADR-0055 amendment; DEV-094 row closure; docs/05 caveat retirement.
- Exact functions/records touched: WIT `offset-polygons`, `offset-request`; host `fn offset_polygons`, `fn offset_polygons_batch` (`impl hs::Host for HostExecutionContext`); SDK `pub fn offset_polygons`, `pub struct OffsetRequest`, `pub fn offset_polygons_batch` native arm + `mod wit` inline world in `host_batch.rs`; the seven wrapper bodies in `crates/slicer-sdk/src/host.rs`; classic-perimeters call sites (12) + its `use slicer_core::polygon_ops::{…}` list; new guest crate `sdk-host-bridge-guest`; new test `host_bridge_roundtrip_tdd.rs` + `mod` line in `integration/main.rs`.
- Rejected alternatives:
  - *Migrate offset sites without the arc-tolerance field, arguing Clipper2 ignores arc tolerance under Miter joins.* Rejected: rests on an upstream implementation detail this repo does not pin; a wrong guess silently moves geometry. Carrying the parameter is mechanical and makes AC-6's zero-re-record claim unconditional.
  - *Bridge `offset2_ex`/`opening_ex` via new WIT funcs now.* Rejected: new host services deserve their own ADR-0049-shaped decision, and this packet's evidence exists to price that follow-up; scope stays on already-declared services (plan queue row 1 wording).
  - *Batch-restructure classic-perimeters' gap-collection pairs.* Rejected: 2-item batches under the estimated-work threshold run serially host-side anyway; the churn buys nothing measurable and enlarges the parity diff.
  - *Reconstruct a pre-`088a7a74` support-planner baseline (revert-and-measure).* Rejected: ADR-0055's fuel instrument did not exist before `088a7a74` landed (same day), so a faithful "before" is unconstructible; the commit's own wall-clock record stands as the before-evidence and this packet verifies the adoption rather than re-measuring its delta.

## Files in Scope (read + edit)

Seven primary files is above the 3-file target; the excess is structural, not repetition — ADR-0033's four-layer bridge spans schema/host/SDK/consumer by definition, and each layer is one file:

- `crates/slicer-schema/wit/deps/common.wit` — role: canonical contract; change: `arc-tolerance-mm` on `offset-polygons` + `offset-request`.
- `crates/slicer-wasm-host/src/host.rs` — role: host impls; change: pass-through in `offset_polygons` / `offset_polygons_batch` (two functions only; file otherwise out of bounds).
- `crates/slicer-sdk/src/host.rs` — role: wrapper layer; change: seven wasm32 arms + one inline world + `offset_polygons` signature.
- `crates/slicer-sdk/src/host_batch.rs` — role: batch wrappers; change: `OffsetRequest.arc_tolerance_mm` field, native/wasm arms, inline WIT copy, its two `#[cfg(test)]` literals + doc example.
- `modules/core-modules/classic-perimeters/src/lib.rs` — role: hot consumer; change: 12 call sites + import list.
- `crates/slicer-wasm-host/test-guests/sdk-host-bridge-guest/` (new: `Cargo.toml` with `[workspace]` sentinel + `src/lib.rs`) — role: e2e proof guest.
- `crates/slicer-runtime/tests/integration/host_bridge_roundtrip_tdd.rs` (new) + `crates/slicer-runtime/tests/integration/main.rs` (one `mod` line) — role: e2e proof + S7 registration.

Secondary (same-step fallout, ≤3 lines each): `modules/core-modules/support-planner/src/lib.rs` (`OffsetRequest` literal in the `batch_offset` closure + three `#[cfg(test)]` `host::offset_polygons` calls gain `, 0.0`), `crates/slicer-sdk/tests/host_wrappers_tdd.rs`, `crates/slicer-sdk/tests/smoke.rs` (wrapper-call arity), docs per Doc Impact (`docs/05_module_sdk.md`, `docs/DEVIATION_LOG.md`, `docs/adr/0055-fuel-based-module-profiling.md`).

## Read-Only Context

- `crates/slicer-wasm-host/src/host.rs` — the `impl hs::Host for HostExecutionContext` block (≈ lines 2425–2735) and `ir_offset_polygons` (≈ 2903) only — purpose: impl shapes; `ir_offset_polygons` keeps its hardcoded `0.0` (test helper, not a WIT surface).
- `crates/slicer-sdk/src/host.rs` — wrapper bodies: `log` wasm32 arm (≈ 137–195, the mini-world template), mesh queries (≈ 222–280), polygon ops (≈ 282–366), `medial_axis` wasm32 marshal (≈ 388–495, the ExPolygon marshal template), `now_us` (≈ 856–869).
- `crates/slicer-sdk/src/host_batch.rs` — whole file is 469 lines; the inline `mod wit` (≈ 289–394) is the record-marshal template.
- `modules/core-modules/classic-perimeters/src/lib.rs` — windows only: imports (25–45), config (95–115), top-surface split (475–505), inset+gap loop (676–795), thin-wall (960–990), gap-fill filter (1040–1065), infill inset (1140–1155), second inset loop (1200–1215).
- `modules/core-modules/support-planner/src/lib.rs` — `batch_offset` window (≈ 268–320) and the three test literals (≈ 1928, 1991, 2035).
- `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs` — harness/mesh-builder pattern (`cube_mesh`, `blackboard_with_layer_plan`).
- `xtask/src/build_guests.rs` — `discover_guests` only (confirm auto-discovery contract for the new guest).

## Out-of-Bounds Files

- `.ralph/specs/194-*` … `198-*` (parallel plan), `docs/07_implementation_status.md`, `CONTEXT.md`, `docs/specs/multi-edition-distribution-plan.md` (orchestrator-owned queue) — never edit.
- `docs/adr/*` other than the ADR-0055 amendment appended by Step 7 (ADR-0049's 2026-08-05 amendment is self-dated and stays untouched).
- `OrcaSlicerDocumented/` — not consulted by this packet at all.
- `target/`, `Cargo.lock` (except the mechanical entry for the new test-guest crate if Cargo writes one), generated bindgen output, vendored deps — never load.
- All other core modules (`arachne-perimeters`, `layer-planner-default`, …) — behavioral consumers, not edit targets; verify via their suites, delegate symbol lookups.

## Expected Sub-Agent Dispatches

- Question: run the release evidence slice and return the per-(module, scope) rows for `com.core.classic-perimeters` and `com.core.support-planner` plus `fuel_total`/`wall_total_ns`; scope: the AC-7 command chain; return: `FACT` (≤5 lines: rows + exit codes); purpose: Steps 1 and 6.
- Question: run 3 profiling-off release slices and return the three wall-clock timings + median; scope: `cargo run --bin pnp_cli --release -- slice --model resources/extruder_idler.obj --module-dir modules/core-modules --output target/p200-nop.gcode` (timed); return: `FACT`; purpose: Steps 1 and 6 (absolute timings per ADR-0055).
- Question: does `cargo xtask build-guests --check` report clean?; scope: workspace root; return: `FACT` (clean | STALE list); purpose: after Steps 2, 3, 4, 5.
- Question: full-suite verdicts for the matrix commands; scope: each command in `requirements.md` §Verification Commands; return: `FACT pass/fail` + ≤20-line failure SNIPPETS from `target/test-output.log`; purpose: step exits and the ceremony.

## Data and Contract Notes

- IR/manifest contracts: none change. No IR schema, no manifest key, no claim, no scheduler behavior. The new config key `bridge_probe_object` exists only in the test guest's manifest (test-guest manifests are not core-module manifests).
- WIT boundary: one additive field on one func + one record. Every guest rebuilds (34 guests; the freshness gate is the enforcement). All four inline SDK mini-worlds are audited in Step 3: only `host_batch.rs`'s world declares `offset-request` and must gain the field; the `log`/`medial-axis`/`arachne` worlds declare neither `offset-polygons` nor `offset-request` and are untouched.
- ADR-0049 conformance (S8 pre-empt): ADR-0049's Decision section quotes an `offset-request` record with three fields as an *illustration* of the batched-import shape; its normative clauses are per-item parameters, input-ordered results, estimated-work fan-out gating, and "singular forms stay" — all preserved. Adding a per-item `arc-tolerance-mm` field follows the stated per-item-parameters principle rather than contradicting the ADR; no amendment row is required.
- ADR-0055 amendment conformance (S8 pre-empt): ADR-0055 explicitly leaves "the in-guest-vs-host-native question stays open" and names the profiler as "the instrument that makes it decidable on evidence". Appending an amendment that records the measured answer executes the ADR's own program; no normative clause is contradicted, so no `D-…-ADR-0055-AMENDED` deviation row is required (contrast D-285-ADR-0051-AMENDED, which retired an obligation). The DEV-094 row update follows the log's own convention: status cell must begin with `Closed`.
- Determinism: batch results are input-ordered by contract; the singular arms are plain synchronous calls; nothing in this packet introduces scheduling-dependent output. On wasm32 the migrated geometry moves from in-sandbox clipper2 to host-native clipper2 — same crate, same inputs; ADR-0049 records this as the class of change the parity suites backstop, hence the one-module-at-a-time ordering.

## Locked Assumptions and Invariants

- The native arms of all seven wrappers keep their exact current semantics (MeshSource thread-local, local collinear-dropper, process-start `Instant`) so module unit tests and native harnesses run without a runtime — ADR-0033 layer 3 and ADR-0049 §Decision both lock this.
- `arc_tolerance_mm = 0.0` is the value every pre-existing caller passes after the signature change; no default-value drift anywhere.
- AC-6 (zero fixture re-record) is a hard invariant for Step 5: red fixtures mean the migration is wrong. Per Test Discipline, never re-record to make it pass.
- Evidence ACs assert measurement, never improvement (ADR-0049 marshalling caveat).

## Risks and Tradeoffs

- **Wall-clock regression risk (pre-declared decision rule):** if Step 6's profiling-off median regresses beyond the run-to-run spread measured in Step 1, keep the bridge arms (DEV-094's defect is the phantom bridge, not consumer adoption), revert only the Step 5 call-site migration, and record the measured regression as the ADR-0055 amendment's answer. DEV-094 still closes; the amendment then documents that in-guest was retained on evidence. [FWD-2 encodes this for the implementer.]
- **Trap-on-unknown-object is a behavior change**: pre-fix guests got silent `None` from mesh queries; post-fix an unknown id raises through the host. Audited consumers: `layer-planner-default` calls `object_bounds` only with ids from its own object views (host-known), `support-planner`'s mesh-query use is batch-side. AC-N1 pins the new loud behavior deliberately.
- **Inline-world drift**: the SDK now has five inline WIT copies of parts of `host-services`. Mitigated by: structural-typing (drift fails instantiation loudly, before any wrong answer), the Step 3 audit, and the freshness gate.
- **DEV-093 fuel jitter**: whole-slice fuel totals can drift on a handful of layers; the evidence compares per-(module, scope) rows and states the caveat in the amendment text.
- **Windows/Git Bash command portability**: AC commands use `sh -c` + `rg` only (no python), per this machine's toolchain.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5 — twelve call-site edits in a 1,466-line module, windowed)
- Highest-risk dispatch and required return format: the release evidence slices (Steps 1/6) — `FACT` with the extracted per-scope rows only; absorbing slice stderr JSONL is the fastest way to blow the budget.

## Open Questions

- **[FWD-1]** Evidence model: `resources/extruder_idler.obj` is chosen (real part, in-tree; `enable_support` defaults to `true` so `com.core.support-planner` runs). If its geometry yields no support work (empty `SupportGeometryIR` → near-zero support-planner fuel), fall back to `resources/overhang.obj` or `resources/bridge.obj` and use the same model for both captures; record the choice in the amendment. Resolvable by the implementer at Step 1 (AC-7 asserts the module appears in the summary, so a bad model choice fails loudly, not silently).
- **[FWD-2]** Regression handling: apply the pre-declared decision rule in §Risks verbatim; do not invent a third outcome (e.g. partial re-migration) without recording it in the amendment.
- **[FWD-3]** If `cargo xtask build-guests` reports the new `sdk-host-bridge-guest` missing a per-guest `[workspace]` sentinel or shared-target-dir convention, copy the exact `Cargo.toml` scaffolding of `sdk-support-diagnostic-guest` — the discovery contract is directory-based (`discover_guests` in `xtask/src/build_guests.rs`), not a registration list.
