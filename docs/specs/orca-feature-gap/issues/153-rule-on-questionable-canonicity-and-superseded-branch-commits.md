# 153 — Rule on questionable "canonicity" and superseded branch commits

Type: grilling
Status: resolved
Assignee: wayfinder session (2026-11-26)
Blocked by: 152
Map: ../map.md

## Question

**For each `questionable-for-ruling`, `superseded-by-master`, and `defective` commit surfaced by [152 — Deep audit of non-rename behavioural and "canonicity" commits on `orca-feature-gaps`](./152-deep-audit-non-rename-branch-behaviour-commits.md), what is the ruling: keep as-is (recording a divergence if appropriate), redesign/fix the PnP way, or revert/remove?**

Read `152`'s linked audit asset (`./152-asset-branch-behaviour-audit.md`) in full before grilling — do not re-derive its commit survey. Group related findings into crisp decision rounds (e.g., config/ingest commits, default-value flips, geometry/partition commits, dispatch/fallback commits) and put each non-obvious decision to the user with a concrete recommended disposition.

### Obligations

- **HITL ticket:** call `/grilling` and `/domain-modeling`. Never answer for the human.
- **Plan, don't do:** record the per-commit ruling in `## Answer` and graduate concrete follow-up `task` tickets (one per cohesive remediation unit) onto `map.md` (wired into `## Not yet specified` or created directly when sharp). If any ruling invalidates an open ticket or draft packet, update its `Blocked by:` / cross-reference notes in the same pass.
- Update `map.md` `## Decisions so far` and, where a ruling sets a standing precedent on "canonicity" vs PnP architecture, record it in `## Notes`.

## Answer

Ruled 2026-11-26 against
[152's audit asset](./152-asset-branch-behaviour-audit.md). Every
`questionable-for-ruling` and `superseded-by-master` item is disposed; the
three defects were fixed under the branch's "fix the code first" rule.

| item | ruling | landing |
|---|---|---|
| `71a4c832` fifth fill role | adopt and document the invariant | `cb2fb82c`, DEV-208 |
| `ac035ed8` claim-holder scope | validate every resolved scope | `e4f28a56` |
| `a21e70ee` flush placeholders | implement the canonical fallbacks | `5f0a8501` |
| `f16c9ca5` speed floor | allow 0 on the three ADR-0072 AUTO keys | `70f00731` |
| `202f812d` overlay engine | delete/unify immediately after the merge | [162](./162-delete-production-dead-overlay-engine.md) |
| `e7404ff1` `support_base_pattern` | master wins; reconcile 135 to holder-only | `af5e79b0` |
| `4cdf5692` `fatal=false` | honour on all dispatch paths | `5948de87` |
| `b33f25f6` legacy bridge flag | delete; select per polygon | `43feefea` |
| `2d8d71c1` six `*_filament_id` | declare and test through the live registry | `795f98a3` |

Defects fixed without a separate ruling: `4cdf5692`, `b33f25f6`, `2d8d71c1`
(rows above), and the region-partition stale assertion added by `1c027a3c`
(`20fa929f`).

Standing precedents recorded:

- A fifth canonical fill role is adopted. The four-role master invariant is
  superseded and recorded as DEV-208, awaiting an ADR.
- A host config authority is the only supported precedence path. The
  `host_config: None` compatibility engine is production-dead and scheduled
  for deletion in ticket 162; do not add tests against it.
- `ModuleError::fatal=false` is a dispatch-wide contract, not a wasm-only one.
- A canonical default that changes emitted output is aligned to the Orca
  source, not to `docs/ORCA_CONFIG_REFERENCE.md`; a doc/source disagreement is
  resolved in favour of `PrintConfig.cpp`.
