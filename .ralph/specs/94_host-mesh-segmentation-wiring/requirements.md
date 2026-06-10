# Requirements: 94_host-mesh-segmentation-wiring

## Packet Metadata

- Grouped task IDs:
  - `TASK-244` — Retire the `PrePass::MeshSegmentation` host stage; delete the dead `execute_mesh_segmentation` kernel. Supersedes the original TASK-244 framing per the TASK-250 architectural investigation.
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2 — host:mesh_segmentation kernel wiring" (historical context); TASK-250 architectural finding is the proximate driver.
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The prior framing of TASK-244 wired the `execute_mesh_segmentation` host kernel into the prepass driver as a new `PrePass::MeshSegmentation` stage running before `host:mesh_analysis`. The packet shipped and surfaced four acceptance gaps (AC-6/AC-7/AC-8/AC-10) — all of which the closure audit documented honestly. The most consequential gap: `resources/cube_4color.3mf` triggers `MeshSegmentationError::DegenerateStroke { reason: TangentToFacetEdge }`, meaning the stage fails on the only painted fixture that exercises it.

The TASK-250 investigation traced the failure to a structural mismatch, not a fixture bug:

1. **The loader already does the work the kernel was meant to do.** `crates/slicer-model-io/src/loader.rs:1900-1961` implements `split_triangle_strokes` + `walk_triangle_selector_strokes`, reproducing OrcaSlicer's `TriangleSelector` recursive subdivision exactly. `PaintLayer.strokes` is already in OrcaSlicer's flat-leaf form at the load boundary.
2. **OrcaSlicer has no `stroke` abstraction.** Its `FacetsAnnotation::get_facets()` reconstructs a transient per-extruder flat list from the hex bitstream on demand. Our `PaintLayer.strokes` is the IR-resident equivalent of that transient list.
3. **The kernel's clean-bisection template is structurally incompatible with OrcaSlicer-pattern leaves.** 12+ `TangentToFacetEdge` raise sites in `crates/slicer-core/src/algos/mesh_segmentation.rs` are not a fixture quirk — they are the kernel discovering that arbitrary-depth subdivision leaves (multi-vertex shared edges, all-three-on-edges patterns) systematically don't fit the bisectable shape the kernel expects.
4. **No downstream consumer needs a flat-IR mesh.** P95 (parity doc Phase 3) reads `PaintLayer.strokes` directly via `collect_facets()`. P96 (Phase 5 width-limiting) operates on variant polygons. P97 deletes the WASM-guest path entirely.

The retirement is the right architectural call. The kernel duplicates the loader's work, adds a failure mode, and has no consumer. This packet executes the deletion + clean state for P95 to consume `PaintLayer.strokes` directly.

The loader's stroke path is the canonical TriangleSelector normalization site going forward.

## In Scope

- DELETE `crates/slicer-core/src/algos/mesh_segmentation.rs` (kernel).
- DELETE `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` (kernel unit tests).
- DROP `pub mod mesh_segmentation;` from `crates/slicer-core/src/algos/mod.rs`.
- DELETE `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` (host built-in producer constant).
- DROP `pub mod mesh_segmentation_producer;` from `crates/slicer-runtime/src/builtins/mod.rs`.
- REVERT `Blackboard::replace_mesh` and its doc-comment in `crates/slicer-runtime/src/blackboard.rs` (returns to the pre-P94 shape with `replace_slice_ir` as the sole `replace_*` method).
- REVERT the prepass driver insertion in `crates/slicer-runtime/src/prepass.rs` — drop the `PrePass::MeshSegmentation` stage invocation, the `required_slots` table entry, the `PrepassExecutionError::MeshSegmentation` variant, and the `has_subfacet_strokes` helper (if it was added in P94 and has no other caller).
- DELETE the four P94-introduced integration / contract test files: `blackboard_replace_mesh_tdd.rs`, `prepass_execution_error_mesh_segmentation_variant_tdd.rs`, `cube_4color_mesh_segmentation_strokes_consumed_tdd.rs`, `cube_fuzzy_painted_mesh_segmentation_strokes_consumed_tdd.rs`, `mesh_segmentation_determinism_tdd.rs`, `mesh_segmentation_short_circuit_no_strokes_tdd.rs`.
- DROP the corresponding `mod` declarations in `crates/slicer-runtime/tests/contract/main.rs` and `crates/slicer-runtime/tests/executor/main.rs`.
- UPDATE `docs/07_implementation_status.md` TASK-244 row to reflect the retirement supersession.
- ADD `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` capturing `P93_BASELINE_SHA`, `P94R_POST_CUBE_SHA`, and the supersession rationale paragraph.
- APPEND a TASK-250 supersession note to `docs/specs/paint-pipeline-orca-parity-roadmap.md` §P2 (one paragraph at the end of that section).

## Out of Scope

