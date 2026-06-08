# Requirements: 98_loader-paint-channel-symmetry

## Packet Metadata

- Grouped task IDs:
  - `TASK-248` — Loader symmetry: decode sub-facet strokes on all four 3MF paint channels (paint_color, paint_supports, paint_seam, paint_fuzzy_skin) via a shared helper.
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P5b — Loader symmetry"
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

3MF files emitted by OrcaSlicer encode paint at sub-facet granularity using TriangleSelector subdivision, with the per-facet partition expressed as a hex string. Each of the four documented 3MF paint channels — `paint_color`, `paint_supports`, `paint_seam`, `paint_fuzzy_skin` — uses the SAME hex grammar, but `crates/slicer-model-io/src/loader.rs:1119-1295` only decodes the hex sub-facet partition for `paint_color` (Material semantic) and `paint_supports` (SupportEnforcer / SupportBlocker semantic). The `paint_seam` and `paint_fuzzy_skin` channels parse whole-triangle facet assignments but silently drop their sub-facet hex partitions.

Consequences observable on the cherry-picked fixtures:

- `resources/cube_fuzzyPainted.3mf` carries `paint_fuzzy_skin` strokes that were ENCODED by OrcaSlicer at sub-facet granularity. Pre-P98, those strokes are dropped, so any cube test asserting on per-face fuzzy_skin coverage fails or relies on coincidental whole-triangle paint.
- Any future `paint_seam` per-edge enforcer painting (a documented OrcaSlicer feature) loads as empty.

The fix is mechanical: hoist the existing hex-decoding block at `loader.rs:1237-1295` into a private helper that takes the channel's `PaintSemantic` as a parameter, then call it for each of the four channels. The 3MF format is unchanged; the loader behavior on channels currently working stays byte-identical; the two previously-empty channels now produce strokes.

## In Scope

- Hoist the hex sub-decoding block at `loader.rs:1237-1295` into `fn decode_strokes_for_channel(hex, semantic, tri_verts, byte_offset) -> Vec<PaintStroke>`.
- Add four call sites — `paint_color` (Material), `paint_supports` (Enforcer/Blocker), `paint_seam` (SeamEnforcer/Blocker), `paint_fuzzy_skin` (FuzzySkin Flag).
- Add per-channel unit tests in `model_loader_tdd.rs`.
- Author a small `resources/cube_seam_painted.3mf` fixture ONLY if no current fixture exercises `paint_seam` sub-facet strokes (≤ 30 KB; mirror the cube_fuzzyPainted authoring approach).
- Add malformed-hex / empty-hex / no-channel negative tests.

## Out of Scope

- 3MF format changes — none required.
- IR / WIT / scheduler / kernel changes — none.
- Paint-segmentation behavior — P3 / P4 territory; this packet feeds the kernel's input, doesn't change its algorithm.
- Doc updates to `docs/03_wit_and_manifest.md` etc. — P5c (99).

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P5b".
- `crates/slicer-model-io/src/loader.rs` — range-read lines 1119-1295.
- `crates/slicer-ir/src/slice_ir.rs` — `PaintSemantic` definitions (for the four-channel → semantic mapping).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` — SUMMARY confirming the hex sub-facet partition grammar is identical across the four paint channels.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-11`. Refinements:
  - The per-channel `PaintSemantic` mapping: `paint_color` → `Material(ToolIndex(N))` with N derived from the hex prefix per OrcaSlicer convention; `paint_supports` → `SupportEnforcer` or `SupportBlocker` per the existing 2-bit semantic encoding; `paint_seam` → `SeamEnforcer` or `SeamBlocker` (same 2-bit encoding); `paint_fuzzy_skin` → `FuzzySkin(Flag(true))` (single-bit Flag — any non-zero encoding means "yes fuzzy_skin here").
  - AC-9 cube_4color byte-identical because `paint_color` continues to use the same hex decoder (now via the helper); semantic mapping is unchanged.
- Negative cases: `AC-N1` (malformed hex rejected), `AC-N2` (empty hex no-op), `AC-N3` (no channels → no strokes).
- Cross-packet impact: P5c (99) docs the new symmetry.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo test -p slicer-model-io 2>&1 \| tee target/test-output.log` | All loader tests + new per-channel tests | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_fuzzyPainted 2>&1 \| tee target/test-output.log` | AC-7 — fuzzy_skin strokes normalized in prepass | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p98-wedge.gcode && sha256sum /tmp/p98-wedge.gcode` | AC-8 — wedge byte-identical | FACT (sha256) |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p98-cube.gcode && sha256sum /tmp/p98-cube.gcode` | AC-9 — cube_4color byte-identical (paint_color unchanged) | FACT (sha256) |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_fuzzyPainted.3mf --module-dir modules/core-modules --output /tmp/p98-cube-fuzzy.gcode && sha256sum /tmp/p98-cube-fuzzy.gcode` | AC-10 — fuzzy SHA (may differ; documented) | FACT (sha256) |
| `cargo xtask build-guests --check` | AC-11 — guest clean | FACT pass/fail |

## Step Completion Expectations

- The hoist (Step 2) MUST preserve byte-identical behavior on `paint_color` and `paint_supports`. AC-9 confirms.
- The new helper takes `PaintSemantic` as a parameter; the channel-to-semantic mapping table lives at the call site, not in the helper.
- AC-10 g-code on `cube_fuzzyPainted.3mf` may differ vs P97 baseline — that's the expected behavior change (fuzzy_skin strokes are now respected). The closure log captures the diff with a one-paragraph rationale.

## Context Discipline Notes

- `crates/slicer-model-io/src/loader.rs` is likely > 1500 lines. Range-read at the parse-attributes block (lines 1119-1295). Do not load in full.
- The 3MF format is binary; never load fixtures directly. Delegate any structural inspection.
- The hex sub-facet partition format is documented in `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` — never load that file; SUMMARY only.
