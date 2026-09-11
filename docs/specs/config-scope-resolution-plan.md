# Config Scope and Resolution — Approved Plan

Status: approved; revised 2026-09-11 after a two-axis code review and a
design-interview revision session. The revision decisions that amend governance
are recorded in the Amendment sections of **ADR-0067**, **ADR-0068** and
**ADR-0069** (ADR-0070 is unchanged). This revised file is the
packet-generation source — `/spec-packet-generator` reads this file, not the
review session that produced it.
Source: architecture review of the config pipeline. Every count and
divergence below was measured against a clean `master` working tree at review
time; all are **ledger facts** — re-derive before acting on any of them.

Governing ADRs authored with this plan: **ADR-0067** (one config schema
registry), **ADR-0068** (config scope is a wire encoding), **ADR-0069**
(scope eligibility is a per-key deny list), **ADR-0070** (typed modifier kind;
`ModifierScope` removed).
Vocabulary: `CONTEXT.md` — **config scope**, **scope delta**, **config schema
registry**, **scope eligibility**, **authored value**, **automatic value**,
**layer range**, **modifier kind**.

## Problem

A setting authored in a 3MF does not reliably reach the module that consumes it.
The `.json` `--config` path — the developer path — and the 3MF
`project_settings.config` path — the only path an end user's frontend uses — carry
different type policies, and the weaker one is the production one. Downstream, the
five **config scopes** that do exist are assembled by convention rather than by a
module, so roughly half of every setting is unreachable below object scope, and a
sixth scope (**layer range**) does not exist at all.

Measured on `resources/cube_4color.3mf`, a real OrcaSlicer export carrying 607
config keys: eight module-owned numeric keys arrive as `"0"`/`"1"` and coerce to
`ConfigValue::Bool`; five then resolve to a value the file never authored.

| key | owning module | 3MF authored | value actually used |
|---|---|---|---|
| `skirt_loops` | skirt-brim | 1 | **6** |
| `brim_width` | skirt-brim | 0 | **8.0 mm** |
| `filter_out_gap_fill` | classic-perimeters | 0 | **0.5** |
| `tree_support_wall_count` | tree-support-planner | 0 | **1** |
| `support_interface_bottom_layers` | traditional-support-planner | 0 | **-1** |

Every OrcaSlicer-exported fixture under `resources/` shows the same 7–8 affected
keys. No committed test observes any of it.

## Verified root causes

- **RC-1 — Two disjoint schema declarations, joined by nothing.**
  `declare_resolved_config!` (`crates/slicer-ir/src/resolved_config.rs`) declares 69
  host fields; core-module manifests declare 244 keys under `[config.schema.<key>]`.
  `classify_declared_key` settles a 3MF's ambiguous `"0"`/`"1"` by probing
  `apply_cli_key`, which sees only the host half — so every module-owned key is
  `Undeclared` and falls to the `Bool` heuristic. The manifest already declares the
  type; the coercion never asks it.
  *Revision note: "host declarations" is wider than the DSL macro — see Registry
  below for the full channel list.*

- **RC-2 — Scope is a string prefix, so precedence is call order.**
  `object_config:<id>:<key>`, `paint_config:<semantic>:<key>`,
  `tool_config:<idx>:<key>` and `object_height:<id>` are re-parsed by `starts_with`
  at six points in `crates/slicer-scheduler/src/config_resolution.rs` and rebuilt by
  `format!` at four in `crates/slicer-runtime/src/run.rs` — plus once inside the
  `layer-planner-default` guest, which formats a host namespace across the WIT seam.
  Nothing types or orders the scopes, which is how `run_slice_with_collector` and
  `prepare_prepass_context` came to run different chains with only a prose comment
  recording it.

- **RC-3 — The scope merge covers 28 of 69 fields and infers "was set".**
  `overlay_resolved` (`crates/slicer-core/src/algos/region_mapping.rs`) is the merge
  primitive for paint, region-modifier and tool scope. It hand-enumerates 28 fields,
  leaving 41 — including `infill_overlap`, `support_line_width`, `support_expansion`,
  `bridge_no_support` — unreachable from any scope below object level. Its rule is
  `overlay.field != ResolvedConfig::default().field`, so a field explicitly set to
  its own default is indistinguishable from one never set. `fill_authored_coloring`
  is documented as per-region overridable in `CONTEXT.md`, `docs/02_ir_schemas.md`
  and `docs/03_wit_and_manifest.md`, and is one of the 41.

