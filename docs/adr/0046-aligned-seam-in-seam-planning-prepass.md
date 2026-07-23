# ADR-0046: Aligned seam modes live in the SeamPlanning prepass, not the per-layer seam placer

Status: accepted

Packet 168 ports OrcaSlicer's `aligned` / `aligned_back` seam modes (canonical
`SeamPlacer::place_seam` and the seam-string machinery around
`SeamPlacer.cpp`'s `pick_seam_option` / alignment pass, plus the fitting
utilities in `Curves.hpp`). This ADR records where that machinery lives and the
WIT contract change it required.

## Context

Aligned seam placement is inherently **cross-layer**: canonical OrcaSlicer
chains seam candidates across consecutive layers into "seam strings" and then
smooths each string with a least-squares B-spline fit (canonical
`Curves.hpp::fit_cubic_bspline`), so that seams form a continuous vertical line
instead of jumping per layer. `aligned_back` is the same pass with a rear bias
applied to candidate scoring.

PnP has two candidate homes for this logic:

1. The per-layer `seam-placer` module (`modules/core-modules/seam-placer`),
   Layer tier.
2. The `PrePass::SeamPlanning` stage module
   (`modules/core-modules/seam-planner-default`), which runs once per print
   before any layer work.

Option 1 is structurally impossible under this codebase's execution model:
per-layer modules are **re-instantiated per call and run in parallel across
layers** (ADR-0045 records that no state survives between calls — the module is
rebuilt per call, and packet 102 already ruled cross-call caching forbidden).
A per-layer module can never see two layers, so it can never chain anything.

The only sanctioned cross-layer conduit is the one ADR-0020 established:
`SeamPlanIR` produced by the SeamPlanning prepass, delivered to the per-layer
seam placer as a host-injected `resolved_seam` on each layer's input. There is
no other channel.

## Decision

- **All aligned machinery lives in `seam-planner-default`'s prepass** —
  `modules/core-modules/seam-planner-default/src/comparator.rs` (candidate
  scoring, ported from canonical `SeamPlacer.cpp`'s seam-comparator logic),
  `visibility.rs` (deterministic raycast visibility, reduced budget — see
  `D-168-SEAM-PREPASS-SOURCE` in `docs/DEVIATION_LOG.md`), `align.rs`
  (seam-string chaining + least-squares spline smoothing, ported from canonical
  `SeamPlacer.cpp` + `Curves.hpp`), and `contours.rs` (PnP-original z-plane
  sectioning of `MeshObjectView` triangles into per-layer contours).
  `seam_mode` on `seam-planner-default` accepts `aligned` / `aligned_back`;
  the default was `nearest` initially; amended 2026-07-22 to `aligned` per packet 180 (see `D-283-ADR-0046-AMENDED` in `docs/DEVIATION_LOG.md` and packet 180).

- **The WIT export gains a parameter.** The prepass needs real layer z values,
  so `run-seam-planning` (canonical WIT source
  `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`) now takes
  `layer-plan: layer-plan-view` alongside `objects` / `output` / `config` —
  the same view `run-support-geometry` already consumes.

- **That is a major world-version bump: `slicer:world-prepass` 1.0.0 → 2.0.0.**
  `docs/11_operational_governance_and_acceptance_gate.md` classifies a type
  change to an existing export — which adding a required parameter is — as a
  major bump. DEV-084 (packet 130's `run-infill-postprocess` parameter, shipped
  as 1.1.0 and corrected to 2.0.0) is the precedent this follows.

- **Consumption side:** `seam-placer` (per-layer) reads the host-injected
  planner choice and **snaps it to the nearest of its own seam candidates**
  (unlimited snap radius, falling back to the nearest wall vertex when no
  candidate exists; pristine per-layer behaviour when no planner entry is
  injected). Snapping is what keeps the emitted seam on a real wall vertex even
  though the prepass computed it from mesh-derived contours rather than final
  perimeters (see `D-168-SEAM-PREPASS-SOURCE`).

## Alternatives rejected

- **Per-object anchor accumulator inside `seam-placer`.** A static or
  blackboard-side accumulator that per-layer calls append to. Rejected: layer
  calls run in parallel with no ordering guarantee, so the accumulator would
  see layers out of order and nondeterministically; it also reintroduces
  exactly the cross-call state ADR-0045 and packet 102 forbid.
- **Host-builtin native alignment pass.** Run the chaining/smoothing in the
  host between prepass and layer dispatch. Rejected: it moves slicing policy
  out of the module system, bypassing the manifest/config surface and the
  ADR-0020 injection contract that already exists for precisely this data flow.
- **Deriving z from `layer_height` config instead of the layer plan.** Rejected:
  variable layer height, first-layer height, and catch-up layers make
  `z = i * layer_height` wrong in general; `layer-plan-view` carries the
  planned truth and was already exported to prepass modules for
  `run-support-geometry`.

## Consequences

- `slicer:world-prepass` majors to 2.0.0; all prepass guests rebuild
  (`cargo xtask build-guests`).
- The aligned path's inputs are mesh-derived contours, not final perimeters —
  a recorded deviation from canonical (which runs `SeamPlacer` after perimeter
  generation), mitigated by the seam-placer snap. Tracked as
  `D-168-SEAM-PREPASS-SOURCE`.
- `nearest` mode is still untouched end-to-end and remains available via `seam_mode`; `aligned` and `aligned_back` are now the defaults (set in both `seam-placer.toml` and `seam-planner-default.toml`), matching OrcaSlicer's `spAligned` default. Users may still opt in to any other mode via `seam_mode`.

**Amendment 2026-07-22 (packet 180):** The default `seam_mode` value was changed from `nearest` to `aligned` in both `seam-placer.toml` and `seam-planner-default.toml` to match OrcaSlicer's canonical `spAligned` default. The change is recorded as `D-283-ADR-0046-AMENDED` in `docs/DEVIATION_LOG.md`. The continuous-projection work in packet 180's `seam-placer/src/lib.rs::run_wall_postprocess` closes the source-geometry gap tracked in `D-168-SEAM-PREPASS-SOURCE`. The amendment affects the normative clauses at L50 and L97-98 of this ADR (both now read with the amendment).
