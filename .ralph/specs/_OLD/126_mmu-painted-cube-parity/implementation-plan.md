# Implementation Plan — Packet 126: MMU Painted-Cube OrcaSlicer Parity

## Execution Rules

- One step per session where possible; hand off at 85% context. Each step ends with its falsifying check green before the next begins.
- All slicing evidence uses the curated classic-only module dir (Step 2). A run loading `arachne-perimeters` is invalid.
- Delegate every cargo run and OrcaSlicer read (see `requirements.md` obligations). Never load `OrcaSlicerDocumented/` directly.
- After editing any guest-feeding path, run `cargo xtask build-guests --check` before blaming a guest/dispatch test.
- Tee test output to `target/test-output.log`; grep it, do not re-run to re-read.

## Steps

### Step 1 — Land the prerequisite session work
- Task ids: TASK-246, DEV-009. Objective: commit the corner-displacement removal (`voronoi_graph.rs` Step 9/10 deleted + dead `bv_pre_merge_segments` field removed), the corrected confinement-test predicates (`regions_with_edge_on_face`), and the 4-colour-square corner-share regression guard — all currently uncommitted in the working tree.
- Precondition: working tree has the three modified files; `git stash@{0}` holds the arc-walk WIP (do NOT pop it yet).
- Postcondition: a commit on `parity/perimeter-generation` containing only those changes; AC-N3 + AC-2 green on the cell-shortcut path.
- Files to read: none new. Files to edit: none (commit only).
- Dispatches: `cargo test -p slicer-runtime --test executor -- cube_4color cube_fuzzy` → FACT; `cargo test -p slicer-core --lib --features host-algos -- voronoi extract paint_segmentation` → FACT.
- Context cost: S.
- Verification: `git status --short` clean after commit; both test FACTs `0 failed`.
- Falsifying check: any confinement test fails → the predicate correction regressed; stop.

