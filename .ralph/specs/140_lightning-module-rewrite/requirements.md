# Requirements: 140_lightning-module-rewrite

## Packet Metadata

- Grouped task IDs: `TASK-265`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `L` (justified unsplittable — generation + sampling +
  WIT closure are tightly coupled at the per-layer seam; swarm runs in extended
  band)

## Problem Statement

Everything behind the seam is real (137–139: stage, IR, primitives, generator), yet
lightning prints still come from the 512-LOC single-layer stub — grid samples joined to
the nearest boundary, self-linked in violation of ADR-0025 (DEV-081), with none of the
canonical cross-layer tree behavior. The stub is also the roadmap's last self-linking
module: until it emits raw, the linker's "one place linking happens" invariant has a
standing exception that paths cannot even be detected (no module identity on paths).
This packet deletes the stub, samples the committed trees, and closes DEV-081 —
completing both the lightning-parity sub-roadmap and Architecture A's uniformity.

## In Scope

- Rewrite `modules/core-modules/lightning-infill/src/lib.rs`: per region (sparse role,
  `should_emit` gating unchanged), read the layer's tree segments from the 137 view
  (the `PaintRegionLayerView::lightning_tree_segments_for(object_id, region_id)`
  accessor in `crates/slicer-sdk/src/traits.rs` — 139's per-region keying), emit raw
  `SparseInfill` polylines (`speed_factor` from config; `begin_region` origin
  discipline at `lib.rs:117`); delete `build_branches` (at `lib.rs:234`), the
  inline grid-sampling machinery in `run_infill`/`fill_expolygon` (lines 97-232),
  and the supporting helper functions `nearest_boundary_point`, `polygon_bbox_mm`,
  `point_in_expolygon`, `point_in_polygon`, and any clipping/chaining call. The
  `should_emit(SparseInfill)` gate, the `on_print_start` config reads, and the
  `begin_region` origin discipline are kept.
- Mirror only Orca's sampling-side per-layer transformation (delegated
  `Filler::_fill_surface_single` check) — generation is host-side (139), linking is
  the linker's (133).
- Rewrite the module test suite (`tests/lightning_infill_tdd.rs` — 323 lines, currently
  pins stub behavior): AC-1 sampling equality, AC-N2 empty-trees totality; keep the
  module-binding test (`tests/slicer_module_binding_tdd.rs`).
- Pipeline test: `lightning_pipeline_linked` (AC-3) in the runtime executor bucket
  (`crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs`, new).
