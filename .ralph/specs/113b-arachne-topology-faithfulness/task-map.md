# Task Map: 113b-arachne-topology-faithfulness

This packet spans 7 sequential steps that build a synthetic quad/rib topology pass and re-port 4 Arachne passes that depend on it, plus 3 downstream stage re-validations. The task map below shows how each step maps to the M2 plan items in `docs/specs/perimeter-modules-orca-parity-roadmap.md` (T-220..T-227, T-231, T-232 follow-ups) and the corresponding deviation closures. No `TASK-###` entries exist in `docs/07_implementation_status.md` for this work per the packet-112 handoff; the M2 plan doc + `docs/DEVIATION_LOG.md` are the authoritative crosswalk.

## Task Crosswalk

| Packet Step | Deviation Touched | M2 Plan Reference | OrcaSlicer Ref |
|---|---|---|---|
| Step 1: Quad/rib topology pass (L, exception documented) | (gates all topology-chain deviations) | `T-220` (Phase 12, §"M2 — Real Arachne") — re-port | `SkeletalTrapezoidationGraph.cpp:452` |
| Step 2: Faithful `filter_central` | `D-112-CENTRALITY-ADAPT` (predicate half — closed) | `T-220` (Phase 12, §"M2 — Real Arachne") — re-port | `SkeletalTrapezoidation.cpp:672` |
| Step 3: Per-NODE bead_count | `D-112-CENTRALITY-ADAPT` (bead_count half — closed) | `T-221` (Phase 12, §"M2 — Real Arachne") — re-port | `SkeletalTrapezoidation.cpp:777` |
| Step 4: Faithful transitions + propagation re-port | `D-112-PROPAGATION-ADAPT` (closed) | `T-222` (Phase 12, §"M2 — Real Arachne") — re-port | `SkeletalTrapezoidation.cpp:925,1487,1800,1833` |
| Step 5: Faithful `connectJunctions` | `D-113B-CONNECTJUNCTIONS` (new, closed same-packet) | `T-223` (Phase 12, §"M2 — Real Arachne") — re-port | `SkeletalTrapezoidation.cpp:2260` |
| Step 6: Re-validate stitch + simplify + remove_small | (cascade, no deviation closed directly) | `T-225..T-227` (Phase 12) | N/A |
| Step 7: Close 2 + register+close 1 + re-verify MMU + workspace gate | All 3 above + `D-112-MMU-TOPOLOGY` re-verification | `T-232` (Phase 13) | N/A |

## Deviation Disposition (Post-Packet)

| Deviation | Status After P113b | Mechanism |
|---|---|---|
| `D-112-CENTRALITY-ADAPT` | **CLOSED** | Step 1 (quad/rib) + Step 2 (faithful predicate `dR < dD * sin(angle/2)`) + Step 3 (per-NODE bead count) |
| `D-112-PROPAGATION-ADAPT` | **CLOSED** | Step 1 (quad/rib) + Step 4 (faithful `generateTransitionMids`/`applyTransitions` from `transition_ratio`, propagation re-ported to read quad state) |
| `D-113B-CONNECTJUNCTIONS` (new) | **CLOSED** | Step 5 (faithful `connectJunctions` port — per-edge 2-junction fragments replaced with full `ExtrusionLine` stitching) |
| `D-112-MMU-TOPOLOGY` | **Re-verified** (Step 7) — closed if the "tens of mm outside the naive per-face footprint" symptom is gone with the faithful `connectJunctions` output; re-targeted with new evidence if the symptom persists. Either outcome is acceptable. |
| `D-112-SELFCAPTURED-BASELINES` | Still open (accepted) | No OrcaSlicer binary; matches D-109 precedent |
| `D-112-SIMPLIFY-DP` | Closed (P113a) | P113a Step 1 (Visvalingam port) |
| `D-112-THIN-WALL-WIDENING` (residual) | Closed (P113a) | P113a Steps 2-3 (config wiring) |

## Cross-Packet Dependencies

- **Depends on P113a** (`status: active`): P113a's 6 independent S/M items must ship first. P113a's `Visvalingam-Whyatt` port (Step 1) is a no-op on P112's 2-junction input but becomes active once P113b's `connectJunctions` produces multi-junction input. P113a must reach `status: implemented` before this packet can activate.
- **Depends on P112** (`d9466fd7`, `status: implemented`): the existing Arachne pipeline source, fixtures, and host-service bridge.
- **Does NOT depend on ADR-0033.** The original packet draft listed "ADR-0033 (Algorithm Faithfulness as OrcaSlicer Parity Definition)" as a P113b dependency. That ADR does not exist in `docs/adr/` and the user has not asked for it. The acceptance criteria assert algorithm fidelity via OrcaSlicer code references — code references are sufficient on their own; a formal ADR is not required for this packet.
- **Unblocks:** the M2-faithful closure ceremony. After P113a + P113b ship, the perimeter parity roadmap can flip its "M2 — Real Arachne" marker to "complete with algorithm faithfulness."

## OrcaSlicer Reference Paths (per `requirements.md` §OrcaSlicer Reference Obligations)

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452` — `makeRib()` (Step 1)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:672` — `updateIsCentral()` (Step 2)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:777` — `updateBeadCount()` (Step 3)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:925` — `generateTransitionMids()` (Step 4)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1487` — `applyTransitions()` (Step 4)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1800` — `propagateBeadingsUpward()` (Step 4)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1833` — `propagateBeadingsDownward()` (Step 4)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260` — `connectJunctions()` (Step 5)

## Step Dependency Graph

```
Step 1 (quad/rib) — L, exception documented
    ├── Step 2 (filter_central)
    ├── Step 3 (bead_count)
    └── Step 4 (transitions + propagation)
            └── Step 5 (connectJunctions)
                    └── Step 6 (re-validate downstream)
                            └── Step 7 (deviation closures + re-verify MMU + workspace gate)
```

Steps 2, 3, and 4 can run in parallel after Step 1 lands (they all read the quad/rib topology but don't depend on each other's output). Step 5 depends on Step 4 (the propagation re-port must land before `connectJunctions` can walk the quad graph correctly). Step 6 depends on Step 5 (the downstream stages see the new multi-junction input from `connectJunctions`). Step 7 depends on all prior steps.

## L-Step Exception (Step 1)

The spec-packet-generator skill rule "No step may be L; if it would, split" is OVERRIDDEN for Step 1 at the user's explicit decision during packet refinement. The justification is that the `makeRib` algorithm is monolithic — partial rib insertion produces incorrect topology that blocks all 4 dependent passes, and there is no natural split point. The override is documented in `packet.spec.md` §Prerequisites and Blockers, `design.md` §Context Cost Estimate, and `implementation-plan.md` §Per-Step Budget Roll-Up. If subsequent design work surfaces a natural split point (e.g., a separate "rib classification" pass that doesn't need quad cells), the packet SHOULD be split before activation. The override is a one-time exception to the skill rule, not a precedent for future L-step packets.
