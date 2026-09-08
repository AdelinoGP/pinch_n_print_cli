---
status: implemented
packet: 200-batched-host-bridge-wasm-arms
task_ids:
  - DEV-094
backlog_source: docs/DEVIATION_LOG.md (DEV-094 row) + docs/specs/multi-edition-distribution-plan.md (queue row 1)
context_cost_estimate: M
---

# Packet Contract: 200-batched-host-bridge-wasm-arms

## Goal

Close DEV-094 by adding the `#[cfg(target_arch = "wasm32")]` bridge arm to all seven still-unbridged SDK host-service wrappers in `crates/slicer-sdk/src/host.rs` (three mesh queries, three polygon ops, `now_us`) per the ADR-0033 four-layer shape, carrying `arc-tolerance-mm` through the WIT `offset-polygons`/`offset-request` contract so `classic-perimeters`' direct `slicer_core::polygon_ops` call sites can migrate onto the wrappers geometry-identically, and produce the fuel/wall-clock before/after evidence that closes ADR-0055's open in-guest-vs-host-native question.

## Scope Boundaries

In: SDK singular-wrapper wasm32 arms; the `arc-tolerance-mm` field on the WIT `offset-polygons` func and `offset-request` record (host impl, SDK wrapper, `host_batch` record, and all struct-literal/call-site fallout); migration of `classic-perimeters`' `offset`/`difference_ex` call sites onto `slicer_sdk::host` wrappers; an end-to-end bridge-proof test guest plus integration test; ADR-0055-methodology A/B evidence recorded in an ADR-0055 amendment and the DEV-094 closure row. Out: any new WIT host service (`offset2_ex`/`opening_ex` have no WIT form and stay in-guest), the through-slicer-core clipper2 path (`slicer_core::top_surface_split::split_top_surfaces` stays in-guest; measured, not migrated), the DEV-094 "stronger fix" (compile-error-on-missing-bridge guard), DEV-093, and every integrated-module/edition concern (packets 201+). `support-planner`'s collision-cache batch migration already landed in commit `088a7a74` (2026-07-25); this packet verifies it and records evidence — it does not re-migrate it.

## Prerequisites and Blockers

