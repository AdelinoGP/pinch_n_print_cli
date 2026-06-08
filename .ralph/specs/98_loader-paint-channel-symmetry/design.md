# Design: 98_loader-paint-channel-symmetry

## Controlling Code Paths

- Primary code paths: `crates/slicer-model-io/src/loader.rs` (lines 1119-1295 — the parse-attributes block + the existing decoder).
- Neighboring tests or fixtures: `crates/slicer-model-io/tests/model_loader_tdd.rs` (extend with 4 per-channel tests + 3 negative tests); `crates/slicer-runtime/tests/executor/cube_fuzzyPainted_*.rs` (one new normalization test).
- OrcaSlicer comparison surface: see `requirements.md`.

## Architecture Constraints

- The hex grammar is identical across the four paint channels per OrcaSlicer's TriangleSelector.cpp; SUMMARY confirms.
- The four-channel-to-semantic mapping table lives at the call site (loader.rs), not in the helper. The helper is channel-agnostic.
- Behavior preservation invariant: cube_4color SHA stays identical to post-P97 baseline (AC-9). Wedge byte-identical (AC-8). Cube_fuzzyPainted SHA may differ (AC-10 — fuzzy_skin strokes now respected).
- Negative-hex grace invariant: malformed hex returns a structured error rather than panicking (AC-N1).

## Code Change Surface

- Selected approach: hoist + four call sites + tests.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-model-io/src/loader.rs`**:
    - New private function `fn decode_strokes_for_channel(hex: &str, semantic: PaintSemantic, tri_verts: &[Vec3], byte_offset: usize) -> Result<Vec<PaintStroke>, ModelLoaderError>` — body identical to the existing inlined block at lines 1237-1295 modulo:
      - `PaintSemantic` is parameterized (replaces hardcoded `Material` / `Support*`).
      - Returns `Result` to surface malformed hex per AC-N1 (existing code may or may not already; preserve or add).
    - Update the parse-attributes block to call this helper four times, once per channel:
      ```rust
      // After extracting raw triangle XML attributes:
      if let Some(hex) = attrs.paint_color {
          strokes.extend(decode_strokes_for_channel(&hex, semantic_for_paint_color(&hex), &tri_verts, ..)?);
      }
      if let Some(hex) = attrs.paint_supports {
          strokes.extend(decode_strokes_for_channel(&hex, semantic_for_paint_supports(&hex), &tri_verts, ..)?);
      }
      if let Some(hex) = attrs.paint_seam {
          strokes.extend(decode_strokes_for_channel(&hex, semantic_for_paint_seam(&hex), &tri_verts, ..)?);
      }
      if let Some(hex) = attrs.paint_fuzzy_skin {
          strokes.extend(decode_strokes_for_channel(&hex, PaintSemantic::FuzzySkin(PaintValue::Flag(true)), &tri_verts, ..)?);
      }
      ```
    - The four `semantic_for_*` helpers either exist (for paint_color and paint_supports they're embedded inline pre-hoist) or are added (for paint_seam, mirroring paint_supports' 2-bit encoding).
  - **`crates/slicer-model-io/tests/model_loader_tdd.rs`** (extend):
    - 4 positive per-channel tests (AC-3, AC-4, AC-5, AC-6).
    - 3 negative tests (AC-N1, AC-N2, AC-N3).
  - **`crates/slicer-runtime/tests/executor/cube_fuzzyPainted_paint_fuzzy_skin_strokes_normalized_tdd.rs`** (NEW) — AC-7.
  - **Possibly `resources/cube_seam_painted.3mf`** (≤ 30 KB) — only if no existing fixture exercises `paint_seam` sub-facet strokes (Step 1's inventory dispatch confirms).
- Rejected alternatives:
  - **Decode the semantic INSIDE the helper** (rather than as a parameter): rejected — the channel-to-semantic mapping is channel-specific knowledge; passing it as a parameter keeps the helper general.
  - **Replace `decode_strokes_for_channel` with a fully-OrcaSlicer-parity TriangleSelector port**: rejected — out of scope; existing hex decoder already matches OrcaSlicer's encoding format. The work to align is "call it four times" not "rewrite".

## Files in Scope (read + edit)

- `crates/slicer-model-io/src/loader.rs` — range-read lines 1119-1295 + edit.
- `crates/slicer-model-io/tests/model_loader_tdd.rs` — extend.
- `crates/slicer-runtime/tests/executor/cube_fuzzyPainted_paint_fuzzy_skin_strokes_normalized_tdd.rs` (NEW).
- Optionally `resources/cube_seam_painted.3mf` (≤ 30 KB).

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P5b".
- `crates/slicer-ir/src/slice_ir.rs` — `PaintSemantic` and `PaintStroke` definitions.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate (SUMMARY only).
- `target/`, `Cargo.lock`, generated code — never load.
- Binary 3MF fixtures — never `Read`.
- The full `loader.rs` outside lines 1119-1295 — not needed.

## Expected Sub-Agent Dispatches

- "Open `crates/slicer-model-io/src/loader.rs` lines 1119-1295; return SNIPPETS (≤ 60 lines) of the existing hex decoder block".
- "Summarize `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` hex format; SUMMARY ≤ 150 words confirming identical grammar across paint_color / paint_supports / paint_seam / paint_fuzzy_skin".
- "Locate any existing fixture exercising `paint_seam` sub-facet strokes in `resources/`; return FACT (file path or 'none')" — purpose: Step 1 decides if cube_seam_painted authoring is needed.
- "Run `mkdir -p target && cargo test -p slicer-model-io 2>&1 | tee target/test-output.log`; FACT pass/fail".
- "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_fuzzyPainted 2>&1 | tee target/test-output.log`; FACT".
- "Run wedge slice + sha256sum (AC-8)".
- "Run cube_4color slice + sha256sum (AC-9 — byte-identical)".
- "Run cube_fuzzyPainted slice + sha256sum (AC-10 — may differ, document)".

## Data and Contract Notes

- IR contracts: none touched.
- WIT boundary: none.
- Determinism: hex decoder is pure functions; same input → same output.

## Locked Assumptions and Invariants

- **Identical hex grammar across channels**: OrcaSlicer parity (confirmed via SUMMARY).
- **Cube_4color and wedge byte-identical**: regression contract (AC-8, AC-9).
- **Cube_fuzzyPainted SHA may differ**: expected behavior change (AC-10).

## Risks and Tradeoffs

- **Risk: a stroke on `paint_seam` decodes to an unexpected `PaintSemantic` because the 2-bit encoding for SeamEnforcer/Blocker isn't documented in our codebase yet.** Mitigation: SUMMARY against TriangleSelector.cpp surfaces the encoding; if unclear, the closure log documents the chosen encoding as a tentative parity assumption and flags for verification with the OrcaSlicer team.
- **Risk: the helper's `Result` return type breaks an existing caller** that expected `Vec<PaintStroke>`. Mitigation: workspace check + per-test gates.

## Context Cost Estimate

- Aggregate: `S` (small, mechanical).
- Largest single step: `S` (Step 2 — hoist + 4 call sites in one file).
- Highest-risk dispatch: the OrcaSlicer TriangleSelector SUMMARY (must surface the per-channel semantic encoding).

## Open Questions

- `[FWD]` — Does the existing inlined block return `Result` or panic on malformed hex? Step 1 confirms; AC-N1 may require minor expansion of the helper if it currently panics.
- `[FWD]` — Is there a `paint_seam`-carrying fixture already? Step 1 confirms; author `cube_seam_painted.3mf` if not.
- `[BLOCK]` — None.
