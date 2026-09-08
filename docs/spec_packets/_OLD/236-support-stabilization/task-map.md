# Task Map: 236-support-stabilization

Nine task IDs span nine steps; the crosswalk exists because `docs/07_implementation_status.md` rows are registered per-packet (packet-232 Step 7 precedent) and because the queue plan (`docs/specs/support-families-anchored-entities-plan.md` §Packet Queue) records these IDs as the row-#1 allocation.

Registration contract: rows are NOT hand-added at authoring time. Implementation Step 9 registers them via a worker dispatch, in `docs/07`'s local backlog format:

```
- [ ] TASK-344 — support-stabilization — AC-8 per-region family assignment minting in commit_support_analysis_builtin; planner_emits_one_entry_per_region_in_region_map green with assertions unchanged. Spec: docs/spec_packets/236-support-stabilization/.
- [ ] TASK-345 — support-stabilization — G-21 validator red fixtures: family_scoped_support_claims_do_not_conflict_globally + genuine_claim_conflict_still_rejected_after_family_exemption. Spec: docs/spec_packets/236-support-stabilization/.
- [ ] TASK-346 — support-stabilization — G-21 family-scoped exemptions in validate_startup_dag global passes; docs/04 Validation Passes note. Spec: docs/spec_packets/236-support-stabilization/.
- [ ] TASK-347 — support-stabilization — G-23 tripwire real collision/avoidance inputs + classified rebless via SUPPORT_PLANNER_REGEN_GOLDEN (workspace-root resources/golden). Spec: docs/spec_packets/236-support-stabilization/.
- [ ] TASK-348 — support-stabilization — G-22 declare support_threshold_angle + legacy alias support_overhang_angle [0,90] in traditional-support-planner manifest; gen-config-docs same-commit; OutOfRange negative test. Spec: docs/spec_packets/236-support-stabilization/.
- [ ] TASK-349 — support-stabilization — G-24 assert_guest_freshness in integrated_parity_harness run_integrated_parity before native/wasm comparison. Spec: docs/spec_packets/236-support-stabilization/.
- [ ] TASK-350 — support-stabilization — native/wasm layer-view field-identity seam test + paint BASE fallback own-object-contours fix. Spec: docs/spec_packets/236-support-stabilization/.
- [ ] TASK-351 — support-stabilization — delete drafts 215–218, remediation-plan rows 3–6 absorption updates, ADR-0059 accepted with Ruling-1 amendment, packet-236 re-measurement note. Spec: docs/spec_packets/236-support-stabilization/.
- [ ] TASK-352 — support-stabilization — green gate (clippy, xtask test --summary --workspace --no-fail-fast, check-literals count unchanged), human-gate artifacts + evidence file, docs/07 registration dispatch. Spec: docs/spec_packets/236-support-stabilization/.
```

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-344` | `Step 1` | plan §3 Ruling 1 / §12 item 1 | `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`; `tests/executor/prepass_support_geometry_layer_plan_tdd.rs` | none | `M` | Proves AC-1. Count assertion unmodified; mesh-path-gate hypothesis forbidden (plan T11) |
| `TASK-345` | `Step 2` | gap register G-21 | scheduler validation test file (three new tests) | none | `S` | Proves AC-N1 placement; positive tests authored red-first/guard |
| `TASK-346` | `Step 3` | gap register G-21; `docs/04_host_scheduler.md` "Validation Passes" | `crates/slicer-scheduler/src/validation.rs` | none | `M` | Proves AC-2 + AC-N1 (both halves). Claim exemption mirrors fill-role block (~line 553), global pass only; IR-write advisories resolved by orderability recognition per ADR-0059's own write-topology clause — no ADR amendment |
| `TASK-350` | `Step 7` | plan §12 view-seam/paint-fallback items; AGENTS.md coordinate hazard | new `crates/slicer-wasm-host/tests/contract/view_seam_identity_tdd.rs` (+ main.rs mod line); `crates/slicer-core/src/algos/paint_segmentation/mod.rs`; new standalone `crates/slicer-core/tests/paint_segmentation_base_fallback_tdd.rs` | none | `M` | Proves AC-8 + AC-9. E6 flag on slicer-core run; slicer-core has no test aggregator (auto-discovered binaries) |
| `TASK-347` | `Step 4` | plan §7 E3; gap register G-23; `docs/DEVIATION_LOG.md` new row | `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`; workspace-root `resources/golden/benchy_tree_support_regression_*` | none | `M` | Proves AC-3 + AC-4. Plan-correction applies: goldens at repo root, gate is SUPPORT_PLANNER_REGEN_GOLDEN (not the dead wedge gate) |
| `TASK-348` | `Step 5` | gap register G-22; `docs/config/host-keys.toml` line 57 | `traditional-support-planner.toml` `[config.schema]`; scheduler bounds test; `docs/15_config_keys_reference.md` regen | `PrintConfig.cpp` `support_threshold_angle` (delegate, only if disputed) | `S` | Proves AC-5 + AC-N2. Manifest edit + doc regen same commit (T8) |
| `TASK-349` | `Step 6` | plan §7 E4; AGENTS.md guest staleness | `crates/slicer-runtime/tests/common/integrated_parity_harness.rs` | none | `S` | Proves AC-6 + AC-7. Four support parity suites covered transitively |
| `TASK-350` | `Step 7` | plan §12 view-seam/paint-fallback items; AGENTS.md coordinate hazard | new wasm-host seam test; `crates/slicer-core/src/algos/paint_segmentation/mod.rs` | none | `M` | Proves AC-8 + AC-9. E6 flag on slicer-core run |
| `TASK-351` | `Step 8` | plan §10 mapping; ADR-0059; gap register measurement note | deletions of four draft dirs; `docs/specs/support-generation-remediation-plan.md` rows 3–6; `docs/adr/0059-*.md` | reference gcode artifacts only | `M` | Proves AC-10..AC-12. Deletions+rows one commit; AC-12 may park to gate time |
| `TASK-352` | `Step 9` | plan §8 human validation gate; AGENTS.md test discipline | `evidence/human-gate.md`; `docs/07_implementation_status.md` (dispatch) | none | `M` | Proves gates + Human Gate readiness. Ledger facts re-derived at write time |

Copy costs from `implementation-plan.md`. Aggregate M; no L rows.