- Depends on: nothing in the 200-series queue (queue row 1). Consumes only shipped code: the batch WIT declarations and host impls (`offset_polygons_batch` etc. in `crates/slicer-wasm-host/src/host.rs`), `slicer_sdk::host_batch`, and the `--profile` pipeline (ADR-0055) — all verified present on `master`.
- Unblocks: packet 204-hybrid-pilot-parity (its perf story assumes DEV-094 is done, per ADR-0056 §Context) and any future packet pricing an `offset2-ex` host service from this packet's evidence.
- Activation blockers: none known. Status stays `draft` until the batch-queue activation gate.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the canonical WIT at `crates/slicer-schema/wit/deps/common.wit`, **when** the arc-tolerance contract change lands, **then** the singular `offset-polygons` func signature contains `arc-tolerance-mm: f32` and the `offset-request` record contains an `arc-tolerance-mm: f32` field (kebab-case, exactly this identifier). | `sh -c 'rg -U -q "offset-polygons:[^;]*arc-tolerance-mm: f32" crates/slicer-schema/wit/deps/common.wit && rg -U -q "record offset-request \\{[^}]*arc-tolerance-mm: f32" crates/slicer-schema/wit/deps/common.wit && echo PASS'`
- **AC-2. Given** `crates/slicer-sdk/src/host.rs` after the bridge arms land, **when** grepping the SDK for the seven singular host-service import declarations, **then** each kebab-case func name — `raycast-z-down`, `surface-normal-at`, `object-bounds`, `clip-polygons`, `offset-polygons`, `simplify-polygon`, `now-us` — appears in an inline import-only WIT world inside `crates/slicer-sdk/src/` (today none of these seven kebab identifiers appears anywhere under `crates/slicer-sdk/src/host.rs`; `host_batch.rs` declares only the `-batch` forms). | `sh -c 'for f in raycast-z-down surface-normal-at object-bounds clip-polygons "offset-polygons:" simplify-polygon now-us; do rg -q -- "$f" crates/slicer-sdk/src/host.rs || rg -q -- "$f" crates/slicer-sdk/src/host_wit.rs || { echo "MISSING: $f"; exit 1; }; done; echo PASS'`
- **AC-3. Given** the rebuilt `sdk-host-bridge-guest` test component dispatched through the real WASM path against a host context whose mesh has a top plate at world z = 10.0 for object id `"cube"`, **when** the guest calls `slicer_sdk::host::raycast_z_down("cube", x, y, 50.0)` inside its stage impl and reports the result in its diagnostic message, **then** the integration test asserts the diagnostic carries the hit value 10.0 (tolerance 1e-4) — a result that is impossible before this packet, because the unbridged wrapper consults an uninstalled thread-local `MeshSource` and always returns `None` in a guest. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- host_bridge_roundtrip 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log && rg -q "host_bridge_roundtrip_tdd::" target/test-output.log`
- **AC-4. Given** the same `sdk-host-bridge-guest` dispatch, **when** the guest (a) offsets a 10 mm square by +1.0 mm via `slicer_sdk::host::offset_polygons` with join `Miter` and arc tolerance `0.0`, and (b) calls `slicer_sdk::host::now_us()` twice, **then** the test asserts the reported offset-result width is 12.0 mm (tolerance 0.05 mm) and the two `now_us` readings are monotonically non-decreasing without trapping — `now_us` traps before this packet because its native fallback needs `std::time::Instant`, which `wasm32-unknown-unknown` cannot provide. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- host_bridge_roundtrip 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log && rg -q "host_bridge_roundtrip_tdd::" target/test-output.log`
- **AC-5. Given** `modules/core-modules/classic-perimeters/src/lib.rs` after migration, **when** grepping for direct `slicer_core::polygon_ops` usage, **then** `offset` and `difference_ex` no longer appear in its `use slicer_core::polygon_ops` import list or as `polygon_ops::`-qualified calls (all 8 `offset` sites and all 4 `difference_ex` sites moved to `slicer_sdk::host::offset_polygons` / `slicer_sdk::host::clip_polygons` with `ClipOperation::Difference`), while `offset2_ex`, `opening_ex`, and `remove_small_and_small_holes` remain imported from `slicer_core::polygon_ops` by design. | `sh -c '! rg -U -q "use slicer_core::polygon_ops::[^;]*\b(difference_ex|offset)\b" modules/core-modules/classic-perimeters/src/lib.rs && ! rg -q "polygon_ops::(difference_ex|offset)\(" modules/core-modules/classic-perimeters/src/lib.rs && rg -q "offset2_ex" modules/core-modules/classic-perimeters/src/lib.rs && (rg -q "host::offset_polygons" modules/core-modules/classic-perimeters/src/lib.rs || rg -q "slicer_sdk::host::offset_polygons" modules/core-modules/classic-perimeters/src/lib.rs || rg -q "\boffset_polygons\(" modules/core-modules/classic-perimeters/src/lib.rs) && echo PASS'`
- **AC-6. Given** the migrated `classic-perimeters` (native arm delegates to the identical `slicer_core::polygon_ops` functions with the same `perimeter_arc_tolerance` argument carried through the new parameter), **when** the perimeter parity fixture suite runs, **then** it passes with zero fixture re-records (byte-identical native geometry is the falsifiable claim: a red fixture here means the migration changed an argument, not "expected drift"). | `mkdir -p target && cargo test -p slicer-runtime --test integration -- perimeter_parity gap_fill_emission 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log`
- **AC-7. Given** the post-migration tree, **when** a release profile run slices `resources/extruder_idler.obj` with the core-module tree, **then** the `profile_summary` extracted from the capture lists both `com.core.classic-perimeters` and `com.core.support-planner` as profiled modules. | `sh -c 'cargo run --bin pnp_cli --release -- slice --model resources/extruder_idler.obj --module-dir modules/core-modules --output target/p200.gcode --profile 2> target/p200-profile-after.jsonl && cargo run --bin pnp_cli --release -- profile --from target/p200-profile-after.jsonl --json > target/p200-summary.json && rg -q "com.core.classic-perimeters" target/p200-summary.json && rg -q "com.core.support-planner" target/p200-summary.json && echo PASS'`
- **AC-8. Given** the evidence obligations, **when** the packet closes, **then** (a) the DEV-094 row's Status cell in `docs/DEVIATION_LOG.md` begins with `Closed`, and (b) `docs/adr/0055-fuel-based-module-profiling.md` carries an `## Amendment` section that quotes before/after per-scope guest fuel for `com.core.classic-perimeters` and a profiling-off wall-clock comparison with an explicit run-to-run spread statement, answering the ADR's open in-guest-vs-host-native question in either direction. | `sh -c 'rg -q "^\| DEV-094 \|.*\| Closed" docs/DEVIATION_LOG.md && rg -q "^## Amendment" docs/adr/0055-fuel-based-module-profiling.md && rg -q "in-guest" docs/adr/0055-fuel-based-module-profiling.md && echo PASS'`
- **AC-9. Given** `docs/05_module_sdk.md` §Host Service Wrappers, **when** the bridge lands, **then** the stale caveat "the host bridge is not wired" is gone and the section states that the geometry wrappers execute host-side on wasm32 (anchor phrase `bridged to the host` present). | `sh -c '! rg -q "the host bridge is not wired" docs/05_module_sdk.md && rg -q "bridged to the host" docs/05_module_sdk.md && echo PASS'`

