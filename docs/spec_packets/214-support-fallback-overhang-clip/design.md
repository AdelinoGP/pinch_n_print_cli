# Design: support-fallback-overhang-clip

## Controlling Code Paths

- Primary code path: `TraditionalSupport::run_support` at `modules/core-modules/traditional-support/src/lib.rs:107-174`; `TreeSupport::run_support` at `modules/core-modules/tree-support/src/lib.rs:130-194`; `sliced_region_to_data` at `crates/slicer-wasm-host/src/marshal/in_.rs:342-424`.
- Neighboring tests/fixtures: `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs`, `crates/slicer-wasm-host/tests/contract/slice_region_view_contract_tdd.rs`, and each support module's tests.
- OrcaSlicer comparison: no new parity claim; use the existing overhang contract as the source of truth.

## Architecture Constraints

- `overhang_areas()` is already region-clipped and flattened from `overhang_quartile_polygons`; do not recompute or broaden it.
- Paint precedence remains Blocked, Enforced, then DefaultEligible. Only the DefaultEligible fill input changes.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: split each fallback's iteration into policy-selected source polygons: `polygons` for Enforced and `overhang_areas()` for DefaultEligible; replace the host literal with the existing overhang vector's emptiness.
- Exact functions, traits, manifests, tests, and fixtures: the two `run_support` functions, `sliced_region_to_data`, focused module tests, and host contract tests; no IR/WIT or manifest edits.
- Rejected alternatives and reasons: adding a new IR field duplicates an existing field; recomputing overhangs inside modules would violate the host-prepared region contract.

## Files in Scope (read + edit)

- `modules/core-modules/traditional-support/src/lib.rs` - role: fallback filler; expected change: DefaultEligible uses `overhang_areas()`.
- `modules/core-modules/tree-support/src/lib.rs` - role: fallback filler; expected change: DefaultEligible uses `overhang_areas()`.
- `crates/slicer-wasm-host/src/marshal/in_.rs` - role: host boundary; expected change: derive `needs_support` from `overhang_areas`.
- `modules/core-modules/traditional-support/tests/**` - role: fallback regression coverage; expected change: tests only if existing harness supports it.
- `modules/core-modules/tree-support/tests/**` - role: fallback regression coverage; expected change: tests only if existing harness supports it.
- `crates/slicer-wasm-host/tests/contract/**` - role: marshalling regression coverage; expected change: exact needs-support assertion.

## Read-Only Context

- `crates/slicer-sdk/src/views.rs` - lines `34-38`, `440-466` only - accessor and semantics.
- `crates/slicer-wasm-host/src/host.rs` - lines `176-204`, `3200-3220` only - host field and accessor shape.
- `crates/slicer-ir/src/slice_ir.rs` - lines `623-648` only - existing overhang domain shape.
- `docs/specs/support-generation-defect-verified-findings.md` - lines `88-127` only - authoritative root-cause evidence.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**`, `target/**`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-ir/src/slice_ir.rs` and WIT schemas - read-only contract context; no edits.
- Support planner, scheduler, raft/interface modules, and G-code emitters - no edits.

## Expected Sub-Agent Dispatches

- Question: identify existing fallback test fixtures that can assert source polygon selection; scope: `modules/core-modules/{traditional-support,tree-support}/tests/**`; return: `LOCATIONS`; purpose: test placement.
- Question: identify host contract fixtures that construct `SliceRegionData` and assert `needs_support`; scope: `crates/slicer-wasm-host/tests/contract/**`; return: `LOCATIONS`; purpose: blast-radius-free test update.
- Question: run guest freshness, targeted tests, and visual-debug; scope: repository commands only; return: `FACT`; purpose: validation.

## Data and Contract Notes

- IR/manifest contracts: no changes; `SliceRegionData.needs_support` and `overhang_areas` retain their existing shapes.
- WIT boundary: no identifier or schema change; only the host-populated boolean changes.
- Determinism/scheduler constraints: preserve module selection and paint policy ordering; polygon iteration order comes from existing vectors.

## Locked Assumptions and Invariants

- Enforced fills full `region.polygons()` even when `overhang_areas()` is empty.
- DefaultEligible with empty `overhang_areas()` emits nothing because host `needs_support` is false and module selection also uses the empty clip.
- Blocked emits nothing regardless of either polygon set.

## Risks and Tradeoffs

- Existing fixtures intentionally using default `needs_support: true` may need explicit overhang areas or an enforced policy; do not weaken the production contract to preserve stale fixtures.
- This fix clips fallback geometry but does not create downward propagation; planner packet TASK-322 owns plate-reaching tree geometry.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: host contract fixture inventory; `LOCATIONS` limited to 20 entries.

## Open Questions

None.
