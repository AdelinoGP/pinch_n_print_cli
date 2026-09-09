---
status: implemented
packet: 200-batched-host-bridge-wasm-arms
task_ids:
  - DEV-094
---

# 200-batched-host-bridge-wasm-arms

## Goal

Close DEV-094 by adding the `#[cfg(target_arch = "wasm32")]` bridge arm to all seven still-unbridged SDK host-service wrappers in `crates/slicer-sdk/src/host.rs` (three mesh queries, three polygon ops, `now_us`) per the ADR-0033 four-layer shape, carrying `arc-tolerance-mm` through the WIT `offset-polygons`/`offset-request` contract so `classic-perimeters`' direct `slicer_core::polygon_ops` call sites can migrate onto the wrappers geometry-identically, and produce the fuel/wall-clock before/after evidence that closes ADR-0055's open in-guest-vs-host-native question.

## Problem Statement

DEV-094 ("phantom host bridge", filed 2026-07-25): the `slicer:common/host-services` WIT interface declares mesh queries, polygon ops, and `now-us`, the host implements all of them in `crates/slicer-wasm-host/src/host.rs`, but the guest-callable SDK wrappers in `crates/slicer-sdk/src/host.rs` never call the imports. Only `log` (2026-07-25 partial remediation), `medial_axis`, and `generate_arachne_walls` carry the ADR-0033 `#[cfg(target_arch = "wasm32")]` arm. Seven wrappers remain unbridged: `raycast_z_down`, `surface_normal_at`, `object_bounds` (phantom — thread-local `MeshSource` has no production installer, so guests always get `None`/`Err`), `clip_polygons`, `offset_polygons`, `simplify_polygon` (correct-but-misplaced — clipper2 runs inside the sandbox, single-threaded), and `now_us` (phantom — `std::time::Instant` is unavailable on `wasm32-unknown-unknown`).

Two adjacent gaps make this one coherent slice rather than three: (a) `classic-perimeters` bypasses the wrappers entirely for its hot loops (`use slicer_core::polygon_ops::{difference_ex, offset, offset2_ex, opening_ex, remove_small_and_small_holes}`), so bridging the wrappers alone moves none of the measured-hot work; (b) ADR-0055 records the in-guest-vs-host-native question as OPEN because wall-clock noise swallowed the 2026-07-25 measurement — the fuel profiler this packet uses as its evidence instrument was built precisely to make this packet's question decidable.

**Plan-assumption correction (supersedes queue-row wording, no packet superseded):** the plan lists `support-planner` as a hot consumer to migrate. That migration already landed — commit `088a7a74` (2026-07-25) moved its collision-cache loop onto `slicer_sdk::host_batch::batch_offset`, and the same commit shipped `host_batch.rs` (all five batch wrappers, wasm32 arms included) and the host-side `map_batch` fan-out. What that commit explicitly did NOT deliver (its own message: "NOT verified: that adoption is behaviour-preserving") is verification and evidence; this packet supplies those.

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