- **RC-4 — The flattening hand-restates the key list.** `to_config_map` emits 51
  keys; 18 declared CLI keys never reach a guest's `ConfigView` or the G-code
  `CONFIG_BLOCK`. Three of the 18 are deliberate and documented
  (`mmu_segmented_region_*`, held back to keep `CONFIG_BLOCK` bytes stable); the
  other 15 are simply absent. The unit tests mirror the same shape — a hand-listed
  `KEYS` array and a `field()` match ending in `panic!("unknown key")` — so no test
  can assert that every declared key survives.

- **RC-5 — `ConfigView` has two meanings.** The per-region path is built from
  `config_for(&key).to_config_map()` and is fully resolved. The object-level fallback
  is built by `bind_module_config_view` (`crates/slicer-scheduler/src/execution_plan.rs`)
  from the **raw unresolved** `config_source`. Same type, different guarantees, and a
  guest cannot tell which it holds. 243 of 244 manifest entries declare a default but
  `ConfigBoundsIndex::schema_defaults` threads only the 18 percent-family ones, so
  the 54 `.unwrap_or(literal)` sites across 8 guest modules are load-bearing rather
  than defensive.

- **RC-6 — Modifier kind is a magic string and modifier values skip validation.**
  `PartSubtype` (`crates/slicer-model-io/src/sidecar.rs`) is parsed typed, then
  stringified into `config_delta.fields["subtype"]` by `resolve_object`, and
  re-derived by string comparison at ten production sites across `slicer-core` and
  `slicer-runtime`. `stamp_modifier_sub_region_configs` copies every delta key
  straight into `ResolvedConfig.extensions` without calling `apply_cli_key`, so a
  region modifier is the one scope whose values are never type- or bounds-checked
  and are invisible to any host code reading the typed field.

- **RC-7 — There is no layer-range scope.** No occurrence of `layer_range`,
  `height_range` or `z_range` as a config concept anywhere in the tree. OrcaSlicer
  writes ranges to a dedicated zip part `Metadata/layer_config_ranges.xml`
  (`_BBS_3MF_Exporter::_add_layer_config_ranges_file_to_archive`), which the loader
  never opens. The only adjacent surface, `ModifierScope::LayerHeight`, is a variant
  of a write-only enum.

- **RC-8 — Automatic values are expanded inconsistently or not at all.** The width
  family is implemented twice with different precedence depth — `resolve_role_width`
  (`crates/slicer-core/src/flow.rs`) and `resolve_support_line_width_mm`
  (`crates/slicer-ir/src/resolved_config.rs`). The speed family's `0 = auto` fallback
  (canonical `GCode::_extrude`, to `filament_max_volumetric_speed / mm3_per_mm`) is
  absent entirely. `feedrate.rs::read_speed` returns `None` for any `Percent`, so a
  percent-authored `overhang_*_speed` from a real Orca profile is silently dropped
  rather than resolved against its `ratio_over` base. Eight negative `-1 = auto`
  sentinels, including `support_interface_bottom_layers`, are unresolved anywhere.

