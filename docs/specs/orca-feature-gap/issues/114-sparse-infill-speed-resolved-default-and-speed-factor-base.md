# 114 — `sparse_infill_speed`: align the `ResolvedConfig` default and re-base `speed_factor`

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22, from the 2026-09-01 grilling ruling **Q11(b)**:
*"`ResolvedConfig` default 50.0 → 100.0; `speed_factor` relative to resolved
default, not `BASE_SPEED`"* — because *"modules receive the `ResolvedConfig`
value via `to_config_map`, shadowing the manifests' 100.0; `BASE_SPEED = 50.0`
only coincidentally yields factor 1.0."*

This is the **confirmed instance** of the map's standing hazard (carried finding
1): for a plain-typed key that also has a `ResolvedConfig` field, the manifest
`default =` is dead. Ticket 107 aligned three manifests to canonical `100`, and
every module still receives `50.0`. The two numbers currently cancel out — a
`50.0` value against a `BASE_SPEED` of `50.0` gives factor `1.0`, which is
canonical's 100 mm/s — so **today's output is right by coincidence**, and any
change to either number alone breaks it. That is exactly the state a future agent
"fixes" into a regression.

**Correct the ruling's scope before designing.** Q11(b) says it *"touches 3
infill modules"*. Verified against the tree (2026-09-02) — the three do not share
one shape:

- `modules/core-modules/gyroid-infill/src/lib.rs` — has `const BASE_SPEED: f32 = 50.0;`
- `modules/core-modules/lightning-infill/src/lib.rs` — has `const BASE_SPEED: f32 = 50.0;`
- `modules/core-modules/rectilinear-infill/src/lib.rs` — **has no `BASE_SPEED`**.
  It uses a local `configured_base_speed` and, on at least one path, a literal
  `let speed_factor = 1.0;`.

So it is two modules to re-base plus one to investigate, not three of a kind.
Establish what rectilinear actually does before changing anything, or the
"alignment" will silently change only two of the three.

Also in scope for the same reason (same key, third spelling): the host
`FeedrateConfig.sparse_infill_speed` carries `100.0`. Ticket 107 left it
untouched deliberately. Three declarations of one key with two different values
is the condition to end here.

Decide and execute:

1. Move the `ResolvedConfig` default (`crates/slicer-ir/src/resolved_config.rs`)
   from `50.0` to canonical `100.0`.
2. Re-base each module's speed factor on the resolved default rather than a
   module-private constant, so the factor means the same thing everywhere and
   cannot drift from the config.
3. Resolve rectilinear-infill's different shape into the same contract.
4. **Prove output is unchanged at defaults** — this is the whole risk. A
   before/after G-code comparison on a fixture that actually emits sparse infill,
   not just a unit test on the factor arithmetic.
5. Decide whether `speed_factor`-style module-private base constants are a
   pattern to retire generally; the same shape may exist for other speed keys.

Not a queue key (it is a defaults/plumbing correction, not a gap), so the queue
count is unchanged. Ticket 109 raises the same "two mechanisms for one quantity"
question for `support_ironing_speed`; whoever takes either should read both.

## Answer