## Negative Test Cases

- **AC-N1. Given** the bridged mesh-query path, **when** the `sdk-host-bridge-guest` is dispatched with a config key (`bridge_probe_object`, snake_case) naming an object id absent from the host's MeshIR and the guest calls `slicer_sdk::host::raycast_z_down` on it, **then** the module invocation fails with a dispatch error naming the unknown object (the host's singular mesh-query impls raise on unknown ids — see `lookup_object_mesh` error propagation in `crates/slicer-wasm-host/src/host.rs`) instead of the pre-fix behavior of silently returning `None`. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- host_bridge_unknown_object_errs 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log && rg -q "host_bridge_unknown_object_errs" target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (must report clean; a `STALE:` report after the WIT/SDK/module edits means the rebuild step was skipped)
- `mkdir -p target && cargo test -p slicer-runtime --test integration -- host_bridge 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log && rg -q "host_bridge_roundtrip_tdd::" target/test-output.log`

## Authoritative Docs

- `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — 42 lines; direct read. The four-layer bridge shape every new arm must follow.
- `docs/adr/0049-batched-host-services-over-threaded-guests.md` — 186 lines; direct read of §Decision, §Consequences, and §Amendment 2026-08-05. Batch semantics, marshalling-ledger caveat, adopt-one-module-at-a-time rule.
- `docs/adr/0055-fuel-based-module-profiling.md` — 127 lines; direct read. Evidence methodology (fuel primary, wall-clock secondary, profiling-off absolute timings) and the open question this packet answers.
- `docs/DEVIATION_LOG.md` — large; delegate. Only the DEV-094 row (single line) is needed; grep for `^\| DEV-094`.
- `docs/05_module_sdk.md` — large; ranged read of §Host Service Wrappers only (the code-block region containing "the host bridge is not wired").

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/05_module_sdk.md` section "Host Service Wrappers" — replace the "the host bridge is not wired" caveat with the bridged-on-wasm32 statement. Verify: `rg -q 'bridged to the host' docs/05_module_sdk.md`
- `docs/DEVIATION_LOG.md` DEV-094 row — Status cell updated to begin with `Closed`, quoting the seven bridged wrappers and the evidence summary. Verify: `rg -q '^\| DEV-094 \|.*\| Closed' docs/DEVIATION_LOG.md`
- `docs/adr/0055-fuel-based-module-profiling.md` — append an `## Amendment` section recording the measured in-guest-vs-host-native answer (this conforms to the ADR: it declares the question open and names the profiler as the instrument that decides it; no normative clause is contradicted). Verify: `rg -q '^## Amendment' docs/adr/0055-fuel-based-module-profiling.md`

These greps are duplicated inside AC-8/AC-9 so the acceptance ceremony re-checks them.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
