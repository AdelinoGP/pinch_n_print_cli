# T27 probe iteration 2 — Slice internals, phase-B split, tail decomposition, drop window

**Correction (2026-09-28):** the sibling-region memo described below has no
eligible siblings on the matched fixtures, and its proposed key would not by
itself establish identical inputs. See [PREMISE-AUDIT.md](PREMISE-AUDIT.md).
The measured offset work below remains valid; the proposed saving does not.

Continues [FINDINGS-SUBSTAGE.md](FINDINGS-SUBSTAGE.md) (iteration 1). Probe vintage: 2026-09-27/28, HEAD `bf77f4f7` + `PERF-T27-PROBE` v2 (temporary; removed before any commit). Guests fresh (exit 0) before and after the batch. Four uninstrumented slices, matched-job configs, `RAYON_NUM_THREADS=12`. Raw captures in gitignored `target/matched-pair/t27-probe2/`; extracted `.probe.txt` files committed. Bracket semantics unchanged: `*_wall` = calling-thread wall; everything else = accumulated across rayon workers (ranking only, §3.3 class). Core-side names carry the `core:` prefix.

**Label note (read first).** Iteration 2 wired reset/flush only around `PrePass::Slice`, `PrePass::OverhangAnnotation`, and the run tail. The ShellClassification sub-brackets therefore flush under the `slice_tail` label (no reset ran between the gate and the tail). The gate_* names under `slice_tail` are ShellClassification brackets — treat the label as "end-of-run drain", not stage identity.

## New results per question

### 1. `PrePass::Slice` per-layer internals (accumulated, base)

| Bracket | base off | base on | benchy off/on |
| --- | ---: | ---: | ---: |
| `core:slice_assemble_flat_bridge_areas` | **453.5 s** | **409.9 s** | 23.8 / 26.1 s |
| `core:slice_assemble_bridge_areas` | 136.0 s | 128.6 s | 1.0 / 1.0 s |
| `core:slice_classify_region_surfaces` | 27.1 s | 24.8 s | 1.0 / 1.1 s |
| `core:slice_closing_radius` | 15.1 s | 14.0 s | 1.1 / 1.3 s |

Sum of accumulated fuel ≈ 578 s (off) / 577 s (on) against the stage's 47–56 s parallel-map wall (iteration 1's `_wall` bracket) → ≈ 10–12x worker sum over wall: the per-layer body is parallel-saturated, and **`assemble_flat_bridge_areas` is ~70% of the stage's fuel** (both base cells). It is the diff/offset/intersection chain that turns each object's bottom-surface footprint against the layer's unsupported region into flat-bridge areas (`assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`)). Second: `assemble_bridge_areas` (sloped-bridge assembly, intersection + Miter offset + re-intersection per bridge region). Classify and closing-radius are small.

Ranked reduction candidate for the stage: reduce flat-bridge clipper work per layer (the `unsupported` input is already computed once per layer; the flat path re-diffs and re-intersects per region). Requires a wall-qualified A/B before any keep/drop.

### 2. ShellClassification phase-B sub-split (accumulated, base-on: 23.7 s of the 25.1 s phase-B wall)

| Bracket | base off | base on | calls |
| --- | ---: | ---: | ---: |
| `gate_deep_infill_clip_offset` | 11.95 s | 12.07 s | 121 |
| `gate_erosion_offset` | 9.11 s | 9.34 s | 36 |
| `gate_internal_bridge_angles` | 1.97 s | 2.03 s | 10 |
| `gate_per_surface_expand_intersect` | 112 ms | 114 ms | 182 |
| `gate_qualified_retain_intersections` | 14 ms | 14 ms | 121 |
| `gate_expansion_area_unions` | 148 ms | 159 ms | 10 |

The two full-layer Miter offsets (`offset(deep_infill_area, +1.5×spacing)` per timeline entry and the `−(4.5×spacing)` erosion) are **~90% of the serial phase-B cost** — the angles step I suspected in iteration 1 is only 2 s. These two offsets are computed *per (region, timeline)* — 121 and 36 calls — over the same full-layer `deep_infill_area` polygons. A per-layer memo (key the offset by `(object, layer, spacing)` and share across regions of the same object-layer) is the obvious representation-safe reduction: same inputs, same join type, same delta → identical output by construction. Prize: up to ~20 s serial wall per base cell (phase B was 25–30 s wall).

### 3. Tail decomposition — the 6.8-vs-16.9 s question resolves into two findings

Bracketed tail (this batch, base-on): diagnostics replay 32 µs, layer-count scan 17 ms, `take_final_gcode_ir` + `estimate_print` 105 ms, slice-stats build 114 ms total. **`estimate_print` on this content is ~0.1 s, not 16.9 s** — the second estimator walk is not the tail. The bracketed window leaves a **7.53 s unbracketed gap** between the postpass `phase_complete` timestamp and `slice_stats` on base-on (0.07–0.23 s on the other three cells). The gap sits inside `run_pipeline_core`'s return path (`crates/slicer-runtime/src/pipeline.rs`; `run_pipeline_with_instrumentation` is a thin forwarder): after `instrumentation.on_phase_end(Phase::PostPass)` and the `PipelineOutput` construction, the function scope drops `blackboard` (mesh, layer plan, region map, the committed `SliceIR`, support-analysis and support-geometry IRs), `layer_irs` (`Vec<LayerCollectionIR>`), `plan`, `runners`, and `wasm_handles` — hundreds of MB of polygon IR freed single-threaded between the last phase event and the caller's first tail bracket. That teardown is real wall on the slice critical path, invisible to every phase bracket, and varying with vintage/content (16.9 s in the dev174-after captures, 7.5 s here; the accelerated 6.8 s sits in range — the "ordinary vs accelerated" mystery was this teardown all along, not estimator behavior). The drop cost is reducible in principle (parallel teardown, or releasing the blackboard before the tail), but it is below-fold: 1–3% of the cell, and never auto-commit — the candidate design returns to the human.

### 4. OverhangAnnotation wall (resolves iteration 1's missing bracket)

`oh_stage_wall`: 14.24 s base-off / 12.18 s base-on / 0.84–1.01 s benchy. Against iteration 1's accumulated interior (224.8 s base-off / 138.4 s base-on) that is ≈ 11–16x worker sum over wall — the stage is parallel-saturated, confirming the below-fold reading. No serial-candidate graduates from this stage.

## Updated ranked next actions

1. **Phase-B offset memoization** (per-layer shared `deep_infill_clip` / erosion across sibling regions) — identical-output-by-construction candidate, up to ~20 s serial wall per base cell. Needs a wall-qualified paired A/B per the standing metric.
2. Flat-bridge reduction inside `PrePass::Slice` (70% of the stage's fuel; parallel, so the wall prize is bounded by the 47–56 s wall) — needs a representation analysis first.
3. Drop-window teardown (7.5–17 s, vintage-dependent) — bracket + parallelize or early-release; below-fold.
4. Overhang footprint union → bbox/coverage predicate (from iteration 1; unchanged).
5. `apply_opening` join-type parity (from iteration 1; unchanged, small).

Measurement notes carried forward: the `slice_tail` label mixes ShellClassification gate brackets into the end-of-run drain in this iteration's wiring (documented above); the tail brackets themselves (`tail_*`) are honest walls; teardown between `phase_complete(postpass)` and the tail is unbracketed by construction of the pipeline boundary.
