# Design: 278-gcode-output-emitter-modes

## Controlling Code Paths

- Host emission: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) walks `LayerCollectionIR.ordered_entities`; each `PrintEntity.region_key.object_id` supplies stable object identity, and `current_tool` plus `orca_type_label` supply verbose extrusion diagnostics.
- Dialect: `GcodeFlavor` and `GcodeFlavor::from_config_str` (`crates/slicer-gcode/src/flavor.rs`) already implement five canonical wire spellings and pressure/temperature/motion syntax. `run_slice` (`crates/slicer-runtime/src/run.rs`) already passes the resolved flavor to emitter and serializer.
- Retraction policy: `PathOptimizationDefault::run_path_optimization` (`modules/core-modules/path-optimization-default/src/lib.rs`) owns retract/no-retract decisions and emits `TravelRetract`, `ZHop`, and `TravelMove`; `DefaultGCodeEmitter::emit_gcode` only serializes these side tables.
- Output routing: the `Cmd::Slice` success arm (`crates/pnp-cli/src/main.rs`) writes the caller's explicit `--output` path or stdout after `run_slice` succeeds. This proves `filename_format` is not an emitter concern.
- Neighboring tests: `crates/slicer-gcode/tests/gcode_emit_tdd.rs`, `crates/slicer-gcode/tests/pressure_advance_emission_tdd.rs`, `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs`, and runtime's aggregated `integration` binary.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Runtime config strings are snake_case. Canonical on-wire values are exact: bools are typed booleans in manifests; `gcode_flavor` accepts only `marlin|klipper|reprapfirmware|repetier|marlin2` and defaults to `marlin`.
- No cross-module algorithm-selection trigger fires. These are output modes, not competing implementations; no claim holder or module is added.
- No WIT/IR/schema version changes. Existing `RegionKey.object_id`, `GCodeCommand::Raw`/`Comment`, `TravelRetract`, `TravelMove`, and `ZHop` carry all behavior.
- `DefaultGCodeEmitter` must not synthesize or remove travel retractions. Packet-15/TASK-120d1 established `path-optimization-default` as the policy owner.
- Structural comments required for preview (`;LAYER_CHANGE`, `;Z:`, `;HEIGHT:`, `;TYPE:`) remain unconditional. `gcode_comments` gates only new verbose diagnostics.
- Object-derived text must not permit line injection. Firmware names use `pnp_` + lowercase UTF-8 hex. Human comments replace `\r` and `\n` with `_`.
- `ORCA_CONFIG_PADDING` and `crates/slicer-gcode/src/serialize.rs` are out of bounds; canonical value spellings are enforced by manifest schemas, not padding edits.

## Tier and Owner Derivation

- `exclude_object`, `gcode_comments`, and `gcode_label_objects`: Tier B behavior in existing owner `crates/slicer-gcode`.
- `gcode_flavor`: behavior already live through `GcodeFlavor`, pressure advance, temperature, and runtime wiring; this packet is Tier A schema enforcement plus Tier B object-marker syntax. It remains an emitter mode, not a claim.
- `reduce_infill_retraction`: Tier B in existing owner `path-optimization-default`, correcting the tier table's `slicer-gcode` attribution. The current module does not emit intra-region travel between ordered non-wall entities and its tests treat multiple same-region wall loops as implicitly suppressed; this packet builds the real consecutive-entity decision, makes suppression opt-in, and restores canonical-default retraction.
- `filename_format`: excluded and returned to host-export because its canonical consumer selects an artifact filename, and PnP's artifact write is CLI-owned.

## Code Change Surface

- Selected approach: add three host-resolved emitter booleans and exact manifest declarations; derive object runs directly while walking ordered entities; share one run-transition helper between independent labels/firmware markers; validate flavor via the owner schema; add `reduce_infill_retraction` to the existing path optimizer and make its current internal suppression conditional.
- Object-run algorithm: collect unique raw object IDs in a sorted set for M486 ordinal assignment and Klipper DEFINE lines. During entity traversal, compare the next entity's object ID with the active run. On change, close the old run, open the new run, then emit its TYPE/entity commands; close the final run at layer end. Repeat starts/ends if an object has multiple non-contiguous runs.
- Marker matrix:

| Flavor | Enabled `exclude_object` start/end | Leading definitions |
| --- | --- | --- |
| `Klipper` | `EXCLUDE_OBJECT_START NAME=pnp_<hex>` / `EXCLUDE_OBJECT_END NAME=pnp_<hex>` | one sorted `EXCLUDE_OBJECT_DEFINE NAME=pnp_<hex>` per object |
| `Marlin`, `Marlin2`, `RepRapFirmware` | `M486 S<n>` / `M486 S-1` | none |
| `Repetier` | none | none |

