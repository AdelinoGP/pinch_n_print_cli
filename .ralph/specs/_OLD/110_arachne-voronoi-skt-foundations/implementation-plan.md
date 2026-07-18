# Implementation Plan: 110_arachne-voronoi-skt-foundations

## Step Order Rationale

ADR (Step 1) → boostvoronoi wrapper (Step 2) → SKT graph (Step 3) → parabolic discretize (Step 4) → 9-stage + per-color preprocess (Step 5) → module skeleton (Step 6) → docs (Step 7).

The ordering follows the data dependency chain: Voronoi output is required to construct the SKT graph; discretization converts the half-edge graph's curved edges to line segments (consumed by graph construction); the 9-stage preprocess feeds the polygons that downstream wire-up sends INTO `voronoi_from_segments` (P112); the module skeleton MUST come after the slicer-core additions because the new module's `Cargo.toml` depends on `slicer-core`. Docs are last because they reference paths added by earlier steps.

The packet does NOT activate any new IR fields, so schema versioning is untouched. T-224 (Phase 12 / P112) is what bumps IR.

## Step 1 — Write ADR-0023 + Update D-7 Row in Roadmap

- **Tasks:** T-200.
- **Objective:** Draft `docs/adr/0023-arachne-port-strategy.md` recording boostvoronoi as the Voronoi crate, pure-Rust constraint, degeneracy strategy, and pinned version (v0.12, already in `slicer-core/Cargo.toml`). Edit `docs/specs/perimeter-modules-orca-parity-roadmap.md` D-7 row to reference ADR-0023. D-7 is ALREADY marked CLOSED in the roadmap (line 92). Do NOT add D-7 to `docs/14_deviation_audit_history.md` — D-7 lives only in the roadmap and that file has no D-7 entry (verified).
- **Precondition:** None.
- **Postcondition:** `docs/adr/0023-arachne-port-strategy.md` exists; ADR records boostvoronoi version + license note (BSL-1.0) + `epsilon_offset` hazard; D-7 row in roadmap references ADR-0023.
- **Files allowed to read:** `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` (format template — the `0009-perimeter-module-scope.md` ADR does NOT exist in tree; use 0013 instead), `docs/08_coordinate_system.md` (full), `docs/01_system_architecture.md` (§slicer-core), `docs/specs/perimeter-modules-orca-parity-roadmap.md` (D-7 row context).
- **Files allowed to edit:** `docs/adr/0023-arachne-port-strategy.md` (NEW), `docs/specs/perimeter-modules-orca-parity-roadmap.md` (EDIT D-7 row to reference ADR-0023).
- **Expected sub-agent dispatches:** ONE WebFetch SUMMARY of https://docs.rs/boostvoronoi/ — return ≤ 200 words on `VoronoiBuilder` + `VoronoiDiagram` API + the latest 0.x version number.
- **Context cost:** S.
- **Authoritative docs:** `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` (ADR format template).
- **OrcaSlicer refs:** None this step.
- **Narrow verification:** `test -f docs/adr/0023-arachne-port-strategy.md && rg -q 'boostvoronoi' docs/adr/0023-arachne-port-strategy.md && rg -q 'epsilon_offset' docs/adr/0023-arachne-port-strategy.md`.
- **Cheapest falsifying check:** `test -f docs/adr/0023-arachne-port-strategy.md && rg -q 'epsilon_offset' docs/adr/0023-arachne-port-strategy.md` returns success.

## Step 2 — Voronoi Wrapper (T-201)

