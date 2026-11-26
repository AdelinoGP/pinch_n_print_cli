# 112 — Derive the CONFIG_BLOCK padding table from the resolved config

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-04)
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

**Resolved 2026-09-04 — decided, not executed. The four questions are answered from measurement; the implementation is ruled a packet, filed as [132](132-author-packet-config-block-reader-contract.md), because the investigation found a live correctness bug on the resolved-config side of the same seam that is larger than the padding table this ticket was scoped to.**

Everything below was measured in this session against the tree, or read from the designated canonical oracle (the ticket-33 checkout; delegated read, cited by file + function only). No code changed.

### The premise inverts: the hardcoded table is the *safe* part

The ruling's concern was that hardcoded padding values drift from what the slicer did. The measurement says the opposite is the live hazard. `ORCA_CONFIG_PADDING` spells its values the way canonical's parser wants — bools as `1`/`0`, `wall_generator` as lowercase `arachne`. The **resolved-config emission**, which shadows padding and is supposedly the source of truth, does not:

- **`ConfigOptionBool::deserialize` (`Config.hpp`) accepts only `"1"` and `"0"`.** `emit_config_kv`'s `ConfigValue::Bool(b) => b.to_string()` (`crates/slicer-gcode/src/serialize.rs`) emits `true` / `false`. The G-code viewer path calls `load_from_gcode_file` with `ForwardCompatibilitySubstitutionRule::EnableSilent` (`GCodeProcessor.cpp`), so the coBool fallback in `ConfigBase::set_deserialize_raw` fires and routes the value through `ConfigHelpers::enum_looks_like_true_value`, which matches only `"enabled"` / `"on"`. Result: **every `true` this port emits reads back in OrcaSlicer as `false`** — silently, unlogged under `EnableSilent`, and still counted toward the floor. Measured in a real emitted file (`crates/slicer-runtime/target/infill_overlap_0_30.gcode`): `support_remove_small_overhang = true` and `support_sharp_tails = true` are both corrupted this way. Five resolved bools take this path (`enable_support`, `bridge_no_support`, `support_critical_regions_only`, `support_remove_small_overhang`, `support_sharp_tails`); `false` survives only by coincidence, because the fallback's miss also yields `false`.
- **`wall_generator` is emitted as `format!("{:?}", self.wall_generator)`** (`crates/slicer-ir/src/resolved_config.rs`) → `Classic` / `Arachne`. Canonical's keyword map (`PrintConfigDef::init_fff_params`) is lowercase `classic` / `arachne`. An unmapped enum keyword takes the same fallback's `else` arm, which does `opt->set(optdef->default_value.get())` — it **resets the option to canonical's declared default**. `infill_type` is Debug-formatted too, but canonical has no such key at all, so it is dropped.
- **This exact bug was already found and fixed once, for one enum only.** `SupportType::as_canonical_str` (`crates/slicer-ir/src/slice_ir.rs`) exists precisely because `format!("{:?}")` emitted `Tree` and "fell through to the traditional family" — the comment in `to_config_map` says so. `WallGenerator` and `InfillType` never got the same treatment.

None of this aborts the load: `BadOptionValueException` is a sibling of `UnknownOptionException` (both derive from `ConfigurationError`) and would escape `load_from_gcode_file`'s catch, but the substitution fallback means it is never thrown on this path. The failure mode is silent wrong values, which is worse to find and better to fix.

### 1. The derivation source

**Module manifest `[config.schema]` defaults, via `ConfigFieldEntry.default`** — which holds a string for *every* field type. The percent-only `parsed_default` that feeds `ConfigBoundsIndex::schema_defaults` is the wrong door; the raw `default` is right there on the same struct (`crates/slicer-scheduler/src/manifest.rs`).

**The map's standing hazard inverts here, and that is what makes this safe.** The Notes warn that for a plain-typed key with a `ResolvedConfig` field the manifest default is dead. True — but such a key is already emitted by `to_config_map`, so `emit_config_kv`'s dedup means padding never fires for it. Exactly the keys padding covers are the keys whose manifest default *is* the live value.

Measured taxonomy of the 69 rows:

| class | count | what it means |
|---|---|---|
| dead — shadowed by `to_config_map` | 9 | value never reaches output |
| live, module-declared → derivable | 24 | should equal the manifest default; several do not |
| live, declared by no module | 36 | nothing in-tree can derive them |

The 9 dead rows are `enable_support`, `sparse_infill_density`, `bottom_shell_layers`, `printable_height`, `wall_loops`, `top_shell_layers`, `infill_direction`, `wall_generator`, `support_type`. Ticket 107's note that the `sparse_infill_density = 15%` twin "is now deduped by the typed 20.0 emission" describes this mechanism.