- Verbose comments: before an entity's first move with nonzero E, emit one `GCodeCommand::Raw` containing `; filament: tool=<current_tool> role=<orca label without ;TYPE:>`. Do not emit per-segment spam or alter structural comments.
- Retraction algorithm: after entity ordering, examine each consecutive pair's `end_point`→`start_point`. Resolve the matching `PerimeterRegionView` by exact `RegionKey`; a net-new private `travel_segment_inside_expolygon` uses `slicer_ir::point_in_polygon_winding` for both mm endpoints plus deterministic segment/ring intersection checks (contour and holes, converting ring points with `Point2::to_mm`) to require the complete segment to stay inside one sparse polygon. Emit travel for every non-coincident pair. Suppress its retract/unretract/Z-hop only when the bool is true, both keys match, the destination role is not a perimeter role, and containment succeeds; otherwise emit the configured matched sequence.
- Config carriers: add `exclude_object`, `gcode_comments`, and `gcode_label_objects` to `ResolvedConfig`, its manual `to_config_map`/`PartialEq`/`Hash` coverage, host-key docs, and `machine-gcode-emit.toml` as the loaded G-code-output schema carrier; add `gcode_flavor` enum there for strict normal-path validation. Add `reduce_infill_retraction` only to `path-optimization-default.toml` and module state.
- Exact implementation/test symbols:
  - `DefaultGCodeEmitter::emit_gcode`, a private object-token/comment sanitizer, and private marker helpers in `crates/slicer-gcode/src/emit.rs`.
  - `ResolvedConfig` macro declaration plus `to_config_map`, `PartialEq`, and `Hash` in `crates/slicer-ir/src/resolved_config.rs`.
  - `PathOptimizationDefault::from_config` and `PathOptimizationDefault::run_path_optimization` in `modules/core-modules/path-optimization-default/src/lib.rs`.
  - `[config.schema]` in `machine-gcode-emit.toml` and `path-optimization-default.toml`.
  - New `crates/slicer-gcode/tests/gcode_output_modes_tdd.rs`; extend `travel_policy_tdd.rs`, runtime integration registration/case, scheduler bounds case, manifest schema guard, host-key lock, and generated docs.
- Rejected alternatives:
  - `filename_format` in `slicer-gcode`: rejected because an emitter returns text/IR and never owns filesystem paths; it would couple library output to CLI routing.
  - Retraction filtering in the emitter: rejected because it cannot prove travel containment and violates the established module-owned policy seam.
  - A new object-label postprocess module: rejected because object identity is present at host emit time and no algorithm selection/extension seam is needed.
  - Raw object IDs in firmware commands: rejected due whitespace/newline injection and collisions after lossy sanitization.
  - Bambu M624 under Marlin: rejected as fabricated dialect behavior; `GcodeFlavor` has no Bambu variant.

## Files in Scope (read + edit)

Primary behavior files (three):

- `crates/slicer-gcode/src/emit.rs` - object runs, exclusion markers, verbose diagnostics.
- `crates/slicer-ir/src/resolved_config.rs` - three typed emitter booleans and manual coverage.
- `modules/core-modules/path-optimization-default/src/lib.rs` - configurable internal-travel policy.

Contract/test companions justified by the three cross-crate seams:

- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, `modules/core-modules/path-optimization-default/path-optimization-default.toml` - exact schemas.
- `crates/slicer-gcode/tests/gcode_output_modes_tdd.rs`, `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs` - behavior tests.
- Runtime integration registry/case, scheduler bounds case, machine manifest schema guard, host-key lock - config delivery and rejection.
- `docs/config/host-keys.toml`, generated `docs/15_config_keys_reference.md`, and the two 04/05 feature-gap assets - contract documentation/dispositions.

## Read-Only Context

- `crates/slicer-gcode/src/flavor.rs` - `GcodeFlavor`, `from_config_str`, `config_str`, pressure/temperature helpers.
- `crates/slicer-runtime/src/run.rs` - only flavor resolution and emitter/serializer construction anchored on `gcode_flavor` / `with_flavor`.
- `crates/pnp-cli/src/main.rs` - `Cmd::Slice.output` and successful output-write arm only.
- `crates/slicer-ir/src/slice_ir.rs` - `PrintEntity`, `LayerCollectionIR`, `TravelRetract`, and `TravelMove` definitions only.
- `docs/01_system_architecture.md` - named sections in `requirements.md` only.
- `docs/08_coordinate_system.md` - conversion checklist only.
- Orca sources listed in `requirements.md` - delegated only.

## Out-of-Bounds Files

