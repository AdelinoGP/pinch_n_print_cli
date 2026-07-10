---
status: implemented
packet: benchy-acceptance-evidence
task_ids:
  - TASK-135
---

# 21_benchy-acceptance-evidence

## Goal

Extend the real Benchy end-to-end acceptance suite so it asserts the final live output now contains support, top/bottom fill, seam, and retract/unretract evidence on the canonical Workstream 3 path, using feature-fragment assertions instead of a byte-for-byte Orca golden file.

## Problem Statement

The repo already has a real Benchy end-to-end test harness, but it stops at structural or MVP-content checks. The remaining Workstream 3 closure gap is feature evidence: once the underlying producer packets land, the acceptance suite needs to assert that the final live output actually contains support, top/bottom fill, seam, and retract/unretract evidence. This packet deliberately avoids byte-for-byte golden diffing because the repo does not stage an Orca golden Benchy artifact. Instead it chooses feature-fragment assertions on the real path.

## Architecture Constraints

- Selected approach: extend the existing real Benchy suite with feature-fragment assertions instead of a byte-for-byte Orca golden diff.
- The packet must stay on the real binary + real module tree + real Benchy fixture path.
- Feature evidence must fail with targeted diagnostics naming the missing family.

## Data and Contract Notes

- IR or manifest contracts touched:
  - final emitted `.gcode` feature fragments from packet `11`
  - intermediate seam evidence from packets `14` and `15`
- WIT boundary considerations:
  - none in this packet; it is purely end-to-end acceptance coverage
- Determinism or scheduler constraints:
  - repeated identical Benchy runs must remain byte-deterministic

## Locked Assumptions and Invariants

- The acceptance suite uses the real `resources/benchy.stl` fixture and the real core-module tree.
- Feature-fragment assertions are the chosen parity evidence until a committed golden artifact exists.

## Risks and Tradeoffs

- Risk: a feature fragment may appear spuriously without correct semantics. Mitigation: use both final text evidence and intermediate seam evidence where text alone is weak.
- Risk: the suite can become brittle if it assumes one exact path count. Mitigation: assert presence, balance, and targeted fragments rather than entire file equivalence.
