## Preflight Gate: 264-top-bottom-surface-keys

Reviewed: 2026-09-01 · Mode: `--preflight` · Symbol-inventory dispatched: 1 packet
Re-authored under the wayfinder map's Authoring rules 1–6; this report supersedes the prior Tier A draft's report.

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files)     | PASS | all five present and non-empty |
| S1 Prerequisite-status truth      | PASS | 262b is `status: draft`; the packet now labels it **FORWARD-DEP on draft packet 262b** in `packet.spec.md`, `design.md` invariant 0, and the plan's Execution Rules, with a name/shape reconciliation and a "do not start Step 1 until 262b is `implemented`" gate |
| S2 Deviation-ID conformance       | PASS | only referenced ID is `D-209-ADJUST-SOLID-SPACING-DIVERGENCE` (verb: reference) — present and Open in `docs/DEVIATION_LOG.md`. No IDs created, closed, or superseded |
| S3 Schema-version computed        | PASS (N/A) | packet pins no `*_SCHEMA_VERSION`; explicitly asserts no IR schema bump |
| S4 ADR slot allocation            | PASS | authors no new ADR; the two cited (`0027-gyroid-multi-role-fill-holder.md`, `0056-integrated-modules-native-dispatch.md`) both exist |
| S5 Shipped-symbol existence/shape | PASS (1 shape correction applied) | `from_config` was cited as an inherent fn; it is a `LayerModule` trait method — corrected to `LayerModule::from_config` across all four files. All other symbols verified: holder fields, `resolve_global_config` / `apply_overlay` / `resolve_per_object_configs`, `FILL_CLAIM_IDS` / `FillHolders::holder_for` / `module_id_matches_holder` / `resolve_held_claims`, `SOLID_DENSITY` (=1.0, used exactly twice in `run_infill`), `solid_fill_role`, `adjust_solid_spacing`, `load_modules_from_roots`, `ORCA_CONFIG_PADDING`, `overlay_resolved` |
| S6 WIT/IR identifier drift        | PASS | no WIT type named. IR variants `TopSolidInfill` / `BottomSolidInfill` / `InternalSolidInfill` / `SparseInfill` / `BridgeInfill` all exist in `crates/slicer-ir/src/slice_ir.rs`; `SliceRegionView::should_emit` confirmed to map `TopSolidInfill → claim:top-fill` and `BottomSolidInfill → claim:bottom-fill` |
| S7 Test-target wiring             | PASS | `scheduler_integration` is a real declared `[[test]]`; the net-new `top_bottom_pattern_holder_tdd.rs` has its `mod` registration in the Step 8 edit list. `contract` / `e2e` are real auto-discovered `slicer-runtime` targets (`tests/contract/main.rs`, `tests/e2e/main.rs`). **Corrected from the prior draft**, which used `--test integration` for `slicer-scheduler` — a target that does not exist |
| S8 ADR conformance                | PASS | ADR-0027 is recorded and honoured (gyroid solid path untouched; AC-N5 pins the omission), ADR-0056's registration contract is followed in Step 1. No ADR's normative content is contradicted |
| (existing) AC runnable command    | PASS | 20 of 20 ACs carry a pipe-suffixed runnable command; none uses `cargo test --workspace` |
| (existing) Doc Impact Statement   | PASS | present, four entries, each with a verification command |

### Blockers (S4/S5/S6) — fix before any commit

None.

### High (S1/S2/S3/S7/S8) — fix or convert to justified FORWARD-DEP

None outstanding. The S1 item was converted to an explicit, reconciled FORWARD-DEP during this gate.

### Accepted FORWARD-DEPs (consumer name/shape matches the producer packet's plan)

- `modules/core-modules/monotonic-infill/**` ← produced by draft packet 262b, which plans it holding `claim:top-fill`; this packet appends `claim:bottom-fill`. Names reconciled ✓
- 262b's pattern→holder derivation helper in `crates/slicer-scheduler/src/config_resolution.rs` ← produced by draft packet 262b, writing `sparse_fill_holder` / `top_fill_holder`; this packet extends it with two more key→field pairs. Shape reconciled ✓; the helper's final *name* is resolved by a FACT dispatch at Step 1 rather than frozen here.

### Map-specific gates (wayfinder Authoring rule 6)

| Gate | Result | Evidence |
|------|--------|----------|
| (a) zero declaration-only keys in the disposition table | **PASS** | all four keys are class **(b)**; counts (a) 0 · (b) 4 · (c) 0 · (d) 0. No "declared-with-gap", no "decision-point gap recorded" |
| (b) every key has ≥1 AC asserting a behaviour change at a non-default value | **PASS** | `top_surface_density` → AC-4 (=50), AC-5 (=0); `bottom_surface_density` → AC-4 (=50); `top_surface_pattern` → AC-1 + AC-6/7/8/9; `bottom_surface_pattern` → AC-2. AC-N1 (default identity) is explicitly labelled additional and is the sole evidence for nothing |

**Verdict:** PREFLIGHT PASS (0 blockers, 0 high)
