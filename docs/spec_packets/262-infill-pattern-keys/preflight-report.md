# Preflight Report: 262-infill-pattern-keys

Reviewed: 2026-09-01 · Mode: --preflight · Symbol-inventory dispatched: 4 (canonical reads ×3 — 7-key declarations/consumers, fill-algorithm details, enum value lists; in-tree pipeline survey ×1; S5/S6/S7/S8 verification ×2)

## Preflight Gate

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files) | PASS | all five files present and non-empty (`packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, `task-map.md`) |
| S1 Prerequisite-status truth | PASS | deps are resolved wayfinder tickets (06/05/04/105/107); packets 253–261 cited as authoring precedent only, never claimed as satisfied dependencies (grep for `implemented`/`shipped` claims about other packets: none) |
| S2 Deviation-ID conformance | PASS | no deviation IDs created, superseded, closed, or referenced (zero DEV-/D- tokens in the five files) |
| S3 Schema-version computed | PASS | no schema/version constants touched (zero `*_SCHEMA_VERSION` tokens) |
| S4 ADR slot allocation | PASS | no ADRs authored or referenced (zero ADR tokens) |
| S5 Shipped-symbol existence/shape | PASS | 28/28 symbols verified by dispatch: `RectilinearInfill::from_config`/`run_infill`/`scan_expolygon` (rectilinear-infill/src/lib.rs), `GyroidInfill::from_config`/`fill_expolygon`/`rotate_expolygon`/`solid_fill_role` (gyroid-infill/src/lib.rs), `solid_fill_role` (rectilinear-infill/src/lib.rs), `ORCA_CONFIG_PADDING` with `("sparse_infill_pattern","grid")` + `("gap_fill_target","nowhere")` (crates/slicer-gcode/src/serialize.rs), `ConfigBoundsIndex::from_modules`/`resolve_global_config`/`ConfigResolutionError::{OutOfRange,TypeMismatch}` (crates/slicer-scheduler/src/config_resolution.rs), `load_module_from_paths` (crates/slicer-scheduler/src/manifest.rs), `run_pipeline_with_raw_config` (crates/slicer-runtime/src/pipeline.rs) + `region_between` (test), `bridge_orientation_deg` (region view), `mm_to_units` (crates/slicer-ir/src/slice_ir.rs), `guest_input_paths` (xtask/src/build_guests.rs — fingerprints module manifests, confirming the wasm-staleness constraint), `render_deviations` numeric-only comparison (xtask/src/gen_config_docs.rs), `cooling_config_schema_tdd.rs` guard pattern, `seam_position` enum+values form (seam-planner-default.toml), `machine_start_gcode` string form (machine-gcode-emit.toml), `cli "sparse_fill_holder"`/`"top_fill_holder"` default `"rectilinear-infill"` (crates/slicer-ir/src/resolved_config.rs), `filter_out_gap_fill` (classic-perimeters manifest + src), `infill_direction` (rectilinear-infill.toml + read site), `description` field parsed by `crates/slicer-scheduler/src/manifest.rs` (on `ConfigFieldEntry`), `ExtrusionRole::{SparseInfill,TopSolidInfill,BottomSolidInfill,InternalSolidInfill,BridgeInfill}` (crates/slicer-ir/src/slice_ir.rs), `InfillOutputBuilder` (crates/slicer-sdk/src/builders.rs), `ConfigViewBuilder` (slicer-sdk test fixtures), package names `rectilinear-infill`/`gyroid-infill` |
| S6 WIT/IR identifier drift | PASS | no WIT identifiers named (no WIT changes); IR role variants verified (see S5); `InfillIR`/`ExtrusionPath3D` shapes confirmed at authoring |
| S7 Test-target wiring | PASS | module tests dirs are file-per-binary with NO aggregator (no `tests/main.rs`; no `[[test]]` in either Cargo.toml — auto-discovery); net-new `infill_config_schema_tdd.rs` lands as its own `--test` binary; scheduler/runtime additions go into existing binaries (no new integration files, no `main.rs` registration needed) |
| S8 ADR conformance | PASS | ADR-0027 `gyroid-multi-role-fill-holder` governs the `*_fill_holder` claim resolution — this packet does not change the default holder config (Decision #2), does not point solid roles at gyroid, and does not remove top/bottom/bridge emission from gyroid-infill (both Future-Reviewer notes); conformance note added to `design.md` Architecture Constraints. ADR-0030 (modifier splits) and ADR-0061 (bridge-orientation tie-break) not touched. No amendment deviation required |
| (existing) AC runnable command | PASS | 8 ACs + 2 negatives, all pipe-suffixed; every `--test` binary verified to exist and drive the asserted behavior (`infill_config_schema_tdd` net-new auto-discovered; `rectilinear_raw_emit_tdd` + `gyroid_infill_tdd` drive `run_infill` via the `make_config`/`ConfigViewBuilder` harness — the angle/per-layer test patterns exist today; `config_bounds_enforcement_tdd.rs` proves the real-manifest bounds pattern incl. `OutOfRange`/`TypeMismatch`/enum rejection; `gcode_header_thumbnail_config_blocks_tdd.rs` drives CONFIG_BLOCK emission — proven at packet 258/259/260/261 authoring) |
| (existing) Doc Impact Statement | PASS | key-presence greps for all 7 keys + deviation-block row-count probe (26, sed-pattern matched against the real generated doc block at authoring — 26 data rows measured, re-derived 2026-09-01) |

## Corrections made during preflight

1. **Pattern keys re-adjudicated declared-with-gap (module identity).** The tier
   table's Tier A rows for `sparse_infill_pattern` / `internal_solid_infill_pattern`
   assumed a pattern decision point in the infill modules; authoring-time grounding
   proved the port's pattern IS the module identity (rectilinear/gyroid/lightning each
   implement one family; the host selects via `*_fill_holder`). The keys are declared
   with-gap; the port-side decision point (holder mapping) is recorded as host-side
   config-resolution work; the two behavior divergences at defaults (port rectilinear
   vs canonical crosshatch/monotonic) are recorded, not deviation rows.
2. **`gap_fill_target` owner nuance.** The tier table's owner `infill modules` is
   right for the canonical side (`FillBase.cpp::Fill::_create_gap_fill` is fill-step);
   the port's gap fill is the perimeter-side `process_classic` mechanism (already
   ported in classic-perimeters/arachne-perimeters, gated by `filter_out_gap_fill`),
   which canonical's `gap_fill_target` does not gate. Declared-with-gap; no wiring to
   the perimeter mechanism (would change default behavior against canonical).
3. **Padding twin correction.** `ORCA_CONFIG_PADDING`'s `("sparse_infill_pattern",
   "grid")` contradicts the canonical default `crosshatch` (verified in
   `PrintConfig.cpp`); corrected in the packet (ticket 14's `fuzzy_skin` precedent).
   `("gap_fill_target", "nowhere")` matches canonical and stays.
4. **Template metalanguage scoped out.** Canonical's `calculate_infill_rotation_angle`
   metalanguage (joints/repeats/units) is declared-with-gap; only the comma-separated
   list form is wired (metalanguage strings fall back to the base angle with a logged
   warn). Default "" is identity either way.
5. **Enum value lists canonical-exact.** `sparse_infill_pattern` carries the full
   26-value InfillPattern list; `internal_solid_infill_pattern` carries the 8-value
   top-fill list (`def_top_fill_pattern`'s `enum_values` — canonical assigns it
   directly); `gap_fill_target` carries everywhere/topbottom/nowhere. Verified by
   dispatched canonical read.
6. **Deviation-row count re-derived.** The generated deviations block measures **26
   data rows** at authoring (post-ticket-107 state); AC-8 pins 26 post-packet (the two
   numeric declared defaults 1/45 match canonical; enum/string defaults never enter
   the numeric comparison map in `render_deviations`).
7. **ADR-0027 conformance note added.** The S8 sweep surfaced ADR-0027
   `gyroid-multi-role-fill-holder` as governing the holder-resolution surface; the
   packet conforms (no holder-default change, no gyroid emission removal) and the
   conformance is now stated explicitly in `design.md` Architecture Constraints.

## Accepted FORWARD-DEPs

- None. The declared-with-gap keys' future consumers (a pattern-dispatch packet
  wiring `sparse_infill_pattern` / `internal_solid_infill_pattern` to the host
  `*_fill_holder` resolution; a fill-side gap-fill packet wiring `gap_fill_target` to a
  new `_create_gap_fill`-equivalent) do not exist as authored packets — the gaps are
  recorded in the per-key evidence table, not forward-dep'd.

## Verdict

**PREFLIGHT PASS** (0 blockers, 0 high)
