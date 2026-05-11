# Task Map: 51_paint-semantic-region-overrides

This packet closes DEV-045 (registered 2026-05-10 by the spec-review of Packet 43-rev1) and introduces the new backlog task TASK-181 to `docs/07_implementation_status.md`. End-to-end testability is gated on Packet 50 closure (DEV-044).

## docs/07 → Packet Steps

| docs/07 Task ID | docs/07 Status (current) | Packet Steps | Closure Trigger |
| --- | --- | --- | --- |
| `TASK-181` ("Make RegionMap paint-semantic-aware: add `paint_config:<semantic>:<key>` namespace, extend `RegionPlan` with `paint_overrides`, and make `region_mapping.rs` overlay per-semantic configs into `RegionPlan.config` via polygon overlap with `PaintRegionIR`. Covers DEV-045.") | NEW — added at Step 6 with status `[~]` then closed `[x]` at packet completion | Steps 1-8 (all steps touch TASK-181) | All 13 ACs (10 positive + 3 negative) GREEN; DEV-045 flipped to Closed in `docs/DEVIATION_LOG.md`. |

## Packet Steps → docs/07 Tasks (reverse map)

| Packet Step | Task IDs Touched | Notes |
| --- | --- | --- |
| 1 — Activation grounding | TASK-181 | Resolves Q1-Q4 open questions; flips packet `draft` → `active`. |
| 2 — Author failing unit tests | TASK-181 | Two new test files (5 tests RED). |
| 3 — config_resolution extension | TASK-181 | `paint_config:` namespace + `resolve_per_paint_semantic_configs`; AC-1 + AC-NEG-1 GREEN. |
| 4 — RegionPlan IR additive + schema bump | TASK-181 | `paint_overrides` field; RegionMapIR 1.0.0 → 1.1.0; AC-2 GREEN. |
| 5 — region_mapping paint-aware overlay | TASK-181 | Read PaintRegionIR; polygon overlap; precedence; stamp config + paint_overrides; AC-3, AC-NEG-2, AC-NEG-3 GREEN. |
| 6 — Docs + deviation closure | TASK-181 | `docs/01`, `docs/02`, `docs/07`, `docs/DEVIATION_LOG.md`, `docs/14` edits; AC-8, AC-10 GREEN. |
| 7 — Regression + E2E sweep | TASK-181 | Backward-compat + Packet 50 regression + 43-rev1 regression; AC-5, AC-6, AC-7, AC-9 GREEN. AC-4 GREEN if Packet 50 closed. |
| 8 — Acceptance ceremony | TASK-181 | Re-run all AC commands; flip packet to `implemented`. |

## Aggregate

- 1 backlog task id closed (TASK-181 — new at packet start; closed at packet end).
- 1 deviation row closed (DEV-045).
- 1 doc cross-reference updated (`docs/14_deviation_audit_history.md` chronology).
- 0 prior packets superseded (this is a fresh packet, not a revision).

## Cross-Packet Dependencies

- **Depends on Packet 50 (`50_paint-input-3mf-ingestion`, closes DEV-044) for end-to-end testability.** AC-4 (E2E test on painted Benchy) cannot turn GREEN until Packet 50's `resources/benchy_painted.3mf` fixture is committed and the 3MF loader extension lands. Steps 1-6 of this packet can proceed in parallel using synthetic in-memory `paint_data`; Step 7 marks AC-4 deferred if Packet 50 has not closed.
- **Unblocks future paint-semantic packets.** Once the override mechanism exists, follow-up packets can add new semantics (e.g. `Custom("ironing")`, `Custom("seam_avoid")`) without re-implementing the resolution layer.

## Why this packet exists

DEV-045 was registered 2026-05-10 by a spec-review of Packet 43-rev1 (`43-rev1_macro-prepass-segmentation-output-drain`, closed 2026-05-08). The review identified that RegionMap is paint-blind despite PaintSegmentation having a green WIT/IR contract: `region_mapping.rs` contains zero "paint*"/"semantic" tokens, `RegionPlan` has no paint-semantic dimension, and `config_resolution.rs` has no `paint_config:` namespace. Consequently `PaintSemantic::Custom(...)` values cross IR but cannot bind to per-region `ResolvedConfig` overrides on the live host scheduler. This packet closes that gap.

The packet's scope was tightened during authoring (2026-05-10) when grounding revealed that Layer-tier modules consume config via `ConfigView` — meaning the override resolution can happen entirely host-side without any module-side changes. This collapses what could have been a 10-module change set into a 3-file host-side change.
