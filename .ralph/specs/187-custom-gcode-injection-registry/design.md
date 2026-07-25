# Design: 187-custom-gcode-injection-registry

## Controlling Code Paths

- Primary code path: `run_gcode_postprocess` in `modules/core-modules/machine-gcode-emit/src/lib.rs`. Today it reads two templates, builds one flat `HashMap<String, String>` lookup (seeded with the two temperature keys, then swept over `config.keys()`), substitutes both, and frames the re-emitted stream with `push_raw`. After this packet it iterates `INJECTION_POINTS`, resolves each non-empty template against a **per-site** lookup, and splices the results at the sites the enum names.
- Site source: `DefaultGCodeEmitter::…` in `crates/slicer-gcode/src/emit.rs` pushes three consecutive `GCodeCommand::Raw` markers before each emitted layer's first command — `";LAYER_CHANGE"`, `format!(";Z:{}", format_xyz(layer_z, gcode_xy_decimals))`, `format!(";HEIGHT:{}", …)` — and skips layers with no output entirely, so the marker count equals the header's `; total layer number:`. **Read-only in this packet.**
- Existing pins on those markers: `crates/slicer-gcode/tests/golden_emit_tdd.rs` and `crates/slicer-gcode/tests/gcode_emit_tdd.rs`. The marker text is an established contract, not an assumption this packet introduces.
- Neighboring tests/fixtures: `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` (top-level `#[test]` fns; `slicer_sdk::test_prelude::config_with` builds a `ConfigView` with **exactly** the supplied pairs, so a unit test controls the placeholder domain precisely) and `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` (real WASM component through `run_pipeline_with_raw_config`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Config key strings must be snake_case in Rust and in the manifest. Note that OrcaSlicer's own registry disagrees with its own config key here: `s_CustomGcodeSpecificPlaceholders` keys timelapse as `timelapse_gcode` while `PrintConfigDef::init_fff_params` registers the option as `time_lapse_gcode`. PnP uses the **config key** spelling, `time_lapse_gcode`, everywhere — manifest, registry entry and docs.
- `ConfigView::keys` (`crates/slicer-ir/src/slice_ir.rs`) returns its own `fields` map sorted, with manifest scoping applied by the host at construction rather than by `keys()` itself. The per-site lookup is therefore `{manifest keys} ∪ {site variables}`, and the site variables must be inserted *after* the config sweep so a hypothetical config key of the same name cannot shadow the computed value.
- **The observable error chain, verified link by link. Do not paraphrase from memory.** A guest `ModuleError` becomes `DispatchError.reason`, formatted in `crates/slicer-wasm-host/src/dispatch.rs` as `"module error (code={}, fatal={}): {}"` — five occurrences of that literal; the workspace has **zero** occurrences of any `code={}, n={}` form, which an earlier draft of this packet quoted. `PostpassStageRunner::run_gcode_postprocess` wraps it as `slicer_ir::PostpassError::FatalModule { message: e.to_string() }`, so the dispatch string is **embedded in** the Display output rather than being the whole of it; `Display for PostpassError` (`crates/slicer-ir/src/stage_io.rs`) renders `"fatal postpass module failure in {stage_id} for {module_id}: {message}"`; `crates/slicer-runtime/src/postpass.rs` returns it unchanged and `crates/slicer-runtime/src/pipeline.rs` renders `"postpass failed: {e}"`. Any assertion on this must stop at the `module error (code=` prefix plus the code and key name — never the `fatal={}` field. **Verify the literal with a one-line grep of `crates/slicer-wasm-host/src/dispatch.rs` before asserting on it**; that grep is explicitly authorised in §Read-Only Context, because stating the fact here while forbidding the file made the earlier error unfalsifiable.
- No geometry and no millimetre/internal-unit conversion occurs here. `layer_z` and `max_layer_z` are carried as **text**, never as numbers to be re-rendered, so the `coord-system` constraint does not apply and `mm_to_units` must not appear anywhere in this change surface.
- Determinism: the site walk is a single forward pass over `commands` with no map iteration in the output path, and packet 186's unresolved-key set is a `BTreeSet`. Nothing in this packet may introduce `HashMap` iteration into emitted text.

## Code Change Surface

- **Selected approach — the registry.**
  ```
  enum InjectionSite { PrintStart, BeforeLayerChange, TimeLapse, LayerChange, PrintEnd }
  struct InjectionPoint { config_key: &'static str, site: InjectionSite }
  const INJECTION_POINTS: &[InjectionPoint] = &[ 5 entries, in canonical emission order ];
  ```
  `InjectionSite` is the extension point packet 188 adds toolchange and role variants to; `InjectionPoint` is deliberately a plain data record with no behaviour, so the site→placement mapping lives in one `match` inside the emission walk rather than being scattered across trait impls.
- **Per-site variables.** A `LayerContext { layer_num: u32, layer_z: String, max_layer_z: String }` is threaded to the four sites that have one. `PrintStart` gets `None`. The lookup for a site is the shared config-derived map plus, when the context is `Some`, the three entries `layer_num`, `layer_z`, `max_layer_z`. This is the mechanism AC-N1 tests: the same macro is fatal at one site and resolvable at another, which is precisely canonical's behaviour — `s_CustomGcodeSpecificPlaceholders` gives `machine_start_gcode` an empty extra-variable set and `machine_end_gcode` `{layer_num, layer_z, max_layer_z, filament_extruder_id}`.
- **`layer_z` / `max_layer_z` carry source text, not a re-rendered float.** The `;Z:` marker's payload is whatever `format_xyz(layer_z, gcode_xy_decimals)` produced. Parsing it to `f32` and re-rendering would emit `0.20000000298023224` for `;Z:0.2` and would silently disagree with the Z the surrounding G-code carries. The parse is used **only** for the numeric comparison that finds the running maximum; the substituted value is the winning marker's original text.
- **Emission order at a layer boundary.** Canonical `GCode::process_layer` appends, in order: the `;LAYER_CHANGE` tag, the Z-height tag, the `;HEIGHT:` tag, `before_layer_change_gcode`, `GCode::change_layer`, the non-BBL `time_lapse_gcode`, `layer_change_gcode`. **An earlier draft justified the adjacency with a false claim — that `change_layer` emits nothing outside spiral-vase mode — and told the implementer not to question it. Here is the measured behaviour.** `travel_to_z` is genuinely spiral-vase-only inside `change_layer`, but outside it `change_layer` still appends `m_writer.update_progress(++m_layer_index, m_layer_count)` (M73, guarded by `m_layer_count > 0`), a full `retract(...)` block when `retract_when_changing_layer && m_writer.will_move_z(z)` (with a forced `SpiralLift` when `z_hop_types == zhtAuto`), and `m_writer.add_object_change_labels(gcode)` **unconditionally**. So canonical's three templates are **not** guaranteed consecutive.
  The adjacency is nevertheless correct for PnP, on PnP's own evidence: none of those three emissions has a counterpart in the command stream at that point. M73 is injected by the host's own `inject_m73`, not produced at the layer boundary; layer-change retracts are already materialised as `GCodeCommand::Retract` where `DefaultGCodeEmitter` placed them; and PnP models no EXCLUDE OBJECT labels at all. Consecutive splicing after the `;HEIGHT:` marker is therefore the closest available placement, and the residual (canonical may interleave a retract or an object label; PnP interleaves nothing) is recorded by AC-13 item (d) rather than asserted away.
  Searching for a Z-bearing `Move` to split the templates remains **wrong**, now for the surviving reason: PnP has no dedicated layer-Z move (Z rides on extrusion moves as `z: Some(point.z)` and on z-hop moves as `z: Some(hop_z)`, where `hop_z = layer_z + zh.hop_height` is computed a line earlier in `crates/slicer-gcode/src/emit.rs`), so any such search lands on a z-hop as often as on the layer transition.
- **Marker-contract enforcement.** On seeing `Raw(";LAYER_CHANGE")` the walk looks ahead at most two commands for a `Raw` whose text starts with `";Z:"`. Absent ⇒ `ModuleError::fatal(ERR_MALFORMED_LAYER_MARKER, …)` naming the command index. The alternative — reuse the previous layer's Z — would emit a plausible wrong number into printer G-code, which is the class of failure this whole trilogy exists to remove.
- **Exact functions, manifests, tests, and fixtures.**
  - `run_gcode_postprocess` — replace the two hand-read templates with a registry-driven resolve; add the single forward walk that splices at layer boundaries.
  - New `InjectionSite`, `InjectionPoint`, `INJECTION_POINTS`, `LayerContext`, `const ERR_MALFORMED_LAYER_MARKER: u32` (distinct from packet 186's `ERR_UNRESOLVED_PLACEHOLDER` and from the file's existing 1-11 push-failure codes).
  - `machine-gcode-emit.toml` — three new `[config.schema.*]` string blocks, `default = ""`, `group = "Machine G-code"`.
  - `machine_gcode_emit_tdd.rs` — add `layer_scoped_points_emit_in_canonical_order`, `layer_variables_are_one_based_and_carry_source_text`, `machine_end_gcode_sees_final_layer_context`, `unset_layer_points_emit_nothing`, `layer_macro_in_start_gcode_is_fatal`, `layer_change_without_z_marker_is_fatal`, `unknown_macro_in_layer_change_gcode_is_fatal`.
  - `machine_start_end_gcode_emission_tdd.rs` — add `layer_change_gcode_fires_once_per_emitted_layer`.
- **Rejected alternatives.**
  - *A trait-object per injection point.* Rejected — five points, one placement rule each, no polymorphic behaviour; a trait would spread the ordering across five impls and make the canonical order unreadable at a glance.
  - *Splitting `before_layer_change_gcode` from `layer_change_gcode` around a Z-bearing move.* Rejected on **PnP** evidence: no dedicated layer-Z command exists to split around, so the search would land on z-hop moves. It is **not** rejected on the ground that canonical emits nothing there — measured, `GCode::change_layer` emits `update_progress`, a conditional `retract(...)`, and an unconditional `add_object_change_labels`; only `travel_to_z` is spiral-vase-only.
  - *Deriving layer boundaries from `Move.z` transitions instead of the `;LAYER_CHANGE` marker.* Rejected — z-hop moves carry `z: Some(hop_z)` with `hop_z = layer_z + zh.hop_height`, and would register as spurious boundaries, and the marker is already a golden-pinned host contract.
  - *One flat lookup shared by all sites.* Rejected — it would make `[layer_num]` silently resolvable inside `machine_start_gcode`, where canonical errors, and would destroy the only observable difference between the sites.
  - *Reproducing canonical's `max_layer_z` behaviour at `layer_change_gcode`.* Rejected as a faithful-bug port — and the bug is worse than an earlier draft described. Measured: that block's own `DynamicConfig` never sets `max_layer_z` before the parse; the `set_key_value("max_layer_z", …)` sits **after** the parse call and writes into a local destroyed at the closing brace, and no base or global config layer carries the key. Canonical's `layer_change_gcode` therefore resolves **no `max_layer_z` at all**, rather than a one-layer-late value. PnP supplies one; the divergence is "absent vs present" and is recorded in this packet's new residual row.

## Files in Scope (read + edit)

Two primary source files plus the two test files that must gain the new pins. The extra files are justified in the same terms as packet 186: a behaviour this packet introduces has no home in either primary file.

- `modules/core-modules/machine-gcode-emit/src/lib.rs` — role: registry, site walk, per-site lookup; expected change: new types and constants, `run_gcode_postprocess` rewritten around them.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — role: the three new injection-point keys; expected change: three `[config.schema.*]` blocks.
- `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — role: module-level behaviour pins; expected change: seven tests added.
- `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` — role: end-to-end pin against the real component; expected change: one test added.
- `docs/15_config_keys_reference.md` — role: the user-facing injection-point contract; expected change: §"Machine start / end G-code" rewritten, generated block regenerated.
- `docs/DEVIATION_LOG.md` — role: parity ledger; expected change: `DEV-085` updated, one new residual row.
- `docs/07_implementation_status.md` — role: backlog; expected change: one `TASK-306` row outside the generated markers.

## Read-Only Context

- `crates/slicer-gcode/src/emit.rs` — **long; read only the layer-boundary block** that pushes `Raw(";LAYER_CHANGE")`, `Raw(";Z:…")`, `Raw(";HEIGHT:…")` and the `has_output` skip immediately above it — purpose: confirm the three markers are consecutive and that skipped layers emit no marker, so the splice count equals `; total layer number:`.
- `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`'s `run` and `raw_texts` helpers — purpose: the new tests reuse them rather than building their own harness.
- `crates/slicer-ir/src/slice_ir.rs` — do **not** open; the needed facts (`ConfigView::keys` returns the view's own `fields` map sorted, with manifest scoping applied by the host at construction rather than by `keys()`; `GCodeCommand::Raw { text: String }` is the marker variant) are stated here.
- `crates/slicer-wasm-host/src/dispatch.rs` — **long; read exactly one thing**: grep it for `module error (code=` and confirm the format literal before writing any assertion that depends on it. Do **not** browse the file. §Architecture Constraints states the mapping and the literal, but stating a fact here while forbidding the file outright is what made an incorrect literal unfalsifiable in an earlier draft; a one-line grep is the cheapest possible check against that recurring.
- `crates/slicer-ir/src/stage_io.rs` — grep only, for `Display for PostpassError`'s `"fatal postpass module failure in …"` arm, if a message assertion needs widening. Do not browse.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `crates/slicer-gcode/src/emit.rs` and `crates/slicer-gcode/tests/golden_emit_tdd.rs` — **read-only, edit-forbidden**; AC-10 fails the packet if either is modified.
- `crates/slicer-gcode/src/serialize.rs` — untouched here; it is the file a future `file_start_gcode` packet would need.
- `.ralph/specs/186-*`, `.ralph/specs/188-*` — sibling packets; never edit from here.
- The `<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->` … `<!-- END GENERATED: open-deviations -->` block of `docs/07_implementation_status.md` and the `<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->` … `<!-- END GENERATED: module-config-keys -->` block of `docs/15_config_keys_reference.md` — regenerate via `cargo xtask check-deviations` / `cargo xtask gen-config-docs`; never hand-edit inside the markers.
- Unrelated crates — delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: in `GCode::process_layer`, what is the precise ordered list of items appended between the start of a layer and the first extrusion, and where do `before_layer_change_gcode`, `time_lapse_gcode` and `layer_change_gcode` fall in it? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` ≤ 200 words; purpose: Step 2's ordering.
- Question: does `GCode::change_layer` emit a Z move outside spiral-vase mode? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `FACT` ≤ 3 lines; purpose: the adjacency justification in Step 2.
- Question: what extra placeholder variables does `s_CustomGcodeSpecificPlaceholders` list for `machine_start_gcode`, `before_layer_change_gcode`, `layer_change_gcode`, `timelapse_gcode` and `machine_end_gcode`? Scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` ≤ 6 lines; purpose: Step 2's per-site sets and AC-N1.
- Question: which variables does `GCode::generate_timelapse_gcode` set that the inline non-BBL path does not? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `FACT` ≤ 5 lines; purpose: Step 5's residual row.
- Question: what is the current highest `DEV-###` in `docs/DEVIATION_LOG.md`, and does the `DEV-085` row already contain the string `TASK-306`? Scope: `docs/DEVIATION_LOG.md`; return: `FACT` ≤ 3 lines; purpose: Step 5.
- Question: does `cargo xtask build-guests --check` report `STALE:` after the module edit? Scope: cargo run; return: `FACT` clean/stale; purpose: Steps 2 and 3.

## Data and Contract Notes

- **IR/manifest contracts.** Three new scalar string keys on one module's `[config.schema]`. No IR schema version change, no new struct field on a shared type, no public version constant bump — so the struct-literal blast-radius discipline does not apply. `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` enrols `classic-perimeters` and `arachne-perimeters` only, so no reconcile row is needed for `machine-gcode-emit`. `gcode_header_thumbnail_config_blocks_tdd`'s CONFIG_BLOCK assertion is a lower bound ("at least 80 key-value lines"), so three more keys cannot break it. The e2e harness `slice_with_raw` seeds manifest defaults generically from `module.config_schema().entries` and routes **string** defaults to `binding_source` as the real value and to `pipeline_source` as an empty sentinel, so three new string keys with `default = ""` need no harness edit and produce `; <key> = ` lines in the CONFIG_BLOCK.
- **The host↔module marker contract.** `;LAYER_CHANGE` / `;Z:<z>` / `;HEIGHT:<h>` become a real interface between `crates/slicer-gcode/src/emit.rs` and this module. It is not a WIT contract and has no version; `ERR_MALFORMED_LAYER_MARKER` is what makes a future emitter change that breaks it fail loudly instead of silently mis-splicing. Say so in the module's doc comment, and cite `crates/slicer-gcode/tests/golden_emit_tdd.rs` as the pin on the emitter side.
- **WIT boundary.** Unchanged. `run_gcode_postprocess` keeps its signature; only new failure values and new emitted `Raw` commands appear. No `crates/slicer-schema/wit/**` edit and therefore no bindgen invalidation — but `modules/core-modules/machine-gcode-emit/src/**` and its `.toml` **are** guest-WASM inputs, so the freshness gate still applies.
- **Determinism/scheduler constraints.** `machine-gcode-emit` is the sole module registered at `PostPass::GCodePostProcess`, so there is no sibling-ordering interaction. The layer walk is a single forward pass; per-layer state is `(layer_num, layer_z_text, running_max_text)` only.

## Locked Assumptions and Invariants

- **Locked: the layer-boundary marker triple is the injection site.** `;LAYER_CHANGE`, then a `;Z:`-prefixed `Raw` within two commands. A future emitter change that reorders or renames them must update this module in the same commit; `ERR_MALFORMED_LAYER_MARKER` is the tripwire.
- **Locked: `layer_num` is 1-based at every site.** The canonical citation is more subtle than a single expression: `before_layer_change_gcode` runs **before** `GCode::change_layer` and uses `m_layer_index + 1`, while `time_lapse_gcode` and `layer_change_gcode` run **after** it and use bare `m_layer_index` — `change_layer` having incremented it via `++m_layer_index` inside `m_writer.update_progress`, which is itself guarded by `m_layer_count > 0`. All three yield the same 1-based value on any real print, so PnP's single 1-based counter is right; only the earlier one-expression citation was imprecise. Changing the base later silently rewrites every user's start/layer G-code.
- **Locked: `layer_z` / `max_layer_z` substitute the emitter's own formatted text.** Anything else desynchronises the template from the G-code around it.
- **Locked: the placeholder lookup is per-site.** A macro available at one site must not become available at another by widening a shared map.
- Not locked: the numeric value of `ERR_MALFORMED_LAYER_MARKER`; AC-N2 asserts the constant's name and the behaviour, not a digit.
- Not locked: `INJECTION_POINTS`' entry count — packet 188 grows it, and no AC in this packet asserts a total beyond the five it introduces.

## Risks and Tradeoffs

- **The module now depends on host-emitted comment text, and it is not the only consumer.** `crates/pnp-cli/src/visual_debug_gcode.rs` already parses `;LAYER_CHANGE`, `;Z:` and `;TYPE:` (its own module doc says so, and it does `line.starts_with(";LAYER_CHANGE")` / `line.strip_prefix(";Z:")`). This packet makes `machine-gcode-emit` a **second** consumer of an unversioned marker contract that has no single owner, so a future emitter change now breaks two call sites in different crates. Mitigated by `ERR_MALFORMED_LAYER_MARKER` (loud failure, not silent drift), by the existing goldens in `crates/slicer-gcode/tests/golden_emit_tdd.rs`, and by AC-10's guard that this packet does not touch the emitter — but the coupling itself is real and should be named in the module's doc comment alongside the `visual_debug_gcode.rs` cross-reference.
- **A start/end refactor could move the start block.** The start block must still precede the M73 pair and `ExtrusionMode` — `emit.rs`'s own comment records that `machine-gcode-emit` rebuilds the stream rather than splicing into it, which is why the ordering holds. **Two guards, in two different binaries.** `machine_start_gcode_precedes_m73_and_extrusion_mode` lives in `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` and is covered by **AC-8**; `start_block_position_before_extrusion_mode_and_first_g1` and `end_block_position_after_last_g1_before_config_block` live in `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` and are covered by **AC-9**. An earlier draft attributed all three to AC-9.
- **`time_lapse_gcode` ships as the non-BBL inline form only.** A user on a BBL-style workflow gets a materially smaller feature than canonical. Recorded in the new residual row rather than half-implemented.
- **Guest staleness.** Every `--test integration` result is meaningless until `cargo xtask build-guests --check` is clean.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — registry, site walk, per-site lookup, and the start/end migration in one atomic change)
- Highest-risk dispatch and required return format: the `GCode::process_layer` ordering question. It must return `SUMMARY` ≤ 200 words as an **ordered list of named items**, never source. `process_layer` is very long and a careless dispatch will try to return it.

## Open Questions

- **Resolved, not `[FWD]`.** Canonical's Z-height tag is **not** a build variant. Re-measured: `GCode::process_layer` contains a single runtime ternary inside one `sprintf` — `print.is_BBL_printer() ? "; Z_HEIGHT: %g
" : ";Z:%g
"` — so the spelling is chosen per printer at run time, and the non-BBL branch is exactly PnP's `;Z:`. Keying only on `;Z:` is therefore correct and complete for every printer PnP models. If PnP ever grows a BBL flavour, extend the lookahead rather than replacing it.
- `[FWD]` `docs/15_config_keys_reference.md` §"Machine start / end G-code" is titled for two keys and now covers five. Retitle it (for example "Custom G-code injection points") in Step 4; AC-11 asserts content, not the heading text, so the retitle is safe but should be done once rather than twice across 187 and 188.
- `[FWD]` `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`'s `[module] description` still says the module "Emits machine_start_gcode / machine_end_gcode". Update it in Step 3; it is prose inside the manifest, so no AC can see it, and `cargo xtask gen-config-docs` does not regenerate it.
