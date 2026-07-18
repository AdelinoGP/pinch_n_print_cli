# Implementation Plan: 145-arachne-local-maxima-and-construction-epilogue

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`none` — provenanced by the audit + red tests at `b2ea52b7`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.

## Steps

### Step 1: `generateLocalMaximaSingleBeads` (N9)

- Task IDs:
  - `none` (N9 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N9)
- Objective: Port `generateLocalMaximaSingleBeads` (`SkeletalTrapezoidation.cpp:2383-2413`) as `generate_local_maxima_single_beads` in `generate_toolpaths.rs`, called as the final step of `generate_toolpaths` after A2's `connectJunctions` emission. For nodes with odd `beading.bead_widths.size()`, `isLocalMaximum(true)`, and not central, emit a 6-segment hexagonal micro-loop (radius `width/8`, `is_odd = true`). **`is_local_maximum` reuse:** `centrality.rs:269` already defines `pub(super) fn is_local_maximum` (wired into `bead_count.rs:169` by commit `79f2a8f0`). Step 1 widens it to `pub(crate)` and calls it from `generate_toolpaths.rs` — do NOT add a second definition. The swarm's OrcaSlicer delegation must confirm whether canonical's `isLocalMaximum(true)` (`strict=true`) matches PNP's no-argument version (which uses `>` strictly-higher) before assuming reuse is safe.
- Precondition: `141` (A1), `142` (A2), `143` (B), and `144` (C) are all
  `status: implemented` — D's `generateLocalMaximaSingleBeads` runs after A1's
  canonical junction generation and A2's `connectJunctions` emission, reads
  bead-width state B's transition/rib passes shape, and reads the normalized
  centrality C's `filterNoncentralRegions` + configured angle produce. This
  step's own gate check below (N1/N2/N4/N3 stay green) already presumes A1,
  A2, and B are implemented — the precondition text must say so explicitly,
  not just name C.
