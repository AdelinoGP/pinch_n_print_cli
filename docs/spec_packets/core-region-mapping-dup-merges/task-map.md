# Task Map: core-region-mapping-dup-merges

The user-approved program IDs are used directly; no `TASK-###` mapping is authorized or required.

| Approved program task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/DUP-CORE (region mapping)` | Steps 1 and 2 | `docs/specs/test-quality-remediation-plan.md` §5.1 and §7; `docs/22_test_quality.md`; ADR-0064; `docs/spec_packets/_OLD/93_region-mapping-cross-product.md` (alias provenance) | Step 1: `crates/slicer-core/tests/algo_region_mapping_tdd.rs` duplicate deletions plus survivor doc-comment extension; Step 2: `docs/specs/test-quality-remediation-plan.md` §7 `core` row | None; pre-existing API behavior | `S` | Merge the two strict-subset cross-product duplicates (`region_mapping_enumerate_chains`, `region_mapping_cross_product_entry_count`) into the plan-named survivor `region_mapping_two_semantics_produces_cross_product_cardinality` while retaining the full chain-set, cardinality, and ordering coverage and the absorbed AC-4 formula prose alongside the survivor's pre-existing AC-3 wording, then record partial survivor/validation evidence without closing other core work. |

This exact crosswalk owns the region-mapping-only sub-item and its mandatory §7 evidence bookkeeping. It does not imply closure of the remaining `core/DUP-CORE` items (bridge, support, wall sequence, geometry, beading), `core/RETIRE`, `core/DUP-CORE-strengthen`, `core/PAINT`, `core/BRITTLE`, `core/CROSS`, or `core/PARITY`, or the whole core wave, and it exports no symbols to later packets.
