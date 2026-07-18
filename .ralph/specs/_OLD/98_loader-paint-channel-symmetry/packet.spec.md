---
status: implemented
packet: 98
task_ids: [TASK-248]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: S
---

# Packet 98 — Loader Symmetry: `paint_seam` + `paint_fuzzy_skin` Sub-Facet Stroke Decoding

## Goal

Hoist the existing sub-facet-stroke hex decoder block at `crates/slicer-model-io/src/loader.rs:1237-1295` (currently bound to the `paint_color` Material channel and the `paint_supports` channels) into a private helper `decode_strokes_for_channel(hex: &str, semantic: PaintSemantic, tri_verts: &[Vec3], byte_offset: usize) -> Vec<PaintStroke>` that takes the channel's `PaintSemantic` as a parameter, then call it for all four 3MF paint channels (`paint_color` → `PaintSemantic::Material(ToolIndex)`, `paint_supports` → `PaintSemantic::SupportEnforcer` / `SupportBlocker` per the existing hex sub-decoding, `paint_seam` → `PaintSemantic::SeamEnforcer` / `SeamBlocker`, `paint_fuzzy_skin` → `PaintSemantic::FuzzySkin(Flag)`), so that 3MFs encoding paint at sub-facet granularity via TriangleSelector subdivision in OrcaSlicer get decoded symmetrically across all four channels — not just the two already covered. Sub-facet strokes parsed from any channel are subsequently normalized into `paint_data.layers[*].facet_values` by `host:mesh_segmentation` (P94's wiring); this packet ensures the strokes actually arrive at the kernel for every channel. Add per-channel stroke tests to `crates/slicer-model-io/tests/model_loader_tdd.rs`, exercising the existing cube fixtures (`cube_fuzzyPainted.3mf` carries `paint_fuzzy_skin` strokes; a synthetic single-channel fixture may be authored for `paint_seam` if no current fixture exercises it).

## Scope Boundaries

This is a surgical loader fix: hoist a 60-line block into a parameterized helper, call it four times instead of two. The 3MF format itself is unchanged; the decoder doesn't add new functionality, just symmetrizes existing decoding across the four documented paint channels. No IR change, no WIT change, no scheduler change, no kernel change. Full file-by-file list in `requirements.md`.

## Prerequisites and Blockers

- Depends on: P94 (host:mesh_segmentation wiring) — strokes need a consumer. P94 already integrates with all four channels through the existing `facet_values` field; this packet just feeds it.
- Unblocks: nothing structurally. P5c (99) updates docs after this.
- Activation blockers: P94 closed.

## Acceptance Criteria

### AC-1 — `decode_strokes_for_channel` private helper exists in `crates/slicer-model-io/src/loader.rs`

