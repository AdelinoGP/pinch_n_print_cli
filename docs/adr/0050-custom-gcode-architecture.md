# ADR-0050 — Custom G-code: warn-and-pass unknown placeholders, a manifest-scoped domain, and a module-private engine and injection registry

<!-- filename: 0050-custom-gcode-architecture -->

## Status

Accepted (2026-07-25). Authored ahead of the custom-G-code trilogy —
`.ralph/specs/186-custom-gcode-placeholder-engine`,
`.ralph/specs/187-custom-gcode-injection-registry`,
`.ralph/specs/188-custom-gcode-conditional-points` — to record four decisions
those packets take that no existing ADR governs.

## Context

`machine-gcode-emit` is the sole module registered at `PostPass::GCodePostProcess`.
`run_gcode_postprocess` (`modules/core-modules/machine-gcode-emit/src/lib.rs`)
reads two templates from `ConfigView`, builds one flat `HashMap<String, String>`
lookup seeded with `bed_temperature_initial_layer_single` and
`nozzle_temperature_initial_layer` and then swept over `config.keys()`,
substitutes both templates through `substitute_placeholders`, and frames the
re-emitted command stream with `push_raw`.

Four architectural questions are latent in that shape and are answered
differently by each of the three packets, so they are recorded once here rather
than three times in packet prose:

1. What happens to a bracketed token that resolves to nothing.
2. What set of names is allowed to resolve at all.
3. Where the expansion engine lives.
4. Who is allowed to add an injection point.

The state of the tree at authoring time was superseded by packet 186:
`substitute_placeholders` now returns rendered text plus unresolved keys,
passes unresolved `[key]` through verbatim, and `run_gcode_postprocess` emits
one aggregated warning before continuing. The manifest declares the five
config keys (`machine_start_gcode`, `machine_end_gcode`,
`bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer`,
and `nozzle_diameter`) and applies the legacy alias table. The registry and
per-site context remain the target architecture of packets 187 and 188.

## Decision

### 1. An unresolved placeholder passes through with one warning

An `[key]` with no entry in the lookup remains verbatim in the rendered text,
and the slice proceeds. `run_gcode_postprocess` collects unresolved keys across
all templates, unions them into a `BTreeSet<String>` (sorted and deduplicated),
names every contributing injection point, and emits one
`slicer_sdk::host::log_warn` before continuing with normal output emission.

This conforms to the recoverable side of [ADR-0010](./0010-typed-diagnostic-channel.md):
the module cannot distinguish a nonexistent key from a key owned by a module
that is not loaded. Failing the latter would make template validity depend on
the current module graph and would break composition. The warning is the
module-local diagnostic; the verbatim text preserves the user's template
rather than silently producing a malformed command such as `M104 S`.

**There is no opt-out config key and no escape syntax for a literal `[foo]`.**
A `strict_placeholders` toggle is rejected because a module-scoped view cannot
reliably determine whether a missing key belongs to an unloaded module. An
escape syntax is unnecessary: a bracketed non-key is passed through unchanged.
Changing this policy requires a new packet and a new deviation row.

### 2. The placeholder domain is one module's manifest plus an alias table

The resolvable name set is **exactly `machine-gcode-emit`'s manifest-declared
`[config.schema]` keys**, as handed to the guest through `ConfigView`, plus a
`const PLACEHOLDER_ALIASES: &[(&str, &str)]` table applied *after* the
`config.keys()` sweep so a real config key of the same name would win. Its first
and only entry is
`("first_layer_temperature", "nozzle_temperature_initial_layer")`, which is a
port of canonical's own parser alias in
`GCode::update_placeholder_parser_with_variant_params`, not a convenience. An
alias must not become a second manifest key: two config keys for one value can
disagree.

The domain is emphatically **not the resolved print config**. Note that
`ConfigView::keys` (`crates/slicer-ir/src/slice_ir.rs`) enforces no scoping
itself — it returns every key in the view's own `fields` map, sorted. The
manifest scoping happens when the **host constructs the view**. "The placeholder
domain is the manifest" is therefore a property of the live pipeline, not of the
accessor, and must not be written as if it were the latter.

**Known wrinkle, recorded rather than fixed here.** Because the domain is
"whatever is in the view", the `for key in config.keys()` sweep in
`run_gcode_postprocess` makes the template keys *themselves* —
`machine_start_gcode` and `machine_end_gcode` — resolve as placeholders. A
template containing `[machine_start_gcode]` expands to the template's own text,
unsubstituted, in one pass. **This is a side effect of the sweep, not an
intended feature, and it is not excluded today.** It is left in place because
the sweep is the mechanism by which any newly declared key becomes resolvable,
and excluding the two template keys is a behaviour change that belongs to
whichever packet needs it, with its own criterion. Anyone adding an exclusion
must add it as an explicit skip list, not by narrowing the sweep.

