# 153 — Rule on questionable "canonicity" and superseded branch commits

Type: grilling
Status: open
Assignee: —
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