**Given** the hoist,
**When** the loader is inspected,
**Then** a private function `fn decode_strokes_for_channel(hex: &str, semantic: PaintSemantic, tri_verts: &[Vec3], byte_offset: usize) -> Vec<PaintStroke>` (or equivalent signature accommodating the existing code's types) exists; it implements the same hex sub-decoding logic previously inlined at lines 1237-1295; it parameterizes the `PaintSemantic` mapping per channel.

| `rg -q 'fn decode_strokes_for_channel' crates/slicer-model-io/src/loader.rs`

### AC-2 — All four paint channels call the helper

**Given** the new helper,
**When** the parse-attributes block in `loader.rs` is inspected,
**Then** a call site exists for each of `paint_color`, `paint_supports`, `paint_seam`, `paint_fuzzy_skin`, each with the channel-appropriate per-state semantic mapping. The old inlined decode blocks are gone.

> **AS-BUILT correction (P98):** the packet originally assumed a single parse loop and `-eq 4`. `loader.rs` actually has **two** parse loops (`parse_3mf_model_xml` + `parse_sub_model_objects`); genuine symmetry requires 4 call sites in each = **8 call sites + 1 definition = 9** occurrences. The `-eq 4` form would only pass if one loop were left un-symmetrized (i.e. the bug half-fixed), so it was corrected to `-eq 9`. See closure-log.md.

| `[ $(rg -c 'decode_strokes_for_channel\(' crates/slicer-model-io/src/loader.rs) -eq 9 ]`

### AC-3 — Per-channel stroke test: `paint_color` decodes to `PaintSemantic::Material(ToolIndex)`

**Given** the existing test scenario for paint_color,
**When** a synthetic or existing fixture exercises sub-facet strokes on `paint_color`,
**Then** the parsed `PaintStroke`s carry `PaintSemantic::Material(ToolIndex(N))` for the expected N.

| `cargo test -p slicer-model-io paint_color_subfacet_strokes_decoded 2>&1 | tee target/test-output.log`

### AC-4 — Per-channel stroke test: `paint_supports` decodes to `SupportEnforcer` / `SupportBlocker`

| `cargo test -p slicer-model-io paint_supports_subfacet_strokes_decoded 2>&1 | tee target/test-output.log`

### AC-5 — Per-channel stroke test: `paint_seam` decodes to `SeamEnforcer` / `SeamBlocker`

**Given** a synthetic `paint_seam` fixture (authored if not present),
**When** the loader runs,
**Then** the parsed strokes carry `PaintSemantic::SeamEnforcer` or `::SeamBlocker` per the hex value.

| `cargo test -p slicer-model-io paint_seam_subfacet_strokes_decoded 2>&1 | tee target/test-output.log`

### AC-6 — Per-channel stroke test: `paint_fuzzy_skin` decodes to `FuzzySkin(Flag)`

**Given** the existing `resources/cube_fuzzyPainted.3mf` fixture (or a sub-fixture if needed),
**When** the loader runs and the parsed strokes are inspected,
**Then** the `paint_fuzzy_skin` strokes carry `PaintSemantic::FuzzySkin(Flag(true))`.

| `cargo test -p slicer-model-io paint_fuzzy_skin_subfacet_strokes_decoded 2>&1 | tee target/test-output.log`

### AC-7 — Loaded paint strokes reach the live slice consumer (`host:paint_segmentation`) and produce observable `SlicedRegion` effects, per channel

> **REWRITTEN (P98):** the original AC asserted strokes were normalized into `facet_values` by `host:mesh_segmentation`. That consumer was retired by P94r and deleted by P97. The live consumer is `host:paint_segmentation` (P95 — `slicer_core::algos::paint_segmentation::execute_paint_segmentation`, called at `prepass.rs:540-566`), which reads `PaintLayer.strokes` directly. AC-7 is rewritten to assert the strokes reach that consumer with a per-channel behavior assertion. See deviation **D-98-AC7-CONSUMER-PATH** in closure-log.md.

**Given** a model whose paint channels populate `PaintLayer.strokes` at load time,
**When** `execute_paint_segmentation` runs over the loaded `MeshIR`,
**Then** each channel produces its live observable on `SlicedRegion`:
- `paint_color` → a region whose `variant_chain` contains `("material", PaintValue::ToolIndex(N))`;
- `paint_fuzzy_skin` → a region whose `variant_chain` contains `("fuzzy_skin", PaintValue::Flag(true))` (the primary P98 win — these strokes were dropped pre-P98);
- `paint_supports` → the live SupportEnforcer/Blocker observable populated from a `PaintLayer` stroke;
- `paint_seam` → the strokes reach `SlicedRegion` (`variant_chain` `("seam_enforcer"/"seam_blocker", …)`) **but no live module consumes them** — recorded as deviation **D-98-SEAM-NO-CONSUMER** (seam-placer scores seams geometrically, not from paint). The test documents the gap.

| `cargo test -p slicer-runtime --test executor paint_channel_ 2>&1 | tee target/test-output.log`

### AC-8 — Behavior preservation on unpainted regression_wedge.stl (no paint channels)

**Given** an unpainted STL,
**When** loading runs,
**Then** no strokes are emitted on any channel; the loader's behavior on STL inputs is unchanged.

| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p98-wedge.gcode && sha256sum /tmp/p98-wedge.gcode`

### AC-9 — Behavior preservation on cube_4color.3mf (paint_color channel; unchanged from P97 baseline)

**Given** the cube_4color fixture (paint_color channel),
**When** loading + slicing runs,
**Then** post-packet g-code is byte-identical to the post-P97 baseline (the loader still calls `decode_strokes_for_channel` for `paint_color`; the new helper produces the same strokes as the old inlined block).

| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p98-cube.gcode && sha256sum /tmp/p98-cube.gcode`

### AC-10 — Behavior change on cube_fuzzyPainted.3mf is bounded to fuzzy_skin assignment

**Given** the cube_fuzzyPainted fixture (paint_fuzzy_skin channel),
**When** the slice runs,
**Then** the g-code MAY differ from the post-P97 baseline because the fuzzy_skin strokes are now decoded (they were previously dropped); the closure log documents the diff with rationale.

| `cargo run --bin pnp_cli --release -- slice --model resources/cube_fuzzyPainted.3mf --module-dir modules/core-modules --output /tmp/p98-cube-fuzzy.gcode && sha256sum /tmp/p98-cube-fuzzy.gcode`

### AC-11 — Guest WASM `--check` clean

> **AS-BUILT (P98): PASS.** Initial `--check` reported STALE; a full `cargo xtask build-guests` (31 guests rebuilt) followed by `cargo xtask build-guests --check` returns **CLEAN** (exit 0, no STALE lines). The staleness was simply unbuilt artifacts, not an unfixable condition — verified by an actual rebuild per CLAUDE.md §"Guest WASM Staleness".

| `cargo xtask build-guests --check`

## Negative Test Cases

### AC-N1 — Malformed hex on `paint_seam` rejects with structured error

**Given** a malformed hex stroke on `paint_seam`,
**When** loading runs,
**Then** a structured error is returned naming the channel and the offending hex; loader fails gracefully (no panic).

| `cargo test -p slicer-model-io paint_seam_malformed_hex_rejected 2>&1 | tee target/test-output.log`

### AC-N2 — Empty hex on any channel is a no-op (no strokes, no error)

| `cargo test -p slicer-model-io paint_channel_empty_hex_noop 2>&1 | tee target/test-output.log`

### AC-N3 — A 3MF with NONE of the four channels loads without error and produces no strokes

| `cargo test -p slicer-model-io threemf_no_paint_channels_no_strokes 2>&1 | tee target/test-output.log`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-model-io 2>&1 | tee target/test-output.log` (per-channel tests)
4. `cargo test -p slicer-runtime --test executor cube_fuzzyPainted 2>&1 | tee target/test-output.log` (AC-7)
5. `cargo xtask build-guests --check`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P5b — Loader symmetry" (~30 lines).
- `crates/slicer-model-io/src/loader.rs` — range-read lines 1119-1295 (the parse-attributes block + the existing decoder).

## Doc Impact Statement

- `crates/slicer-model-io/src/loader.rs` doc-comment for `decode_strokes_for_channel` — `rg -q 'fn decode_strokes_for_channel' crates/slicer-model-io/src/loader.rs`.

No `docs/*.md` change — internal refactor.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` — sub-facet hex encoding format; SUMMARY confirming the hex grammar is the same for all four channels.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
