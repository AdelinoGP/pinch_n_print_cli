# Task Map: 96_paint-segmentation-phase5-width-limit

Bridge from backlog task IDs to packet steps. Source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4" + the inherited P95 deviation `D-95-AC22-BISECTOR-DEDUP`.

## Backlog → Packet

| Backlog Task ID | Source | Packet Coverage |
| --- | --- | --- |
| `TASK-246` | `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4" | Phase 5 erosion + interlocking kernel + integration + tests (Steps 0–3, 4a, 5–9). |
| `TASK-246-BISECTOR` | Inherited from P95 deviation `D-95-AC22-BISECTOR-DEDUP` (recorded in `.ralph/specs/95_paint-segmentation-orca-port/packet.spec.md`) | Bisector-edge ownership mechanism for outer-wall emission (Steps 4b, 4c). Drives the previously-`#[ignore]`d test `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` GREEN (AC-22b). |

## Packet Step → Task ID

| Step | Task IDs | ACs covered |
| --- | --- | --- |
| Step 0 — Baselines | `TASK-246` | AC-8 prerequisite |
| Step 1 — Schema landing + spec summary | `TASK-246` | AC-3 prerequisite |
| Step 2 — Kernel + 3 positive unit tests + 2 negative + 1 short-circuit no-op | `TASK-246` | AC-1, AC-N1, AC-N2 (AC-N3 is driver-level, covered in Step 4a) |
| Step 3 — Config-schema entries | `TASK-246` | AC-3 |
| Step 4a — Integrate `cut_segmented_layers` into driver | `TASK-246` | AC-2, AC-4 |
| Step 4b — Add bisector-edge ownership field to `SlicedRegion` + tag in driver | `TASK-246-BISECTOR` | AC-22b (IR + tag) |
| Step 4c — Consume ownership in classic-perimeters outer-wall emission | `TASK-246-BISECTOR` | AC-22b (emission) |
| Step 5 — 3 integration tests (band width, alternation, beam skip) | `TASK-246` | AC-5, AC-6, AC-7 |
| Step 6 — Regression (SHAs + 21 cube tests: 11 cube_4color + 10 cube_fuzzy_painted) | `TASK-246` + `TASK-246-BISECTOR` | AC-8, AC-10 |
| Step 7 — Visual report capture | `TASK-246` | AC-9 |
| Step 8 — Guest WASM `--check` | `TASK-246` | AC-11 |
| Step 9 — Acceptance ceremony (full per-AC re-dispatch) | both | AC-1 through AC-11, AC-22b, AC-N1–N3 |

## Closure-Log Bindings

On packet closure, the closure log MUST record:

- `TASK-246` complete (with pre/post wedge + cube SHAs matching).
- `TASK-246-BISECTOR` complete (with `#[ignore]` removed from
  `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one`
  and the test driving GREEN — evidence as test-name + `test result: ok` line).
- Update to `docs/07_implementation_status.md` for both task IDs (delegate).
- Doc Impact Statement greps (see `packet.spec.md`) all PASS.