- Postcondition: AC-1 passes (hexagonal micro-loop at local maximum). N1, N2, N3, N4 stay GREEN. N10 epilogue NOT yet in place (Step 2 owns it).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — range-read the end of `generate_toolpaths` (where `generate_local_maxima_single_beads` is appended) + the `Beading`/`ExtrusionLine` emission patterns A1/A2 use.
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — range-read `:100-200` (`updateIsCentral` predicate convention — `is_local_maximum` may mirror this); read-only.
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — range-read the `STVertex` struct def + the `distance_to_boundary` field; `is_local_maximum` reads this.
  - `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full (202 lines); the `run_arachne_pipeline` + `inset0_lines` helper pattern D's test mirrors.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs`
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (for `is_local_maximum` if placed here; alternatively `centrality.rs`)
  - `crates/slicer-core/tests/arachne_local_maxima_single_beads.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs::from_polygons` (Step 2's epilogue scope)
  - `crates/slicer-core/src/arachne/pipeline.rs` (A1/A2/B/C's scope)
  - `crates/slicer-core/src/beading/*` (B's scope)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:2383-2413` `generateLocalMaximaSingleBeads` — explicitly ask for the hexagonal micro-loop geometry (6 segments, radius `width/8`, `is_odd = true`) + the `isLocalMaximum(true)` + not-central + odd-bead-count conditions; return ≤ 200 words, no code unless asked" — purpose: confirm Step 1's emission.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_local_maxima_single_beads --nocapture`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — N1/N2/N4/N3 stay green)" — purpose: gate D didn't regress A1/A2/B.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"Arachne extrusion-line geometry" (lines ~1091-1150) — `ExtrusionLine::is_odd`.
  - `docs/08_coordinate_system.md` §"Constant Conversion Table" — `width/8` conversion.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2383-2413` — delegate.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_local_maxima_single_beads --nocapture 2>&1 | tee target/test-output-d-step1-ac1.log` — FACT pass (AC-1).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-d-step1-stays-green.log` — FACT pass.
  - `cargo check -p slicer-core --all-targets` — FACT pass.
- Exit condition: AC-1 passes; N1/N2/N3/N4 stay green; `cargo check -p slicer-core --all-targets` passes.

### Step 2: Construction epilogue (N10) + fixture re-baseline + deviation log

- Task IDs:
  - `none` (N10 — provenanced by §N10)
- Objective: Port the `constructFromPolygons` epilogue (`SkeletalTrapezoidation.cpp:538-546`) as two additive passes appended to `from_polygons` in `graph.rs`: `separate_pointy_quad_end_nodes` (duplicate shared boundary start-nodes; skip the `incident_edge` SET line), `collapse_small_edges` (remove degenerate zero-length edges; skip the `incident_edge` SET/READ lines). **Incident-edge normalization is a documented no-op** — PNP's `STVertex` has no `incident_edge` field (confirmed by OrcaSlicer ground-truth as a fan-walk optimization, not correctness; PNP's all-edges scans produce the same results). Add a comment in `from_polygons` explaining the skip. Re-baseline `centrality_*.json` + `toolpaths_tapered_wedge.json`. Add `D-145-LOCAL-MAXIMA-EPILOGUE` deviation-log entry + `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` addendum.
- Precondition: Step 1 is green (`generate_local_maxima_single_beads` in place).
- Postcondition: AC-2 passes (no zero-length edges, normalized incident edges, unique quad-start nodes). AC-N1 passes (N1 red tests stay green). `centrality`/`bead_count`/`propagation`/`generate_toolpaths` regression green (fixtures re-baselined). `D-145-LOCAL-MAXIMA-EPILOGUE` present.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — range-read `:306-371` (the current `from_polygons` end at line 371, where the epilogue is appended) + the `STHalfEdge`/`STVertex` struct defs (`:102-164`) + confirm `STVertex` has NO `incident_edge` field (it doesn't — the normalization is a no-op).
  - `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full (202 lines); AC-N1 oracle.
  - `docs/08_coordinate_system.md` §"Constant Conversion Table" (~30 lines) — `collapseSmallEdges`'s ε conversion.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`
  - `crates/slicer-core/tests/arachne_construction_epilogue.rs` (NEW)
  - `docs/DEVIATION_LOG.md` (addendum only — new `D-145-LOCAL-MAXIMA-EPILOGUE` + one-line addendum on `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`; no in-place edits)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` (Step 1's scope — `generate_local_maxima_single_beads`)
  - `crates/slicer-core/src/arachne/pipeline.rs` (A1/A2/B/C's scope)
  - `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` (re-record via self-capture; never read directly)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:538-546` `constructFromPolygons` epilogue — ask for the two-pass order (`separatePointyQuadEndNodes` → `collapseSmallEdges`; incident-edge normalization is a no-op in PNP); return ≤ 200 words" — purpose: confirm Step 2's epilogue.
  - "SUMMARY of `SkeletalTrapezoidationGraph.cpp` `collapseSmallEdges` — ask for the zero-length ε constant + the endpoint-merge rule; return ≤ 200 words" — purpose: confirm `collapse_small_edges`.
  - "SUMMARY of `SkeletalTrapezoidationGraph.cpp` `separatePointyQuadEndNodes` — ask for the node-duplication rule; return ≤ 200 words" — purpose: confirm `separate_pointy_quad_end_nodes`.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_construction_epilogue --nocapture`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-2.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass (expected — AC-N1, N1 stays green)" — purpose: validate AC-N1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — N2/N4/N3 stay green)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test centrality --test bead_count --test propagation --test generate_toolpaths 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/d-cube4color.gcode && cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT + summary line — purpose: record e2e closure delta (record-only)."
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` §"Constant Conversion Table" — `collapseSmallEdges`'s ε.
  - `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — addendum target.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:538-546` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` — delegate (`collapseSmallEdges`/`separatePointyQuadEndNodes`).
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_construction_epilogue --nocapture 2>&1 | tee target/test-output-d-step2-ac2.log` — FACT pass (AC-2).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-d-step2-neg1.log` — FACT pass (AC-N1).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-d-step2-stays-green.log` — FACT pass (N2/N4/N3 stay green).
  - `cargo test -p slicer-core --features host-algos --test centrality --test bead_count --test propagation --test generate_toolpaths 2>&1 | tee target/test-output-d-step2-regression.log` — FACT pass (fixtures re-baselined).
  - `rg -q 'D-145-LOCAL-MAXIMA-EPILOGUE' docs/DEVIATION_LOG.md` — FACT pass.
- Exit condition: AC-2, AC-N1 pass; N1/N2/N3/N4 stay green; regression green (fixtures re-baselined); `D-145-LOCAL-MAXIMA-EPILOGUE` present; `cargo check -p slicer-core --all-targets` + `cargo clippy -p slicer-core --all-targets -- -D warnings` pass; e2e closure delta recorded (record-only).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (N9 generateLocalMaximaSingleBeads) | M | Heaviest dispatch: `generateLocalMaximaSingleBeads` SUMMARY. |
| Step 2 (N10 epilogue + fixtures + deviation log) | M | Heaviest dispatch: 3 OrcaSlicer SUMMARYs + regression suite. |

Aggregate: M + M = M (Step 2 shares Step 1's `generate_toolpaths.rs`/`graph.rs` context). If the sum exceeds M aggregate in practice, hand off after Step 1.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1, AC-2, AC-N1 dispatched and returned PASS).
- N1, N2, N3, N4 stay GREEN.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask build-guests --check` returns clean.
- `D-145-LOCAL-MAXIMA-EPILOGUE` present in `docs/DEVIATION_LOG.md` with addendum on `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`.
- Affected `centrality_*.json` + `toolpaths_tapered_wedge.json` fixtures re-baselined with rationale in commit messages.
- e2e closure delta recorded (record-only — Packet F blocks on green).
- `docs/07_implementation_status.md` updated (via worker dispatch).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1, AC-2, AC-N1).
- Confirm packet-level verification commands are green.
- Confirm N1/N2/N3/N4 "stays green" commands returned as expected.
- Record the e2e closure delta explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.