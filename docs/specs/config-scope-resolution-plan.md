# Config Scope and Resolution — Approved Plan

Status: approved (2026-09-11, grill session)
Source: architecture review of the config pipeline, this session. Every count and
divergence below was measured against a clean `master` working tree at review
time; all are **ledger facts** — re-derive before acting on any of them.

Governing ADRs authored with this plan: **ADR-0067** (one config schema registry),
**ADR-0068** (config scope is a wire encoding), **ADR-0069** (scope eligibility is a
per-key deny list), **ADR-0070** (typed modifier kind; `ModifierScope` removed).
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

**Registry** — One **config schema registry** assembled per run at module load,
joining host declarations and every loaded module's manifest (ADR-0067). Type must
agree across declarers or it is a load error; bounds intersect and the intersection
is reported; a default comes from the host declaration where the host declares the
key, otherwise from the alphabetically-first declaring module id. The 48 module
default declarations for host-owned keys stay as documentation with a load warning.
**Automatic values** expand inside the registry, so no consuming module ever reads
a placeholder. Order becomes: discover modules → assemble registry → type config →
select claim holders, which removes the raw pre-resolution read of `wall_generator`,
`spiral_vase`, `support_type` and `support_family`.

**Ingestion** — Adapters parse; the registry types. The prefixed flat key stays the
wire format so a flat OrcaSlicer-shaped sidecar remains a drop-in, and is decoded
exactly once into a typed **config scope** (ADR-0068). Namespace prefixes are
stripped before classification. Keys no declaration recognises warn with near-miss
suggestions and are dropped rather than carried. No alias mechanism is introduced —
key naming belongs to the orca-feature-gap rename workstream, which runs after this
plan.

**Resolution** — One module, two entry points: a Z-grid query that
`PrePass::LayerPlanning` calls, and the scope-stack resolve that
`PrePass::RegionMapping` calls. Scope layers are **scope deltas**, so absence is
absence and the diff-against-`Default` rule disappears along with the 41-field
ceiling. Lives in a new crate, `slicer-config`, depending on `slicer-ir` and
`slicer-scheduler` — also the natural place to close the open ADR-0019 dependency.

**Eligibility** — Per-key `denied_scopes` on the schema entry, authored by host and
module authors alike; absent means statable at every scope (ADR-0069).
`[config.overridable-per-region]` and `[config.overridable-per-layer]` are deleted
along with their `ingest_manifest` requirement.

**Extensions** — `ResolvedConfig.extensions` splits: module-declared keys keep a
home but become typed and bounds-checked; genuinely undeclared keys are dropped
after warning.

**Guests** — `ConfigView` has one meaning, always resolved. `bind_module_config_view`
stops reading raw `config_source`. Every declared key is then present with its
registry default, `None` becomes a real signal, and the 54 `unwrap_or(literal)`
fallbacks are deleted.

**Modifiers** — Typed **modifier kind** across the IR seam, matched exhaustively at
the ten sites; deltas route through the registry so they are typed, bounds-checked
and subject to `denied_scopes`; `ModifierScope` is deleted (ADR-0070, superseding
the future work named in ADR-0030 §3).

**Layer range** — A first-class scope, ingested from OrcaSlicer's
`Metadata/layer_config_ranges.xml`. Canonical applies a range two ways —
`layer_height` re-derives the Z grid via `layer_height_profile_from_ranges`
(`Slicing.cpp`), every other key overrides `PrintRegionConfig` for intersecting
regions — which is what the two entry points exist to serve.

**Type conflicts** — Repaired in this plan, toward canonical:
`bridge_line_width` and `initial_layer_line_width` become `float_or_percent`
(canonical declares both `coFloatOrPercent`, `ratio_over = nozzle_diameter`);
`support_style` becomes `enum` with `tree-support-planner`'s seven-value list. The
five plain-float readers migrate to `get_abs_value` in the same packet, so no
percent value is silently dropped.

## Cross-cutting requirements

