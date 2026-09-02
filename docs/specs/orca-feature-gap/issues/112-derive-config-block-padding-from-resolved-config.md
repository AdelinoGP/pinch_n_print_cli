# 112 — Derive the CONFIG_BLOCK padding table from the resolved config

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22, from the 2026-09-01 grilling ruling **Q5**: the padding table
is *"derive[d] mechanically from the resolved config"* rather than hardcoded —
*"Padding is load-bearing (Orca throws below 80 keys), so it cannot be retired —
but hardcoded values must not be able to drift."* The companion ruling on the
same row: *"padding is never coverage"* (Authoring rule 2), which leaves the
existing wrong values **unowned** — no packet may claim them, so they need this
ticket.

Why it matters, and why "cosmetic" is the wrong mental model: canonical
`ConfigBase::load_from_gcode_file` (`Config.cpp`) *throws*
`Slic3r::RuntimeError` when a CONFIG_BLOCK yields fewer than 80 key-value pairs,
on the same delimited path this port emits. Padding fires only for keys the host
config map does **not** emit — which is every module-manifest-owned key — so for
those keys the hardcoded padding value is the only value a viewer or a re-slicer
ever sees. A wrong twin is a wrong answer, not a cosmetic one.

Verified in-tree (2026-09-02), all in `crates/slicer-gcode/src/serialize.rs`:

- `ORCA_CONFIG_PADDING` — **69 entries**.
- The consuming loop lives in `serialize_config_block`, with the break
  `if emitted.len() >= 96` — a deliberate margin over canonical's 80 floor.
- `emit_config_kv` is the dedup writer (`emitted.insert(key)` gates the
  `writeln!`), which is why an explicitly emitted key already wins over its
  padding twin.

The known-contradictory twins the audit collected (re-verify each; several have
been overtaken by later rulings — `skirt_loops` / `skirt_distance` /
`brim_width` were realigned by Q14(a), `sparse_infill_density` by ticket 107,
`slowdown_for_curled_perimeters` by Q9, and packets 259/262/264 corrected
`fuzzy_skin`, `sparse_infill_pattern`, and `top_surface_pattern` in place):
`slow_down_layer_time`, `detect_thin_wall`, `fan_cooling_layer_time`,
`reduce_fan_stop_start_freq`, `resolution`, plus the duplicate twin pairs
`top_fill_pattern`/`top_surface_pattern`, `raft_layers`/`support_raft_layers`,
and `support_material`/`enable_support`.

Decide and execute:

1. **The derivation source.** Module manifest defaults are the obvious source,
   but note the standing hazard (map Notes, grilling carried finding 1): for a
   plain-typed key that also has a `ResolvedConfig` field the manifest default is
   dead, and the module actually receives the `ResolvedConfig` value. Derive from
   whatever the slicer *used*, which is the resolved config — that is the whole
   point of the ruling.
2. **How the ≥80-pair floor stays guaranteed** once the values are computed
   rather than listed. The count must be provable, not hoped for; the ruling
   explicitly carries this as the follow-up's obligation.
3. **The duplicate twin pairs** — are both spellings still emitted, and is that
   correct for canonical's reader?
4. **What happens to keys with no resolved value at all**, which is the case
   padding exists to cover.

Not a queue key; changes no queue count. Touches every packet indirectly, so it
is worth landing before more packets activate.

## Answer
