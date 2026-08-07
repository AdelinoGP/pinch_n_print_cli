# 04 — Define the cost rubric that makes "cheapest-first" decidable

Type: grilling
Status: open
Blocked by: 01
Map: ../map.md

## Question

What makes a missing feature "cheap" in *this* codebase, and how is every
in-scope key assigned a tier without hand-arguing each one?

The effort's chosen ordering is cheapest-first, which is only actionable if
"cheap" is a mechanical classification. Draft rubric to grill and sharpen:

- **Tier A — pure config plumbing.** Key declared in an existing module manifest
  or `docs/config/host-keys.toml` and consumed at a decision point that already
  exists. No IR change, no WIT change, no new module.
- **Tier B — new logic in an existing module.** Key drives new behaviour inside
  a module that already owns the relevant stage.
- **Tier C — new surface.** Requires a new core-module, a new IR field, a WIT
  change, or a new host-service bridge arm — i.e. anything the repo gates behind
  an ADR and a guest-WASM rebuild.

Settle: are three tiers enough? What is the tie-breaker *within* a tier — Orca UI
section, owning module, or print-quality impact? Does a key that is Tier A but
whose consumer decision point is itself missing get demoted to Tier B, and how
is that detected without reading every module?

Output: the rubric, plus the tier assignment applied to the in-scope inventory
(as a linked asset), since the tier counts directly size the packet queue.
