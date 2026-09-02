# 109 — Rename `ironing_speed` → `support_ironing_speed` and decide `SPEED_KEYS` membership

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22 (P15 authoring), which found this decision sitting inside the
manifest it was specifying and deliberately did not fold it in.

The 2026-09-01 grilling ruled (**Q11(a)**, `key-correction-inventory.md`
§Decisions): rename `support-surface-ironing`'s `ironing_speed` to
`support_ironing_speed`, keep the `30.0` default, and flag a follow-up to decide
the feedrate-table membership. Rationale recorded there: it matches its siblings
`support_ironing_flow` / `support_ironing_spacing` and removes a name collision
that has no canonical basis — one key name, two manifests, two different defaults
(`top-surface-ironing` declares `20.0`, matching canonical `coFloat 20`;
`support-surface-ironing` declares `30.0`, and canonical derives support ironing
speed inside `SupportParameters` rather than from a second `ironing_speed`
default).

Two things must be decided together:

1. **The rename itself.** Manifest table, the `from_config` read in
   `modules/core-modules/support-surface-ironing/src/lib.rs`, the module's two
   test binaries, the support integrated-parity contract test, and a guest
   rebuild (guest WASMs embed config key names — ticket 101's byte-search).
2. **Feedrate-table membership.** The grilling row names a table
   `FEEDRATE_KEYS`. **That symbol does not exist in this tree** — verified by
   ticket 22's preflight. The real table is `SPEED_KEYS`
   (`crates/slicer-ir/src/feedrate.rs`), a `&[(&str, fn(&mut FeedrateConfig) ->
   &mut f32)]` registration list. Decide whether the renamed key joins it, and
   what that means for the module-side read: today the module computes its own
   `speed_factor` from `ironing_speed / BASE_SPEED`, so a host feedrate arm and
   a module-side factor would be two mechanisms for one quantity — the shape
   Authoring rule 5 forbids.

Note the standing hazard when checking defaults here: for a plain-typed key that
also has a `ResolvedConfig` field, the manifest `default =` is dead (map Notes,
grilling carried finding 1). Establish which value actually reaches the module
before asserting anything about `20.0` vs `30.0`.

Not a queue key: `support_ironing_speed` is a PnP naming fix with no canonical
counterpart, so it does not change the queue's key count either way.

## Answer
