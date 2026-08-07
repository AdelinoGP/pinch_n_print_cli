# 01 — Build a mechanically verified FFF gap inventory

Type: research
Status: resolved
Blocked by: —
Map: ../map.md

## Question

What is the *actual* set of FFF OrcaSlicer config keys Pinch 'n Print does not
implement — derived mechanically, not read off the hand-maintained "In Codebase"
column of `docs/ORCA_CONFIG_REFERENCE.md`?

The ✅/❌ column is hand-edited. `xtask/src/gen_config_docs.rs` reads that file
only for the `Default` column (deviation detection); nothing validates the
presence flags. Every downstream decision in this map — tiering, granularity,
packet count, numbering range — is sized off this number, so an unverified
inventory poisons the whole route.

Produce a gap inventory by diffing the reference's key list against the live
registries: module `[config.schema]` manifests under
`modules/core-modules/*/*.toml`, host keys in `docs/config/host-keys.toml`, and
any `ConfigView::get_*` call sites that read a key without declaring it.

Deliver as a linked asset (`gap-inventory.md` beside this ticket) with, per key:
Orca key name, Orca section, present/absent, and where it is registered if
present. Report the discrepancy count against the ❌ column explicitly — a large
discrepancy is itself a finding worth surfacing to the map.

Exclude SLA sections (out of scope). Do **not** fix the column in this ticket;
whether to regenerate it is a separate question left in the map's fog.

## Answer

Asset: [`01-asset-gap-inventory.md`](./01-asset-gap-inventory.md) — per-section
summary, full per-key table (574 rows), and the rename-adjudication appendix.

### Headline

**The FFF gap is between 419 and 481 keys — not the ~640 the ❌ column implies.**
The band is the unresolved rename question (below); the count cannot be
collapsed to a single number in this ticket without adjudicating renames, which
is ticket 03's job.

| | count |
|---|---:|
| Orca FFF keys in scope (Quality → Printer/Machine, SLA excluded, deduped) | 574 |
| …the ❌/✅ column claims missing | 529 |
| …the ❌/✅ column claims present | 45 |
| …exact key name found live in the tree | 93 |
| …exact key name **not** found live | 481 |

### The ❌/✅ column is wrong on 66 of 574 keys (11.5%)

Both directions, and the two directions fail differently:

- **57 keys marked ❌ are actually live.** These are *false gaps* — packet work
  that would have been specced and then found already done. They cluster hard:
  the whole of Quality/Line width (6), Wall generator — Arachne (6),
  Speed/Overhang speed (8), Quality/Overhangs (5). Full list in the asset.
- **9 keys marked ✅ were not found under their Orca name.** Eight of the nine
  (`wall_loops`, `fuzzy_skin`, `support_type`, `extruder`, `wipe`,
  `support_filament`, `support_interface_filament`, `disable_m73`) do appear in
  the tree and are reached through typed struct fields rather than string
  lookups — an extraction blind spot, not a gap. The ninth,
  `initial_layer_print_height`, has **zero** occurrences under that spelling; it
  is implemented as `first_layer_height` (`crates/slicer-ir/src/resolved_config.rs`).

So the column's error rate is real but it is not uniformly pessimistic — it
over-reports gaps far more than it under-reports them.

### The finding that matters more than the count: a pervasive rename layer

Pinch 'n Print does **not** use Orca's key names. Of 154 declared config keys
(123 in module `[config.schema]` manifests + 42 in `docs/config/host-keys.toml`),
**62 have no exact Orca counterpart**. Confirmed renames found while checking:

| Pinch key | Orca key |
|---|---|
| `first_layer_height` | `initial_layer_print_height` |
| `seam_mode` | `seam_position` |
| `wall_count` | `wall_loops` |
| `infill_density` | `sparse_infill_density` |
| `wipe_tower_*` | `prime_tower_*` / `enable_prime_tower` |
| `retract_length` / `retract_speed` | `retraction_length` / `retraction_speed` |
| `ironing_enabled` / `ironing_flow_rate` / `ironing_spacing_mm` | `ironing_type` / `ironing_flow` / `ironing_spacing` |
| `support_top_z_distance_mm` | `support_top_z_distance` |
| `enable_overhang_fan` | `enable_overhang_bridge_fan` |

Consequence: **481 is an upper bound on the gap, not the gap.** At most 62 of
those 481 are already implemented under a Pinch name (each unmatched declared
key can absorb at most one Orca key), giving the 419 floor. The floor is loose —
several of the 62 are genuinely Pinch-specific (`slice_has_paint`,
`apply_to_all`, `thumbnail_path`, `path_optimization_emit_layer_markers`, the
`*_fill_holder` keys) — so the true figure sits nearer 481 than 419.

This also means the pre-existing rule in `CLAUDE.md` about snake_case is not the
only naming convention in play here: there is an undocumented Orca→Pinch key
alias map that exists only implicitly, scattered across manifests.

### Reproduction

Re-derive rather than trusting the numbers above (they are ledger facts and will
rot as packets land):

```bash
# Orca FFF keys, with section + the hand-maintained flag
sed -n '17,876p' docs/ORCA_CONFIG_REFERENCE.md \
  | awk -F'|' '/^## /{s=$0;sub(/^## /,"",s)} /^### /{b=$0;sub(/^### /,"",b)}
               /^\| "/{k=$2; gsub(/[ "]/,"",k);
                       print k"\t"s"\t"b"\t"(($0~/✅/)?"present":"missing")}' \
  | awk -F'\t' '!seen[$1]++' > /tmp/orca_fff.tsv

# Live declared keys
grep -h '^\[config\.schema\.' modules/core-modules/*/*.toml \
  | sed 's/^\[config\.schema\.//;s/\]$//' > /tmp/live.txt
grep -hE '^[a-z_][a-z_0-9]*  *= *\{' docs/config/host-keys.toml \
  | sed 's/ *=.*//' >> /tmp/live.txt
sort -u /tmp/live.txt -o /tmp/live.txt
```

The `sed -n '17,876p'` bound is the line range from `## Quality` to just before
`## SLA Printing`; re-derive it with `grep -n '^## ' docs/ORCA_CONFIG_REFERENCE.md`
rather than reusing those numbers.

### Caveats — do not over-trust this inventory

1. **Exact-name matching only.** `live=no` means "this Orca spelling is not in
   the tree", not "this feature is unimplemented". The rename layer is the whole
   reason for the 419–481 band.
2. **Declaration ≠ consumption.** A key present in a manifest may be declared and
   never read, or read and ignored at the decision point. Ticket 04's Tier A/B
   split depends on this distinction and cannot inherit it from here.
3. **The `get_*` scrape has false positives.** It matched test-fixture literals
   (`b`, `fail`, `secret`, `intentional_error_code`). Those inflate the *live*
   union slightly, which makes the gap count if anything conservative. The
   declared-key figures (154 / 62) are clean — they come from manifests only.
4. **Not wired into CI.** Nothing prevents the ❌ column drifting again.

### Handoffs

- **Ticket 03** inherits the 62-key rename-adjudication pool. Its "keys we
  already provide under a different name" exclusion class is now sized and
  listed, not hypothetical.
- **Ticket 04** must not treat "declared in a manifest" as Tier A; see caveat 2.
- **New question for the map's fog:** the Orca→Pinch alias map is undocumented.
  Whether it gets written down (and whether the ❌ column is replaced by
  generated output gated in `gen-config-docs --check`) is now a sharper question
  than when the map was charted.
