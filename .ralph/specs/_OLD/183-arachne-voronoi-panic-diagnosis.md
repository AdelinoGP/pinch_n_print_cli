---
status: implemented
packet: 183-arachne-voronoi-panic-diagnosis
task_ids:
  - TASK-296
---

# 183-arachne-voronoi-panic-diagnosis

## Goal

Close the defensive asymmetry that makes D-167 invisible: wrap the one unprotected boostvoronoi `Builder::build()` call — in `voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`) — in `catch_unwind`, mapping a caught `robust_fpt` assertion panic to a distinct `VoronoiError` variant exactly as `medial_axis.rs` and `algos/paint_segmentation/voronoi_graph.rs` already do, then use the now-observable failures to answer D-167's open question: do these panics drop live wall geometry, or are they inert? Record the verdict and either close D-167 or narrow it to a named successor.

## Problem Statement

Deviation **boostvoronoi panic observation** (Open — "observed, undiagnosed", 2026-07-16). During the D-160 session, a `perimeter_parity` run printed 13 background-thread panics of the form `rhs.fpv_.is_finite()` originating inside the `boostvoronoi` dependency's `robust_fpt` module, while the suite still reported all tests passing. The output impact was never determined, so the row has sat undiagnosed.

Grounding identified the structural reason these are invisible, and it is an asymmetry rather than a mystery. There are three boostvoronoi call sites in `crates/slicer-core`:

| Call site | Guard |
| --- | --- |
| `medial_axis.rs` | wraps the builder in `std::panic::catch_unwind(AssertUnwindSafe(...))`; its comment explicitly names `assertion failed: fpv.is_finite()` at `robust_fpt.rs`. **Its catch arm returns `Err(())`, which the caller converts to `return Ok(vec![])` — a silent empty result, no error and no diagnostic.** Guarded, but not in a way this packet may copy. |
| `algos/paint_segmentation/voronoi_graph.rs` | wraps the builder in `catch_unwind` and maps the catch arm to the distinct typed error `MmuGraphError::PredicatePanic`, which propagates. **This is the pattern this packet copies.** |
| **`voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`)** | **no `catch_unwind`** — only `map_err(map_bv_error)` on the returned `Result` |

A `robust_fpt` failure is an `assert!` **panic**, not a `Result::Err`, so `map_err` and the `?` operator cannot observe it. The skeletal/Arachne path — `voronoi_from_segments` ← `SkeletalTrapezoidationGraph::from_polygons` ← `run_arachne_pipeline` — is therefore the one boostvoronoi entry point with no backstop. Because per-layer work runs under a rayon `par_iter()` (`crates/slicer-runtime/src/layer_executor.rs`) and `arachne-perimeters` forwards to the host bridge `generate_arachne_walls` which runs `run_arachne_pipeline` natively on the host, the panic executes on a rayon worker: it prints to stderr and unwinds that worker's region, which is exactly the "swallowed background-thread panic" the row describes.

What remains genuinely unknown, and what this packet exists to settle: because `voronoi_from_segments` has no local guard, a panic unwinds `from_polygons` and therefore that region's entire arachne result rather than `?`-returning a clean error. Whether the resulting walls are silently dropped or the panic lands on a discarded/retried path **cannot be decided statically**. The suite passing proves only that the *asserted* geometry was unaffected — not that no geometry was lost.

This is one coherent slice: add the missing guard (which is also the instrumentation), use it to capture the degenerate inputs and measure the output delta, and record a verdict.

## Architecture Constraints

