# Preflight Report: 263-infill-pattern-specific-keys

Reviewed: 2026-09-01 · Mode: --preflight · Symbol-inventory dispatched: 3 (canonical reads ×2 — 10-key declarations/consumers + `symmetric_infill_y_axis` activation-gate/mirror math; in-tree zero-occurrence + decision-point survey ×1; S5/S6/S7/S8 + AC-runnable + Doc-Impact verification ×1, 14-item FACT batch)

## Preflight Gate

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files) | PASS | all five files present and non-empty (`packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, `task-map.md`) |
| S1 Prerequisite-status truth | PASS | deps are resolved wayfinder tickets (06/05/04); packets 253–262 cited as authoring precedent / queue ordering only, never claimed as satisfied dependencies (grep for `implemented`/`shipped` claims about other packets: none); packet 262's same-manifest relationship is recorded as merge churn, not a dependency |
| S2 Deviation-ID conformance | PASS | no deviation IDs created, superseded, closed, or referenced (zero DEV-/D- tokens in the five files) |
| S3 Schema-version computed | PASS | no schema/version constants touched (zero `*_SCHEMA_VERSION` tokens) |
| S4 ADR slot allocation | PASS | no ADRs authored or referenced beyond ADR-0027's conformance mention (no slot claim; 0027 verified to exist at `docs/adr/0027-gyroid-multi-role-fill-holder.md`) |
| S5 Shipped-symbol existence/shape | PASS | 21/21 symbols verified by dispatch: `ConfigFieldEntry.description` + `[config.schema]` parser read (`crates/slicer-scheduler/src/manifest.rs`), `ConfigBoundsIndex::from_modules` / `resolve_global_config` / `ConfigResolutionError::{OutOfRange,TypeMismatch}` (`crates/slicer-scheduler/src/config_resolution.rs`), `ORCA_CONFIG_PADDING` / `serialize_config_block` / `emit_config_kv` (`crates/slicer-gcode/src/serialize.rs`), `guest_input_paths` including module manifests (`xtask/src/build_guests.rs`), `render_deviations` with `parse::<f64>` fallible canonical-default compare (`xtask/src/gen_config_docs.rs`), `run_pipeline_with_raw_config` (`crates/slicer-runtime/src/pipeline.rs`) + `region_between` (test), `RectilinearInfill` / `from_config` / `run_infill` (`rectilinear-infill/src/lib.rs`), the `sparse_infill_density` percent form + `internal_solid_infill_line_width` width form + `enable_prime_tower` bool form, `rectilinear_raw_emit_tdd.rs` + `cooling_config_schema_tdd.rs` guard pattern, `config_bounds_enforcement_tdd.rs` with real `rejects_value_below_min`/`OutOfRange` arms; the 10-key alternation greps over `crates/` and `modules/` return 0 matches each (zero-occurrence claim pinned) |
| S6 WIT/IR identifier drift | PASS | no WIT identifiers named (no WIT changes); `ConfigView` / `InfillIR` ride previously-verified plumbing; no IR variants claimed |
| S7 Test-target wiring | PASS | module tests dir is file-per-binary with NO `[[test]]` section in `rectilinear-infill/Cargo.toml` (auto-discovery); net-new `infill_pattern_specific_config_schema_tdd.rs` lands as its own `--test` binary; scheduler/runtime additions go into existing binaries (no new integration files, no aggregator registration needed) |
| S8 ADR conformance | PASS | ADR-0027 `gyroid-multi-role-fill-holder` governs the `*_fill_holder` claim resolution — the packet's design does not change the default holder config (Decision #2: defaults stay `"rectilinear-infill"`), does not point solid roles at gyroid (Future-Reviewer note), and does not remove top/bottom/bridge emission from gyroid-infill; the ADR's actual clauses quoted by the verification dispatch match the design's conformance note. ADR-0030 / ADR-0061 untouched. No amendment deviation required |
| (existing) AC runnable command | PASS | 5 ACs + 2 negatives, all pipe-suffixed; every `--test` binary verified to exist and drive the asserted behavior (`infill_pattern_specific_config_schema_tdd` net-new auto-discovered; `rectilinear_raw_emit_tdd` drives `run_infill` via the `make_config`/`InfillOutputBuilder` harness; `config_bounds_enforcement_tdd.rs` proves the real-manifest bounds pattern incl. `OutOfRange`/`TypeMismatch`; `gcode_header_thumbnail_config_blocks_tdd.rs` drives CONFIG_BLOCK emission via `run_pipeline_with_raw_config` + `region_between` — proven at packet 257/258/259/260/261/262 authoring; AC-5's compound command verified against the live generated doc) |
| (existing) Doc Impact Statement | PASS | key-presence greps for all 10 keys (AC-5 loop) + deviation-block row-count probe (26) — measured against the real generated block at authoring: 26 data rows, none of the 10 keys inside; re-derived 2026-09-01 |

## Corrections made during preflight

1. **All 10 keys re-adjudicated declared-with-gap (unshipped pattern classes / pattern-gated flag).**
   The tier table's Tier A rows assumed a decision point in the infill modules; authoring-time
   canonical grounding proved otherwise: six keys are consumed only by `FillLockedZag::fill_surface_locked_zag`,
   two only by `FillLateralLattice::fill_surface`, one only by `FillLateralHoneycomb::fill_surface`
   (all unshipped patterns), and `symmetric_infill_y_axis` — the one key with a live in-port
   decision point (the rectilinear scan-line generator) — is canonical-activated only when the
   sparse pattern is zigzag/crosszag/lockedzag (`Fill.cpp` `Layer::make_fills` gate, verified
   verbatim), never for the port's shipped patterns. All 10 declared with-gap in
   `rectilinear-infill.toml`; zero module-source reads; no divergence at defaults; the
   zigzag-family re-open condition recorded in the key's disposition.
2. **Rectilinear-width citation corrected.** The in-tree width-table form cited for the two
   coFloatOrPercent keys is `internal_solid_infill_line_width` in `rectilinear-infill.toml`
   (verified); the earlier draft cited `sparse_infill_line_width` (that table lives in
   `gyroid-infill.toml`).
3. **Guard binary kept distinct from packet 262's.** `infill_pattern_specific_config_schema_tdd.rs`
   (net-new) vs 262's `infill_config_schema_tdd.rs` — the two packets touch the same
   `rectilinear-infill.toml` and Cargo.toml but no net-new file collides; merge churn on the
   shared manifest append region is recorded in design.md, and the `toml = "0.8"` dev-dep is
   add-if-absent (verified absent at 263 authoring).

### Blockers (S4/S5/S6) — fix before any commit

None.

### High (S1/S2/S3/S7/S8) — fix or convert to justified FORWARD-DEP

None.

### Accepted FORWARD-DEPs

- None — the packet's one adjacent draft (262) is not consumed; the only shared surfaces are
  append-region files (`rectilinear-infill.toml`, its Cargo.toml dev-dep), handled as
  queue-order merge churn per the packet design.

**Verdict:** PREFLIGHT PASS
