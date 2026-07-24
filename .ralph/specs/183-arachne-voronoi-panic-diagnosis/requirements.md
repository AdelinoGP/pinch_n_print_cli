# Requirements: 183-arachne-voronoi-panic-diagnosis

## Packet Metadata

- Grouped task IDs: `TASK-296`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Deviation **D-167-BOOSTVORONOI-ROBUST-FPT-PANICS** (Open — "observed, undiagnosed", 2026-07-16). During the D-160 session, a `perimeter_parity` run printed 13 background-thread panics of the form `rhs.fpv_.is_finite()` originating inside the `boostvoronoi` dependency's `robust_fpt` module, while the suite still reported all tests passing. The output impact was never determined, so the row has sat undiagnosed.

Grounding identified the structural reason these are invisible, and it is an asymmetry rather than a mystery. There are three boostvoronoi call sites in `crates/slicer-core`:

| Call site | Guard |
| --- | --- |
| `medial_axis.rs` | wraps the builder in `std::panic::catch_unwind(AssertUnwindSafe(...))`; its comment explicitly names `assertion failed: fpv.is_finite()` at `robust_fpt.rs` |
| `algos/paint_segmentation/voronoi_graph.rs` | wraps the builder in `catch_unwind` the same way |
| **`voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`)** | **no `catch_unwind`** — only `map_err(map_bv_error)` on the returned `Result` |

A `robust_fpt` failure is an `assert!` **panic**, not a `Result::Err`, so `map_err` and the `?` operator cannot observe it. The skeletal/Arachne path — `voronoi_from_segments` ← `SkeletalTrapezoidationGraph::from_polygons` ← `run_arachne_pipeline` — is therefore the one boostvoronoi entry point with no backstop. Because per-layer work runs under a rayon `par_iter()` (`crates/slicer-runtime/src/layer_executor.rs`) and `arachne-perimeters` forwards to the host bridge `generate_arachne_walls` which runs `run_arachne_pipeline` natively on the host, the panic executes on a rayon worker: it prints to stderr and unwinds that worker's region, which is exactly the "swallowed background-thread panic" the row describes.

What remains genuinely unknown, and what this packet exists to settle: because `voronoi_from_segments` has no local guard, a panic unwinds `from_polygons` and therefore that region's entire arachne result rather than `?`-returning a clean error. Whether the resulting walls are silently dropped or the panic lands on a discarded/retried path **cannot be decided statically**. The suite passing proves only that the *asserted* geometry was unaffected — not that no geometry was lost.

This is one coherent slice: add the missing guard (which is also the instrumentation), use it to capture the degenerate inputs and measure the output delta, and record a verdict.

## In Scope

- Wrap the boostvoronoi `Builder::build()` call in `voronoi_from_segments` in `std::panic::catch_unwind(AssertUnwindSafe(...))`, matching the existing pattern in `medial_axis.rs` and `algos/paint_segmentation/voronoi_graph.rs`.
- Add a distinct `VoronoiError` variant for a caught builder panic, so the failure becomes a value the caller can observe instead of an unwind.
- Capture, for each caught panic, the offending segment set (count, coordinate bounds, duplicate / zero-length / near-collinear classification) and the owning layer/region identifiers.
- Run the `perimeter_parity` workload and measure whether any wall loops are lost on affected layers/regions relative to the pre-change baseline.
- Add a degenerate-input regression test to the existing `voronoi_stress` binary proving `voronoi_from_segments` returns a `Result` rather than unwinding.
- Write `FINDINGS.md` in this packet directory recording counts, input characterization, and an explicit verdict.
- Update the D-167 row in `docs/DEVIATION_LOG.md` with the verdict.

## Out of Scope

