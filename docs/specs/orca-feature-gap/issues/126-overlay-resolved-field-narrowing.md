# 126 — Close `overlay_resolved`'s 29-of-83 field narrowing, and prove the precedence defect

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

**Make `overlay_resolved` (`crates/slicer-core/src/algos/region_mapping.rs`)
cover every declared `ResolvedConfig` field, and settle whether its
default-comparison test causes a real precedence defect.**

Filed by [ticket 118](./118-inventory-per-tool-config-mechanism.md); the
mechanism detail is in
[118's asset](./118-asset-per-tool-config-inventory.md), *Stage 3*.

### Part 1 — the narrowing

`overlay_resolved` composes the per-object, per-paint-semantic and per-tool
overlays onto a region's effective config. It enumerates its fields **by hand**
and then merges `extensions` wholesale. At the time 118 measured it: **29**
enumerated fields against **83** declared on `ResolvedConfig`. Re-derive both
counts from disk before acting — they are ledger facts and the declaration set
moves.

Consequence: an override naming a declared field outside the enumerated set
resolves correctly in `resolve_per_tool_configs` / `resolve_per_object_configs`
/ `resolve_per_paint_semantic_configs`
(`crates/slicer-scheduler/src/config_resolution.rs`, all of which route through
`apply_overlay` → `ResolvedConfig::apply_cli_key` and so reach every CLI-bound
field) and is then **silently dropped** at composition. Module-manifest keys
survive, because they live in `extensions` and that half is a blanket merge. So
the drop is invisible in exactly the case a module author is least likely to
test.

**The fix must be structural, not a longer list.** A hand-enumerated allowlist
against a macro-declared field set is a drift generator: a new
`declare_resolved_config!` row compiles fine and is silently un-overlayable. Make
the overlay derive from the declaration — e.g. emit an overlay arm per field from
`__drc!` alongside the existing `apply_cli_key` / `host_config_keys` arms
(`crates/slicer-ir/src/resolved_config.rs`) — so adding a field cannot forget the
overlay. Verify with a test that fails when a field is added without one.

### Part 2 — the suspected precedence defect

**Read from code in ticket 118's session, not reproduced with a test.** Prove or
disprove it here before doing anything about it.

`overlay_resolved` copies a field when it differs from
`ResolvedConfig::default()`. But a per-tool (and per-object) config is built as
`global.clone()` + overrides (`apply_overlay`). So every field the **global**
config sets away from its default is non-default in the overlay too, and the
overlay writes the *global* value back over the region's effective config —
clobbering a lower-precedence override the tool never touched.

Shape of the case to write:

- global `line_width = 0.6`;
- a paint semantic sets `line_width = 0.5`;
- `tool_config:1:retract_length` is set, and says nothing about `line_width`;
- a painted region on tool 1 — expected `0.5` (documented precedence is
  `global < per_object < per_paint_semantic < per_tool`, and tool 1 did not
  override `line_width`), suspected actual `0.6`.

If it reproduces, the test is the regression test and the fix is to compare
against the **base being overlaid**, or to carry which keys an overlay actually
set rather than inferring it from inequality with the default. Note that
inferring-from-default also cannot express "override this field back to its
default value" — the same root cause, and worth fixing together.

If it does not reproduce, say why in the answer and close; the asset's claim is
then wrong and must be corrected there too.

### Why it matters to the destination

[125 — Rule on the port's per-tool config model](./125-rule-per-tool-config-model.md)
is blocked on this: any per-tool model is unmeasurable while most per-tool
overrides silently vanish. The narrowing also affects the per-object and
per-paint axes, which are already shipped, so this is a live correctness gap and
not only prerequisite work.

### Obligations

- Verification: the narrowest tests that prove both parts, plus
  `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo xtask check-literals`. Tee output to `target/test-output.log` per
  `CLAUDE.md`.
- If Part 2 reproduces, file the deviation or fix it outright — do not leave a
  measured precedence defect recorded only in a ticket.
- Declare no config key as part of this ticket.

## Answer
