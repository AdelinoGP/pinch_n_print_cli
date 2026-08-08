# Design: 207-paint-segmentation-per-region-shell-config

## Controlling Code Paths

- Primary code path: `execute_paint_segmentation` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) → `painted_subsets` accumulation → the `match region_map.configs.first()` shell-parameter block → the `for ((sname, value), (semantic, painted_mesh, source_objects)) in &painted_subsets` loop → `top_bottom::propagate_top_bottom` (`crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs`). The defect is entirely in the middle two links; the outer two are unchanged.
- Precedent to mirror, **in shape only**: `resolve_shell_counts` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) — builds a `RegionKey`, guards with `entries.contains_key`, calls `config_for`, and falls back to `(3, 3)` when no plan entry exists. Its `RegionKey` uses an empty `variant_chain` because it runs over unpainted BASE regions; **this packet's resolver must use the painted `variant_chain`, not an empty one** (see §Locked Assumptions "Granularity lock"). Copy the `config_for` + fallback structure, not the empty chain. Read-only; not edited.
- The lookup idiom this packet reuses — stated precisely, because `execute_paint_segmentation` contains **three** distinct `chain_key`-family bindings and they do different things:
  1. `base_chain_key` (the empty `Vec`, in the BASE-emission block of Phase 4) — used in the `region_map.entries` key scan `rk.global_layer_index == global_layer_index && rk.variant_chain == base_chain_key` that produces `matching_base`.
  2. The **Phase-4** `chain_key` (`vec![(sem_name.clone(), value.clone())]`, built from the per-semantic `sem_name` and the `polys_by_color` colour) — used in the `region_map.entries` key scan `rk.global_layer_index == global_layer_index && rk.variant_chain == chain_key` that produces `matching_keys`, and in `paint_variant_region_id`.
  3. The **Phase-6/7** `chain_key` (`vec![(sname.clone(), value.clone())]`, built from the `painted_subsets` loop's own `sname` / `value`, immediately *after* the `propagate_top_bottom` call) — this one does **not** touch `region_map` at all. It filters SliceIR regions: `region.variant_chain == chain_key` in the clip loop, `.position(|r| r.variant_chain == chain_key)` for the merge target, and `variant_chain: chain_key.clone()` on the synthesised region.

  So the `region_map.entries` scan on `rk.variant_chain == …` exists only in the Phase-4 bindings (1) and (2); the Phase-6/7 binding (3) is the one at this packet's call site and it is purely a SliceIR-region filter. `region_key_for_chain` is therefore a **new scan that reuses the Phase-4 idiom**, not a second consumer of an existing one: it drops the `global_layer_index` equality (Phase 4 pins the layer; the resolver must not), adds an `object_id` equality (Phase 4 has none), and reduces with `min_by_key` instead of collecting all matches. The saving on effort is the pattern, not the code.
- Correct `RoleWidthContext` exemplar to mirror: the `width_context` construction in `modules/core-modules/classic-perimeters/src/lib.rs`, which populates `bridge_line_width`, `initial_layer_line_width`, `outer_wall_line_width` and `inner_wall_line_width` via `get_abs_value(key, nozzle_diameter)` — the percent-aware accessor `ext_abs_mm` mirrors. The arachne construction covers the same fields but reads them with `get_float`, which drops percent forms; treat it as a field-coverage reference only.
- `ConfigView::get_abs_value` (`crates/slicer-ir/src/slice_ir.rs`) is the normative percent-resolution rule `ext_abs_mm` must reproduce.
- The two host-side sites (this one, and the one in `crates/slicer-core/src/algos/lightning/mod.rs`) are the only ones with a hardcoded nozzle and zero role widths; this packet fixes one of them.
- Neighboring tests/fixtures: `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` (the gated sibling whose `#![cfg(feature = "host-algos")]` preamble and `build_region_map` / `run_paint_segmentation` helper shape the new test file copies); the six packet-128 tests inside `mod driver_v2_tests` in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, plus its `empty_region_map()` and `region_map_with_base_entry()` fixtures.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **`configs.first()` was already forbidden.** `docs/02_ir_schemas.md` §"Config Interner Contract (Normative — Packet 91)" states that all production code reads a region's config via `region_map.config_for(&key)` and that the interner model is the only supported read path. This packet does not introduce a new rule; it removes the last production violation of an existing one. Any replacement that still indexes `configs` positionally fails the packet regardless of test results.
- **The `None` arm is dead and must be deleted, not repaired.** `RegionMapIR::default()` pre-seeds `configs: vec![ResolvedConfig::default()]`, and `execute_region_mapping_inner` (`crates/slicer-core/src/algos/region_mapping.rs`) interns a config for every plan it inserts, so `configs.first()` is `Some` unconditionally. Keeping a `None` arm would preserve the `0.45`-vs-`0.4` disagreement that DEV-122 names as a secondary defect.
- **Exactly one *default value*, even though the lookup ladder has two tiers.** The ladder is painted chain → the object's BASE chain → `(3, 3)` + `ResolvedConfig::default()`; only the last tier is a hardcoded default, and it matches `resolve_shell_counts`' own `(3, 3)` deliberately so the two resolvers cannot drift. See §Locked Assumptions "Fallback-ladder lock" for the authoritative statement.
- **`entries` is a `HashMap`.** Plan selection — at BOTH ladder tiers — MUST be an explicit `min_by_key(|(k, _)| (k.global_layer_index, k.region_id))` over the filtered candidates. A `.find(...)` or first-iteration pick would make shell depth depend on hash order, which is a determinism violation, not a style preference. Note this differs from the file's existing `matching_keys` scan, which collects *all* matches because it merges into every one of them; the resolver needs a single deterministic winner.
- **Packet 128's contract is preserved, not replaced.** After the re-key each `painted_subsets` value's `source_objects: BTreeSet<String>` holds exactly one id. The Phase 6/7 `None` arm that stamps `object_id` from that set keeps working unchanged and becomes trivially unambiguous. `assert_per_object_shell_index_invariant` and the shell-index propagation block are read-only in this packet.
- **`propagate_top_bottom`'s signature is unchanged.** It already takes `top_shell_layers`, `bottom_shell_layers`, `extrusion_width_mm`, `layer_height_mm` as scalars. Moving the call inside a per-`(object, chain)` key means each invocation receives that object-and-chain's scalars — no signature change is needed, contradicting the plan's assumption that one is.
- **The four role-width keys are not typed fields.** `ResolvedConfig` declares only `line_width: f32 = 0.0`, `initial_layer_line_width: f32 = 0.0`, `layer_height: f64 = 0.2`, `top_shell_layers: u32 = 3`, `bottom_shell_layers: u32 = 3` among the values needed. `nozzle_diameter`, `outer_wall_line_width`, `inner_wall_line_width` and `bridge_line_width` route to `extensions: BTreeMap<String, ConfigValue>` because the generating macro sends unknown keys there. Reading them from `extensions` is the correct access path; adding typed fields would be a `slicer-ir` schema change with a workspace-wide struct-literal blast radius and is explicitly out of scope.
- **`cfg.layer_height` is `f64`; `propagate_top_bottom` takes `f32`.** The existing cast is correct and its rationale (shell-window math uses it as a thickness, not a Z-plane coordinate) must be preserved in the moved code.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- **Selected approach.** Three coordinated edits, all inside `crates/slicer-core/src/algos/paint_segmentation/mod.rs`:

  1. **Re-key `painted_subsets`.** `BTreeMap<(String, PaintValue), (PaintSemantic, IndexedTriangleSet, BTreeSet<String>)>` becomes `BTreeMap<(String, String, PaintValue), (PaintSemantic, IndexedTriangleSet, BTreeSet<String>)>` where the new leading element is `obj.id.clone()`. Both accumulation arms (facet paint and stroke paint) already sit inside `for obj in &mesh.objects`, so the object id is in scope at both `entry(key)` calls; the change is mechanical. The value tuple is untouched, so packet 128's `source_objects` insert stays as written and simply always inserts the same id. `BTreeMap` ordering stays deterministic with `ObjectId` (a `String`) leading the key.

  2. **Replace the config block with a resolver.** Delete the whole `match region_map.configs.first()` expression, its four `let` bindings (`top_shell_layers`, `bottom_shell_layers`, `shell_line_width`, `shell_layer_height`), the `None => (3, 3, 0.4, 0.2)` arm and the `TODO:` comment. Add, at module scope:

     ```text
     struct ShellParams { top: usize, bottom: usize, width_mm: f32, layer_height_mm: f32 }
     fn region_key_for_chain(
         region_map: &RegionMapIR,
         object_id: &str,
         chain_key: &[(String, slicer_ir::PaintValue)],
     ) -> Option<RegionKey>
     fn ext_abs_mm(cfg: &ResolvedConfig, key: &str, base: f32) -> Option<f32>
     fn shell_params_from_config(cfg: &ResolvedConfig) -> ShellParams
     fn resolve_shell_params(
         region_map: &RegionMapIR,
         object_id: &str,
         chain_key: &[(String, slicer_ir::PaintValue)],
     ) -> ShellParams
     ```

     `region_key_for_chain` filters `region_map.entries` keys on `k.object_id == object_id && k.variant_chain == chain_key` and returns the `min_by_key` on `(k.global_layer_index, k.region_id)`. It serves both ladder tiers: tier 1 passes the painted `chain_key`, tier 2 passes `&[]` (the BASE chain), so there is one filter implementation, not two. `resolve_shell_params` is therefore:

     ```text
     region_key_for_chain(region_map, object_id, chain_key)
         .or_else(|| region_key_for_chain(region_map, object_id, &[]))
         .map(|k| shell_params_from_config(region_map.config_for(&k)))
         .unwrap_or_else(|| shell_params_from_config(&ResolvedConfig::default()))
     ```

     with the terminal arm pinned at `top = 3, bottom = 3` (`ResolvedConfig::default()` already carries `top_shell_layers = 3` / `bottom_shell_layers = 3`, so `shell_params_from_config` on the default yields the documented fallback without a second literal). `shell_params_from_config` builds the `RoleWidthContext` (below) and calls `crate::flow::resolve_role_width(ExtrusionRole::OuterWall, false, false, &ctx)` for `width_mm`, takes `cfg.top_shell_layers as usize` / `cfg.bottom_shell_layers as usize`, and `cfg.layer_height as f32` with the existing precision comment carried over.

     **The Phase-6/7 `chain_key` is already built in the loop, just below the call site — but it is a SliceIR-region filter, not a `region_map` lookup.** The Phase 6/7 body constructs `let chain_key: Vec<(String, slicer_ir::PaintValue)> = vec![(sname.clone(), value.clone())];` immediately after `propagate_top_bottom` returns, and uses it only against SliceIR regions (`region.variant_chain == chain_key`, `.position(|r| r.variant_chain == chain_key)`, `variant_chain: chain_key.clone()`). It is nevertheless the exact value `resolve_shell_params` needs, because a subset's chain and the region's chain are the same `Vec`.

     The edit **hoists that Phase-6/7 binding above the `propagate_top_bottom` call** (it depends only on the loop's `sname` / `value`, so the move is trivially valid) and adds `let params = resolve_shell_params(&region_map, object_id, &chain_key);` before the call, passing its four fields in place of the old bindings.

     **Identify the binding to hoist by position, not by name.** Three `chain_key`-family bindings already exist in this function (see §Controlling Code Paths): `base_chain_key` and the Phase-4 `chain_key`, both of which scan `region_map.entries` with a `global_layer_index` pin, plus the Phase-6/7 one. The hoist target is unambiguously the third: the binding inside `for (… ) in &painted_subsets`, sitting between the `propagate_top_bottom` call and the `for (l, polys) in phase6.per_layer.iter().enumerate()` loop. **Do not touch the two Phase-4 bindings, do not reuse either of them here** (Phase 4's is scoped to a different loop and pinned to one layer), and **do not add a fourth binding inside the Phase-6/7 loop** — hoist the one that is there, or the resolver's chain and the region filter's chain can drift.

  3. **Repair `RoleWidthContext`.** Replace the `{ line_width: cfg.line_width, nozzle_diameter: DEFAULT_NOZZLE_DIAMETER_MM, ..Default::default() }` literal with a full construction mirroring the classic/arachne exemplars: `nozzle_diameter` = `ext_abs_mm(cfg, "nozzle_diameter", 0.0)` falling back to the file-local `DEFAULT_NOZZLE_DIAMETER_MM` when absent or non-positive (preserving today's behaviour rather than silently substituting `0.0`); `line_width` = `cfg.line_width`; `initial_layer_line_width` = `cfg.initial_layer_line_width`; `outer_wall_line_width`, `inner_wall_line_width`, `bridge_line_width` = `ext_abs_mm(cfg, <key>, nozzle_diameter).unwrap_or(0.0)`, matching the exemplars' `get_abs_value(<key>, nozzle_diameter)` base. `..RoleWidthContext::default()` still covers `top_surface_line_width`, `internal_solid_infill_line_width` and `sparse_infill_line_width`, which the outer-wall role never reads.

  `ext_abs_mm` mirrors `ConfigView::get_abs_value` (`crates/slicer-ir/src/slice_ir.rs`) **clause for clause**, over all seven `ConfigValue` variants:

     - `ConfigValue::Percent(p)` → `Some(p / 100.0 * base)` when `base > 0.0`, else `None`
     - `ConfigValue::FloatOrPercent { value, is_percent }` → when `is_percent`, the same base rule as `Percent`; otherwise `Some(value)`
     - `ConfigValue::Float(f)` → `Some(f)`
     - `Bool`, `Int`, `String`, `List` → `None` (note: canonical `get_abs_value` deliberately does **not** coerce `Int`; its match arm is `_ => None`. `ext_abs_mm` must not add an `Int` arm or it stops mirroring)
     - missing key → `None`

     `FloatOrPercent` is load-bearing, not a completeness footnote: it is the variant `crates/slicer-macros/src/lib.rs` produces from the WIT config bridge, and `outer_wall_line_width` is a `coFloatOrPercent` key upstream — so omitting the arm would make `ext_abs_mm` return `None` for exactly the key AC-4 exists to prove. The `Percent`/`FloatOrPercent` zero-base rule is what AC-N3's second and third clauses pin. Result is cast to `f32` at the boundary; `get_abs_value` works in `f64`.

- **Exact functions, traits, manifests, tests, and fixtures.**
  - New (module-private, `crates/slicer-core/src/algos/paint_segmentation/mod.rs`): `ShellParams`, `region_key_for_chain`, `ext_abs_mm`, `shell_params_from_config`, `resolve_shell_params`.
  - Modified: `execute_paint_segmentation`'s `painted_subsets` type and both `entry(key)` sites; the Phase 6/7 loop destructuring pattern, its hoisted `chain_key` binding, and its `propagate_top_bottom` call.
  - Deleted: the `match region_map.configs.first()` expression, its `None => (3, 3, 0.4, 0.2)` arm, and the `TODO: when per-object/per-region paint configs are wired…` comment.
  - **Ten tests, split across two homes by reachability** (see §Test Homing below): four end-to-end tests in the new integration binary `crates/slicer-core/tests/paint_segmentation_per_region_shell_config_tdd.rs` (AC-2, AC-6, AC-9, AC-N2), and six unit tests in a new in-crate `#[cfg(test)] mod shell_config_resolver_tests` inside `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (AC-1, AC-4, AC-5, AC-N1, AC-N3, AC-N4).
  - The integration file opens `#![cfg(feature = "host-algos")]`. No `[[test]]` entry is added to `crates/slicer-core/Cargo.toml` — the sibling `paint_segmentation_mmu_partition_tdd.rs` has none either and relies on the inner attribute.
  - No manifest, WIT, IR, or schema-version change. `crates/slicer-ir/**` is untouched.

- **Test Homing (NORMATIVE — the five new items stay module-private).**

  The five new items are `fn` / `struct` with no `pub`, so they are reachable only from inside `crates/slicer-core`. `crates/slicer-core/tests/*.rs` is a **separate crate**; it can call `execute_paint_segmentation` (which is `pub`) and nothing else on that list. Every AC that names one of the five directly must therefore live in-crate, and every AC that drives behaviour through the public entry point lives in the integration binary.

  | AC | Target it must reach | Home | Runner |
  | --- | --- | --- | --- |
  | AC-1 | `resolve_shell_params` (private) | in-crate `mod shell_config_resolver_tests` | `--lib` |
  | AC-4 | `shell_params_from_config` (private) | in-crate | `--lib` |
  | AC-5 | `shell_params_from_config` (private) | in-crate | `--lib` |
  | AC-N1 | `resolve_shell_params` (private), tier-3 arm | in-crate | `--lib` |
  | AC-N3 | `ext_abs_mm` (private), `Option` return | in-crate | `--lib` |
  | AC-N4 | `resolve_shell_params` (private), tier-2 arm | in-crate | `--lib` |
  | AC-2 | `execute_paint_segmentation` (`pub`) | integration `paint_segmentation_per_region_shell_config_tdd.rs` | `--test <file>` |
  | AC-6 | `execute_paint_segmentation` (`pub`) | integration | `--test <file>` |
  | AC-9 | `execute_paint_segmentation` (`pub`) | integration | `--test <file>` |
  | AC-N2 | `execute_paint_segmentation` (`pub`) | integration | `--test <file>` |

  - **The in-crate module is new, and it sits alongside the existing one.** `crates/slicer-core/src/algos/paint_segmentation/mod.rs` already ends with a `#[cfg(test)] mod driver_v2_tests` (packet 128's shell-index tests plus the `empty_region_map()` / `region_map_with_base_entry()` fixtures). That module is **read-only in this packet** — it is exactly what AC-7 gates. Add a *sibling* `#[cfg(test)] mod shell_config_resolver_tests` after it, with its own `use super::*;`. Reuse `driver_v2_tests`' fixtures by copying their shape, not by importing across the two test modules.
  - **`--lib` needs `--features host-algos` too.** `crates/slicer-core/src/lib.rs` gates `pub mod algos` behind `#[cfg(feature = "host-algos")]`, so a bare `cargo test -p slicer-core --lib` compiles the whole `paint_segmentation` module — and therefore the new test module — out of existence and prints `ok` with zero tests. This is the same trap as the integration file's inner `cfg`, one level up. Every `--lib` AC command carries `--features host-algos` **and** a nonzero-pass assertion.
  - **Rejected: making the five items `pub` (or `pub(crate)` plus a re-export) so the integration binary could call them.** That would widen `slicer-core`'s public surface purely for test reach, and `ext_abs_mm` in particular is a deliberate local mirror of `ConfigView::get_abs_value` — publishing it would create a second public percent-resolution API that callers could reasonably prefer over the canonical one. In-crate unit tests give the same coverage at zero API cost, which is why this file already keeps `driver_v2_tests` in-crate.
  - **AC-N3's "absent, never `0.0`" clause is observable only in-crate — which is why it goes there.** `ext_abs_mm` returns `Option<f32>`, so `None` versus `Some(0.0)` is a direct assertion at the unit boundary. Through the public path it is *not* observable, because item 3 above funnels both through `.unwrap_or(0.0)` into the same `RoleWidthContext` field. Homing AC-N3 in-crate makes the clause falsifiable rather than decorative; it is not weakened and not dropped.

- **Rejected alternatives and reasons.**
  - *Keep `painted_subsets` merged across objects and resolve the config from the lexicographically-first `source_objects` entry.* Rejected: it invents a tie-break policy for exactly the multi-object case DEV-122 flags as needing a decision, and it silently gives object B object A's shell depth. Smaller diff, wrong answer.
  - *Keep the merged subset and require all `source_objects` to resolve to an equal `ShellParams`, erroring otherwise.* Rejected: turns a legal scene (two objects, different shell settings, same paint colour) into a slice-fatal error.
  - *Resolve per object only, from the object's BASE (empty-`variant_chain`) plan, mirroring `resolve_shell_counts` exactly.* **Rejected by user decision (2026-08-07)**, and rejected on the evidence. An earlier draft of this packet claimed per-region resolution was "not implementable because painted facets carry no region identity". That claim is false at the granularity that matters: the subset key `(sem_name, PaintValue)` **is** one `RegionKey.variant_chain` element, `execute_region_mapping_inner` writes a separate `entries` row per chain whose `RegionPlan.config` points at a config with the `paint_config:<semantic>:<key>` overlay already folded in (the `ConfigId` itself is deduped across equal configs — the chain distinction is in the `RegionKey`, see §Locked Assumptions), and the `rk.variant_chain == …` scan over `region_map.entries` already exists in the same function's Phase-4 block. Reading the BASE plan would drop every `paint_config:<semantic>:top_shell_layers` / `bottom_shell_layers` / `line_width` overlay — the same class of silent-placeholder defect DEV-122 was filed for. It is also *more* code than the correct version, since the chain filter already exists.
  - *Re-key on the geometric `region_id` as well (`(object_id, region_id, sem_name, value)`).* Rejected, and this is the part the false claim was reaching for: `painted_subsets` accumulates 3D mesh triangles with no per-layer region membership, and `region_id` is a per-layer, post-slicing concept. The `(object_id, variant_chain)` pair is the finest key `painted_subsets` can express, and where one object/chain spans several `region_id`s the lowest-`(global_layer_index, region_id)` plan wins — the same tie-break `resolve_shell_counts` uses, recorded as a `[FWD]` below.
  - *Add `nozzle_diameter` / `outer_wall_line_width` / `inner_wall_line_width` / `bridge_line_width` as typed `ResolvedConfig` fields.* Rejected: a `slicer-ir` struct change with a workspace-wide struct-literal blast radius, gated by the packet-194 churn rules, for no gain over the `extensions` path the macro already provides.
  - *Change `propagate_top_bottom` to take a per-object/per-chain slice of parameters.* Rejected: unnecessary. Moving the call site inside a per-`(object, chain)` key gives each invocation its own scalars with no signature change, and the plan's assumption that a signature change is required was falsified by grounding.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - role: the entire defect and the entire fix, **and the home of the six unit-level AC tests**; expected change: `painted_subsets` re-key, five new module-private items, config block replaced, `RoleWidthContext` repaired, plus a new `#[cfg(test)] mod shell_config_resolver_tests` sibling to the existing `driver_v2_tests`.
- `crates/slicer-core/tests/paint_segmentation_per_region_shell_config_tdd.rs` - role: NEW; expected change: file created with the **four** end-to-end AC tests (AC-2, AC-6, AC-9, AC-N2) and a `#![cfg(feature = "host-algos")]` preamble. The other six ACs are unreachable from an external test crate and live in-crate — see §Test Homing.
- `docs/DEVIATION_LOG.md` - role: DEV-122 closure; expected change: one Status cell.
- `docs/02_ir_schemas.md` - role: normative interner contract; expected change: one sharpening bullet in the Packet-91 contract subsection.

One production file. The change is deliberately confined to a single module so the `painted_subsets` re-key and the resolver land atomically.

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - symbol-located windows ONLY: the `painted_subsets` declaration and both accumulation arms; the `match region_map.configs.first()` block; the Phase 6/7 loop, its `chain_key` binding and its `propagate_top_bottom` call; the Phase-4 `matching_base` / `matching_keys` scans (read-only — they show the `entries` filter idiom the resolver widens, and confirm which `chain_key` binding is which); and, strictly read-only, the shell-index propagation block plus `assert_per_object_shell_index_invariant` and the `driver_v2_tests` fixtures `empty_region_map()` / `region_map_with_base_entry()`. The tail `#[cfg(test)] mod driver_v2_tests` is read-only in this packet; the new `mod shell_config_resolver_tests` is added after it.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - `resolve_shell_counts` only - purpose: copy its `config_for` call and `(3, 3)` fallback **shape**. Do NOT copy its empty `variant_chain` — this packet resolves against the painted chain; see §Locked Assumptions "Granularity lock".
- `crates/slicer-core/src/flow.rs` - `RoleWidthContext`'s nine fields and `resolve_role_width`'s branch order including the zero-width `1.125 * nozzle_diameter` auto rule.
- `crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs` - `propagate_top_bottom`'s signature and the `shell_step` / `small_thr` / `top_depth.max(1)` block only - purpose: confirm exactly which four scalars matter and that `.max(1)` guarantees AC-N2. Do not edit.
- `crates/slicer-ir/src/slice_ir.rs` - `RegionMapIR`, its hand-written `Default`, `config_for`, `intern_config`, `RegionPlan`, `ConfigId`, `RegionKey`, `ConfigValue` - locate by symbol; do not page the file.
- `modules/core-modules/classic-perimeters/src/lib.rs` - the `width_context` construction only - purpose: the **primary** `RoleWidthContext` exemplar. Use this one, not arachne's: classic reads each width with `get_abs_value(key, nozzle_diameter)`, which is exactly the percent-resolving behaviour `ext_abs_mm` must mirror, whereas arachne uses `get_float` and silently drops percent forms.
- `modules/core-modules/arachne-perimeters/src/lib.rs` - the `width_context` construction only - purpose: secondary exemplar for field coverage only; do NOT copy its `get_float` access pattern.
- `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` - the header and its `build_region_map` / `run_paint_segmentation` helpers - purpose: the gated-test preamble and fixture shape the new file copies.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-ir/src/resolved_config.rs` - macro-generated and very long; every needed fact is in `requirements.md` §Context Discipline Notes. Delegate any further lookup.
- `crates/slicer-runtime/src/run.rs` - the `slice_has_paint` injection is the *cause* of the placeholder surviving, and it is correct. Do not edit.
- `crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs` - read-only; its shell math is out of scope.
- `docs/07_implementation_status.md` Open Deviation Map - generated by `cargo xtask check-deviations`; never hand-edit.
- `docs/15_config_keys_reference.md` - generated; never hand-edit. **It is regenerated by `cargo xtask check-deviations` as well as by `cargo xtask gen-config-docs`** — the `check-deviations` arm in `xtask/src/main.rs` calls `check_deviations::run` and then `gen_config_docs::run`, and the doc itself says the CI `check-deviations --check` step "invokes the same `gen-config-docs --check` code path". So AC-8's `check-deviations --check` gates doc-15 drift too. This packet adds no config key, so it should not need regenerating; if `--check` says otherwise, run `cargo xtask check-deviations` (no `--check`) — do not hand-edit and do not declare the drift out of gate.
- `docs/spec_packets/206-seam-paint-delivery/**` - another packet's directory; never modify.
- `docs/spec_packets/_OLD/128_paint-segmentation-shell-index-invariant.md` - read only through a delegated `SUMMARY` if needed at all.

## Expected Sub-Agent Dispatches

- Question: after the `painted_subsets` re-key, do all six packet-128 shell-index tests still pass?; scope: `cargo test -p slicer-core --features host-algos shell_index` (filter `shell_index`, NOT `shell_index_invariant` — the latter misses `phase6_7_none_arm_stamps_shell_index_on_new_region`); return: `FACT` pass/fail with the test-result line AND the pass count, which must be exactly 6; purpose: Step 2 exit.
- Question: list every read of `painted_subsets` (declaration, `entry(` calls, iteration sites) in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`; scope: that file; return: `LOCATIONS` (≤20 entries); purpose: Step 2 blast radius for the key-type change.
- Question: does `crates/slicer-core/Cargo.toml` declare a `[[test]]` target for `paint_segmentation_mmu_partition_tdd`, and does that file gate itself with an inner `#![cfg(feature = "host-algos")]`?; scope: `crates/slicer-core/Cargo.toml`, the test file's first 15 lines; return: `FACT` (≤5 lines); purpose: Step 1 — the new test file must follow the same pattern or it compiles to zero tests.
- Question: `cargo xtask build-guests --check` — any `STALE:` line?; scope: cargo; return: `FACT` pass/fail; purpose: Steps 2 and 3 exits.
- Question: does `layer_color_stat` in `multi_material_segmentation_by_painting` (`MultiMaterialSegmentation.cpp`) read shell counts and extrusion width from the layer's own `PrintRegion` config, with no default-substitution path?; scope: `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp`; return: `SUMMARY` (≤200 words), no line numbers; purpose: the parity claim in the DEV-122 closure note.

## Data and Contract Notes

- IR/manifest contracts: none changed. `RegionMapIR`, `RegionPlan`, `ConfigId`, `RegionKey` and `ResolvedConfig` are read-only here; no schema version constant is bumped, so no struct-literal or constant-assertion blast radius applies. The config keys read (`top_shell_layers`, `bottom_shell_layers`, `line_width`, `initial_layer_line_width`, `layer_height`, and the four `extensions` widths) are all snake_case, per the repo-wide key convention; no new key is introduced.
- WIT boundary: none crossed. `execute_paint_segmentation` is host-side; its output `SliceIR` shape is unchanged.
- Determinism/scheduler constraints: two determinism-critical points. (1) Plan selection over the `HashMap` `entries` — at the painted-chain tier and at the BASE-chain tier alike — must be `min_by_key` on `(global_layer_index, region_id)`. (2) `painted_subsets` stays a `BTreeMap`, so adding `ObjectId` as the leading key element keeps iteration order total and stable, which in turn keeps the Phase 6/7 merge order stable. Neither the stage graph nor any scheduler edge changes.

## Locked Assumptions and Invariants

- **Granularity lock (USER DECISION, 2026-08-07).** Shell parameters are resolved **per painted `variant_chain`, per object** — from the `RegionKey` whose `object_id` matches the subset's object and whose `variant_chain == vec![(sem_name.clone(), value.clone())]`, selected deterministically by `min_by_key` on `(global_layer_index, region_id)`. This is expressible on this data. The `chain_key` *value* is already built at the call site (the Phase-6/7 binding), and the `region_map.entries` key scan on `rk.variant_chain == …` already exists in the **Phase-4** block — as two separate bindings, not one; see §Controlling Code Paths for the exact three-binding inventory. The resolver reuses that scan's idiom (widened to `object_id`, unpinned from `global_layer_index`, reduced by `min_by_key`) rather than inventing one.

  The reason this is not merely *possible* but *required*: `execute_region_mapping_inner` (`crates/slicer-core/src/algos/region_mapping.rs`) writes **one `entries` row per `(layer, object, region, chain)`** — a `RegionKey` carrying `variant_chain: chain` mapped to a `RegionPlan`. For each chain it folds the matching per-semantic config in with `overlay_resolved(effective, sem_cfg)` — the `paint_config:<semantic>:<key>` overlay resolved by `resolve_per_paint_semantic_configs` (`crates/slicer-scheduler/src/config_resolution.rs`) — then calls `region_map_out.intern_config(effective)` and stores the resulting `ConfigId` on that row's `RegionPlan.config`.

  **`intern_config` does NOT mint a distinct `ConfigId` per chain.** It is a linear-scan dedup (`self.configs.iter().position(|c| c == &rc)`) on `ResolvedConfig`'s `PartialEq`, so two chains whose *resolved* configs are equal share one `ConfigId` — exactly as `docs/02_ir_schemas.md`'s interner-contract bullet already states ("equivalent configs reuse the same `ConfigId`"). The per-chain distinction therefore lives in **`entries`** (the `HashMap<RegionKey, RegionPlan>`, keyed on `variant_chain` among other fields), **not** in the `configs` pool. This does not weaken the decision — it is what makes it work: the resolver must key on `RegionKey.variant_chain` and go through `config_for`, because the `ConfigId` alone cannot tell chains apart. It also explains why the fixtures matter: an AC that interns *equal* configs under two chains proves nothing, so AC-1 / AC-6 / AC-9 / AC-N4 must intern configs that genuinely differ (see `implementation-plan.md` Step 1's fixture note).

  `overlay_resolved` is a hand-written 28-field diff — one `if overlay.<field> != d.<field>` arm per field, an implicit whitelist rather than a general "any field that differs" copy — followed by an *unconditional* merge of every `extensions` key. All four keys this packet needs are covered: `top_shell_layers`, `bottom_shell_layers`, `line_width` and `layer_height` each have their own explicit arm, and the four `extensions`-routed width keys ride the unconditional merge. So resolving shell params from the BASE (empty-`variant_chain`) plan would silently discard `paint_config:<semantic>:top_shell_layers` — reproducing DEV-122's own failure mode one axis down. AC-1 and AC-9 exist to catch exactly that. (The whitelist shape is worth knowing for a *future* key: a `ResolvedConfig` field with no arm would not overlay at all.)

  Note in passing: the in-code comment in `execute_region_mapping_inner` that cites `slicer-scheduler::config_resolution::resolve_paint_overrides` names a symbol that does not exist — the function is `resolve_per_paint_semantic_configs`. That stale comment is the origin of the same wrong name in earlier drafts of this packet. **Do not fix the comment in this packet**; `crates/slicer-core/src/algos/region_mapping.rs` is not in the change surface.

- **Fallback-ladder lock.** Exactly two lookup tiers and exactly one terminal default:
  1. `RegionKey` with matching `object_id` and `variant_chain == chain_key` → `config_for` on it.
  2. No such key (the case Phase 4's own `matching_keys.is_empty()` arm already handles by synthesising a variant region) → the same object's BASE `RegionKey` (`variant_chain.is_empty()`), same `min_by_key` tie-break.
  3. Neither exists → `(3, 3)` plus `ResolvedConfig::default()`-derived width and layer height, matching `resolve_shell_counts`' own `(3, 3)` so the two resolvers cannot drift.

  Tier 2 is a *lookup*, not a default value — there is still only one hardcoded default, so the `0.45`/`0.4` disagreement DEV-122 names does not return. AC-N4 pins tier 2; AC-N1 pins tier 3.
- **Read-path lock.** `region_map.config_for(&key)` is the only permitted config read. Positional indexing of `configs` is forbidden, per `docs/02_ir_schemas.md`'s Packet-91 normative contract.
- **Packet-128 lock.** `source_objects` remains in the `painted_subsets` value tuple and the Phase 6/7 `None`-arm stamping is unchanged; after the re-key the set is a singleton, which strengthens the invariant rather than bypassing it.
- **Contact-layer lock.** `propagate_top_bottom`'s `top_depth = top_shell_layers.max(1)` floor is preserved; a configured zero shell count must still yield the contact layer (AC-N2).
- Reversibility: no config default changes and no schema version moves, so a scene whose interned config happens to equal `ResolvedConfig::default()` produces bit-identical output before and after.

## Risks and Tradeoffs

- **This moves painted-model output — deliberately, and for every user whose shell/width/layer-height settings, or whose `paint_config:<semantic>:*` overlays, differ from the placeholder.** That is the fix. The compensating guards are AC-7 (packet 128's six invariants), the MMU partition suite, and the `cube_4color` / `cube_4color_modifier_part` e2e runs, all of which use single-object default-config fixtures and must stay byte-identical.
- **Multi-object projection changes shape.** Today one merged `propagate_top_bottom` call covers all objects painted the same colour; afterwards there is one call per object. Where two objects' painted projections overlap in XY, the Phase 7 `difference_ex` sequence now runs twice on disjoint geometry instead of once on the union. For physically separated objects the result is identical; for overlapping ones it is more correct (each object's own shell depth applies). No existing fixture exercises the overlap case — a gap worth noting in the closure record rather than papering over.
- **`ext_abs_mm` is a fourth implementation of percent resolution** (alongside `ConfigView::get_abs_value` and the two module-side `get_abs_value` call patterns). It is module-private and deliberately mirrors `get_abs_value` clause-for-clause; if it drifts, AC-N3 fails. Promoting a shared helper onto `ResolvedConfig` would be cleaner but is a `slicer-ir` API addition outside this packet's surface.
- **Feature-gate blindness is the likeliest way this packet reports a false green.** `slicer-core`'s `default = []`; a bare `-p slicer-core` run compiles the new gated test file to an empty binary and prints `ok`. Every AC command carries `--features host-algos` plus a nonzero-pass assertion for exactly this reason.
- **Guest staleness is the silent kind here.** A `slicer-core` edit does not fail typed instantiation; it just leaves every guest running the previous geometry code. `cargo xtask build-guests --check` is a required exit on every step that touches the crate.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — the re-key plus the resolver, which must land together)
- Highest-risk dispatch and required return format: the packet-128 invariant run after the re-key — `FACT pass/fail` with the `test result:` line **and** the pass count, because a zero-count "ok" from a missing `--features host-algos` is indistinguishable from success in the summary line alone.

## Open Questions

- `[FWD]` Where a single object/chain spans several regions or layers with divergent `top_shell_layers` (there is one `entries` row per `(layer, object, region, chain)`, each with its own `RegionPlan.config`, so this is possible), this packet takes the lowest-`(global_layer_index, region_id)` plan, matching `resolve_shell_counts`' `timeline.first()`. Whether that tie-break is *right* — as opposed to merely consistent — is unresolved in both places. The implementer should not change it here; if the fixtures show it mattering, file a new `DEV-###` row covering both resolvers, deriving the next free ID at the moment of filing (e.g. `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`).
- `[FWD]` `crates/slicer-core/src/algos/lightning/mod.rs` carries the second copy of the file-local `DEFAULT_NOZZLE_DIAMETER_MM = 0.4` and the same hardcoded-nozzle / zero-role-width `RoleWidthContext` pattern. It is out of this packet's surface, but it is the same defect class. The implementer should confirm whether it is already covered by an existing deviation row before filing a new one — do not fix it here.
- `[FWD]` No existing fixture has two objects whose painted projections overlap in XY, so the merged-vs-per-object Phase 7 difference is untested at the overlap boundary. AC-6 covers two objects with independent shell counts, which is the user-visible half. If the implementer can build an overlapping fixture cheaply, add it; if not, record the gap in the DEV-122 closure note rather than claiming full coverage.