- `modules/core-modules/mesh-segmentation/` directory deletion — P97 (WASM mesh-segmentation deletion, TASK-247).
- WASM-guest infrastructure (`mesh-segmentation-output` WIT resource, `mesh_segmentation_marks` host-side field, `MeshSegmentationIR`, `FacetPaintMark`, `PrepassStageOutput::MeshSegmentation`, `BlackboardPrepassSlot::MeshSegmentation`) — P97 owns the deletion.
- The loader's `split_triangle_strokes` / `walk_triangle_selector_strokes` path — NOT touched; this is the canonical normalization site post-P94.
- Paint-segmentation kernel changes — P95 (TASK-245) territory.
- The `mesh-segmentation.toml.disabled` rename — stays as-is from the original P94 work; P97 deletes the file along with the directory.
- Any test currently passing on `cube_4color.3mf` outside the deleted P94 test files (the cherry-pick 5c272ef RED tests in `cube_4color_paint_tdd.rs` stay RED awaiting P95).

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2" (~80 lines; read directly) — historical roadmap intent; the TASK-250 supersession note appended at the end of §P2 documents the retirement.
- `crates/slicer-model-io/src/loader.rs:1900-1961` — read only if confirming the loader's `split_triangle_strokes` path is intact (AC-N3); otherwise treat as out-of-scope for this packet.
- `docs/specs/orca-paint-segmentation-parity.md` §Phase 3 (lines 140-141) — `collect_facets()` design — read once to confirm the canonical input contract P95 inherits.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- None directly. The TASK-250 investigation already established the parity surface via delegated reads against `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:2490`, `Model.cpp:3806`, and `TriangleSelector.cpp:1542-1606`. The findings are encoded in the Goal of `packet.spec.md`.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-9` from `packet.spec.md`. The deletion is purely subtractive; no new behavior is introduced.
- Negative cases: `AC-N1` (no surviving references), `AC-N2` (WASM module stays for P97), `AC-N3` (loader's stroke path untouched).
- Cross-packet impact: P95 unblocks — its `collect_facets()` reads `PaintLayer.strokes` directly. P97 slightly simplifies — no host stage to coordinate with. P96 unaffected. The original P94 acceptance gaps (AC-6 / AC-7 / AC-8 / AC-10 of the prior framing) retire with the stage; they are no longer outstanding.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Workspace compiles after the deletions | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings introduced | FACT pass/fail |
| `cargo test --workspace 2>&1 \| tee target/test-output.log` | AC-5 — workspace gate (deletion blast spans multiple crates; per `CLAUDE.md` §Test Discipline rule 2) | FACT per-bucket count |
| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p94-wedge.gcode && test "$(sha256sum target/p94-wedge.gcode \| awk '{print $1}')" = "$(grep -oE 'P93_BASELINE_SHA=[a-f0-9]+' .ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md \| head -1 \| cut -d= -f2)"` | AC-6 — wedge byte-identical vs P93 baseline | FACT pass/fail (exit 0 == match) |
| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output target/p94-cube.gcode && test -s target/p94-cube.gcode && sha256sum target/p94-cube.gcode \| awk '{print $1}'` | AC-7 — cube slices to completion; capture P94R_POST_CUBE_SHA | FACT (sha256 hash) |
| `cargo xtask build-guests --check` | AC-8 — guest WASM clean | FACT pass/fail |
| `rg -n --glob '!.ralph/specs/94_host-mesh-segmentation-wiring/**' --glob '!docs/specs/paint-pipeline-orca-parity-roadmap.md' --glob '!docs/07_implementation_status.md' 'execute_mesh_segmentation\|MESH_SEGMENTATION_PRODUCER\|MeshSegmentationError\|host:mesh_segmentation\|PrePass::MeshSegmentation\|replace_mesh' crates/ modules/ docs/ ; test $? -eq 1` | AC-N1 — zero surviving references | FACT pass/fail |

## Step Completion Expectations

- Step 0 (baseline capture) MUST write `P93_BASELINE_SHA=<hex>` to `closure-log.md` BEFORE any deletion begins. AC-6 reads the value back; without the closure-log line the gate cannot pass.
- Step 1 (cargo check after deletion) is the cheapest falsifying check. If `cargo check` fails after the kernel + producer deletions, a downstream consumer was missed; pause and resolve before continuing.
- Step 5 (`cube_4color.3mf` end-to-end slice) captures the new cube SHA. This becomes P95's input baseline — P95's acceptance ceremony will assert against `P94R_POST_CUBE_SHA`.
- AC-9 (`docs/07` TASK-244 row update) is a delegated edit; never load the full backlog file.

## Context Discipline Notes

- `crates/slicer-runtime/src/prepass.rs` is large (> 700 lines). The deletion sites are narrow: the P94-introduced driver block + the `required_slots` entry + the error variant. Range-read at each site; do not load the full file.
- `crates/slicer-runtime/src/blackboard.rs` is ~400 lines. Range-read at the `replace_mesh` definition site and its doc-comment block.
- `crates/slicer-core/src/algos/mesh_segmentation.rs` — never load. The file is being deleted; reading it is wasted context.
- `docs/07_implementation_status.md` — delegate the TASK-244 row update; never load the full backlog.
- Out-of-bounds: `modules/core-modules/mesh-segmentation/**` (P97 territory), `OrcaSlicerDocumented/**` (delegated), the loader's stroke path (AC-N3 sole reference).