- Implementing the `preprocess_input_outline` pre-snapping hardening (`crates/slicer-core/src/arachne/preprocess.rs`) that a "geometry is lost" verdict would call for. ADR-0023 assigns near-collinear/T-junction/duplicate pre-snapping to the caller, and `voronoi_from_segments`'s own doc comment restates it — but that fix is a separate, larger packet. This packet names the successor; it does not implement it.
- D-154-DISCRETIZE-POINT-POINT-CASE. It shares the graph-construction path and was originally queued alongside this work, but it requires new `is_secondary` plumbing on `HalfEdge` and is queued as its own T3 packet. This packet's verdict gates that packet's design.
- Upgrading, forking, or patching the `boostvoronoi` dependency. The guard is on our side of the boundary.
- Changing `medial_axis.rs` or `algos/paint_segmentation/voronoi_graph.rs` — they already have the guard and are the pattern being copied, not modified.

## Authoritative Docs

- `docs/adr/0023-arachne-port-strategy.md` — direct read; boostvoronoi selection and the caller-pre-snaps degeneracy contract. Not amended by this packet.
- `docs/DEVIATION_LOG.md` — the D-167 row only; large file, never read in full.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (guard + distinct error variant present), `AC-2` (zero raw `is_finite()` panic lines from the `perimeter_parity` workload, pass/fail status unchanged), `AC-3` (`FINDINGS.md` records counts, input characterization, and an explicit verdict), `AC-4` (D-167 row carries the verdict).
- Negative: `AC-N1` (degenerate segment set returns a `Result` instead of unwinding the calling thread).
- Cross-packet impact: the verdict gates the queued T3 D-154 packet's design; if the verdict is "geometry is lost", a successor packet owning `preprocess_input_outline` hardening must be filed and named in the D-167 row.

## Verification Commands

**Copy the runnable commands from `packet.spec.md`, never from this table.** The descriptions below are summaries; markdown table cells require escaping `|` as `\|`, and transcribing an escaped pipe into a ripgrep pattern silently changes an alternation into a literal-pipe match — which is exactly how an earlier draft of AC-4 became unpassable.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -c 'rg -q "catch_unwind" crates/slicer-core/src/voronoi.rs && rg -q "AssertUnwindSafe" crates/slicer-core/src/voronoi.rs && echo PASS \|\| echo FAIL'` | AC-1 guard present | FACT PASS/FAIL |
| `build-guests --check`, then `cargo test -p slicer-runtime --test integration -- perimeter_parity` tee'd to `target/183-parity.log`, then a raw-panic count over that log | AC-2 workload evidence | FACT: suite status + raw-panic count |
| `bash -c 'rg -q "## Caught panic count" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && rg -q "## Input characterization" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && rg -q "## Verdict" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && echo PASS \|\| echo FAIL'` | AC-3 findings artifact | FACT PASS/FAIL |
| `rg -q` over `docs/DEVIATION_LOG.md` for the D-167 row with a Status cell beginning `Closed` or `Open — narrowed`, anchored to the row's final column | AC-4 verdict recorded | FACT PASS/FAIL |
| `cargo test -p slicer-core --features host-algos --test voronoi_stress -- voronoi_from_segments_degenerate_input_returns_result_not_panic --exact 2>&1 \| tail -20` | AC-N1 no unwind | FACT pass/fail |
| `cargo check --workspace --all-targets` | Compilation gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |

## Step Completion Expectations

The baseline for AC-2 must be captured **before** the guard lands — the pre-change `perimeter_parity` pass/fail status and raw-panic count are the comparison basis, and they cannot be reconstructed after the guard converts panics into errors. The verdict in `FINDINGS.md` and the D-167 row text must agree; if the verdict is "geometry is lost", both must name the same successor deviation id, re-derived from the log at the moment of writing rather than assumed.

## Context Discipline Notes

`crates/slicer-core/src/skeletal_trapezoidation/graph.rs` is 1715 lines — never read it in full; this packet needs only the `voronoi_from_segments` call site inside `from_polygons`, located by `rg`. `docs/DEVIATION_LOG.md` is large; read only the D-167 row. The two already-guarded call sites (`medial_axis.rs`, `algos/paint_segmentation/voronoi_graph.rs`) are read-only pattern references — copy the guard shape, do not load them wholesale.