### Step 2 — Curated module dir + baselines (read-only setup)
- Task ids: TASK-245. Objective: build `scratchpad/modules-classic` (all `modules/core-modules/*` except `arachne-perimeters`; copy each module's non-Cargo `*.toml` + `*.wasm`), then record pre-fix baselines.
- Precondition: `cargo build --bin pnp_cli --release` succeeds.
- Postcondition: curated dir exists (arachne excluded); baseline recorded: `PNP_PAINTSEG_CELL_TILING_DEBUG=1` slice reports the current >1% layer count (expected 21 on the post-Step-1 cell-shortcut path).
- Files to read: none. Files to edit: none (scratchpad only).
- Dispatches: build → FACT; slice + `grep -c cell-tiling` → FACT integer.
- Context cost: S.
- Verification: `ls scratchpad/modules-classic | grep -c arachne` → `0`; slice exits 0.
- Falsifying check: arachne present in curated dir → wall-coverage evidence invalid.

### Step 3 — Arc-walk port: graph parity + `get_next_arc`
- Task ids: TASK-245, TASK-246. Objective: confirm `from_colored_lines` builds BORDER/NON_BORDER arcs as Orca's `build_graph` expects; fix `get_next_arc` to (a) exclude different-colour BORDER arcs (Orca 401-405), (b) use the leftmost-angle convention (acos+cross2 over reverse-travel, 430-447), (c) orient candidate direction by traversal via `dir_from_node`, not the stored `point_a→point_b`.
- Precondition: Step 1 committed. Resume edits from `scratchpad/wip_extract_segments_arcwalk.rs` / `git stash@{0}`.
- Postcondition: `extract_segments.rs` unit tests pass (square + two-colour walks separate at colour change); graph-parity dispatch confirms construction matches.
- Files to read: `scratchpad/wip_extract_segments_arcwalk.rs`, `voronoi_graph.rs::from_colored_lines` (±40), `extract_segments.rs`. Files to edit: `extract_segments.rs` (+ `voronoi_graph.rs::from_colored_lines` only if parity gap found).
- Dispatches: graph-construction parity SUMMARY; Orca angle-convention SNIPPETS (≤30 lines); `cargo test -p slicer-core --lib --features host-algos -- extract` → FACT.
- Context cost: M.
- Verification: `cargo test -p slicer-core --lib --features host-algos -- extract 2>&1 | grep "test result"` → `0 failed`.
- Falsifying check: `extract_two_color_walk_separates_at_color_change` fails → angle/colour logic still wrong; fix before Step 4.

### Step 4 — Arc-walk port: seed-colour emission + make it live; retire cell shortcut
- Task ids: TASK-245, TASK-246. Objective: fix `segments_to_expolygons_by_color` to emit one polygon per walk under its seed colour; swap `execute_paint_segmentation`'s decompose call to `extract_colored_segments` + `segments_to_expolygons_by_color`; delete the now-dead `cells_to_expolygons_by_color` and its call site. Add `cube_4color_left_face_circles_tile_without_gap` (segmentation-level: no residual polygon between adjacent colour regions at z=4/z=18).
- Precondition: Step 3 green.
- Postcondition: AC-1 (cell-tiling count 0), AC-2 (4 confinement tests), AC-3 (seed-colour), AC-4 (circle tiling), AC-N2 (unpainted unchanged), AC-N3 all green.
- Files to read: `mod.rs::{execute_paint_segmentation,segments_to_expolygons_by_color}` (±40). Files to edit: `mod.rs`, `voronoi_graph.rs` (delete shortcut), `crates/slicer-core/.../<new test>`.
- Dispatches: cargo test FACTs for AC-1/AC-2/AC-3/AC-4/AC-N2/AC-N3; slice + `grep -c cell-tiling` → FACT 0.
- Context cost: M.
- Verification: AC-1 prints `0`; AC-2 `4 passed`; AC-3/AC-4 `0 failed`.
- Falsifying check: any confinement test bleeds (foreign colour edge on back/right) → walk geometry still wrong; do not proceed to gap steps.

### Step 5 — G1: top/bottom diagonal-seam sliver
- Task ids: DEV-009. Objective: eliminate the green/red sliver on the top face and green/red on the bottom face along the two-triangle diagonal seam (fix `propagate_top_bottom` union/opening + the Phase-7 precedence diff so side regions fully cover the seam).
- Precondition: Step 4 green (decomposition is faithful).
- Postcondition: AC-G1 — top tool set ⊆ {0,3}, bottom ⊆ {0,2}.
- Files to read: `top_bottom.rs::propagate_top_bottom` (±40), `mod.rs` Phase-7 merge (±40). Files to edit: `top_bottom.rs`, `mod.rs`.
- Dispatches: `cargo test -p slicer-runtime --test executor -- cube_4color_top_face cube_4color_bottom_face` → FACT.
- Context cost: M.
- Verification: AC-G1 `0 failed`.
- Falsifying check: top still shows ToolIndex(1) → seam not covered.

### Step 6 — G2: spurious inner walls on detail faces
- Task ids: DEV-009. Objective: bring inner-wall segment count within target (≤3600) by removing duplicate/spurious painted lines (facet vs stroke) feeding the detail-face decomposition.
- Precondition: Step 4 green (re-measure first — count may already drop).
- Postcondition: AC-G2 — `grep -c ";TYPE:Inner wall"` ≤ 3600.
- Files to read: `painted_line_collection.rs` (±40), `colorize.rs::post_process_painted_lines` (±40). Files to edit: `painted_line_collection.rs` and/or `colorize.rs` (≤2).
- Dispatches: slice + `grep -c ";TYPE:Inner wall"` → FACT integer; re-measure baseline first.
- Context cost: M.
- Verification: AC-G2 count ≤ 3600.
- Falsifying check: count unchanged/higher → wrong source of duplication.

### Step 7 — G3: first-layer / bottom-shell perimeter colour
- Task ids: DEV-009. Objective: first-layer side-face walls inherit the bottom-surface colour (orange present, not 100% green). Add `cube_4color_first_layer_perimeter_colour_matches_bottom_face`.
- Precondition: Step 4 green. Resolve the `[FWD]` placement question via the G3 Orca dispatch first.
- Postcondition: AC-G3 — first-layer outer-wall material set contains ToolIndex(0) and is not {1} only.
- Files to read: `painted_line_collection.rs` first-layer handling (±40), `mod.rs` bottom projection (±40). Files to edit: `painted_line_collection.rs` or `mod.rs` (≤2) + new test.
- Dispatches: G3 bottom-shell colouring SUMMARY (Orca); `cargo test ... cube_4color_first_layer_perimeter_colour_matches_bottom_face` → FACT.
- Context cost: M.
- Verification: AC-G3 `1 passed`.
- Falsifying check: first layer still 100% green → inheritance not wired.

### Step 8 — G7 + G8: default extruder + subdivided horizontal face
- Task ids: DEV-009. Objective: (G7) read the object base extruder from 3MF and project unpainted facets as `ToolIndex(extruder−1)`; (G8) skip the tool-0 default projection for `facet_values=None` facets that carry strokes. Add the two unit tests.
- Precondition: Step 4 green.
- Postcondition: AC-G7 + AC-G8 green.
- Files to read: `mod.rs` painted_subsets `None` arm (±40), `loader.rs` extruder metadata (±40). Files to edit: `loader.rs`, `mod.rs` (≤2) + tests.
- Dispatches: `cargo test -p slicer-core --lib --features host-algos -- default_face_colour_uses_object_base_extruder subdivided_horizontal_face_skips_default_tool0_projection` → FACT. If `loader.rs`/IR feeds guests, run `build-guests --check`.
- Context cost: M.
- Verification: both new tests `1 passed`.
- Falsifying check: unpainted facet still tool 0 when base extruder ≠ 1.

### Step 9 — Verify already-landed fixes (G4/G5/G6/RC1/RC2/RC3)
- Task ids: TASK-245, TASK-246. Objective: confirm the commit-`17bb59bd`/S1/S2 fixes still hold after Steps 3–8 (decomposition change must not regress flow/infill/palette).
- Precondition: Steps 4–8 green.
- Postcondition: AC-V-G4 (>0 internal solid infill), AC-V-G5 (flow test), AC-V-G6 (shell inset test), AC-V-RC1 (palette CSV present), AC-V-RC3 (4 tools) all green.
- Files to read: none (verification). Files to edit: none unless a regression is found.
- Dispatches: the AC-V-* commands → FACT each.
- Context cost: S.
- Verification: all AC-V-* pass.
- Falsifying check: any AC-V regresses → a Step 3–8 change broke a landed fix; bisect.

### Step 10 — Doc sync + packet completion gate
- Task ids: TASK-245, TASK-246, DEV-009. Objective: update `docs/07_implementation_status.md` (record arc-walk port + G1–G8/RC1–RC5 closure; note cell-shortcut + corner-displacement retired); add closure/deviation note cross-referencing ADR-0013; mark packet `96`'s `external_contour` line as superseded if not already.
- Precondition: Steps 1–9 green.
- Postcondition: docs updated; full gate green.
- Files to read: `docs/07` relevant section (delegate). Files to edit: `docs/07_implementation_status.md` (+ `docs/DEVIATION_LOG.md` if a deviation is recorded).
- Dispatches: `cargo check --workspace --all-targets` → FACT; `cargo clippy --workspace --all-targets -- -D warnings` → FACT.
- Context cost: S.
- Verification: clippy clean; check clean.
- Falsifying check: clippy warning → not shippable.

## Per-Step Budget Roll-Up

S: Steps 1, 2, 9, 10. M: Steps 3, 4, 5, 6, 7, 8. No L step. Aggregate across the packet is large (6 workstreams) — land iteratively, one step per session, handing off at 85%.

## Packet Completion Gate

All ACs in `packet.spec.md` green (AC-1…AC-4, AC-G1/G2/G3/G7/G8, AC-V-G4/G5/G6/RC1/RC3, AC-N1/N2/N3); `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` clean; `docs/07` updated. A single workspace-wide `cargo test --workspace` (dispatched to a sub-agent returning FACT pass/fail) is run ONCE at the acceptance ceremony, only after every narrower AC command above has passed.

## Acceptance Ceremony

Dispatch the workspace test suite to a sub-agent (FACT pass/fail + any failing test names). On green, hand the diff to `spec-review` / `review` against this packet for the post-completion review the user requested. Record the final SHA in `docs/07`.
