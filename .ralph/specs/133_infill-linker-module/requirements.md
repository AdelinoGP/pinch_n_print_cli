# Requirements: 133_infill-linker-module

## Packet Metadata

- Grouped task IDs:
  - `TASK-258`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Under Architecture A (ADR-0025) no infill module links its own output — yet nothing links it
at all: the `Layer::InfillPostProcess` stage runs no module, so every print's infill is
whatever the emitting module produced (today: rectilinear's disjoint 2-point segments with
maximum travel). The contract (130), per-region config (131), and wall-less sub-regions (132)
land ahead of this packet (129–132 are gated prerequisites — all `draft` at authoring, and
must reach `status: implemented` before 133 activates); this packet ships the module that
makes them pay: the single place infill connection happens, uniformly for every region and
every emitting module. Landing the linker
BEFORE the module rewrites (roadmap D1) means output improves immediately and 134/135's raw
output is linked the day those packets land.

## In Scope

- New guest module `modules/core-modules/infill-linker/` (`#[slicer_module]`,
  `Layer::InfillPostProcess`, `run_infill_postprocess`): manifest with
  `holds = ["claim:infill-link"]` and `[config.schema.infill_overlap]` (float, default 0.45);
  workspace member registration.
- `claim:infill-link` catalog entry (docs/03 table) + non-fill first-winner dedup coverage
  (AC-N3; reuse the existing non-fill dedup mechanism — no `FILL_CLAIM_IDS` change, no
  `ResolvedConfig` field, per grilling decision D4).
- In-module ports (OrcaSlicer attribution header per `docs/ORCASLICER_ATTRIBUTION.md` on each
  ported file): `ExPolygonWithOffset` (FillRectilinear.cpp:388-490, with overlap-sign
  verification), `BoundaryInfillGraph` (FillBase.cpp:1432-1544), `connect_infill`
  (FillBase.cpp:1580-1818), `chain_or_connect_infill` (FillBase.cpp:1820-2246),
  `remove_short_polylines` (FillGyroid.cpp:356-359).
- Linking orchestration: group regions into wall-sharing groups (via
  `wall_source_region_id`); per (group, role): branch (a) same-config → union role polygons,
  one `ExPolygonWithOffset`, `connect_infill` over the union boundary, majority-length bucket
  assignment; branch (b) different-config → per-region linking along the region's own
  boundary including wall-less shared arcs, no overlap inset on wall-less arcs. Compatibility
  predicate: same object-id + tool-index + role + group + path-compatible (equal
  `speed_factor`, endpoint widths within epsilon).
- Re-clip via `slicer_core::polygon_ops::clip_polylines` (129); per-role spacing from the
  packet-131 per-region config accessor (`infill_density`, `line_width`).
- Full re-emit: output is the complete `InfillIR` replacement; `ironing` passes through
  identically.
- Module test suite (AC-1…AC-8, AC-N1, AC-N2) + one runtime pipeline smoke test (AC-10) +
  scheduler dedup test (AC-N3); update the core-module count assert in
  `manifest_ingestion_tdd` (20 → 21).
- Doc Impact: docs/03 claim row; docs/01 module inventory/pipeline mention.

## Out of Scope

- Rewrites of rectilinear/gyroid/lightning (134/135/140) — the linker handles their CURRENT
  output as-is (2-point segments link; clipped gyroid waves re-clip + chain).
- Golden restore/re-bless (136) and the `infill_overlap` CLI flag (136).
- `no_linker_module_degraded_raw_output` integration test (136 — it asserts the absence
  case at the roadmap level).
- Monotonic ordering, `multiline_fill`, anchor-based strategies — deferred per the spec's
  Out-of-scope list.
- Any change to `Layer::PathOptimization` (entity-level ordering is its job; ADR-0025
  pipeline note).

## Authoritative Docs

- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` + Amendment,
  `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` — binding; full reads (short files).
- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` — §Phase 4 only (rewritten step 6
  carries the two branches verbatim).
- `docs/ORCASLICER_ATTRIBUTION.md` — header template for ported files.
- `docs/08_coordinate_system.md` — delegate SUMMARY; every Orca length constant ÷ 100.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp:1580-1818` — `connect_infill` (core port; dispatch section-by-section, ≤30-line snippets).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp:1820-2246` — `chain_or_connect_infill`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp:388-490` — `ExPolygonWithOffset`; the overlap sign/direction verification is MANDATORY before Step 2 codes the offset.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp:356-359` — short-polyline threshold.

## Acceptance Summary

- Positive cases: `AC-1`–`AC-10` in `packet.spec.md`. Refinements: AC-3's test constant
  comment must cite `FillRectilinear.cpp:388-490` and state the verified sign in words
  ("expands outward into the perimeter zone" or "insets inward"), so a future reviewer can
  re-verify without re-reading Orca; AC-6's majority-length bucket rule ties to grilling
  decision D5.4; AC-7's 0.5 × spacing reach tolerance operationalizes "no unfilled band".
- Negative cases: `AC-N1` (wall-backed never crossed), `AC-N2` (tool mismatch), `AC-N3`
  (claim dedup).
- Cross-packet impact: 134/135 rely on the linker being present; 136 restores goldens over
  this packet's output (the carve list from 131 may gain entries here — each addition is a
  recorded deviation).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p infill-linker 2>&1 \| tee target/test-output.log \| grep "^test result"` | module suite (AC-1…AC-8, N1, N2) | FACT + counts |
| `cargo test -p slicer-runtime --test executor -- infill_linker_pipeline_smoke 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-10 pipeline smoke | FACT |
| `cargo test -p slicer-scheduler -- infill_link_claim_dedup 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N3 dedup | FACT |
| `cargo clippy -p infill-linker --all-targets -- -D warnings` | lint gate | FACT |
| `cargo xtask build-guests --check` (rebuild if STALE) | new guest joins the build set | FACT clean/STALE |
| `cargo test -p slicer-runtime --test contract -- manifest_ingestion 2>&1 \| tee target/test-output.log \| grep "^test result"` | module count 21 | FACT |
| `rg -q 'claim:infill-link' docs/03_wit_and_manifest.md && rg -q 'infill-linker' docs/01_system_architecture.md && echo DOCS-OK` | Doc Impact greps | FACT |

## Step Completion Expectations

- Cross-step invariant: from Step 1 onward the module is live in the default module dir —
  every later step's pipeline-touching test runs WITH the linker active. If an unrelated
  suite regresses mid-packet, first check whether the linker's pass-through (Step 1 behavior)
  or linking (Step 5+) legitimately changed output — those belong on the 131 carve list as
  recorded deviations, not silent test edits.
- Step ordering rationale: ports land smallest-first (offset structure → graph → connect →
  chain/orchestration) so each has a testable seam before the next builds on it.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated: ALL OrcaSlicer reads
  (delegated, section-by-section — `connect_infill` alone is ~700 lines: dispatch it in ≤
  30-line snippet windows keyed to the port's current section); `crates/slicer-sdk/src/`
  builder/view files (ranged to the postprocess surfaces).
- Likely temptation reads: other core-modules' `lib.rs` for idiom — read ONE small module's
  scaffold (e.g. `top-surface-ironing`) and stop; the `#[slicer_module]` macro handles the
  rest.
- Sub-agent return-format hints: Orca port dispatches return SUMMARY (algorithm intent) +
  SNIPPETS (≤30 lines, the exact section being ported); cargo runs return FACT.