- **Tasks:** T-201.
- **Objective:** Extend `boostvoronoi` from optional (`host-algos` feature, v0.12) to always-on (or widen the feature gate); create `crates/slicer-core/src/voronoi.rs` with `voronoi_from_segments`, `Segment`, `HalfEdgeGraph`, `VoronoiError`; write 3 stress fixture tests + 1 negative test. Do NOT add a new boostvoronoi dep — it already exists at v0.12 in `crates/slicer-core/Cargo.toml:16`.
- **Precondition:** Step 1 done (ADR documents version).
- **Postcondition:** `cargo test -p slicer-core voronoi` green; AC-2 + AC-3 + AC-N1 falsifiable; `cargo check --workspace --all-targets` green.
- **Files allowed to read:** `docs/adr/0023-arachne-port-strategy.md` (Step 1 output), `crates/slicer-core/src/lib.rs` (current `pub mod` set), `crates/slicer-core/src/geometry.rs` (to confirm it imports `slicer_ir::Point2` — NOT defining its own), `crates/slicer-ir/src/slice_ir.rs` (lines 80-95 — `Point2` struct definition). NOTE: Use `slicer_ir::Point2` in `voronoi.rs` — do NOT define a new `Point2`.
- **Files allowed to edit:** `crates/slicer-core/Cargo.toml`, `crates/slicer-core/src/lib.rs`, `crates/slicer-core/src/voronoi.rs` (NEW), `crates/slicer-core/tests/voronoi_stress.rs` (NEW), `crates/slicer-core/tests/fixtures/voronoi/` (NEW JSON goldens).
- **Expected sub-agent dispatches:** ONE `cargo test -p slicer-core voronoi 2>&1 | tee target/test-output.log` — return FACT pass/fail + failing assertion ≤ 20 lines.
- **Context cost:** M.
- **Authoritative docs:** `docs/adr/0023-arachne-port-strategy.md` (Step 1 output), https://docs.rs/boostvoronoi/.
- **OrcaSlicer refs:** None this step (wrapper is boostvoronoi-shaped, not OrcaSlicer-shaped).
- **Narrow verification:** `cargo test -p slicer-core --features host-algos voronoi_square_four_segments 2>&1 | tee target/test-output.log` + `cargo test -p slicer-core --features host-algos voronoi_stress 2>&1 | tee target/test-output.log` + `cargo test -p slicer-core --features host-algos voronoi_empty_input_returns_err 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** AC-N1 first — run `voronoi_empty_input_returns_err`. If it panics (i.e., wrapper forwards empty input to boostvoronoi which panics), the error variant isn't being checked at the API boundary.

## Step 3 — Skeletal Trapezoidation Graph (T-202)

- **Tasks:** T-202.
- **Objective:** Create `crates/slicer-core/src/skeletal_trapezoidation/{mod.rs, graph.rs}`; port `SkeletalTrapezoidationGraph` (half-edge with `r_min`, `r_max`, `central` fields); write square + wedge golden fixture tests.
- **Precondition:** Step 2 done (Voronoi wrapper provides `HalfEdgeGraph` input).
- **Postcondition:** `cargo test -p slicer-core skt_graph` green; AC-4 falsifiable; `cargo check --workspace --all-targets` green.
- **Files allowed to read:** `crates/slicer-core/src/voronoi.rs` (Step 2 output), `crates/slicer-core/src/lib.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/lib.rs`, `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` (NEW), `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (NEW), `crates/slicer-core/tests/skt_graph_golden.rs` (NEW), `crates/slicer-core/tests/fixtures/skt/` (NEW JSON goldens).
- **Expected sub-agent dispatches:** ONE OrcaSlicer LOCATIONS dispatch for `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` — return ≤ 20 entries naming the half-edge struct fields + the `r_min`/`r_max`/`central` field definitions + the graph construction entry function. ONE `cargo test -p slicer-core skt_graph` — return FACT pass/fail.
- **Context cost:** M.
- **Authoritative docs:** OrcaSlicer SkeletalTrapezoidation.cpp half-edge layout (via dispatch).
- **OrcaSlicer refs:** delegated per OrcaSlicer Reference Obligations.
- **Narrow verification:** `cargo test -p slicer-core --features host-algos skt_graph_square_and_wedge 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** `rg -n 'r_min' crates/slicer-core/src/skeletal_trapezoidation/graph.rs && rg -n 'r_max' crates/slicer-core/src/skeletal_trapezoidation/graph.rs && rg -n 'central' crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — three matches expected.

## Step 4 — Parabolic Edge Discretization (T-203)

