# 115 — Retire `support_sharp_tails`: remove the field, hardcode `true`

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22, from the 2026-09-01 grilling ruling **Q13**: *"remove field,
hardcode `true`"*, classed rule-out-of-scope — *"Canonical lists it in
`PrintConfig.cpp`'s obsolete-key `ignore` set (rule 3) and froze behaviour at
`g_config_support_sharp_tails = true`; port runs it off."* Flagged for its own
ticket because it is a **geometry change needing its own verification**, unlike
the two sibling severed-plumbing keys (`enforce_support_layers`,
`bridge_no_support`) which were ruled in-session fixes and which packet 265 has
since handled for `enforce_support_layers`.

Verified in-tree (2026-09-02):

- `crates/slicer-ir/src/resolved_config.rs` declares the CLI/typed field
  `support_sharp_tails: bool = true`.
- `crates/slicer-core/src/algos/overhang_annotation.rs` carries the consuming
  params field.
- `resolve_contact_params`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) hardcodes
  `support_sharp_tails: false`, under a comment claiming these knobs *"have no
  production config source yet"* — which is false; the field exists and is
  emitted into the config map.

So the port declares `true`, never reads it, and runs the behaviour **off**. Both
the declared default and canonical's frozen behaviour say `true`. Flipping it is
a real geometry change on every model with sharp tails — which is why this is a
ticket and not a one-line edit.

Decide and execute:

1. **Confirm canonical's frozen state** — that the key is genuinely in
   `PrintConfig.cpp`'s obsolete-key `ignore` set and that
   `g_config_support_sharp_tails` is fixed `true` in `libslic3r.h`. Cite by file
   + symbol, never line numbers.
2. **Remove the config key** (the `ResolvedConfig` field and any manifest
   declaration) rather than wiring it — a key canonical no longer honours should
   not be a knob here either. Confirm no back-compat alias is warranted; an old
   profile setting it will fall into `extensions` silently, which is the same
   deliberate break as Q14(b) and ticket 107.
3. **Hardcode the behaviour `true`** at the consumer and delete the misleading
   comment in `resolve_contact_params`.
4. **Verify the geometry change**, which is the substance of this ticket:
   identify which fixtures change output, re-baseline them with measured
   justification, and confirm the change is the sharp-tail behaviour appearing —
   not something else moving.
5. Check whether `bridge_no_support` was fully handled by packet 265 or is still
   severed; the audit grouped the three together and only one has a recorded
   resolution.

Out of scope as a queue key (dead in canonical, rule 3), so it does not change
the queue count.

## Answer