- `crates/slicer-gcode/src/serialize.rs` - no CONFIG_BLOCK or padding work.
- `crates/pnp-cli/src/main.rs` and `crates/pnp-cli/tests/**` - `filename_format` is returned, not implemented.
- `crates/slicer-schema/wit/**`, generated bindings, and all IR struct definitions - no boundary changes.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - postprocessing is not the owner of these decisions.
- `OrcaSlicerDocumented/**` - delegate; never load during implementation.
- `target/`, `Cargo.lock`, vendored dependencies, unrelated packets, and ticket 51's header.

## Expected Sub-Agent Dispatches

- Question: confirm canonical defaults/value spellings and object/retract flavor gates; scope: the Orca files listed in `requirements.md`; return: `SUMMARY` <=200 words; purpose: Steps 1-3.
- Question: inventory all `ResolvedConfig {` literals and manual coverage arms before adding fields; scope: `crates/**` and `modules/**`, Rust only; return: `LOCATIONS` <=20; purpose: Step 1 blast radius.
- Question: identify the runtime integration registration pattern and a test helper that executes real `run_slice`; scope: `crates/slicer-runtime/tests/integration/`; return: `LOCATIONS` <=20; purpose: Step 4.
- Question: run each cargo command; scope: the command's package/test only; return: `FACT` pass/fail and <=20 failure lines; purpose: all exits.

## Data and Contract Notes

- IR/manifest: no new IR. The machine manifest is the loaded schema carrier for four emitter settings; the path-optimizer manifest owns its bool.
- WIT boundary: unchanged. `reduce_infill_retraction` is delivered through existing `ConfigView`; outputs use existing builder commands.
- Determinism: sorted raw object IDs define M486 ordinals and Klipper definitions; entity order defines run boundaries; no hash iteration order reaches output.
- Error contract: manifest enum/bool violations use existing `ConfigResolutionError::TypeMismatch`. Helper encoding is total over UTF-8 strings and does not introduce a new error variant.

## Locked Assumptions and Invariants

- Canonical declarations: `exclude_object=false`, `gcode_comments=false`, `gcode_label_objects=true`, `reduce_infill_retraction=false`, `support_object_skip_flush=false`; `filename_format` is a string with canonical default `{input_filename_base}_{filament_type[initial_tool]}_{print_time}.gcode`; flavor default/value spellings are as listed above.
- Empty layers create no object run. Object-run boundaries never cross layer boundaries.
- Human labels and exclusion markers are independent toggles.
- Repetier receives no fabricated object-exclusion command.
- Internal suppression never applies to perimeter-bound or inter-region travel, and suppressing retract also suppresses its paired unretract and Z-hop.
- `filename_format` and `support_object_skip_flush` acquire no declaration/read from this packet.

## Risks and Tradeoffs

- PnP lacks instance/copy identity; `RegionKey.object_id` identifies model objects, so multiple physical copies cannot be excluded independently. This is a bounded divergence and must not be described as full instance-level parity.
- Klipper DEFINE lines omit canonical center/polygon metadata because `LayerCollectionIR` lacks an object footprint at this stage. START/END cancellation remains functional by name; geometry-rich definitions are future additive work.
- Changing current unconditional same-region suppression to canonical default false increases retracts on default profiles. This is intended parity, pinned bidirectionally.
- Adding fields to `ResolvedConfig` has a struct-literal/manual-impl blast radius; inventory before editing and keep all fallout in Step 1.
- Manifest and path-optimizer edits stale guest WASM; freshness is part of Steps 1 and 3.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 object-run emitter behavior and tests)
- Highest-risk dispatch: canonical `GCode::process_layer` / `needs_retraction` summary, return `SUMMARY` <=200 words.

## Open Questions

- `[FWD]` Add a Bambu `GcodeFlavor` plus M624/M625 label-id and filament-flush carrier before re-entering `support_object_skip_flush`.
- `[FWD]` Host-export must decide whether absent `--output` keeps stdout or opts into `filename_format`; that product choice belongs to the future host-export packet, not this emitter packet.
- `[FWD]` Instance-level exclusion needs copy identity preserved into `PrintEntity`; this packet intentionally keys by existing object identity only.

**No `[BLOCK]`.** Every retained key has an existing carrier and a falsifiable non-default behavior; both returned keys name the missing owner/carrier.

## Map and Ticket Updates Required

Implemented as the final documentation step, not during packet authoring:

1. In 04, owner-correct `filename_format` to host-export and `reduce_infill_retraction` to `path-optimization-default`; annotate P44 as five implemented keys after `filename_format` returns.
2. In 05, remove `filename_format` from P44's covered set while retaining it queued for host-export; annotate `support_object_skip_flush` as sequenced after Bambu/M624 flush support rather than folded.
3. Do not modify ticket 51's claimed header. Its answer/link is wayfinder-session work outside this implementation packet.
