# 128 — Fix the units mismatch on `percent` / `float_or_percent` config keys

Type: grilling
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

**A `percent` / `float_or_percent` key spelled as a bare JSON number is read in
the wrong unit: manifest bounds treat it as a percent, the guest treats it as a
fraction. Which unit wins, and where is the coercion fixed?**

Filed by ticket 34 (P27), which measured it on `bridge_density`. The defect is
class-wide, not specific to those keys.

### What was measured (2026-09-03, this tree)

`bridge_density` is declared `float_or_percent`, `default = "100%"`, `min = 10`,
`max = 125` in `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`.
Canonical declares it `coPercent` with the same range, where a bare `100` **is**
100%.

- `is_numeric_field_type` (`crates/slicer-scheduler/src/config_resolution.rs`)
  includes `"percent"` and `"float_or_percent"`, so `ConfigBoundsIndex::check`
  enforces `[10, 125]` against the **raw** number — `check_value`'s own comment
  says the percent→absolute base is unknown at resolve time.
- `ConfigView::get_abs_value` (`crates/slicer-ir/src/slice_ir.rs`) reads a bare
  `ConfigValue::Float` as an **absolute** value, not a percent.
- `json_to_config_value` (`slicer_scheduler::execution_plan::parse_cli_config_source`)
  never produces `Percent` / `FloatOrPercent` — a JSON number becomes `Int` or
  `Float`, a JSON string becomes `String`.

The four spellings therefore behave like this:

| JSON | bounds | value the guest sees | verdict |
|---|---|---|---|
| `"bridge_density": 100.0` | passes (10..125) | **100.0** — spacing / 100 | silently ~100x over-dense bridge |
| `"bridge_density": 1.0` | **rejected**, below min 10 | — | the fraction the guest wants is unusable |
| `"bridge_density": 100` | passes | `Int` is unhandled by `get_abs_value` → fallback `1.0` | right answer by accident |
| `"bridge_density": "100%"` | not checked (`String` skips bounds) | 1.0 | the only correct spelling, and the unvalidated one |

Canonical's own spelling (`100`) is the one that produces the worst output, and
nothing warns.

### What this ticket must settle

1. Which unit a bare number carries for a `percent` / `float_or_percent` key.
   Canonical says percent; the guest read sites currently say fraction, and
   module tests spell fractions (`.float("internal_bridge_density", 0.5)`).
2. Where the coercion lands. The schema-driven option is to parse a bare number
   for a schema-`percent` key into `ConfigValue::Percent` at ingest, so bounds and
   guests finally agree on one unit — the same route
   `ConfigFieldEntry::parsed_default` / `ConfigBoundsIndex::schema_defaults`
   already takes for manifest defaults. Changing `get_abs_value` instead has a
   blast radius across every percent key in the tree.
3. Whether `ConfigValue::String` should keep bypassing bounds
   (`check_value`'s `Bool | String => Ok(())` arm). A `"200%"` string currently
   defeats every declared range. Overlaps ticket 113 (feedrate range validation).
4. Whether `get_abs_value` should handle `ConfigValue::Int` at all, or whether
   ingest coercion makes that moot.

**Until this resolves, "the key is live" claims in this map are proven only for
the percent-string and fraction-float spellings** — the ones module tests use.
Do not read a passing module test as proof that a user's numeric profile value
reaches the decision point in the right unit.

## Answer

## Measured evidence — `sparse_infill_density` is INERT on the numeric spelling (2026-09-04)

Found by ticket 130 while probing an unrelated red; recorded here because it
sharpens this ticket's table with an end-to-end measurement rather than a
read-site inspection.

Through the `pnp_cli --config` sidecar on `resources/regression_wedge.stl`
(`slice --no-integrated-modules` with the core-module dirs):

| config | `gcode_filament_length_mm` |
|---|---|
| `{"sparse_infill_density": 5}` | 11054.1767578125 |
| `{"sparse_infill_density": 25}` | 11054.1767578125 |
| `{"sparse_infill_density": 90}` | 11054.1767578125 |
| `{"sparse_infill_density": "5%"}` | 6210.55859375 |
| `{"sparse_infill_density": "90%"}` | 33465.38671875 |

Control on the same route and the same run: `{"layer_height": 0.3}` yields 133
layers and 11332.9765625 mm against 200 layers at the default — the sidecar is
live, and this key is dead on the numeric spelling specifically.

Two corrections to this ticket's analysis:

1. The table above predicts a bare `100.0` is read as an **absolute** value
   (silently ~100x over-dense). On this path the observed outcome is **inert** —
   byte-identical output across a 18x range of inputs, so the module is keeping
   its own fallback. Inert is quieter than over-dense and harder to notice.
2. It is user-facing today. `sparse_infill_density` is a documented CLI key and
   the bare number is the spelling a user reaches for first; it silently does
   nothing.

This does not change the ruling being sought (which unit wins, and where the
coercion lands) — it raises the stakes and gives the fix a ready end-to-end
regression: the three numeric rows above must separate once the coercion is in.

## Correction (2026-09-04) — the sidecar half was a separate defect, fixed on ticket 131

The measurement recorded above ("`sparse_infill_density` is INERT on the numeric
spelling") was **not** an instance of this ticket. Ticket 131 traced it: bare
JSON `90` becomes `ConfigValue::Int` while `90.0` becomes `Float`
(`json_to_config_value` tries `as_i64()` first), modules read the raw source map
via `ConfigView::from_declared`, and `Int` fell into the `_ => None` arm of
`get_abs_value` / `get_float`. Inert, because the module kept its fallback. Fixed
by giving `Int` the same arm as `Float`.

That explains the mismatch between this ticket's prediction and that measurement.
The reasoning here — that a bare number reaches `get_abs_value`'s `Float` arm and
is read as **absolute**, so a `percent` key silently goes ~100x over-dense — is
correct, and is now reachable from the sidecar for the first time: post-131,
`{"sparse_infill_density": 90}` and `{"sparse_infill_density": 90.0}` both land
on that arm and agree.

**This ticket's question is unchanged and still open**: for a `percent` /
`float_or_percent` key, does a bare number mean percent-of-base or absolute, and
where does the coercion belong? 131 deliberately did not answer it — it only made
the two numeric spellings agree, which is a precondition for any ruling here
rather than a substitute for one. The four open questions above stand.

One consequence worth noting for the ruling: `sparse_infill_density` does **not**
discriminate between the two readings, because its callers pass `base = 100.0`
(`slicer_sdk::config_resolution::resolve_percent_float`), where percent-of-base
and absolute coincide. Pick a key with a non-100 base — `bridge_density`
(`base = 1.0`) is the one this ticket was filed from — when building the
regression that proves the ruling.
