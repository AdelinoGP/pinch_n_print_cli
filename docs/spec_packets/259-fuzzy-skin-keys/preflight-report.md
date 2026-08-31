# Preflight Report: 259-fuzzy-skin-keys

Reviewed: 2026-08-31 · Mode: --preflight · Symbol-inventory dispatched: 1 (canonical read ×1, in-tree survey ×1, S5/S7 sweep ×1)

## Preflight Gate

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files) | PASS | — |
| S1 Prerequisite-status truth | PASS | deps are resolved wayfinder tickets (06/05/04/103) + packet 258 described as queue *ordering*, never claimed implemented |
| S2 Deviation-ID conformance | PASS | `DEV-126` referenced as context only (exists in `docs/DEVIATION_LOG.md`, dispatch-verified); no deviation IDs created, superseded, or closed |
| S3 Schema-version computed | PASS | no schema/version constants touched |
| S4 ADR slot allocation | PASS | no ADRs authored or referenced |
| S5 Shipped-symbol existence/shape | PASS | `FuzzySkinModule::from_config` / `run_wall_postprocess` / `apply_fuzzy_skin` (modules/core-modules/fuzzy-skin/src/lib.rs); manifest keys `fuzzy_skin_thickness`/`fuzzy_skin_point_distance`/`apply_to_all`; `ConfigBoundsIndex::from_modules`, `resolve_global_config`, `ConfigResolutionError::{OutOfRange,TypeMismatch}` (crates/slicer-scheduler/src/config_resolution.rs, re-exported from slicer-ir); `ORCA_CONFIG_PADDING` + `emit_config_kv` (crates/slicer-gcode/src/serialize.rs); `guest_input_paths` (xtask/src/build_guests.rs, covers `modules/core-modules/*/src` + module TOMLs); `cooling_config_schema_tdd.rs` guard pattern; `seam-planner-default.toml` enum-table form; `tree-support-planner` enum-read fallback — all verified by dispatch |
| S6 WIT/IR identifier drift | PASS | `LoopType::{Outer,Inner,ThinWall,NonPlanarShell,GapFill}` (no `Hole` — the structural basis for the `hole`/`all` gap) and `WallLoop{perimeter_index,loop_type,path,width_profile,feature_flags}` verified in `crates/slicer-ir/src/slice_ir.rs`; no WIT identifiers named |
| S7 Test-target wiring | PASS | fuzzy-skin tests are file-per-binary (no aggregator — verified: no `[[test]]` sections, no `tests/main.rs`); net-new `fuzzy_config_schema_tdd.rs` lands as its own `--test` binary; scheduler/runtime integration additions go into existing binaries already registered in `tests/integration/main.rs` |
| S8 ADR conformance | PASS | ADRs 0011/0018/0022 mention fuzzy skin / `run_wall_postprocess` but govern no config-key, loop-selection, or padding surface; packet conforms (no IR/WIT/claim changes) |
| (existing) AC runnable command | PASS | 6 ACs + 2 negatives, all pipe-suffixed; every `--test` binary verified to exist and drive the asserted behavior (`config_bounds_enforcement_tdd.rs` proves the real-manifest bounds pattern incl. enum `TypeMismatch`; `gcode_header_thumbnail_config_blocks_tdd.rs` drives CONFIG_BLOCK emission and asserts `; key = value` block lines) |
| (existing) Doc Impact Statement | PASS | key-presence grep `rg -q 'fuzzy_skin_scale'` + `rg -q 'fuzzy_skin_first_layer'` against the real generated doc (modules are table rows with an owner column, no per-module headings — packet 257/258's corrected form) |

## Corrections made during preflight

1. **`ORCA_CONFIG_PADDING` discovery (S5 sweep).** The sweep found `fuzzy_skin` and
   `fuzzy_skin_mode` **already present** in the padding table
   (`crates/slicer-gcode/src/serialize.rs`: `("fuzzy_skin", "none")`,
   `("fuzzy_skin_mode", "displacement")`), so the packet's original AC-5 claim
   ("none of the seven appears at defaults") was wrong for those two. Corrected:
   AC-5 now pins the two pre-existing padding lines at defaults, and the packet
   gains a one-line value correction — `("fuzzy_skin", "none")` →
   `("fuzzy_skin", "disabled_fuzzy")` — because the pre-existing value
   contradicts the canonical default this packet declares (no entries gained or
   lost; the 254/255/257/258 read-only rule holds). Rippled through the scope
   boundaries, requirements (problem statement, in-scope, out-of-scope, wiring
   notes), design (change surface, files in scope, read-only/out-of-bounds,
   locked assumptions, risks), and implementation-plan Step 3.
2. **Step 2 test-fallout refinement.** Per-vertex-flag tests map to
   `fuzzy_skin = "none"` (painted-only — the flag path's faithful enum value),
   apply-to-all tests to `fuzzy_skin = "all"` (preserving their intent), rather
   than a blanket `"all"`/`"external"` assignment.

## Accepted FORWARD-DEPs

- None — packet depends only on resolved wayfinder tickets and queue ordering.

## Verdict

**PREFLIGHT PASS** (0 blockers, 0 high)
