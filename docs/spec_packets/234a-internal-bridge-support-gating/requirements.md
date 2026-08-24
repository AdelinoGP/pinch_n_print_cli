# Requirements: 234a-internal-bridge-support-gating

## Packet Metadata

- Grouped task IDs: none (no `docs/07_implementation_status.md` row; backlog slot is ISSUE-82's internal-bridge filtering half)
- Backlog source: `docs/specs/orca-feature-gap/issues/82-author-packet-p75-quality-bridging-bridge-over-infill.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Measured 2026-08-24 on `tmp/calicat.stl` (artifacts: `tmp/calicat_after.gcode`, `tmp/cmp_after.log`, probe `tmp/cmp_dontfilter.log`): the tree emits `Internal Bridge` extrusion on **148 of 174 layers** totalling **86675.76 mm**, versus canonical OrcaSlicer's **exactly one layer** (Z≈29.45, 90 segs / 526.27 mm / ≈30.2° mean). The ratio of bridge-labelled extrusion to canonical worsened from 8.3× (pre-series) to **91×** after packets 233–235, because 233's construction seam at `LayerStageCommit::InfillPostProcess` treats every region's sparse infill as candidate voids with zero support testing — anchors (walls/inset contours) always exist and sparse infill almost always exists. A control probe proved `dont_filter_internal_bridges` has no effect on this path (byte-identical totals with the key true or false): it only toggles a sub-thread-diameter sliver guard that anchored strips never trip.

Canonical (`PrintObject.cpp::bridge_over_infill`) qualifies candidates completely differently: the surfaces examined are the layer's internal-solid interfaces, tested for unsupported span against the layer BELOW (closing of lower fills shrunk by `expansion_multiplier*spacing`, minus grown lower solids), with an area gate (`> 9*spacing^2` when partially supported), a clip (`expand(unsupported, 4*spacing)` to the surface), and a filter enum where the default applies all gates. Additionally, a correct support test requires committed lower-layer data, which the parallel InfillPostProcess arm cannot see — the same scheduler limitation that made packet 234 relocate false-site gating into the sequential ShellClassification prepass.

## In Scope

- Pure support-math port in `crates/slicer-core/src/algos/bridge_over_infill.rs`: unsupported-area computation (closing, shrink/grow with `expansion_multiplier`, solid subtraction), per-surface intersection, qualification gates, `expand(4*spacing)` clip, leftover-island remerge.
- Relocation of construction from `crates/slicer-runtime/src/layer_executor.rs` (InfillPostProcess arm) into `commit_shell_classification_builtin`'s stage in `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, ordered after 234's gate.
- Canonical enum mapping for `dont_filter_internal_bridges`: `false` → full filter; `true` → bypass area/partial gate (`ibfNofilter`). (`ibfLimited`'s `expansion_multiplier = 1` behaviour is represented through the same multiplier parameter.)
- Resolution of two discovery questions before edits: (Q1) which existing `SliceRegion` field is our `stInternalSolid` equivalent (`bottom_solid_fill` is the leading suspect; verify by writers + calicat outcome under gating); (Q2) whether sparse-infill PATHS regenerate after ShellClassification — determines whether area subtraction alone prevents double-printing, mirroring how today's InfillPostProcess subtraction coexists with module-generated paths.
- Import of the measured model as `resources/calicat.stl` and a deterministic double-slice e2e test asserting the flood is gone.
- Net-new test targets: `crates/slicer-core/tests/bridge_support_gating_tdd.rs` ([[test]], `required-features = ["host-algos"]`); the relocated pass itself is exercised end-to-end by AC-5.

## Out of Scope

- The angle algorithm: `determine_bridging_angle` stays as-is (its calicat divergence at Z≈29.4 — ours ≈90° vs canonical ≈30.2° mean — is expected to shrink once candidate areas are correct; if it persists after this packet's fix, that is follow-up work, measured and filed separately).
- External-site orientation (packet 235 surface) — must not regress; guarded by AC-5.
- Any IR field addition, WIT change, schema bump, manifest key addition, or scheduler stage addition (relocation reuses ShellClassification).
- `counterbore_hole_bridging`; the user-facing `internal_bridge_angle` override semantics beyond passing it through unchanged.
- Module-side changes in `modules/core-modules/**` (host-only relocation).

## Authoritative Docs

- `docs/specs/bridge-parity-plan.md` - ~270 lines; direct read (F3 finding wording, §4 W-C row, §6 I2/I3/I7 invariants).
- `docs/15_config_keys_reference.md` - direct read of the `dont_filter_internal_bridges` entry only.
- `docs/04_host_scheduler.md` - over 300 lines; delegated SUMMARY of the ShellClassification stage section only.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `bridge_over_infill` gather lambda: exact closing/shrink/grow arithmetic, per-surface gates, clip, leftover remerge, apply phase.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` + `PrintConfig.cpp` — `InternalBridgeFilter` enum values and the `dont_filter_internal_bridges` option definition (default `ibfDisabled`; `ibfLimited` sets `expansion_multiplier = 1`).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — role assignment precedent for `erInternalBridgeInfill`.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-6`. Measurable refinements absent from their Given/When/Then text: AC-1..AC-3 fixtures are polygon-primitive constructions (no mesh dependency); AC-5's thresholds are frozen from this authoring session's measurements and must not be tightened without re-measurement; AC-5's byte-identity requirement covers determinism of the new prepass pass across repeated slices.
- Negative: `AC-N1` proves qualification returns nothing for a fully-supported surface (the flood root cause), asserted at the pure-function level in Step 1's test file.
- Cross-packet impact: supersedes 233's "prepass stays free of internal-bridge logic" placement constraint (rationale recorded in design.md); preserves 234's gate ordering (support gating runs strictly after false-site gating); preserves 235's external-site output (AC-5 Z≈3.2 guard).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd` | All support-math ACs (AC-1..AC-3, AC-N1) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -c 'rg -q "construct_anchored_polygon" crates/slicer-runtime/src/layer_executor.rs && exit 1 || exit 0'` plus `rg -q "construct_anchored_polygon\|bridge_over_infill::" crates/slicer-runtime/src/slice_postprocess_prepass.rs` | Structural halves of AC-4 (absence in old arm; presence at prepass) | FACT exit codes |
| `cargo run --bin pnp_cli --release -- slice --model resources/calicat.stl --output target/calicat_a.gcode --module-dir modules/core-modules && cargo run --bin pnp_cli --release -- slice --model resources/calicat.stl --output target/calicat_b.gcode --module-dir modules/core-modules && cmp target/calicat_a.gcode target/calicat_b.gcode` then the e2e assertions inside `calicat_internal_bridge_gating_e2e_tdd` | Determinism + flood bar + external-row guard (AC-5) | FACT pass/fail + printed counts |
| `cargo test -p slicer-runtime --test e2e wedge_linked_infill_report_tdd` | Wedge regression guard (AC-6) | FACT pass/fail |
| `cargo check --workspace --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo xtask check-literals` | Gates | FACT pass/fail, exit 0 |
| `cargo xtask build-guests --check` | Guest freshness arbitration (exit 0 fresh / 1 stale / 3 infra) | FACT exit code |
| `cargo xtask test --summary --workspace` | Closure ceremony only | dispatched once, FACT |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step ordering is load-bearing: Step 1 (pure math + Q1/Q2 discovery dispatches) lands before Step 2 (relocation consumes verified functions and the recorded mechanism decision); Step 3 (model import + e2e + blast radius) runs last because AC-5's bar is only meaningful on the completed relocation.
- Q1 and Q2 answers MUST be recorded in `design.md` Open Questions before Step 2 edits begin; an unresolved Q2 blocks the mechanism choice (area-subtract vs path-replace), not the whole step.
- After any slicer-core edit, `cargo xtask build-guests --check` exit codes arbitrate guest freshness before attributing guest-touching failures elsewhere (expected fresh: host-only change surface).

## Context Discipline Notes

- `tmp/` measurement artifacts cited above are session-local evidence, NOT committed inputs; the committed regression surface is the imported `resources/calicat.stl` + its e2e test.
- Tempting read to skip: `crates/slicer-runtime/src/layer_executor.rs` exceeds 300 lines — rg-locate the `InfillPostProcess` arm and range-read only ±120 lines around it.
