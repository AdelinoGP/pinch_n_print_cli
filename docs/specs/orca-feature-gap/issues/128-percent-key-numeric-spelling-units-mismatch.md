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