- ADR-0023 (`docs/adr/0023-arachne-port-strategy.md`) assigns pre-snapping of T-junctions, duplicates, and near-collinear-within-`epsilon_offset` segments to the **caller**, and `voronoi_from_segments`'s own doc comment restates that it "does not perform that pre-snapping itself". The guard added here must not be mistaken for, or quietly become, that pre-snapping — it converts an unwind into an observable error and nothing more. Any actual hardening belongs in `preprocess_input_outline` and is out of scope.
- **`algos/paint_segmentation/voronoi_graph.rs`'s `MmuGraphError::PredicatePanic` is the SOLE pattern reference for the new guard.** Its shape is: `catch_unwind(AssertUnwindSafe(|| Builder::…build()))` immediately around the builder, then `match { Ok(Ok(d)) => d, Ok(Err(e)) => return Err(…), Err(_) => return Err(MmuGraphError::PredicatePanic) }` — a caught panic becomes a **distinct, propagating typed error**. Copy that. **Do not cite `medial_axis.rs` as a pattern to copy.** An earlier draft did, and it contradicts the "no empty graph on catch" constraint below: `medial_axis`'s catch arm returns `Err(())`, which its own caller converts to `return Ok(vec![])` — the silent-empty outcome this packet explicitly bars. `medial_axis.rs` remains a **read-only reference for the `AssertUnwindSafe` justification comment only**, never for the catch arm's disposition.
- `boostvoronoi` is optional and gated: `host-algos = ["dep:rayon", "dep:boostvoronoi"]` in `crates/slicer-core/Cargo.toml`. All new tests exercising this path must run under `--features host-algos`.
- `boostvoronoi/console_debug` is exposed through the `voronoi-panic-regression` feature and is required only by the separate `voronoi_panic_regression` test target. It must not be a dev-dependency feature, because workspace test feature unification leaks dev-dependency features into unrelated test binaries.
- A caught panic must not be converted into a silently-successful empty graph **at the entry point this packet guards**. That would replace a loud-but-swallowed failure with a quiet one and re-create missing-component dispatch defect's defect class in a different crate. Note the scope limit honestly: shipped `medial_axis.rs` already does exactly the thing this constraint forbids (catch arm `Err(())` → caller `Ok(vec![])`), and this packet does **not** change it. See §Locked Assumptions for the residual and the `DEV-###` row that records it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Trigger note for the bullet above: `FINDINGS.md` reports segment coordinate bounds and near-collinearity thresholds. Those are internal units, not mm — state the unit explicitly in the artifact so the verdict is not misread by a factor of 10⁴.

## Data and Contract Notes

- IR/manifest contracts: none touched.
- WIT boundary: none. `voronoi_from_segments` is host-internal to `slicer-core`; the arachne path reaches it through the host-service bridge `generate_arachne_walls`, which is unchanged.
- Determinism/scheduler constraints: converting an unwind into an `Err` changes failure *shape*, not slice ordering. If the verdict is that geometry was being lost, slices that previously produced quietly-degraded walls will now surface an error — that behavior change is the verdict's consequence and must be recorded in `FINDINGS.md`, not smoothed over.

## Locked Assumptions and Invariants

- Locks the invariant that all three boostvoronoi entry points **guard** the builder. **Scope the invariant precisely — it is NOT "a caught panic must never become a silently-successful empty result" tree-wide.** Stated that broadly it is false the moment it is written, because shipped `medial_axis.rs` violates it: its catch arm returns `Err(())`, which its caller converts to `return Ok(vec![])`. The invariant this packet actually asserts is: **for the two boostvoronoi entry points that return a typed error — `voronoi_from_segments` (new, this packet) and `MmuGraphError::PredicatePanic` (`algos/paint_segmentation/voronoi_graph.rs`, existing) — a caught panic propagates as a distinct typed error and never as a successful empty graph.** Reversing that re-opens D-167.
- **Knowingly inconsistent, recorded not fixed: `medial_axis`'s degrade-to-empty policy.** `medial_axis.rs` keeps its silent-empty catch arm after this packet. That is a real inconsistency with the invariant above, and it is deliberate: changing it turns previously-quiet degenerate regions into hard errors in `classic-perimeters`' gap-fill and thin-wall paths (two `slicer_sdk::host::medial_axis` call sites in `ClassicPerimeters::run_perimeters`), a behaviour change this diagnosis-first packet has not measured and is not chartered to make. **No new ADR is authored for this.** Instead, Step 5 files a `DEV-###` row recording the inconsistency — scope, why it is accepted, and what a fix would cost — owned by whichever packet next touches `medial_axis`. **Re-derive the id at filing time; never carry one forward from this packet:** `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, then take the next free number.
- Does **not** lock any pre-snapping behavior — ADR-0023's caller-responsibility contract is unchanged, and the successor packet remains free to choose its hardening.

## Risks and Tradeoffs

- **The workload may produce zero catches on the current tree.** The 13 panics were observed in the D-160 session; intervening arachne work (packets 147-166) may have removed the triggering inputs. `FINDINGS.md` must record a zero-count outcome as a legitimate verdict ("not reproducible on this tree") rather than leaving the packet unclosable — AC-3 is satisfied by an honest zero, AC-2 by an unchanged suite status.
- Conversely, if the guard reveals that walls were being dropped, the packet's output is a *new* known defect plus a successor packet, not a fix. That is the intended shape of a diagnosis-first packet and must not be presented as closure of the underlying geometry problem.
- **Residual: the no-silent-empty invariant is asserted only for the boostvoronoi entry points that return a typed error.** `medial_axis`'s degrade-to-empty policy stays as shipped and is knowingly inconsistent with it; the packet records that in a `DEV-###` row (id re-derived at filing time) rather than fixing it or authoring an ADR. A reader who takes the invariant tree-wide will be wrong — the deviation row is the guard against that.
- The diagnostic capture runs inside a hot geometry path. It must be cheap and only on the catch branch; adding per-call characterization to the success path would regress arachne performance.