- **DEVIATION CLOSURE (D-137-WIT-RUN-INFILL-NO-PAINT-VIEW):**
  - WIT edit at `crates/slicer-schema/wit/deps/world-layer/world-layer.wit:25`:
    extend the `run-infill` export signature with
    `paint: paint-region-layer-view` (mirrors the `run-perimeters` and
    `run-support` signatures at `:23` and `:27`).
  - WIT package bump: `package slicer:world-layer@2.2.0;` →
    `package slicer:world-layer@2.3.0;` at
    `crates/slicer-schema/wit/deps/world-layer/world-layer.wit:1` (consistent
    with the 2.0.0→2.1.0→2.2.0 chain; the additive export-arg change is a
    minor→major correction, packet 130's DEV-084 precedent).
  - SDK trait extension at `crates/slicer-sdk/src/traits.rs:369-377`: add
    `_paint: &PaintRegionLayerView` to `LayerModule::run_infill` (and mirror
    the perimeters/postprocess convention for the trait).
  - slicer-macros `infill_arm` at
    `crates/slicer-macros/src/lib.rs:1779-1794` and the macro-emitted
    `fn run_infill` glue at `:2804-2809` must pass the new paint-view arg
    through to the module's impl.
  - Host dispatch `Layer::Infill` arm at
    `crates/slicer-wasm-host/src/dispatch.rs:442-465` must build a
    `PaintRegionLayerData` via the existing
    `build_paint_layer_data_with_plan(...)` (the same call already used by
    the `Layer::Support` arm at `:584-619` and the perimeters arm) and pass
    `own(paint)` to `call_run_infill`. The `lightning_tree_ir` field is
    already plumbed in `LayerStageInput` (137).
  - Four `run_infill`-implementing core modules update their `fn run_infill`
    signatures to take the new paint-view arg:
    `modules/core-modules/rectilinear-infill/src/lib.rs:94` (527 LOC),
    `modules/core-modules/gyroid-infill/src/lib.rs:138` (716 LOC),
    `modules/core-modules/lightning-infill/src/lib.rs:97` (512 LOC; the
    D-137 fixee — actually calls
    `paint.lightning_tree_segments_for(object_id, region_id)`),
    `modules/core-modules/top-surface-ironing/src/lib.rs:305` (355 LOC).
    The other three take `_paint: &PaintRegionLayerView` and ignore it.
  - Test-guest extension at
    `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs:113`:
    the existing `fn run_infill(layer_index, regions, output, config)` adds a
    fifth parameter `_paint: PaintRegionLayerView`; the guest calls
    `paint.lightning_tree_segments(object_id, region_id)` for each region in
    its loop and emits a witness path encoding the segment count
    (`width == 137.0, x == segment_count_as_f32`).
  - **AC-3b host-side test driver (new file):**
    `crates/slicer-wasm-host/tests/contract/lightning_infill_guest_calls_lightning_tree_segments_tdd.rs`
    (new; registered in `crates/slicer-wasm-host/tests/contract/main.rs`,
    which currently lists 12 modules). Instantiates the rebuilt
    `layer-infill-guest.component.wasm` against a fixture print configured
    for lightning, drives a `Layer::Infill` dispatch, and asserts the
    guest's emitted witness path encodes the `lightning-tree-segments`
    count for the dispatched region. Without this file, AC-3b's pipe
    command has no driver and is unexercised.
  - `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs` —
    re-baseline the 6 existing `call_run_infill` call sites (lines 94,
    192, 268, 352, 433, 497) to add the paint arg after the trait
    signature change. Same shape of re-baseline as
    `infill_holder_resolution_painted_region_tdd.rs` did in packet 137.
  - Re-baseline `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs:592-616`
    (the `run-infill-postprocess` signature string and the `world-layer@2.2.0`
    version assertion) to the new `run-infill` signature and `2.3.0`.
  - 33 guest artifacts re-stale (21 core-module + 12 test-guests); a single
    `cargo xtask build-guests` rebuild is required after the WIT + macro
    changes.
- **DEVIATION CLOSURE (D-139-LAYER-GROUNDING-SEARCH-STUB) — Step 0 of 140:**
  - Port the full `getBestGroundingLocation` (Orca
    `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp::getBestGroundingLocation`)
    into `crates/slicer-core/src/algos/lightning/layer.rs`. The port must
    include:
    - The TBB-style parallel grid scan over the outline locator (or its
      sequential Rust equivalent using `rayon` if already a workspace
      dependency, or a simple `for` loop with `cancel: &dyn Fn()` per
      packet 139's convention).
    - The tree-node locator (`SparseNodeGrid` equivalent — a
      `HashMap<(i32, i32), Vec<NodeRef>>` keyed by `to_grid_point(...)` at
      `locator_cell_size` resolution).
    - The `wall_supporting_radius` exclusion: candidates within
      `wall_supporting_radius - tree_connecting_ignore_offset` of a wall
      are skipped.
    - The `getWeightedDistance` ranking (already ported in 139 Step 2;
      reused here).
    - Attribution header from `docs/ORCASLICER_ATTRIBUTION.md` (the
      standard header). Cite by file + function name (NOT by line number).
  - Remove the 139 Step-2 stub comment from
    `crates/slicer-core/src/algos/lightning/layer.rs:62` (the
    `// 139 deviation: ...` line).
  - Co-update the 139 test home
    (`crates/slicer-core/tests/algo_lightning_tdd.rs`) with the new
    `lightning_layer_wall_supporting_radius` (AC-G1) and a re-assertion of
    `lightning_generator_tree_continuity` (AC-G2) in the same step. The
    139 tests stay green; the 138 primitive tests stay green.
  - Coordinate system: 1 unit = 100 nm. Divide all OrcaSlicer nm constants
    by 100 at every port boundary. Use `slicer_ir::units_to_mm` /
    `Point2::from_mm` / `mm_to_units()` per `docs/08_coordinate_system.md`.
- DEV-081 closure edit; TASK-262…265 docs/07 closure sweep; contained lightning
  re-bless (AC-5) + the roadmap-close workspace ceremony.

## Out of Scope

- Claims/manifest changes (stays `["claim:sparse-fill"]` — lightning solid shells are
  not a thing in Orca or PnP).
- Linker changes — if linked lightning output looks wrong, the fault is triaged to
  emission (here) vs linking (133-follow-up), never patched in the linker from this
  packet.
- `modules/core-modules/support-surface-ironing/**` — it implements only
  `run_infill_postprocess`, not `run_infill`; the WIT signature change does not
  reach it.
- Any per-region skip-predicate change in the prepass — the skip-predicate stays
  print-wide per 139's `[FWD-resolved]` (the per-region predicate was the
  unrecoverable half of `D-137-LIGHTNING-PER-OBJECT-COLLAPSE`; 139 closes that
  deviation at the IR + dispatch + SDK layer, not the predicate layer).
- **Out-of-bounds note (revised):** the prior design put
  `crates/slicer-core/src/algos/lightning/**` entirely out-of-bounds. Per
  the boundary flip, 140 is the lightning packet and the in-scope list
  here names `layer.rs` and `tree_node.rs` explicitly (Step 0 only).
  Other 138/139 surface (cross-layer `Generator`, `DistanceField`, the
  producer) stays frozen for this packet; if 140 needs a change to
  those, it's a recorded deviation, not an in-scope edit.

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §L4 — full (short).
- `docs/adr/0029-lightning-prepass-tree-generator.md` — module-sampler contract
  (delegate SUMMARY).
- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` §Amendment point 2 — why
  pass-through detection was never an option.
- `CLAUDE.md` §Test Discipline — the workspace ceremony contract.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` (37 lines) — `Filler::_fill_surface_single` only: the per-layer sampling-side handling between `getTreesForLayer` and output (one SUMMARY dispatch).

## Acceptance Summary

- Positive cases: `AC-1`–`AC-5` in `packet.spec.md`, plus the new
  `AC-3a`/`AC-3b`/`AC-3c`/`AC-3d` for the WIT-extension deviation closure.
  Refinements: AC-1 is count + endpoint equality against the view (the
  module adds NO geometry of its own); AC-3 is the Architecture-A
  uniformity proof (lightning → linker → linked polylines); AC-3a
  pins the WIT signature change (run-infill takes paint; world-layer is
  2.3.0); AC-3b is the real `Layer::Infill` test-guest traversing the
  host↔guest WIT seam (the D-137 original AC-4 wording's
  fulfillment); AC-3c is the four-module compile check; AC-3d is the
  WIT-drift re-baseline; AC-4/AC-5 are the closure artifacts.
- Negative cases: `AC-N1` (wedge byte-identity), `AC-N2` (empty-trees
  totality — also proves the stub fallback is really gone).
- Cross-packet impact: closes DEV-081, `D-137-LIGHTNING-PER-OBJECT-COLLAPSE`
  (139), `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` (this packet), TASK-262…265,
  and the entire infill-parity roadmap (129–140).
- **Rejected alternatives for the WIT extension** (per the blast-radius
  investigation in `docs/specs/137-deviations-plan.md`):
  - Second export `run-infill-with-paint` (option a) — viable but adds
    permanent desync from the perimeters/support pattern D-137 explicitly
    invoked. Rejected.
  - `option<paint-region-layer-view>` parameter (option b) — not viable
    in this WIT tree (zero `option<>`-as-top-level-arg precedents; only
    as record field or return type). Rejected.
  - Import function `get-paint-region-layer-view(layer-idx)` (option c) —
    risky; the host call is per-call, not per-stage, so the paint-build
    cost is the same; zero surface win. Rejected.
  - Promote `lightning-tree-segments` to its own resource (option d) —
    risky; breaks the perimeters/support symmetry; strictly larger
    surface. Rejected.
  Canonical D-137 fix selected: extend `run-infill` + bump world-layer to
  2.3.0.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p lightning-infill 2>&1 \| tee target/test-output.log \| grep "^test result"` | module suite | FACT + counts |
| `cargo test -p slicer-runtime --test executor -- lightning_pipeline_linked 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-3 uniformity | FACT |
| `rg -n 'run-infill: func\(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view' crates/slicer-schema/wit/deps/world-layer/world-layer.wit` | AC-3a WIT signature | FACT |
| `rg -n 'package slicer:world-layer@2.3.0;' crates/slicer-schema/wit/deps/world-layer/world-layer.wit` | AC-3a WIT version | FACT |
| `cargo test -p slicer-wasm-host --test contract -- lightning_infill_guest_calls_lightning_tree_segments 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-3b real test-guest | FACT |
| `rg -n 'fn run_infill\(' modules/core-modules/{rectilinear,gyroid,lightning,top-surface-ironing}-infill/src/lib.rs` | AC-3c four-module compile | LOCATIONS (4) |
| `cargo test -p slicer-runtime --test contract -- wit_drift_detection 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-3d WIT drift re-baseline | FACT |
| `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N1 | FACT |
| `rg -q 'DEV-081.*[Cc]losed' docs/DEVIATION_LOG.md && echo OK` | AC-4 | FACT |
| `rg -q 'D-137-WIT-RUN-INFILL-NO-PAINT-VIEW.*[Cc]losed' docs/DEVIATION_LOG.md && echo OK` | DEV-137-WIT closure | FACT |
| `rg -q 'D-139-LAYER-GROUNDING-SEARCH-STUB.*[Cc]losed.*140' docs/DEVIATION_LOG.md && echo OK` | D-139 grounding closure (Step 0) | FACT |
| `cargo test -p slicer-core --features host-algos --test algo_lightning_tdd -- lightning_layer_wall_supporting_radius 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-G1 grounding exclusion | FACT |
| `cargo test -p slicer-core --features host-algos --test algo_lightning_tdd -- lightning_generator_tree_continuity 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-G2 grounding continuity (no regression) | FACT |
| `cargo test -p slicer-core --features host-algos --test algo_lightning_tdd -- lightning 2>&1 \| tee target/test-output.log \| grep "^test result"` | Step 0 full lightning suite (Step 0 exit gate) | FACT + counts |
| `cargo xtask build-guests --check` (rebuild if STALE) | WIT + macro changes | FACT |
| `cargo xtask test --workspace --summary` (sub-agent) | roadmap-close ceremony | FACT verdict + failing names only |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT |

## Step Completion Expectations

- The workspace ceremony runs last, after the bless, with the same summary-only
  dispatch contract as packet 136's.
- Bless order: AC-1/AC-3 (geometry/pipeline) green before any expectation re-bless.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated: the current
  module `lib.rs` (512 lines — one full read at Step 1, then it mostly gets deleted);
  the existing 323-line test file (read once to classify keep/rewrite/delete).
- Likely temptation reads: generator internals (139) "to understand the trees" — the
  IR view contract is the module's whole world; delegate a FACT if an IR semantics
  question arises.
- Sub-agent return-format hints: ceremony returns the `--summary` verdict block only;
  bless dispatches FACT per expectation (old → new + 1-line justification).