The arithmetic consequence is that `docs/15_config_keys_reference.md` is wrong
today: it states "Only the two macros listed above resolve today", naming
`[bed_temperature_initial_layer_single]` and `[nozzle_temperature_initial_layer]`.
Four manifest keys are declared and the sweep admits all four, so **four**
resolve, two of them being the templates. The doc's count is a floor stated as
an exact figure; it must be corrected by the packet that touches that section,
and no ADR, packet, or deviation row should quote it as authority for the
domain's size.

### 3. The expansion engine stays private to `machine-gcode-emit`

`substitute_placeholders` gains the signature
`(&str, &HashMap<String, String>) -> (String, Vec<String>)` — rendered text plus
the sorted, deduplicated list of bracketed keys that found no entry — and
**stays a private function in
`modules/core-modules/machine-gcode-emit/src/lib.rs`**. It is not promoted to
`slicer-sdk` or `slicer-core`.

The rendered text retains the verbatim `[key]` for an unresolved key. The
unresolved-key list is consumed by the caller's one-warning aggregation; it is
not an error list and does not stop output emission.

**Reversal condition.** Promote the engine to `slicer-sdk` when, and only when, a
**second module** needs to expand placeholders. One module's helper is not a
platform API; promoting it early would fix the `(String, Vec<String>)` shape as a
public contract before there is a second caller to constrain it — and packet 187
already anticipates that shape becoming a small struct once per-site context
arrives. `slicer-core` is wrong at any point: guest modules deliberately do not
depend on it (see [ADR-0008](./0008-overhang-as-finalization-module.md)'s
algorithm-self-containment consequence).

### 4. The injection-point registry is a closed, module-private table

Injection points are enumerated by
`const INJECTION_POINTS: &[InjectionPoint]` — a private const in the same guest
module, where `struct InjectionPoint { config_key: &'static str, site: InjectionSite }`
is a plain data record with no behaviour and `enum InjectionSite` names the
placement. Packet 187 introduces five entries; packet 188 grows the same table to
eleven. **Extend, never fork:** a second table or a parallel dispatch path
defeats the registry's reason to exist.

**The injection set is CLOSED to the host and to other modules.** No host code
enumerates injection points, no other module may declare one, and there is no
manifest surface through which a community module registers one. Adding a point
means editing this table in this module, in a packet.

**Ordering and precedence at a shared site is the table's declaration order.**
Two entries mapping to the same `InjectionSite` emit in the order they appear in
`INJECTION_POINTS`, which is maintained in canonical emission order. There is no
priority field and no tiebreaker.

[ADR-0018](./0018-region-split-priority-registry-and-canonical-order.md) is the
nearest analogue in a different subsystem: it also locks a registry of ordered
extension slots in source. The difference is deliberate and is the reason this is
a separate decision — ADR-0018's registry is **open** (community semantics
declare a `priority >= COMMUNITY_PRIORITY_FLOOR` and aggregate into a host-side
`BTreeMap` across all manifests), whereas this one is **closed** and needs
neither a numeric priority space nor a floor, because there is exactly one
declaring module. Do not import ADR-0018's priority machinery here on the
strength of the analogy.

[ADR-0045](./0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md)
is complementary, not overlapping: it assigns the G-code post-process behaviour
to `run_gcode_postprocess` but says nothing about who enumerates injection
points. This ADR answers that second question and leaves ADR-0045 untouched.

### 5. `filament_extruder_id` binds a tool index, and cannot express canonical's two id spaces

Canonical binds this placeholder from **two different id spaces** depending on
the injection site: at the filament-end site it uses
`int(get_extruder_id(old_filament_id))` — an **extruder** id — while at the
filament-start site it uses the raw `int(new_filament_id)` — a **filament** id.
The same key name therefore carries different meanings at the two sites, and the
two diverge on any configuration where one filament does not map one-to-one onto
one extruder.

PnP cannot reproduce that. `GCodeCommand::ToolChange { after_entity_index, from,
to }` (`crates/slicer-ir/src/slice_ir.rs`) carries **tool indices only** and
models no filament-vs-extruder distinction anywhere in the IR. The decision is to
bind `from` at the end site and `to` at the start site: **the direction is
faithful; the id space is not representable.**

This is recorded as a decision rather than a packet residual for two reasons.
First, it is user-facing and permanent — a template author who moves a working
`filament_extruder_id` template from canonical to PnP gets a silently different
value on multi-extruder-per-filament setups, with no diagnostic. Second, and more
consequentially, it is an **IR-level constraint that binds future work**: any MMU
or multi-extruder feature that needs the distinction must widen
`GCodeCommand::ToolChange` (and its WIT record) rather than patch the placeholder
binding, and must supersede this clause when it does.

An acceptance-criterion row predicate proves a token appears in a table row. It
does not make a constraint a decision of record, and this constraint outlives the
packet that discovered it. Packet 188 additionally files a `DEV-###` row for the
divergence; the row and this clause are complementary — the row tracks the
residual, this clause states the constraint.

## Consequences

