# Preflight Report: 258-skirt-type-and-draft-shield-keys

Reviewed: 2026-08-30 · Mode: --preflight · Symbol-inventory dispatched: 1 (in-tree key survey ×1, canonical read ×2, precedent survey ×1, start-point reachability ×1, canonical ordering ×1, S5/S7 sweep ×1)

## Preflight Gate

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files) | PASS | — |
| S1 Prerequisite-status truth | PASS | deps are resolved wayfinder tickets (06/05/04) + packet 257 described as queue *ordering*, never claimed implemented |
| S2 Deviation-ID conformance | PASS | no deviation IDs referenced or created; none of the 5 keys appears in `docs/DEVIATION_LOG.md` (dispatch-verified) |
| S3 Schema-version computed | PASS | no schema/version constants touched |
| S4 ADR slot allocation | PASS | no ADRs authored or referenced |
| S5 Shipped-symbol existence/shape | PASS | `SkirtBrim::from_config` / `run_finalization` / `generate_skirt_entities` / `make_rect_loop` (modules/core-modules/skirt-brim/src/lib.rs); `FinalizationOutputBuilder::push_entity_to_layer` (crates/slicer-sdk traits.rs, arity 4); `ConfigBoundsIndex::from_modules`/`from_declarations`, `BoundsDeclaration{key,min,max,module_id}`, `ConfigResolutionError::{OutOfRange,TypeMismatch}` (crates/slicer-scheduler/src/config_resolution.rs) — all verified by dispatch |
| S6 WIT/IR identifier drift | PASS | `ExtrusionRole::{Skirt,Brim}`, `RegionKey{global_layer_index,object_id,region_id,variant_chain}`, `LayerCollectionView` — verified in the module source |
| S7 Test-target wiring | PASS | skirt-brim tests are file-per-binary (no aggregator); new files land as their own `--test` binaries; scheduler/runtime integration additions go into files already registered in `tests/integration/main.rs` (verified: `mod config_bounds_enforcement_tdd;`, runtime aggregator greps 1 hit) |
| S8 ADR conformance | PASS | no ADR-governed surface (no IR/WIT/claim changes) |
| (existing) AC runnable command | PASS | 7 ACs + 2 negatives, all pipe-suffixed; every `--test` binary verified to exist and drive the asserted behavior (`config_bounds_enforcement_tdd.rs` proves the real-manifest bounds pattern incl. enum `TypeMismatch`; `gcode_header_thumbnail_config_blocks_tdd.rs` drives `run_pipeline_with_raw_config` and asserts `; key = value` block lines) |
| (existing) Doc Impact Statement | PASS | grep corrected during preflight — the generated doc has no per-module subheadings (verified against disk), so verification is key-presence `rg -q 'single_loop_draft_shield'` + `rg -q 'skirt_start_angle'`, matching packet 257's corrected form |

## Corrections made during preflight

1. **Doc-Impact grep** originally referenced a nonexistent `^## skirt-brim` heading in the generated doc; re-probed `docs/15_config_keys_reference.md` on disk (modules are table rows with an owner column, no per-module headings), corrected the grep to key-presence in the Doc Impact Statement, AC-7, and implementation-plan Step 5.
2. Transcription typos found and fixed by a whole-packet garbled-token scan (`OrcaSlider`, a path typo, one duplicated dispatch heading, two garbled placeholder bullets in implementation-plan Steps 3–4).

## Accepted FORWARD-DEPs

- None — packet depends only on resolved wayfinder tickets and queue ordering.

## Verdict

**PREFLIGHT PASS** (0 blockers, 0 high)