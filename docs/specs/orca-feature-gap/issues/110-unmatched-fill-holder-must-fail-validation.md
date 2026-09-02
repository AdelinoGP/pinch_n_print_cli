# 110 — An unmatched `*_fill_holder` must fail validation

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22, from the 2026-09-01 grilling ruling **Q3(b)**
(`key-correction-inventory.md` §Decisions): *"unmatched holder must fail
validation"* — because a holder key naming a module no manifest matches
currently yields a silently hollow part, and no `SchedulerError` variant covers
it. This is load-bearing for the whole holder-only mechanism: **Q3(a) / Authoring
rule 4 removes ten algorithm-selecting enums from the declared-key set and makes
`*_fill_holder` the only selection channel.** If a mistyped or unshipped holder
name fails silently, that mechanism has no safety net, and packets
`260b-support-interface-fill-claim-holders` and `262b-infill-pattern-holder-mapping`
are building on it.

**Correct the ruling's premise before designing.** Q3(b)'s rationale says
`resolve_held_claims` "currently yields empty for every module". Verified against
the tree (2026-09-02) — **that is wrong**. `resolve_held_claims`
(`crates/slicer-scheduler/src/validation.rs`) filters the module's declared
claims to those in `FILL_CLAIM_IDS` whose configured holder matches the module
id; it returns non-empty whenever the holder does match (the default
`sparse_fill_holder = "rectilinear-infill"` matches `rectilinear-infill`), and
empty only for non-matching modules — which is its job. The real gap is narrower
and still real: **nothing detects the case where a holder key names a module that
no loaded manifest matches at all**, so every module filters itself out, no
module holds the claim, and the stage produces nothing.

Verified facts to build on (re-derive at point of use):

- Holder keys are exactly four — `sparse_fill_holder`, `top_fill_holder`,
  `bottom_fill_holder`, `bridge_fill_holder` — declared as CLI/typed fields in
  `crates/slicer-ir/src/resolved_config.rs`, each defaulting to
  `"rectilinear-infill"`.
- `FILL_CLAIM_IDS` and `resolve_held_claims` live in
  `crates/slicer-scheduler/src/validation.rs`.
- `SchedulerError` (same file) has no zero-holder variant. `ClaimConflict`
  covers *two* holders for one claim, not *none*.

Decide:

1. **Where the check lives** — validation time (holder name resolves to no
   loaded module) is the obvious seam, but confirm it can see the full module
   set at that point.
2. **The new `SchedulerError` variant** — its name, its payload (claim id,
   holder name, and the candidate module ids?), and its message.
3. **Whether a holder naming a real module that does not *declare* the claim is
   the same error or a different one.** These are two distinct failure modes and
   the grilling did not separate them.
4. **Blast radius on existing tests and fixtures** — anything that currently
   sets a holder to a name that does not resolve will start failing, which is
   the point, but it needs a sweep before landing.

Scheduler-scoped; not a queue key, so it does not change the queue count.

## Answer
