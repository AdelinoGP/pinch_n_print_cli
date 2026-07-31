# Design: 183-arachne-voronoi-panic-diagnosis

## Controlling Code Paths

- Primary code path: `voronoi_from_segments(&[Segment]) -> Result<HalfEdgeGraph, VoronoiError>` (`crates/slicer-core/src/voronoi.rs`) — builds via `Builder::<i64>::default().with_segments(...).build()` and applies only `map_err(map_bv_error)`. Callers: `SkeletalTrapezoidationGraph::from_polygons` (`crates/slicer-core/src/skeletal_trapezoidation/graph.rs`) ← `run_arachne_pipeline` (`crates/slicer-core/src/arachne/pipeline.rs`).
- Neighboring tests/fixtures: `crates/slicer-core/tests/voronoi_stress.rs` (the ordinary `host-algos` stress target), `crates/slicer-core/tests/voronoi_panic_regression.rs` (the explicitly featured panic target), `crates/slicer-core/tests/voronoi.rs`, `crates/slicer-core/tests/medial_axis_degenerate_input_tdd.rs` (the degenerate-input precedent), and the workload `crates/slicer-runtime/tests/integration/perimeter_parity.rs`.
- OrcaSlicer comparison: none. The guard is a Rust-side defence against a Rust dependency's assertion; canonical has no analogue. This packet consults no `OrcaSlicerDocumented/` source.

## Architecture Constraints

- ADR-0023 (`docs/adr/0023-arachne-port-strategy.md`) assigns pre-snapping of T-junctions, duplicates, and near-collinear-within-`epsilon_offset` segments to the **caller**, and `voronoi_from_segments`'s own doc comment restates that it "does not perform that pre-snapping itself". The guard added here must not be mistaken for, or quietly become, that pre-snapping — it converts an unwind into an observable error and nothing more. Any actual hardening belongs in `preprocess_input_outline` and is out of scope.
- **`algos/paint_segmentation/voronoi_graph.rs`'s `MmuGraphError::PredicatePanic` is the SOLE pattern reference for the new guard.** Its shape is: `catch_unwind(AssertUnwindSafe(|| Builder::…build()))` immediately around the builder, then `match { Ok(Ok(d)) => d, Ok(Err(e)) => return Err(…), Err(_) => return Err(MmuGraphError::PredicatePanic) }` — a caught panic becomes a **distinct, propagating typed error**. Copy that. **Do not cite `medial_axis.rs` as a pattern to copy.** An earlier draft did, and it contradicts the "no empty graph on catch" constraint below: `medial_axis`'s catch arm returns `Err(())`, which its own caller converts to `return Ok(vec![])` — the silent-empty outcome this packet explicitly bars. `medial_axis.rs` remains a **read-only reference for the `AssertUnwindSafe` justification comment only**, never for the catch arm's disposition.
- `boostvoronoi` is optional and gated: `host-algos = ["dep:rayon", "dep:boostvoronoi"]` in `crates/slicer-core/Cargo.toml`. All new tests exercising this path must run under `--features host-algos`.
- `boostvoronoi/console_debug` is exposed through the `voronoi-panic-regression` feature and is required only by the separate `voronoi_panic_regression` test target. It must not be a dev-dependency feature, because workspace test feature unification leaks dev-dependency features into unrelated test binaries.
- A caught panic must not be converted into a silently-successful empty graph **at the entry point this packet guards**. That would replace a loud-but-swallowed failure with a quiet one and re-create DEV-087's defect class in a different crate. Note the scope limit honestly: shipped `medial_axis.rs` already does exactly the thing this constraint forbids (catch arm `Err(())` → caller `Ok(vec![])`), and this packet does **not** change it. See §Locked Assumptions for the residual and the `DEV-###` row that records it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Trigger note for the bullet above: `FINDINGS.md` reports segment coordinate bounds and near-collinearity thresholds. Those are internal units, not mm — state the unit explicitly in the artifact so the verdict is not misread by a factor of 10⁴.

## Code Change Surface

- Selected approach: add the missing `catch_unwind` guard plus a distinct `VoronoiError` variant, which simultaneously (a) removes the three-call-site asymmetry, (b) makes the previously-swallowed failures observable values, and (c) provides the capture point for the diagnostic characterization. Then measure the workload with and without the guard to settle the geometry-loss question, and record the verdict.
- Reopened remediation: keep the guard's synthetic assertion coverage behind `voronoi-panic-regression`, move that test out of the ordinary stress binary, and make `xtask test` surface process-level abort evidence and return a reliable nonzero status.
- Exact functions, traits, manifests, tests, and fixtures:
  - `voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`) — wrap the builder call; capture the segment characterization on catch.
  - The `VoronoiError` enum (`crates/slicer-core/src/voronoi.rs`) — one new variant for a caught builder panic.
  - New test `voronoi_from_segments_degenerate_input_returns_result_not_panic` in `crates/slicer-core/tests/voronoi_stress.rs`.
  - Synthetic panic test `voronoi_from_segments_predicate_panic_fires_on_synthetic_input` in `crates/slicer-core/tests/voronoi_panic_regression.rs`, enabled only by `voronoi-panic-regression`.
  - `xtask::test::collect_process_failure_details` (`xtask/src/test.rs`) and the `pnp_cli` rebuild failure path.
  - New artifact `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md`.