- **Tasks:** T-203.
- **Objective:** Create `crates/slicer-core/src/skeletal_trapezoidation/discretize.rs` with `discretize_parabolic_edge(focus, line_a, line_b, max_segment_len) -> Vec<Point2>`; write golden-fixture test comparing against an OrcaSlicer-discretized reference within 0.005 mm Hausdorff.
- **Precondition:** Step 3 done (SKT graph carries `is_curved` flag and parabolic edges to discretize).
- **Postcondition:** `cargo test -p slicer-core parabolic_discretize` green; AC-5 falsifiable.
- **Files allowed to read:** `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (Step 3 output), `docs/08_coordinate_system.md`.
- **Files allowed to edit:** `crates/slicer-core/src/skeletal_trapezoidation/discretize.rs` (NEW), `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` (add `pub mod discretize;`), `crates/slicer-core/tests/parabolic_discretize.rs` (NEW), `crates/slicer-core/tests/fixtures/skt/parabolic_*.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer LOCATIONS dispatch for `discretize_parabolic_edge` in `SkeletalTrapezoidation.cpp` — return ≤ 10 entries: function signature + tessellation constants.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer parabolic discretization math (via dispatch).
- **OrcaSlicer refs:** delegated.
- **Narrow verification:** `cargo test -p slicer-core --features host-algos parabolic_discretize_matches_orca 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** Hausdorff distance between output polyline and recorded OrcaSlicer reference ≤ 0.005 mm (50 units).

## Step 5 — 9-Stage Preprocess + T-P96-E Per-Color MMU Dedup (T-204, T-P96-E)

- **Tasks:** T-204, T-P96-E.
- **Objective:** Create `crates/slicer-core/src/arachne/{mod.rs, preprocess.rs}` with `preprocess_input_outline` (9 stages from `WallToolPaths.cpp:590-604`) and `preprocess_per_color_inputs` (T-P96-E — boundary-level MMU dedup per ADR-0013 tie-break); write 3 golden fixture tests.
- **Precondition:** Step 4 done. Step 5 has the heaviest content; the implementer reaches Step 5 with ≤ 50% context budget remaining (per Context Discipline §60% cap).
- **Postcondition:** `cargo test -p slicer-core preprocess` green; AC-6 + AC-7 + AC-N3 falsifiable; the hazard string `destroys features < epsilon_offset ~11.5 µm` appears verbatim in `preprocess_input_outline`'s doc-comment.
- **Files allowed to read:** `crates/slicer-core/src/polygon_ops.rs` (existing, from P103/T-040) — for `offset2_ex`, `opening_ex`; `crates/slicer-core/src/polygon_tree.rs` (P103/T-043) — for containment; `docs/specs/orca-mmu-perimeter-investigation.md` (full ≤ 200 lines, from P105/T-P96-A0, `implemented`) — PRESENT in tree; substitute a `MultiMaterialSegmentation.cpp` LOCATIONS dispatch (≤ 15 entries) only if a needed citation is missing from the one-pager.
- **Files allowed to edit:** `crates/slicer-core/src/lib.rs` (`pub mod arachne;`), `crates/slicer-core/src/arachne/mod.rs` (NEW), `crates/slicer-core/src/arachne/preprocess.rs` (NEW), `crates/slicer-core/tests/preprocess_golden.rs` (NEW), `crates/slicer-core/tests/fixtures/arachne_preprocess/` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `WallToolPaths.cpp:590-604` — return ≤ 150 words listing each stage's offset distance + simplify epsilon. ONE FACT dispatch on `docs/specs/orca-mmu-perimeter-investigation.md` — return ≤ 5 lines: tie-break rule (1 sentence) + Arachne MMU citation (file:line). ONE `cargo test -p slicer-core preprocess` — return FACT pass/fail.
- **Context cost:** M.
- **Authoritative docs:** OrcaSlicer 9-stage list (via SUMMARY); `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md`.
- **OrcaSlicer refs:** delegated.
- **Narrow verification:** `cargo test -p slicer-core preprocess_nine_stage_pipeline 2>&1 | tee target/test-output.log && cargo test -p slicer-core preprocess_per_color_mmu_dedup 2>&1 | tee target/test-output.log && cargo test -p slicer-core preprocess_drops_tiny_features_with_warn 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** `rg -q 'destroys features < epsilon_offset ~11\.5 µm' crates/slicer-core/src/arachne/preprocess.rs`. If the hazard string is missing, AC-6 fails — fix at write time.

## Step 6 — `arachne-perimeters` Skeleton Creation (T-205) + DAG Validation (AC-N2)

- **Tasks:** T-205 + AC-N2 test.
- **Objective:** CREATE the NEW `modules/core-modules/arachne-perimeters/` skeleton (FORWARD-DEP on P108 deletion — must confirm `! test -d modules/core-modules/arachne-perimeters` before creating). Create: manifest `arachne-perimeters.toml` with `id = "com.core.arachne-perimeters"`, `holds = ["perimeter-generator"]`, `incompatible-with = ["com.core.classic-perimeters"]` (only); `src/lib.rs` with empty `LayerModule` impl (returns `Ok(())` + `warn!`); `Cargo.toml`. Add `"modules/core-modules/arachne-perimeters"` as workspace member in root `Cargo.toml`. Add `dag_rejects_arachne_and_classic_coexistence` test to the EXISTING `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs` (NOT a new file, NOT `tests/contract/`).
- **Precondition:** Steps 1–5 done. `cargo xtask build-guests --check` reports CLEAN before this step. P108 is `status: implemented` and `! test -d modules/core-modules/arachne-perimeters` is true.
- **Postcondition:** `cargo xtask build-guests --check` CLEAN after skeleton creation; `cargo test -p slicer-runtime --test unit dag_rejects_arachne_and_classic_coexistence` green; AC-8 + AC-N2 falsifiable.
- **Files allowed to read:** `modules/core-modules/classic-perimeters/classic-perimeters.toml` (manifest template), `modules/core-modules/classic-perimeters/src/lib.rs` lines 1–50 (`#[slicer_module]` invocation pattern), `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs` (existing tests — to see how tests are structured before appending), `crates/slicer-runtime/tests/unit/main.rs` (confirm `mod dag_validation_tdd;` at line 15).
- **Files allowed to edit (new + existing):**
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (NEW)
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (NEW)
  - `modules/core-modules/arachne-perimeters/Cargo.toml` (NEW)
  - Root `Cargo.toml` (EDIT — add workspace member entry)
  - `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs` (EDIT — append AC-N2 test function)
