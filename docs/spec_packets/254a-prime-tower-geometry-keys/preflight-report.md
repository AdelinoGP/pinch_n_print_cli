# Preflight Gate: 254a-prime-tower-geometry-keys

Reviewed: 2026-09-02 · Mode: `--preflight` · Symbol-inventory dispatched: 1 packet · Re-authored under map Authoring rules 1–6 · Split half A of former packet 254

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files)     | PASS | all five present and non-empty |
| S1 Prerequisite-status truth      | PASS | `254b` and `255` are cited as **ordering, not gating**; no packet is called implemented |
| S2 Deviation-ID conformance       | PASS | live log format is `DEV-###` (rows `DEV-157`, `DEV-158`); no `D-254a*` token exists in the log; the packet declares `D-254a-*` as packet-local labels and Step 8 re-derives real `DEV-###` IDs at write time |
| S3 Schema-version computed        | PASS | no `*_SCHEMA_VERSION` pinned; the packet states no IR schema bump is required |
| S4 ADR slot allocation            | PASS | no new ADR authored; `docs/adr/` runs 0001–0063, untouched |
| S5 Shipped-symbol existence/shape | PASS | verified in tree: `slicer_ir::ToolChange { after_entity_index: u32, from_tool: u32, to_tool: u32 }`, `LayerCollectionView::{tool_changes, layer_index, z, ordered_entities}`, `FinalizationOutputBuilder::insert_entity_at`, `WipeTower::{from_config, process, run_finalization, generate_purge_paths}`, `ModuleError::fatal`, `ExtrusionRole::{WipeTower, PrimeTower}`, `ConfigValue::Percent`, `ConfigBoundsIndex::{check, schema_defaults}`, `guest_input_paths` (covers the parent module's `src/` and depth-1 `*.toml`), `gen-config-docs --check`, `check-deviations` |
| S6 WIT/IR identifier drift        | PASS | no WIT change claimed; every IR identifier the packet names as pre-existing resolves |
| S7 Test-target wiring             | PASS | `wipe_tower_config_schema_tdd.rs` is **new** and standalone (`wipe-tower/tests/` has no aggregator `main.rs`) and the packet adds the required `toml = "0.8"` dev-dependency; `config_bounds_enforcement_tdd` is registered in `crates/slicer-scheduler/tests/integration/main.rs`; `config_view_binding_tdd` is registered in `crates/slicer-runtime/tests/contract/main.rs` |
| S8 ADR conformance                | PASS | no ADR normatively governs prime-tower geometry. ADR-0015 (ConfigView as the normalized prepass export) is the nearest; the packet conforms — it declares keys through the existing manifest path and adds no parallel export |
| (existing) AC runnable command    | PASS | all 9 ACs and all 3 negative cases end in a single runnable pipe-suffixed command; no `cargo test --workspace` as an AC command |
| (existing) Doc Impact Statement   | PASS | `docs/15_config_keys_reference.md` named, generated-only, verified by AC-9 |

### Blockers (S4/S5/S6)

None.

### High (S1/S2/S3/S7/S8)

None outstanding. Three **fictional-symbol defects inherited from the former packet 254** were found and corrected during this re-authoring; they are recorded here because they are exactly the S5/S7 class this gate exists to catch:

1. **`Print::plan_tower_new` does not exist** in the canonical checkout. Former 254 cited it as the resolver of `prime_tower_brim_width`'s `-1` Auto sentinel. The real symbols are `WipeTower::plan_tower_new` (WipeTower-side) and `Print::wipe_tower_data` (Print-side), both calling `WipeTower::get_auto_brim_by_height`. Corrected throughout.
2. **`crates/slicer-scheduler/tests/wipe_tower_config_bounds_tdd.rs` does not exist** anywhere in the tree. Former 254 used it as an AC command target. AC-8 is re-homed on the real, already-registered `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, and the packet now states explicitly that it **authors** the wipe-tower case there (the file has none today — its manifest-driven cases use the tree-support and traditional-support planner manifests).
3. **`undeclared_prime_tower_keys_stay_hidden_from_other_modules` does not exist** anywhere in the tree — no test fn, no file, no registration. Former 254 used it as AC-N2's command. AC-N1 is re-homed on the real, already-registered `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`, which already exercises `bind_module_config_view` hiding undeclared keys, and the packet states it authors the arm.

### Accepted FORWARD-DEPs

None. All three `[FWD]` items in `design.md` (travel-avoid facility, canonical's tower planner, the rule-4 wall-type candidate) are forwarded *out* of this packet and gate no AC here.

### Map gates (wayfinder Authoring rule 6)

- **(a) zero declaration-only keys** — **PASS.** All three kept keys drive a behaviour-changing decision point this packet builds: `prime_tower_infill_gap` (scan-line pitch), `prime_tower_enable_framework` (uniform layer depth, on top of the per-layer depth model built in the same packet), `prime_tower_brim_width` (first-layer brim rings + Auto). Zero declared-with-gap. One key (`prime_tower_skip_points`) is **returned to the queue**, not declared. Nine keys moved to `254b`. Zero dead-in-canonical.
- **(b) non-default AC per key** — **PASS.** `prime_tower_infill_gap` = `"200%"` (AC-2), `prime_tower_enable_framework` = `true` (AC-4), `prime_tower_brim_width` = `6.0` (AC-5) and `-1.0` (AC-6). AC-N2's enable-gate identity is an *additional* criterion, never the sole evidence for any key.

**Verdict:** PREFLIGHT PASS (0 blockers, 0 high)