- Rejected alternatives and reasons:
  - *Instrument without guarding (log and re-panic).* Leaves the unwind, so the geometry-loss question stays unanswerable and the asymmetry with the other two call sites remains.
  - *Pre-snap in `preprocess_input_outline` first.* Might make the panics disappear without ever establishing whether they were dropping geometry, converting an open question into an unverifiable assumption. Diagnose first, harden second.
  - *Catch and return an empty graph.* Swaps a loud failure for a silent one; explicitly barred by the Architecture Constraints above.

## Files in Scope (read + edit)

- `crates/slicer-core/src/voronoi.rs` — role: owns the unguarded builder call and `VoronoiError`; expected change: `catch_unwind` guard, new error variant, diagnostic capture.
- `crates/slicer-core/tests/voronoi_stress.rs` — role: host-algos-gated stress binary; expected change: retain the ordinary degeneracy and error-shape tests without `console_debug`.
- `crates/slicer-core/tests/voronoi_panic_regression.rs` — role: explicitly featured assertion-coverage target; expected change: synthetic panic test must require `PredicatePanic`.
- `crates/slicer-core/Cargo.toml` — role: isolates `boostvoronoi/console_debug` behind the explicit panic-regression target feature.
- `xtask/src/test.rs` — role: gated workspace test runner; expected change: reliable rebuild failure exit and process-abort summary details.
- `crates/slicer-wasm-host/test-guests/path-optimization-multi-read/src/lib.rs` — role: reopened workspace test-guest surface included in the remediation diff.
- `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md` — role: the packet's diagnostic output; expected change: created with the four required headings.
- `docs/DEVIATION_LOG.md` — role: D-167 disposition and the dedicated open `DEV-099` medial_axis follow-up.
- `docs/07_implementation_status.md` — role: generated status map for `TASK-296` and the closed D-167 anchor.
- `docs/specs/deviation-backlog-remediation-plan.md` — role: deviation/backlog queue reconciliation.
- `.ralph/specs/183-arachne-voronoi-panic-diagnosis/packet.spec.md` — role: active packet contract and acceptance predicates.
- `.ralph/specs/183-arachne-voronoi-panic-diagnosis/implementation-plan.md` — role: step sequencing, verification, and completion ceremony.
- `.ralph/specs/183-arachne-voronoi-panic-diagnosis/design.md` — role: packet scope and architecture constraints.
- `.ralph/specs/183-arachne-voronoi-panic-diagnosis/task-map.md` — role: task and backlog anchor map.

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` — the `catch_unwind` guard block and its `MmuGraphError::PredicatePanic` arm, located by `rg 'catch_unwind'` — purpose: **the sole guard-shape reference** to copy.
- `crates/slicer-core/src/medial_axis.rs` — the `catch_unwind` block only, located by `rg 'catch_unwind'` — purpose: (a) its `AssertUnwindSafe` justification comment, and (b) Step 0's confirmation that its catch arm returns `Err(())` → `Ok(vec![])`, i.e. a silent empty result. **Not a pattern to copy.**
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (1715 lines) — the `voronoi::voronoi_from_segments(&segments)?` call inside `from_polygons` only, located by `rg` — purpose: confirm how the new `Err` propagates to the caller.
- `crates/slicer-core/tests/medial_axis_degenerate_input_tdd.rs` — purpose: reuse its degenerate-input construction for AC-N1 rather than inventing one.
- `docs/adr/0023-arachne-port-strategy.md` — purpose: the caller-pre-snaps contract to cite and not violate.
- `docs/DEVIATION_LOG.md` — the D-167 row only — purpose: verdict text.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — not consulted by this packet; never load.
- `crates/slicer-core/src/arachne/preprocess.rs` — the successor packet's surface; read-only at most, never edited here.
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` beyond the single call site — D-154 territory, queued separately.
- `modules/core-modules/support-surface-ironing/**` — explicitly external fixture blocker; out of scope for packet 183 and must not be edited here.
- `target/`, `Cargo.lock`, generated code, vendored `boostvoronoi` sources — never load.

## Expected Sub-Agent Dispatches

- Question: quote the `catch_unwind` guard block and its `MmuGraphError::PredicatePanic` catch arm; scope: `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`; return: `SNIPPETS` (<=1 x 30 lines); purpose: Step 2 guard shape — the sole pattern reference.
- Question: for up to 5 baseline `is_finite` panic lines under `RUST_BACKTRACE=1`, report the innermost `slicer_core::` frame; scope: `cargo test -p slicer-runtime --test integration -- perimeter_parity`; return: `FACT` (<=10 lines); purpose: Step 0 panic attribution.
- Question: run the `perimeter_parity` workload on the unmodified tree and report only the pass/fail line and the count of `is_finite` panic lines; scope: `cargo test -p slicer-runtime --test integration -- perimeter_parity`; return: `FACT` (<=5 lines); purpose: Step 1 baseline.
- Question: report the D-167 row's current Status cell verbatim, and the highest `D-` / `DEV-` id currently in the log; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (<=5 lines); purpose: Step 5 verdict text and successor-id allocation.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4 — the workload measurement and geometry-delta comparison)
- Highest-risk dispatch and required return format: the baseline `perimeter_parity` run — `FACT` capped at 5 lines, since the raw suite output is large and must not enter the implementer's context.

## Open Questions

- `[FWD]` Whether the caught-panic characterization should be emitted via the existing tracing/diagnostic channel or written directly to `FINDINGS.md` by the test harness. Either satisfies AC-3; the implementer picks based on what the catch branch can cheaply reach. The choice must not add cost to the success path.