- **Expected sub-agent dispatches:** ONE `cargo xtask build-guests --check` after skeleton creation. ONE `cargo test -p slicer-runtime --test unit dag_rejects_arachne_and_classic_coexistence` — return FACT pass/fail.
- **Context cost:** M.
- **Authoritative docs:** `docs/03_wit_and_manifest.md` §"incompatible-with"; `docs/05_module_sdk.md` §"#[slicer_module]".
- **OrcaSlicer refs:** None.
- **Narrow verification:** `cargo xtask build-guests --check 2>&1 | tee target/test-output.log && cargo test -p slicer-runtime --test unit dag_rejects_arachne_and_classic_coexistence 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** `rg -q '"com\.core\.classic-perimeters"' modules/core-modules/arachne-perimeters/arachne-perimeters.toml && ! rg -q 'variable.width' modules/core-modules/arachne-perimeters/arachne-perimeters.toml`.

## Step 7 — Docs Update + Roadmap Flip

- **Tasks:** doc impact (no new T-NNN).
- **Objective:** Register the three new `slicer-core` sub-modules in `docs/01_system_architecture.md`; flip T-200/T-201/T-202/T-203/T-204/T-205/T-P96-E rows to DONE in `docs/specs/perimeter-modules-orca-parity-roadmap.md`.
- **Precondition:** Steps 1–6 done. `cargo test -p slicer-core` green. `cargo xtask build-guests --check` CLEAN.
- **Postcondition:** Doc Impact Statement assertions all return true.
- **Files allowed to read:** `docs/01_system_architecture.md` (§slicer-core section), `docs/specs/perimeter-modules-orca-parity-roadmap.md` (Phase 10 rows + Inherited-from-P96).
- **Files allowed to edit:** `docs/01_system_architecture.md`, `docs/specs/perimeter-modules-orca-parity-roadmap.md`.
- **Expected sub-agent dispatches:** None.
- **Context cost:** S.
- **Authoritative docs:** the two doc files themselves.
- **OrcaSlicer refs:** None.
- **Narrow verification:** `rg -q 'voronoi' docs/01_system_architecture.md && rg -q 'skeletal_trapezoidation' docs/01_system_architecture.md && rg -q 'arachne::preprocess' docs/01_system_architecture.md && rg -q 'T-200.*DONE' docs/specs/perimeter-modules-orca-parity-roadmap.md`.

## Packet Completion Gate

- All 8 ACs + 3 AC-Ns pass per their pipe-suffix commands.
- `cargo check --workspace --all-targets` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo xtask build-guests --check` CLEAN.
- `cargo test -p slicer-core 2>&1 | tee target/test-output.log` shows all new tests passing.
- `cargo test -p slicer-runtime --test unit dag_rejects_arachne_and_classic_coexistence 2>&1 | tee target/test-output.log` green. (S7 FIX: `--test unit`, not `--test contract`.)
- Doc Impact Statement assertions verified by `rg` checks.
- Closure log (`.ralph/specs/110_arachne-voronoi-skt-foundations/closure-log.md`) authored before status flip; records: chosen boostvoronoi version, any deviations from the 9-stage preprocess (e.g., a constant that didn't translate cleanly through /100), and any residual open questions for P111.
- Status flipped to `implemented` in `packet.spec.md` YAML frontmatter.

## Context Budget Cap

Aggregate cost: M. If the implementer reaches 60% context at any step, they STOP, write the partial-state to closure-log.md, and hand off — the remaining steps inherit. Step 5 is the most likely overflow point because it carries the heaviest sub-agent dispatch (9-stage preprocess SUMMARY) AND two new function bodies. If Step 5 overflows, the implementer SHOULD split T-P96-E into a follow-on packet (P110b) and ship T-204 alone first.