- **Oracle independence is the acceptance shape** (`docs/22_test_quality.md` §2.1,
  §4). The oracle for this program is the **authored value** in the source document,
  not a snapshot of current behaviour — a self-captured baseline of today's resolved
  config is precisely the self-referential oracle §2.1 forbids. The population under
  test is derived (fixture ∩ registry), never a hand-listed roster (§2.4).
- **Visual-debug is a gate on every geometry-changing packet** — P3, P4, P5, P8, P9
  and P10 all move resolved values that reach extrusion. Each carries a
  `pnp_cli visual-debug` render plus `manifest.json` check in its acceptance criteria,
  per `docs/19_visual_debug.md`.
- **Guest freshness**: `cargo xtask build-guests --check` must return exit `0` before
  any guest, component or module-dispatch failure is attributed to a change here.
- **Pre-commit gates**: `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo xtask check-literals`.
- **Known-open, deliberately not fixed here**: PnP reads `spiral_vase` while
  OrcaSlicer writes `spiral_mode`, so spiral-vase claim selection can never fire from
  a real 3MF. This is a naming defect owned by the orca-feature-gap rename
  workstream (`docs/specs/orca-feature-gap/map.md` ticket 07), not an alias to add
  here.
- **Carried without a decision**: `to_config_map`'s three deliberate omissions exist
  to keep `CONFIG_BLOCK` bytes stable. Once emission is registry-driven, they need an
  explicit opt-out marker on the declaration or those bytes change. Resolve during P6.

## Packet Queue

Packet directories use `docs/spec_packets/config-scope-resolution_<NN>_<slug>/` (prefix
= this plan's file name minus `-plan`, `NN` = zero-padded queue row, per the
2026-09-11 packet-dir convention). Task ids are **ledger facts**: derive the next
free one at authoring time (`grep -rhoE 'TASK-[0-9]{3}' docs/ | sort -u | tail -1`),
never from this table.

| # | packet slug | goal (one sentence) | depends on | status |
|---|-------------|---------------------|------------|--------|
| 1 | config-schema-registry | Create `slicer-config`, assemble the config schema registry from host declarations plus loaded manifests with ADR-0067's reconciliation rules, repair the 3 type conflicts and migrate the 5 plain-float readers, and emit the schema document under the `gen-config-docs --check` drift gate. | – | queued |
| 2 | authored-value-oracle | Add the round-trip fidelity test — for every key a fixture authors that the registry declares, assert the value reaching the owning module's `ConfigView` equals the authored value — written **red**, failing on the five divergent keys. | #1 | queued |
| 3 | typed-scope-ingestion | Decode the prefixed wire key once into a typed config scope, type every value against the registry, warn on unrecognised keys with near-miss suggestions, and move claim selection onto typed values. Turns #2 green. | #1, #2 | queued |
| 4 | automatic-value-expansion | Unify the two width-family implementations into the registry and resolve percent-relative values against their `ratio_over` base, so no consumer reads a placeholder and no percent value is discarded. | #1 | queued |
| 5 | scope-resolution-module | Replace the five scattered resolvers and `overlay_resolved` with one resolution module over scope deltas, exposing the Z-grid query and the scope-stack resolve. | #3, #4 | queued |
| 6 | resolved-config-view | Give `ConfigView` one meaning (always resolved), split `extensions` into typed module keys versus dropped unknowns, make emission registry-driven, and delete the 54 guest `unwrap_or` literals. | #5 | queued |
| 7 | scope-eligibility | Add per-key `denied_scopes` to host and module schema entries, author the initial denials for machine- and emitter-level keys, derive the per-object admission set from the registry with declarative transforms, and delete the two inert manifest sections. | #5 | queued |
| 8 | typed-modifier-kind | Carry modifier kind typed across the IR seam, match exhaustively at the ten sites, route modifier deltas through the registry, and delete `ModifierScope`. | #5, #7 | queued |
| 9 | layer-range-scope | Ingest `Metadata/layer_config_ranges.xml`, add layer range as a scope wired to both entry points, and author a fixture carrying one. | #5, #7 | queued |
| 10 | remaining-automatic-values | Implement the speed family's `0 = volumetric auto` fallback and the eight negative `-1 = auto` sentinels. | #4, #5 | queued |