- **A template canonical rejects can now slice successfully in PnP, by a wide margin.**
  Canonical's placeholder domain is a persistent parser beneath a local
  `DynamicConfig` override, carrying the full print config plus a large set of
  explicit global assignments; PnP's is one module's manifest plus one alias.
  The asymmetry is accepted and belongs in a residual deviation row. PnP emits
  the bracketed text verbatim and warns rather than treating the module's narrow
  view as proof that the key is invalid globally.
- **A start-G-code line legitimately containing `[...]` around a non-key word
  passes through verbatim with a warning.** Accepted: the module cannot know
  whether the name belongs to an unloaded module, and no escape syntax is
  needed under passthrough.
- **Config values are stringly resolved at expansion time, with no precision
  contract.** The sweep renders each `ConfigValue` with `f.to_string()` for
  `Float(f)`, `i.to_string()` for `Int(i)`, `b.to_string()` for `Bool(b)`, and
  clones `String(s)`. Nothing specifies decimal places, trailing zeros, or
  rounding, and nothing routes these through the emitter's `gcode_xy_decimals`
  or `format_xyz`. A float key whose `f32`→decimal expansion is long will ship
  that long form into printer G-code. **No test pins any of this.** Anyone who
  needs a formatting guarantee for a specific key must add one explicitly; do
  not assume the current rendering is a contract.
- **Keying the layer-change Z tag on `;Z:` models only the non-BBL branch of
  canonical.** `GCode::process_layer` chooses the tag spelling with a single
  runtime ternary inside one `sprintf` — a `; Z_HEIGHT: ` form for BBL printers
  and `;Z:` otherwise. It is a **runtime** choice, not a build variant. `;Z:` is
  therefore correct and complete for every printer PnP currently models, but
  the decision **forecloses BBL support until the lookahead is extended**. If
  PnP grows a BBL flavour, extend the recognition set; do not replace `;Z:`.
- **`time_lapse_gcode` ships as the non-BBL inline form only**, for the same
  reason. A user on a BBL-style workflow gets a materially smaller feature than
  canonical. Recorded as a residual rather than half-implemented.
- **Five canonical custom-G-code points stay unreachable and undeclared.**
  `file_start_gcode`, `wrapping_detection_gcode`, `machine_pause_gcode`,
  `template_custom_gcode` and `printing_by_object_gcode` must not appear in the
  manifest or in `INJECTION_POINTS`. Emitting `file_start_gcode` wherever a
  postpass module happens to be able to reach is a fake, not a partial
  implementation; an absent point is better than one that lands somewhere else.

## Alternatives considered

- **Warn and pass through**, via `slicer_sdk::host::log_warn`. Chosen: it
  composes with a module-scoped `ConfigView`, preserves the user's text, and
  provides one deterministic warning across all contributing templates.
- **Substitute unknown keys with the empty string.** Rejected: turns
  `M104 S[first_layer_temperature]` into `M104 S`, a worse printer command than
  the bracketed form, and silent.
- **Emit canonical's inline `!!!!! Failed to process the custom G-code template`
  marker.** Rejected — it changes emitted printer text and does not compose
  with PnP's module-scoped configuration. The aggregated host warning provides
  the diagnostic without modifying the template.
- **A `strict_placeholders` opt-out key.** Rejected: canonical has no such
  toggle, and it makes the silent path permanently reachable.
- **Declare `[bed_temperature]`, `[layer_count]`, `[x_max]` … as manifest keys
  so they resolve.** Rejected: none is an OrcaSlicer config key under that name;
  the canonical placeholder names are different, and several are computed print
  values rather than options. Declaring them would be inventing config keys.
- **Promote the expansion engine to `slicer-sdk` now.** Rejected under decision
  3's reversal condition — one caller.
- **Let other modules register injection points via manifest.** Rejected: it
  would require a host-side aggregation, an ordering rule between modules, and a
  priority space, for a subsystem with exactly one declaring module. If a second
  module ever needs to inject custom G-code, that is when ADR-0018's shape
  becomes the right one — and it needs its own ADR.

## Cross-references

- ADR-0010 (typed `Diagnostic` channel) — the recoverable-vs-fatal split this
  ADR conforms to; unresolved placeholders are recoverable diagnostics, while
  genuine output-builder failures remain module errors.
- ADR-0018 (region-split priority registry) — the nearest registry analogue, in
  a different subsystem and deliberately open where `INJECTION_POINTS` is closed.
- ADR-0045 (per-stage versioned interfaces) — assigns the behaviour to
  `run_gcode_postprocess`; silent on injection-point enumeration, which is this
  ADR's decision 4.
- ADR-0051 (G-code marker contract ownership) — the `;LAYER_CHANGE` / `;Z:` /
  `;HEIGHT:` contract packet 187's splice depends on.
- `.ralph/specs/186-custom-gcode-placeholder-engine`,
  `.ralph/specs/187-custom-gcode-injection-registry`,
  `.ralph/specs/188-custom-gcode-conditional-points` — the implementing packets.
- `docs/15_config_keys_reference.md` §"Machine start / end G-code" — the
  user-facing macro contract for the manifest domain and warn-and-pass policy.
