# 126 — Close `overlay_resolved`'s 29-of-83 field narrowing, and prove the precedence defect

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-06)
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

**Both parts resolved by direct implementation; no packet** (map's
"Packets are for complex implementation only" rule — the decision points
already existed and the whole fix is one macro-emitted method plus call-site
origin threading, small enough for this session).

### Part 2 verdict: the defect reproduces, and it is fixed outright (no deviation)

The ticket's suspected shape reproduces exactly through the full production
resolution stack (`resolve_global_config` → `resolve_per_object_configs` →
`resolve_per_paint_semantic_configs` → `resolve_per_tool_configs` →
`execute_region_mapping_inner` with `host_config = Some((per_object, global))`,
the same calls `prepass.rs` / `commit_region_mapping_builtin` make):
global `line_width = 0.6`, paint semantic `0.5`, tool 1 overriding only
`retract_length` — the tool-1 painted region came out `0.6`, not `0.5`.
`tool_overlay_silent_on_a_key_keeps_the_paint_semantic_value`
(`crates/slicer-core/tests/algo_region_mapping_tdd.rs`) failed red with
`left: 0.6, right: 0.5` before the fix and passes after.

**Design correction to the ticket's suggested fix.** The ticket offers
"compare against the base being overlaid" as an alternative. That alternative
is wrong for the tool-inheritance case and was not taken: when the tool
overlay is composed, the base already carries the paint value (0.5), while
the overlay carries the inherited global value (0.6) — comparing against the
base would *still* copy 0.6 over 0.5. The correct fixed point is the
**overlay's resolution origin** — the config the overlay was built from
(the global config for paint/tool overlays, `ResolvedConfig::default()` for
modifier deltas built from defaults). An inherited field equals its origin by
construction, so "differs from origin" identifies *exactly* the keys the
overlay explicitly set. This also fixes the ticket's second observation in
the same move: "override back to the default" is now expressible (an
explicit 0.0 differs from a 0.6 origin, so it copies), pinned by
`paint_semantic_overriding_a_field_back_to_its_default_is_expressible`.

The same root cause lived in the `extensions` half: the old blanket merge
copied inherited global module keys over per-object overrides. Origin
comparison now skips keys equal to the origin's entry, pinned by
`paint_overlay_does_not_clobber_object_extension_overrides`.

### Part 1 verdict: the narrowing is closed structurally, not by a longer list

`ResolvedConfig::overlay_onto(base, overlay, origin)`
(`crates/slicer-ir/src/resolved_config.rs`, emitted by the
`declare_resolved_config!` / `__drc!` macro itself via the new
`__drc_overlay_arms!` helper, which re-parses the accumulated struct-field
tokens) carries one copy arm per declared field — currently 73 `cli` /
`cli_opt` rows plus 3 `plain` rows (re-derived from disk; ticket 118's
"79+4" was that session's ledger fact) — so adding a declaration row adds its
overlay arm with no hand-maintained list to forget. The old
`overlay_resolved` hand-enumerated 28 arms and is now a thin origin-aware
delegate. `execute_region_mapping_inner`
(`crates/slicer-core/src/algos/region_mapping.rs`) threads the origin
(global in the host path, `ResolvedConfig::default()` in the legacy
no-host path, where callers build overlays from defaults) to all five
composition sites (paint fold, tool fold, modifier-child paint/tool folds;
modifier stamping uses default-origin, preserving its behaviour exactly).

`tool_override_on_a_field_outside_the_old_allowlist_reaches_the_region`
proves it on `retract_length` (a declared field the old diff had no arm
for): red before (`left: 2.0, right: 5.5`), green after.

The structural guarantee is pinned in `slicer-ir` itself, not just at the
call site:
`overlay_onto_drift_guard::explicit_overrides_reach_the_composed_config_for_every_declared_field`
drives every `host_config_keys()` entry (also macro-derived, so a new
declaration row is automatically covered) through `apply_cli_key` with a
wire-typed sentinel and asserts whole-struct `PartialEq` equality of the
composed config with the overlay; the sibling test pins inherited-skip and
reset-to-default. No config key declared, no packet number taken.

### Verification (all green)

- `cargo test -p slicer-ir` — 36 lib incl. the 2 new drift-guard tests
- `cargo test -p slicer-core --features host-algos` — full crate green,
  incl. `algo_region_mapping_tdd` 28/28 (4 new)
- `cargo test -p slicer-scheduler` — full suite green (the only red seen
  was the stale-`pnp_cli.exe` harness trip; rebuilt the binary, green)
- `cargo test -p slicer-runtime --test unit` — 89/89
- `cargo test -p slicer-runtime --test integration` — 345/345 (1 red was
  the same stale-binary trip, green on re-run)
- `cargo test -p slicer-runtime --test e2e` (paint/modifier subset) —
  14 passed; 4 failures are **not this ticket**: all four are ticket 140's
  class, failing at config-resolution ingest with
  `filament_flush_volumetric_speed: expected Float value, got List` — the
  fixture carries it as a 4-element Orca `coFloats` string list and ticket
  47's strict scalar extractor rejects it before region mapping ever runs
  (verified `left`/`right`: the error fires in `resolve_global_config`,
  upstream of this ticket's composition code; a stash-baseline was
  attempted but is invalid in this tree — see below).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo xtask check-literals` — 0 violations
- `cargo xtask build-guests --check` — exit 0, 0 stale (46 guests rebuilt;
  the `slicer-ir` edit sits in every guest's dependency closure)

**Stash-baseline caveat (for the record):** this tree carries other
sessions' uncommitted work (tickets 43–50 follow-ups: `run.rs`,
`layer_executor.rs`, `machine-gcode-emit`, tickets' assets). Stashing only
this ticket's files breaks compilation because the other sessions' edits to
the same `resolved_config.rs` ride in the same file diff — reverted by the
stash while their `run.rs` (ticket 50's `manual_filament_change` read site)
stays. So the e2e attribution above rests on the error's provenance
(strict-extractor `TypeMismatch` at resolution, a seam this ticket never
touches) plus the fixture's measured list spelling, not on a stash cycle.
Nothing of theirs was edited, committed, or rebuilt-around by this ticket
beyond the shared guest rebuild the freshness gate required.

### For ticket 125

The composition prerequisite is lifted: every declared-field override on
every axis (object, paint, tool) now reaches the region config, precedence
is `global < per_object < per_paint_semantic < per_tool` by test, and an
explicit reset to default is expressible. What 125 still owns: Orca
`coFloats`/`coBools` vector ingest onto the `tool_config:` axis (ticket
140's reds — now joined by ticket 47's `filament_flush_*` scalars — are the
standing evidence). The old 28-field allowlist is gone; overlay coverage
rides the macro's 73+3 field set from here on.