- **RC-9 — Four write-only declaration surfaces.** `ModifierVolume.applies_to`
  (written at 5 sites, always `AllFeatures`, read by no production code);
  `ConfigFieldEntry.validate` (parsed, serialized into `module config-schema` output,
  never evaluated); `[config.overridable-per-region]` and
  `[config.overridable-per-layer]` (required by `ingest_manifest`, declared in all 24
  core manifests, empty in 23, read only by a parse round-trip test —
  `docs/03_wit_and_manifest.md` concedes "the current scheduler does not apply this
  wildcard matcher to those lists").

- **RC-10 — Cross-module schema conflicts are unreconciled.** 51 of 162
  module-declared keys are declared by more than one module. Three disagree on type
  (`bridge_line_width`, `initial_layer_line_width`, `support_style`), five on default,
  five on bounds. Bounds silently intersect, so `layer_height` is capped at 1.0 today
  by `layer-planner-default`'s declaration and nothing says so.

## Verified *not* root causes

Recorded so future reviews do not re-open them:

- **Region-id arithmetic is single-sourced.** `PAINT_VARIANT_REGION_ID_STRIDE`
  (`crates/slicer-core/src/algos/paint_segmentation`) and
  `MODIFIER_VARIANT_REGION_ID_STRIDE` + its high-bit flag
  (`crates/slicer-ir/src/slice_ir.rs`) are each defined once and consumed by call
  everywhere else, including `slicer-wasm-host::dispatch`.
- **The per-layer hot path does not re-resolve config.** All scope resolution is
  front-loaded into `PrePass::RegionMapping` and interned as `ConfigId`; Tier-2
  reads are O(1) index lookups. This design is sound and is not changed.
- **`paint_semantic_namespace_key` duplication is cosmetic**, not a correctness
  defect — but the `slicer-core` copy's stated reason ("avoid a `slicer-scheduler`
  dep") is false, since the same file already imports from that crate. Fold into P5.
- **`docs/15_config_keys_reference.md` generation is already correct** — produced by
  `cargo xtask gen-config-docs` with a `--check` drift gate. The defect is on the
  runtime side, not the documentation side.

## Design decisions (resolved)

### Registry — one schema registry, every declaration channel

One **config schema registry** assembled per run at module load (ADR-0067). The
registry joins **four** declaration channels — RC-1's "host declarations" is
wider than `declare_resolved_config!`:

1. the `declare_resolved_config!` DSL rows (`ResolvedConfig::host_config_keys`,
   `crates/slicer-ir/src/resolved_config.rs`);
2. `FeedrateConfig::SPEED_KEYS` (`crates/slicer-ir/src/feedrate.rs`);
3. `HOST_RUNTIME_KEYS` (`crates/slicer-scheduler/src/manifest.rs`) —
   `use_relative_e_distances`, `thumbnail_path`, `wall_generator`;
4. every loaded module's `[config.schema.<key>]` manifest table.

A key the host actually reads but no channel declares must be impossible; the
packet-1 census test enforces this (see Testing). The registry carries each
key's type, default, bounds, **scope eligibility**, **selector** marking,
percent **base-key**, and declaration provenance. Automatic values are *not*
expanded here — see Expansion. Assembly order: discover modules → assemble
registry → type config → select claim holders (selector keys only — see Claim
selection).

### Crate topology and shared types

- `ConfigFieldEntry` and `ConfigSchema` relocate to `slicer-ir`, beside
  `ConfigValue`/`HostConfigKey`. `slicer-scheduler` re-exports during the
  transition and keeps `ingest_manifest` TOML parsing plus the
  `module config-schema` wire rendering, adapted to the relocated types.
- `AggregatedRegionSplitEntry` relocates to `slicer-ir` and the
  `slicer-core → slicer-scheduler` dependency is removed, **closing ADR-0019**
  (folded into packet 1; the ADR's Status line is amended there).
- New crate `slicer-config` owns registry assembly and resolution and depends
  on `slicer-ir` **only** — config semantics never depend on orchestration.
  This supersedes the draft's "`slicer-config` depends on `slicer-scheduler`".

### Reconciliation rules (multi-declarer keys)

- **Type** must agree across all declarers; disagreement is a load error.
- **Bounds** intersect, as before, and the intersection is **reported** rather
  than applied silently (`layer_height` is capped at 1.0 today by one module's
  declaration, invisibly).
- **Default**: the host declaration wins where the host declares the key;
  otherwise the alphabetically-first declaring module id (ADR-0067, kept).
  **Revision consequence:** when *claim-exclusive* modules (alternatives that
  can never both be active) declare a shared key with divergent defaults or
  bounds — measured example `detect_thin_wall`: `arachne-perimeters` `false`
  vs `classic-perimeters` `true` — assembly emits a non-fatal load warning
  naming both declarers and both values. The coupling stays deterministic but
  becomes visible.
- **`denied_scopes`**: **union** across declarers — a scope denied by any
  declarer is denied (ADR-0069 amendment).
- **Enum domains** must agree across declarers, else load error (same class as
  a type conflict).
- **UI metadata** (display/group/unit/description/tags) is advisory: host
  declaration wins, else alphabetically-first declarer; never a load error.
- Every entry retains **provenance** (declaring host channel or module ids) so
  every diagnostic names its contributors.
- The 48 module default declarations for host-owned keys stay as documentation
  with a load warning.
- **`base-key`** (the `ratio_over` relationship) is a typed field on the schema
  entry, validated at assembly: the base key exists, its type is
  percent-compatible, and the dependency graph is acyclic.

### Selector keys and claim selection

- A `selector` flag on the schema entry (a host DSL row flag; module manifests
  may declare their own selector keys under the same rule). Assembly validates:
  a key marked `selector` that is also statable per-region is a load error.
- `wall_generator`, `spiral_vase`: whole-print selectors, readable at load.
  Startup claim dedup reads only typed selector keys, removing the raw
  pre-resolution reads.
- `support_type`, `support_family`: per-region selectors, resolved in the
  existing region-resolution stage (`module_claims_match_active_region`,
  `resolve_held_claims`), reading typed values only. Both are **denied at
  layer-range scope** — stating one in a layer range is a load error, not a
  silent bypass of load-time selection.
- Per-region claim selection stays where it is. Startup selection and
  region-resolved selection are two named lifecycle points, not one.

### Ingestion

Adapters parse; the registry types. The prefixed flat key stays the **wire**
format (ADR-0068) so a flat OrcaSlicer-shaped sidecar remains a drop-in, and is
decoded exactly once into a typed **config scope**. Namespace prefixes are
stripped before classification. Keys no declaration recognises warn with
near-miss suggestions. **Drop-unknown ships in two steps:** packet 3 lands
warn-mode (warn and keep); packet 6 flips warn→drop once the census and
no-drop e2e gates are green (see Testing). No alias mechanism is introduced —
key naming belongs to the orca-feature-gap rename workstream, which runs after
this plan.

### Expansion — automatic values in three phases

"Automatic values expand inside the registry" is revised: the registry carries
declaration and dependency metadata; expansion happens in the phase that owns
the inputs the derivation needs.

- **Phase A — declaration (registry):** type, default, bounds, base-key,
  selector, eligibility, provenance. No expansion.
- **Phase B — resolution (config-only):** after the applicable scope deltas
  merge, before interning / `ConfigView` delivery, using an explicit
  `ExpansionContext` (nozzle diameter, tool bases). Covers: `line_width = 0`,
  `support_line_width = 0`, percent values resolved against their base-key,
  the speed percent family, and the config-only `-1 = auto` sentinels (e.g.
  `support_interface_bottom_layers = -1` matching the top-side key).
- **Phase C — owning stage (context-dependent):** role/first-layer/bridge
  widths stay in `resolve_role_width` (`crates/slicer-core/src/flow.rs`),
  reading already-expanded bases so its zero-fallbacks shrink to role
  dispatch; the volumetric `0 = auto` speed cap lands in the emitter
  (`crates/slicer-gcode/src/emit.rs`), where the per-move width/height it
  needs exist.

"Always resolved" means: **no raw placeholder reaches a `ConfigView` consumer
at its read point.** A stage-context rule is documented as such, never expanded
with context frozen at load.

### Resolution — one module, two entry points, one precedence matrix

One resolution module in `slicer-config` (replacing the five scattered
resolvers and `overlay_resolved`): the **Z-grid query** that
`PrePass::LayerPlanning` calls, and the **scope-stack resolve** that
`PrePass::RegionMapping` calls. Scope layers are **scope deltas**, so absence
is absence and the diff-against-`Default` rule disappears along with the
41-field ceiling.

Total scope order, low → high. The existing relative order of modifier, paint
semantic and tool is preserved from `docs/02_ir_schemas.md` §Config Key
Namespaces and `docs/04_host_scheduler.md` §RegionMapping; layer range is the
one insertion:

```text
global/print < object < layer range < modifier < paint semantic < tool
```

- **Layer range** sits above object and below modifier: canonical applies a
  range by re-deriving the Z grid (`layer_height`) or overriding
  `PrintRegionConfig` for intersecting regions, while a geometric modifier is
  the narrower selector.
- **Same-scope rules:** multiple modifiers — priority ascending,
  last-writer-wins (existing); multiple paint semantics — lexicographic
  semantic order (existing); tool last (existing).
- **Layer-range overlap:** `layer_height` ranges compose into the Z-grid
  re-derivation with the later-starting range winning within overlap
  (canonical `layer_height_profile` behavior); two overlapping ranges stating
  the same non-`layer_height` key with different values is a load error.
- A key denied at a scope but stated there anyway is rejected loudly
  (ADR-0069) — never accepted and ignored.

### Layer range — geometry semantics

- Ingested from OrcaSlicer's `Metadata/layer_config_ranges.xml`; a first-class
  **per-object** scope.
- Endpoints are world-space Z millimetres as authored, half-open
  `[min_z, max_z)`, matched against each layer's print Z on the global grid.
- `layer_height` ranges re-derive the Z grid (canonical
  `layer_height_profile_from_ranges`, `Slicing.cpp`); every other key
  overrides the intersecting regions' config through the scope-stack resolve —
  which is what the two entry points exist to serve.
- Catch-up layers inherit the range covering their top Z.
- Selector keys and machine/emitter keys are denied at layer-range scope;
  stating one there is a load error.
- Packet 9 verifies the canonical application against a local OrcaSlicer
  checkout, citing function names only (never line numbers).

### Eligibility

Per-key `denied_scopes` on the schema entry, authored by host and module
authors alike; absent means statable at every scope (ADR-0069).
`[config.overridable-per-region]` and `[config.overridable-per-layer]` are
deleted along with their `ingest_manifest` requirement. The initial
machine/emitter denials (`bed_shape`, the `machine_max_*` family,
`gcode_xy_decimals`, `disable_m73`, …) are hand-authored on the host DSL
declarations, cross-checked by a mechanical derivation (registry keys that no
sub-print scope can meaningfully reach) and pinned by a drift test: the
derivation proposes, the author confirms, nothing is silently auto-generated
at runtime.

### Extensions

`ResolvedConfig.extensions` remains the home for module-declared keys — every
entry now registry-typed and bounds-checked at ingestion/resolution (an entry
exists only if the registry declares the key). No new IR field; the interner
and `Hash` semantics are unchanged. Genuinely undeclared keys are dropped
after warning (warn-mode in packet 3; the drop flips in packet 6).

### Guests and delivery

- `ConfigView` has one meaning: always resolved. `bind_module_config_view`
  stops reading raw `config_source`. Every declared key is present with its
  registry default, `None` becomes a real signal, and the 54
  `.unwrap_or(literal)` fallbacks across 8 guest modules are deleted.
- **Layer-planning seam:** `prepass-layer-planning.run` gains one typed
  per-object resolved record (object height, effective `layer_height`,
  `first_layer_height`, `support_raft_layers` — scope-resolved host-side) as a
  new parameter. Adding a parameter is a major package bump
  (`slicer:prepass-layer-planning@1.0.0` → 2.0.0), accepted once and
  coordinated via `cargo xtask build-guests`. The guest deletes its
  `object_height:<id>` / `layer_height:<id>` `format!` sites
  (`modules/core-modules/layer-planner-default/src/lib.rs`) — the last place
  a host namespace was formatted across the WIT seam (ADR-0068).
- **Emission:** `to_config_map` becomes registry-driven with a per-key
  `config_block` flag on the schema entry. The three `mmu_segmented_region_*`
  omissions become three explicit flags instead of hand-list logic, and the
  `CONFIG_BLOCK` byte change they cause is accepted deliberately in the same
  packet.

### Modifiers

Typed **modifier kind** across the IR seam, matched exhaustively at the ten
sites; deltas route through the registry so they are typed, bounds-checked
and subject to `denied_scopes`; `ModifierScope` is deleted (ADR-0070,
superseding the future work named in ADR-0030 §3). `ModifierVolume.applies_to`
is removed in the same packet — versioning per Owner decisions below.

### Type conflicts

Repaired in this plan, toward canonical: `bridge_line_width` and
`initial_layer_line_width` become `float_or_percent` (canonical declares both
`coFloatOrPercent`, `ratio_over = nozzle_diameter`); `support_style` becomes
`enum` with `tree-support-planner`'s seven-value list. The five plain-float
readers migrate to `get_abs_value` semantics in the same packet, so no percent
value is silently dropped. **`ResolvedConfig` fields stay scalar** (`f32` mm):
the widening is at the declaration/ingestion layer and expansion output
remains scalar — no IR field type change.

## Owner decisions (recorded, overriding documented policy)

Two decisions from the 2026-09-11 revision session deliberately take a path a
documented rule would classify otherwise. They are recorded here as owner
decisions per the session's explicit instruction, not as DEVIATION_LOG
entries:

1. **`ConfigFieldEntry.validate` removal ships with a *minor*
   `CONFIG_SCHEMA_WIRE_VERSION` bump**, although the CLI wire policy
   (`docs/11_operational_governance_and_acceptance_gate.md`) classifies
   removal as major. Rationale: the field was never evaluated (RC-9); its only
   consumer is the in-house fork frontend, re-read in the same change window.
   The `[[config.cross-validate]]` doc section retires with it.
2. **`ModifierVolume.applies_to` removal ships as an unconditional *minor*
   `CURRENT_MESH_IR_SCHEMA_VERSION` bump**, without the P109
   compatible-removal criteria ceremony. Rationale: no production reader
   (RC-9; written at 5 sites, always `AllFeatures`). If serialized-fixture
   parsing breaks in the implementing packet, it escalates rather than
   shipping broken compat.

## Cross-cutting requirements

- **Oracle independence is the acceptance shape** (`docs/22_test_quality.md`
  §2.1, §4), in three layers — the oracle for this program is the **authored
  value** in the source document plus independently derived expectations,
  never a snapshot of current behaviour:
  1. **Census (packet 1):** every key in every declaration channel appears in
     the assembled registry, derived from the channels themselves — never a
     hand-listed roster (§2.4). A registry that omits a declared key fails the
     census rather than shrinking a fixture population.
  2. **Ingestion fidelity (packet 2):** for every key a fixture authors that
     the registry declares **and that no narrower scope restates and no
     automatic rule expands**, the value reaching the owning module's
     `ConfigView` equals the authored value — written red on the five
     divergent keys.
  3. **Resolution expectations (packets 4/5):** scope and expansion cases
     assert independently derived expected values (hand-computed from this
     plan's precedence and expansion rules), never values re-derived from the
     resolver.
- **No-drop gate (packet 6):** a no-drop e2e — `resources/cube_4color.3mf`
  plus a synthesized config exercising one value per census key, asserting
  zero unrecognised-key warnings — must be green before the warn→drop flip.
- **Per-packet compatibility checklist:** every packet spec names its IR
  schema versions touched, WIT package bumps, CLI wire versions, and
  manifest-schema changes, applying the IR Versioning Contract
  (`docs/02_ir_schemas.md`) and the compatibility policy (`docs/11`) — with
  the two owner decisions above excepted.
- **Visual-debug is a gate on every geometry-changing packet** — packets 3, 4,
  5, 8, 9 and 10 all move resolved values that reach extrusion. Each carries
  a `pnp_cli visual-debug` render plus `manifest.json` check in its acceptance
  criteria, per `docs/19_visual_debug.md`.
- **Guest freshness**: `cargo xtask build-guests --check` must return exit `0`
  before any guest, component or module-dispatch failure is attributed to a
  change here.
- **Pre-commit gates**: `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo xtask check-literals`.
- **ADR-0019 closure is packet 1's acceptance criterion** (type relocation +
  transitional re-export + dependency removal + ADR status amendment).
- **Known-open, deliberately not fixed here**: PnP reads `spiral_vase` while
  OrcaSlicer writes `spiral_mode`, so spiral-vase claim selection can never
  fire from a real 3MF. This is a naming defect owned by the orca-feature-gap
  rename workstream (`docs/specs/orca-feature-gap/map.md` ticket 07), not an
  alias to add here.

## Packet Queue

Packet directories use `docs/spec_packets/config-scope-resolution_<NN>_<slug>/`
(prefix = this plan's file name minus `-plan`, `NN` = zero-padded queue row,
per the 2026-09-11 packet-dir convention). Task ids are **ledger facts**:
derive the next free one at authoring time
(`grep -rhoE 'TASK-[0-9]{3}' docs/ | sort -u | tail -1`), never from this
table.

| # | packet slug | goal | depends on | status |
|---|-------------|------|------------|--------|
| 1 | config-schema-registry | Relocate `ConfigFieldEntry`/`ConfigSchema` and `AggregatedRegionSplitEntry` to `slicer-ir` (transitional re-exports; drop `slicer-core → slicer-scheduler`; amend ADR-0019 to Closed), create `slicer-config` (depends on `slicer-ir` only), assemble the registry from all four declaration channels plus manifests under the full reconciliation rules (type agreement, bounds intersection reported, host-then-alphabetical defaults with the claim-exclusive divergence warning, union `denied_scopes`, strict enum agreement, provenance), add and validate the typed `base-key` and `selector` entry fields, retire `ConfigFieldEntry.validate` and the `[[config.cross-validate]]` doc section (minor wire bump per owner decision), repair the 3 type conflicts and migrate the 5 plain-float readers, emit the schema doc under the `gen-config-docs --check` gate, and land the registry census test. | – | queued |
| 2 | authored-value-oracle | Add the ingestion-fidelity oracle — for every key a fixture authors that the registry declares and that no narrower scope restates and no automatic rule expands, assert the value reaching the owning module's `ConfigView` equals the authored value — written **red**, failing on the five divergent keys. | #1 | queued |
| 3 | typed-scope-ingestion | Decode the prefixed wire key once into a typed config scope, type every value against the registry, warn on unrecognised keys with near-miss suggestions (warn-mode; unknown keys kept), and move claim selection onto typed selector values. Turns #2 green. | #1, #2 | queued |
| 4 | automatic-value-expansion | Implement Phase B — the resolution-phase expansion with `ExpansionContext` (nozzle diameter, tool bases): unify the width-family implementations, resolve percent values against their typed base-key, cover the config-only `-1 = auto` sentinels — and shrink `resolve_role_width`'s fallbacks to role dispatch over expanded bases. | #1 | queued |
| 5 | scope-resolution-module | Replace the five scattered resolvers and `overlay_resolved` with one resolution module over scope deltas, exposing the Z-grid query and the scope-stack resolve under the normative precedence matrix; add the typed per-object resolved record to `prepass-layer-planning.run` (major package bump, accepted once) and delete the guest's `format!` prefix sites; assert independently derived resolution expectations. | #3, #4 | queued |
| 6 | resolved-config-view | Give `ConfigView` one meaning (always resolved), make `extensions` registry-typed, make emission registry-driven with the per-key `config_block` flag (accepting the `CONFIG_BLOCK` byte change), delete the 54 guest `unwrap_or` literals, land the no-drop e2e, and flip warn→drop for unrecognised keys. | #5 | queued |
| 7 | scope-eligibility | Author per-key `denied_scopes` on host and module schema entries (hand-authored machine/emitter denials cross-checked by the mechanical derivation and pinned by a drift test), derive the per-object admission set from the registry, and delete the two inert manifest sections. | #5 | queued |
| 8 | typed-modifier-kind | Carry modifier kind typed across the IR seam, match exhaustively at the ten sites, route modifier deltas through the registry, and delete `ModifierScope` and `ModifierVolume.applies_to` (minor MeshIR bump per owner decision). | #5, #7 | queued |
| 9 | layer-range-scope | Ingest `Metadata/layer_config_ranges.xml`, add the per-object layer-range scope wired to both entry points under the settled geometry semantics (world-Z, half-open, overlap rules, catch-up inheritance, selector-denial load error), and author a fixture carrying one range. | #5, #7 | queued |
| 10 | remaining-automatic-values | Implement the remaining Phase C expansions — the speed family's `0 = volumetric auto` fallback in the emitter and any geometry-dependent `-1 = auto` sentinels not covered by #4. | #4, #5 | queued |