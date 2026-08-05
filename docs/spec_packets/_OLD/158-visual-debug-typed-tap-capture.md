---
status: implemented
packet: 158-visual-debug-typed-tap-capture
task_ids:
  - TASK-268
---

# 158-visual-debug-typed-tap-capture

## Goal

Add request-gated, typed post-stage capture at the executor boundary so model-backed visual-debug requests execute only the scheduler dependency closure required by their selected taps and export bounded renderer-owned captures through packet 157's request/manifest model.

## Problem Statement

Packet 157 establishes the opt-in visual-debug command and exports the versioned request and bundle-manifest model, but it does not observe typed intermediate IR. TASK-268 supplies the missing runtime seam: selected post-stage, post-host-hook values must be captured without adding module-visible access or retaining unbounded layer snapshots. The implementation must consume packet 157's exported request/manifest model rather than define a parallel request, source-mode, tap, or manifest contract.

## Architecture Constraints

- Taps are runtime-owned, request-gated, and read typed post-stage/post-host-hook state; they do not create scheduler edges, module invocations, module-visible access, WIT APIs, or manifest IR access.
- Blackboard IR is immutable during per-layer work; per-layer IR is borrowed at the executor boundary and copied into renderer-owned capture data before `LayerArena` release.
- The fixed scheduler order and four-phase execution remain authoritative. The closure may stop at the furthest selected tap, while correctness-required extra execution is recorded as expansion.
- Capture failure is fatal to the visual-debug product; no partial capture is reported as successful. Existing progress-event ordering and required failure visibility remain intact.
- This packet does not add rendering, G-code parsing, coordinate logic, WASM, OrcaSlicer parity, or agent-skill behavior.

## Data and Contract Notes

- IR/manifest contracts: adapters consume the exact documented source fields from `docs/specs/visual-pipeline-debug.md`; packet 157 owns the request and manifest model; capture records are renderer-owned and do not alter IR schemas.
- WIT boundary: unchanged. No capture data crosses a module boundary and no module receives a new read capability.
- Determinism/scheduler constraints: preserve fixed stage order, deterministic selected tap/layer ordering, bounded selected-layer retention, and explicit manifest expansion reasons.

## Locked Assumptions and Invariants

- Packet 157 is the sole owner of request parsing, source mode, bundle lifecycle, overwrite behavior, and base manifest semantics.
- Capture occurs only after the selected stage's host hook and commit boundary.
- A successful visual-debug run never contains dangling arena borrows, unrequested retained snapshots, or silently omitted selected taps.
- Ordinary `pnp_cli slice` does not allocate, serialize, or invoke visual-debug capture machinery.

## Risks and Tradeoffs

- `run_visual_debug`'s `Model` branch has no tap validation today (any `TapSelector` name silently "succeeds"); the new tap registry must reject unknown taps itself (AC-N2) rather than assume packet 157 already gates this.
- `layer_info` currently reads only `req.layers.first()`; the implementation must iterate all selected layers instead of preserving this single-layer shortcut, or AC-3's multi-layer retention cannot be satisfied.
- A tap's correctness dependencies may require extra layers or whole-print work; recording expansion in the manifest preserves explainability but must not retain those unselected captures.
- The adapter inventory is broad; each adapter should remain a thin typed projection so IR schema drift causes compile/test failures rather than silent field loss.
- `slicer-runtime` cannot depend on `pnp-cli`'s `Manifest`/`ImageEntry`/`VisualDebugRequest` types (dependency direction is `pnp-cli -> slicer-runtime`); the new runtime capture API must be expressed in runtime/`slicer-ir`-owned types and translated by `pnp-cli`, not by mirroring or importing packet 157's types into `slicer-runtime`.