Among the 24 derivable rows the padding value and the manifest default disagree widely — `brim_width` 0 vs 8.0, `skirt_loops` 1 vs 6, `skirt_distance` 2 vs 3.0, `slow_down_layer_time` 8 vs 5.0, `tree_support_branch_angle` 40 vs 45.0, `raft_first_layer_density` 90% vs 0.4, `filter_out_gap_fill` 0 vs 0.5, `wipe_tower_x` 15 vs 10.0, `wipe_tower_y` 220 vs 10.0. **This corrects a ledger fact in the ticket body:** it says `skirt_loops` / `skirt_distance` / `brim_width` "were realigned by Q14(a)". Re-verified today, they were not — whatever Q14(a) realigned, it was not the padding table.

Two derivation rules the packet still owes: 3 keys whose modules disagree on the default (`inner_wall_speed` 45/60, `line_width` 0/0.4, `outer_wall_speed` 30/60) and 26 declarations carrying no `default` at all (24 in `arachne-perimeters`, 2 in `wipe-tower`). Deriving the pool is not the hard part — 160 module keys carry a default, against a floor of 80.

### 2. How the ≥80 floor stays guaranteed

**It is not guaranteed today, and the existing test does not test it.** Canonical's counter increments *inside* the `try`, after `set_deserialize` returns without throwing:

```cpp
try {
    this->set_deserialize(key, value, substitutions_ctxt);
    ++ key_value_pairs;
} catch (UnknownOptionException & /* e */) {
    // ignore
}
```

So a key canonical does not recognise contributes **nothing** to the 80. The port's real margin is therefore smaller than the 96 lines it emits, and by an unmeasured amount: 6 canonical-unknown keys were identified among the emitted set (`support_material`, `infill_type`, and the four `*_fill_holder` keys), but the full emitted key set was **not** classified against `PrintConfigDef` in this session — so **the true accepted count is unmeasured**. Ticket 132 measures it before and after.

Meanwhile `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` asserts `key_count >= 80` counting *lines*. That is the wrong quantity: it would stay green through a regression that drops the accepted count below canonical's threshold. Replacing it is a ticket-132 obligation.

The guarantee the packet should build: derive the bulk from manifests, keep a residual hardcoded floor list for keys nothing in-tree owns, and pin it with a test asserting every entry is (a) live in `PrintConfigDef` and (b) declared by no loaded module. That is the anti-drift property Q5 wanted — a hardcoded row can never shadow a real value — and it is achievable, where "no hardcoded values at all" is not.

### 3. The duplicate twin pairs

**None of the three is a legacy alias. Two are keys canonical has never heard of, and the third is not in the table at all.** Checked against `PrintConfigDef` and `PrintConfigDef::handle_legacy`:

- `top_surface_pattern` — live. `top_fill_pattern` — appears nowhere in `PrintConfig.cpp`; unknown, silently dropped.
- `enable_support` — live. `support_material` (bare) — unknown, silently dropped.
- `raft_layers` — live. `support_raft_layers` — **is not in `ORCA_CONFIG_PADDING`**; the ticket's pairing is a stale claim.

Emitting both spellings is harmless for correctness — the unknown one is swallowed by the `UnknownOptionException` catch — but it is not free: it costs a line and contributes nothing to the floor. Delete `top_fill_pattern` and `support_material`. Also unknown and deletable on the same evidence: `outer_wall_direction`, `infill_first`, `extra_perimeters`. One genuine alias is present and fine: `solid_infill_filament` maps via `handle_legacy` onto `internal_solid_filament_id` (with a value translation `"1"` → `"0"`); the table emits `0`, so it maps through and counts. For real aliases the last spelling in the block wins, since `set_deserialize_raw` overwrites in file order.

### 4. Keys with no resolved value at all

The 36 live rows no module declares. Nothing in-tree can derive them, and pretending otherwise would be the same fiction the ruling is trying to remove. They stay an explicit list, canonically grounded per entry, guarded by the two-part test in item 2. This is the honest reading of "padding cannot be retired": the *table* shrinks to what only canonical can answer, and the *floor* becomes provable rather than hoped for.

### Why a packet rather than this session

Under the map's "packets are for complex implementation only" rule, this clears the bar on the code, not the key count. It spans four crates — surface `ConfigFieldEntry.default` through `ConfigBoundsIndex` (`slicer-scheduler`), thread it to the serializer through `run_postpass_with_thumbnail` and `ThumbnailAwareSerializer`, which receive no manifest state today (`slicer-runtime`), add canonical spellings to `WallGenerator` / `InfillType` on the `SupportType::as_canonical_str` precedent (`slicer-ir`), and replace `emit_config_kv`'s formatter with a type-aware canonical serializer (`slicer-gcode`) — and it fixes a correctness bug rather than plumbing a key. It also moves the exact-95 e2e canary and replaces an integration assertion. That is more than one session.

**Verification:** no code changed, so no build or test gate applies. This ticket's output is [132](132-author-packet-config-block-reader-contract.md) and a map Note.
