# 131 — A JSON integer in the config sidecar is invisible to modules

Type: bug
Status: resolved
Assignee: wayfinder session (2026-09-04)
Blocked by: —
Map: ../map.md

## Question

Ticket 130 measured that `sparse_infill_density` set through the `pnp_cli
--config` sidecar as a **bare JSON number** had no effect on the slice, while the
percent-string spelling worked. It recorded the finding on ticket 128 as
evidence for the `percent`-unit question and did not fix it.

That attribution was **half right**. There is a second, independent defect
underneath it, and it is not about percent units at all.

## Answer — it is a JSON *spelling* defect, not a unit defect

`serde_json` distinguishes `90` from `90.0`. `json_to_config_value`
(`slicer_scheduler::execution_plan`) tries `as_i64()` first, so:

| sidecar JSON | `ConfigValue` |
|---|---|
| `90` | `Int(90)` |
| `90.0` | `Float(90.0)` |
| `"90%"` | `String("90%")` |

Modules read the **raw source map**, not `ResolvedConfig`:
`bind_module_config_view` builds their view with
`ConfigView::from_declared(source, ...)`. So the JSON spelling survives all the
way to the accessor the module calls — and both float accessors dropped `Int`
into their catch-all arm:

- `slicer_ir::ConfigView::get_abs_value` — `Int` hit `_ => None`
- `slicer_ir::ConfigView::get_float` — same
- `slicer_wasm_host::host`'s WIT `get_float` — same, at the guest boundary

`None` means the module keeps its own fallback, so the user's value silently
does nothing. **Inert, not mis-united.**

This is why ticket 128's prediction did not match what 130 measured. 128 reasoned
from `ConfigView::get_abs_value` reading a bare `Float` as absolute and predicted
a bare number would be ~100x over-dense; that reasoning is correct **for
`Float`** and the `90.0` spelling does behave exactly that way. The spelling
users actually write — an integer — never reached that arm.

### The measurement

`ResolvedConfig` was never the problem. Probed directly, `resolve_global_config`
already agreed on both spellings (`extract_percent_float` accepts `Float` and
`Int`):

```
{"sparse_infill_density": 90}    -> raw=Int(90)         resolved=90
{"sparse_infill_density": 90.0}  -> raw=Float(90.0)     resolved=90
{"sparse_infill_density": "90%"} -> raw=String("90%")   resolved=90
```

The divergence is entirely module-side. `pnp_cli slice` on
`resources/regression_wedge.stl`, every core-module dir,
`gcode_filament_length_mm`:

| sidecar | before | after |
|---|---|---|
| `{"sparse_infill_density": 5}` | 11054.1767578125 | **6082.02587890625** |
| `{"sparse_infill_density": 25}` | 11054.1767578125 | **12437.205078125** |
| `{"sparse_infill_density": 90}` | 10927.99609375 | **33215.7109375** |
| `{"sparse_infill_density": 90.0}` | 33215.7109375 | 33215.7109375 |
| `{"sparse_infill_density": "90%"}` | 33215.7109375 | 33215.7109375 |

(The two "before" baselines differ because the first three were measured on a
module set that excluded `infill-linker` and the last two on the full set; within
each set the integer rows were byte-identical to the default-density output,
which is the symptom. All five "after" rows come from one full-set run.)

After the fix all three spellings of 90 agree exactly, and distinct integer
densities produce distinct output.

## Fix

One arm added in three places, each mirroring the `Float` arm beside it:

- `ConfigValue::Int(i) => Some(*i as f64)` in `slicer_ir::ConfigView::get_abs_value`
- the same in `slicer_ir::ConfigView::get_float`
- `ConfigValueStorage::Int(i) => Some(*i as f64)` in `slicer_wasm_host::host`'s
  WIT `get_float`, so the guest boundary does not re-open the hole

Purely widening: every spelling that worked before is untouched, and the
previously-`None` cases now return a value instead of the module's fallback.
`get_int` is deliberately unchanged and still rejects `Float`.

**This does NOT resolve ticket 128**, and is careful not to. 128 asks whether a
bare number against a `percent` key means percent-of-base or absolute, and where
that coercion belongs. Whatever the ruling, it applies identically to `Float` and
`Int` — making the two spellings agree is a precondition for that ruling, not an
answer to it. For `sparse_infill_density` the two readings coincide anyway,
because its callers pass `base = 100.0`
(`slicer_sdk::config_resolution::resolve_percent_float`).

### Regression tests

`crates/slicer-runtime/tests/integration/sidecar_integer_config_spelling_tdd.rs`
— the real call site, sidecar JSON through `parse_cli_config_source` and
`ConfigView::from_declared` to the accessor:

- `sidecar_integer_reaches_the_module_like_the_float_spelling` — `90`, `90.0`
  and `"90%"` all resolve to 0.9
- `distinct_sidecar_integers_produce_distinct_module_values` — 5 / 25 / 90 give
  0.05 / 0.25 / 0.9, the user-visible symptom
- `sidecar_integer_reaches_a_plain_float_read_too` — `{"line_width": 1}` on
  `get_float`, pinning that this was never percent-specific
- `integer_reads_still_see_integers` — `get_int` undisturbed

Plus four accessor-level unit tests in `crates/slicer-ir/src/slice_ir.rs`
(`config_value_percent_tests`): `Int` == `Float` for `get_abs_value`, `Int` is
absolute and ignores `base`, `Int` on `get_float`, and `get_int` still rejects
`Float`.

## Scope note — how wide was this?

Not percent-specific and not `sparse_infill_density`-specific. **Every**
float-typed config key was affected whenever a profile spelled its value without
a decimal point — `{"line_width": 1}` was as inert as `{"sparse_infill_density":
90}`. Keys whose values are conventionally written with decimals (`0.42`, `0.2`)
were unaffected, which is why this survived: the defect is invisible for exactly
the values people usually write, and fires on the round numbers they occasionally
do.

## Records

- Ticket 128 keeps its open unit question; a note points here for the sidecar half.
