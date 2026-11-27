# 158 — Triage all 52 draft packets for architectural drift, SOTA quality gaps, and Phase 2 batching

Type: research
Status: open
Assignee: —
Blocked by: 149, 154, 155, 157
Map: ../map.md

## Question

**For every renamed draft packet (`docs/spec_packets/orca-feature-gaps_01_*` .. `_52_*`), what actually drifted against current `master`, where did older-LLM authoring fall short of our current SOTA bar, and how should the packets be batched for Phase 2 re-authoring?**

This research ticket prepares the full evidence base for [160 — Graduate Phase 2 SOTA packet re-authoring batches](./160-graduate-phase2-sota-packet-reauthoring-batches.md). Read the assets produced by [149](./149-measure-resolvedconfig-blind-spot-in-gap-inventory.md), [154](./154-reground-config-scope-assumptions-on-map-and-open-tickets.md), [155](./155-reground-layer-preparation-and-feedrate-on-map-and-open-tickets.md), and [157](./157-rename-draft-packets-to-orca-feature-gaps-prefix.md) first — do not re-derive their system-wide findings.

### Audit every draft packet along six axes

1. **Live-key & overlap check (using `149`'s refreshed `ConfigSchemaRegistry` inventory):** Are any of the packet's keys already live on `master` (e.g., `infill_shift_step`, `support_base_pattern`, `ResolvedConfig` keys previously hidden by ticket `01`'s blind spot) or superseded?
2. **Architectural seam drift (using `154` & `155`):**
   - Config/schema/scope drift (`ConfigSchemaRegistry`, `expand_automatic_values`, `resolve_scope_stack`, `ModifierKind`, layer-range scope, `denied_scopes`).
   - Layer-preparation / cross-layer / identity drift (`LayerPreparation::prepare_print`, retired `LightningTreeIR`, `paint_only`, `SupportAnalysisIR::family_assignments`, `STAGE_ORDER`, ticket `148` slice-mutating pass home — especially legacy `306`, `297`, `303`–`305`, `298`, `256`, `307`).
   - Feedrate / emitter / perf drift (ADR-0072 `role_base_speed_mm_per_s` / `resolve_extrusion_feedrate`, plus vanished symbols such as `assemble_bridge_areas` / `angle_override` in legacy `302`, `bottom_surface_footprint` in `256`, `classic_0p4mm_bead_0p2mm_layer` in `303`, and any WIT version bumps like `prepass-layer-planning@2.0.0`).
3. **Format & test-quality drift:** Legacy 5–6 file structure (`requirements.md`, `design.md`, `task-map.md`, `preflight-report.md`) to collapse into modern 2-file `packet.spec.md` + `implementation-plan.md` (`docs/spec_packets/README.md`), plus `docs/22_test_quality.md` compliance (false-green R1–R8 patterns, `assert_ne!`-only liveness without conservation/behaviour assertions, brittle source greps).
4. **SOTA quality & Authoring Rules 1–6 check:** Catch older-LLM shortcuts — wrong owners, leaky abstractions, unnecessary host special cases instead of modular claims/seams, hidden declaration-only keys, weak ACs, or canonical misunderstandings.
5. **Re-sizing & consolidation check:** Flag any packet that should now be:
   - **dissolved** (already live or dead-in-canonical),
   - **demoted to direct implementation** (small enough for a single ticket session under *Packets are for complex implementation only*),
   - **folded** into a neighbouring packet, or
   - **split** because its real seam grew.
6. **Phase 2 batch plan:** Group the surviving packets into coherent, ≤100K-token-session Phase 2 re-authoring tickets (typically **2–4 closely related packets sharing an owner/seam per ticket**, or **1 dedicated ticket** for major architectural designs such as legacy `306` interlocking beams and `307` prime tower body parity), noting explicit blocking edges onto open HITL rulings (`125`, `127`, `128`, `148`, `153`).

### Deliverable

- A linked markdown asset (`./158-asset-packet-drift-triage.md`) containing:
  - One structured row per packet (`orca-feature-gaps_01`..`_52`, with `legacy_id`) recording severity (`architectural-redesign` | `seam-update` | `mechanical-2file-plus-quality` | `dissolve-or-direct-impl`), exact drift vectors, key/symbol findings, and SOTA concerns.
  - The proposed Phase 2 batch table (candidate ticket titles, member packets, dependencies, and estimated complexity) ready for human review in [160](./160-graduate-phase2-sota-packet-reauthoring-batches.md).
- **Read-only research ticket:** do not rewrite any packet files here.

### Obligations

- Re-derive every packet path, symbol, and file check from disk at claim time (delegate bulk mechanical scans to a script or subagent so the session stays clean).
- Cite in-tree code by symbol name + crate-qualified path; OrcaSlicer citations by file + function.

## Answer
