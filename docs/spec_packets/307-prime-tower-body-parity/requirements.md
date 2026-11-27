# Requirements: 307-prime-tower-body-parity

## Packet Metadata

- Grouped task IDs: `TASK-`none — queue packet from the wayfinder map, `task_ids: []` per the 306 precedent
- Backlog source: `docs/specs/orca-feature-gap/issues/122-author-packet-prime-tower-body-parity.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

The port's prime tower is a purge-only stub: per tool change it emits travel plus rectilinear scan-lines plus a prime entity, and it skips every layer without a tool change. Canonical's tower is a solid structure — shell, sparse/solid infill, first-layer brim, and an idle layer on every spanned layer — globally planned top-down. Fifteen census keys select over that absent geometry (ticket 29), plus the dissolved P22 tool selector (`wipe_tower_filament`) and P24 smooth mode (`timelapse_type`, ticket 31). Packets 254a (pitch/depth/brim/framework), 254b (interface/ramming/flat-ironing), and 255 (seven wall primitives) build the parts; this packet builds the body that gives them meaning: `finish_layer` assembly order, `plan_tower` depth propagation, idle-layer entries, tool selection, and smooth mode with its injection-suppression clause.

## In Scope

- Global depth planning in `run_finalization`: per-layer depth from toolchange purge volumes, top-down max-propagation, tower-wide max depth, idle entries for spanned layers without tool changes, `wipe_tower_no_sparse_layers` gate.
- `finish_layer` assembly per tower layer, in canonical order: inner perimeter of the sparse section, CP EMPTY GRID infill (`wipe_tower_bridging` spacing, solid on the adhesion first layer), outer wall via 255's helpers, first-layer brim via 254a's builder; all `ExtrusionRole::WipeTower` on `RegionKey "__wipe_tower__"`.
- The five manifest declarations (AC-1) plus the two `machine-gcode-emit` reader rows and the smooth-suppression extension of the DEV-168 gate.
- `wipe_tower_filament` tool selection with boundary synthesis and fatal out-of-range validation.
- `prime_tower_skip_points` tower-side `use_gap_wall` gate (open vs closed wall).
- Smooth mode: forced tower on every object layer, depth floor + equalisation, outer-wall-only deliverable.
- Bed-bounds re-validation against the planned max depth; three `DEV` rows; generated-doc regen.

## Out of Scope

- 254a's keys and depth/pitch/brim internals (consumed, never re-declared); 254b's interface/ramming/flat-ironing internals; 255's seven wall primitives and rotation frame.
- `wipe_tower_type` tower selection (no queue key; single-tower union, DEV-201); the `filament_tower_interface_*` Tier-D family beyond 254b's scalar seven.
- Emitter-side travel-avoid routing through the skip-points gap (DEV-203); soluble-lookahead solid triggering (no per-filament solubility model); `ORCA_CONFIG_PADDING` or CONFIG_BLOCK twins of any kind (rule 2; spellings ride ticket 132).
- Rib-mode square-tower re-planning (`WipeTower::plan_tower_new`, 255's DIV-1); runtime re-fitting of spacing to a minimum depth (254a's exclusion).

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular-pipeline goals constraining the single-module body shape)
- `docs/03_wit_and_manifest.md` - direct ranges for `[config.schema]` declaration shape + claim-ID table (rule-4 holder check: `[claims]` stays empty)
- `docs/08_coordinate_system.md` - delegated SUMMARY (mm↔unit boundaries in wall/infill/brim loops)
- `docs/04_host_scheduler.md` - delegated SUMMARY (finalization ordering vs purge anchors; push-vs-insert merge semantics)
- `docs/15_config_keys_reference.md` - generated output only, via `cargo xtask gen-config-docs`
- `docs/DEVIATION_LOG.md` - append-only; three rows per the AC-grepped IDs

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations (type/default/bounds) of the five packet keys + `wipe_tower_type` default (DEV-201 context); `PrintConfigDef::init_fff_params`, never line numbers
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` — `finish_layer` emission order, solid-vs-sparse rule, `plan_tower` backward propagation, `first_toolchange_to_nonsoluble_nonsupport`, `use_gap_wall`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `insert_wipe_tower_extruder` forced-filament layer membership
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `has_wipe_tower` / `enable_timelapse_print` force-tower conditions
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `process_layer` smooth-suppression clause (DEV-202 origin)

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Per-Key Canonical Evidence

- `wipe_tower_bridging` (coFloat, default 10.0, no bounds; `PrintConfig.cpp::init_fff_params`): maximal bridging distance driving the sparse-line count in `WipeTower2::finish_layer`. Behaviour: infill lines span at most this distance; smaller values densify the grid.
- `wipe_tower_no_sparse_layers` (coBool, default false): `m_no_sparse_layers` — whether idle tower layers exist at all (`WipeTower2::finish_layer` gate, `WipeTower` ctor, `GCode.cpp` references).
- `prime_tower_skip_points` (coBool, default true): `WipeTower2::use_gap_wall` + travel-avoid gating in `GCode.cpp` + `WipeTower::set_extruder`. Behaviour: the tower wall leaves a gap corridor instead of closing, so travels can pass through.
- `wipe_tower_filament` (coInt, default 0, min 0; 1-based, 0 = auto): `ToolOrdering::insert_wipe_tower_extruder` (layer membership), `WipeTower2::first_toolchange_to_nonsoluble_nonsupport` (finish-extrusion tool), `set_extruder` solubility masking (both towers), `Print::validate` range assert. Behaviour: forces which filament prints the finish extrusions (sparse infill + wall + brim); auto means the layer's incoming filament.
- `timelapse_type` (coEnum, default Traditional, serialised `"0"`/`"1"` — `s_keys_map_TimelapseType`'s *"using 0,1 to compatible with old files"*): `Print::has_wipe_tower` / `Print::enable_timelapse_print` (force single-filament tower), `ToolOrdering::fill_wipe_tower_partitions` (every object layer), `WipeTower::plan_tower_new` / `update_all_layer_depth` (floor + equalise), `only_generate_out_wall` (wall-only deliverable), `GCode::process_layer` suppression of traditional snapshots. Ingest adapter required: the port declares word spellings, so `"0"`/`"1"` must map explicitly (AC-14), never fall through to the default.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-14`; AC-3 pins the propagation arithmetic, AC-5 the assembly order, AC-9/AC-10 the smooth pair across both modules, AC-14 the canonical ingest spelling.
- Negative: `AC-N1` through `AC-N4`; N2 is the dynamic-range fatal no static bound can express.
- Cross-packet impact: 254a's pitch/depth/brim formulas are consumed at their drafted values (relativised, 255's precedent); 255's wall helpers are called, never rebuilt; 254b's interface block coexists per-purge-block while the body runs per-plan-layer. Landing order 254a → 254b → 255 → 307 absorbs merge churn in `wipe-tower/src/lib.rs`.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Body assembly, planning, tool-select, smooth-tower halves | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Purge coexistence + default identity | FACT pass/fail |
| `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Max-depth bed validation | FACT pass/fail |
| `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Five-table schema guard | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Suppression gate (AC-10) | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Bounds arms (AC-12) | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Hiding arm (AC-N1) | FACT pass/fail |
| `cargo xtask gen-config-docs --check; echo "exit=$?"` | Generated-doc coherence (AC-13) | FACT pass/fail |
| `cargo check --workspace --all-targets; echo "exit=$?"` | Whole-tree type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings; echo "exit=$?"` | Lint gate | FACT pass/fail |
| `cargo xtask build-guests --check; echo "exit=$?"` | Guest freshness (both manifests + both lib.rs are fingerprint inputs) | FACT pass/fail; exit 3 is infra, not clean |

## Step Completion Expectations

Landing order 254a → 254b → 255 → 307. Steps 2–6 read 254a/254b/255's drafted helpers at their drafted names and shapes; if any producer's plan changed a name, reconcile in both specs before activation (preflight FORWARD-DEP rule). The schema-guard file and `toml = "0.8"` dev-dep arrive via 254a; Step 1 carries the contingency if 307 lands first. Default-identity (AC-2) is re-verified after every geometry step, not only at the end.

## Context Discipline Notes

Packet-specific hazards: `wipe-tower/src/lib.rs` is 1298 lines — never load in full; work by `LOCATIONS`-pinned ranges (planning block, `finish_layer` block, `from_config`, `run_finalization`). `PrintConfig.cpp` / `WipeTower2.cpp` are delegated only. Tempting skip: re-deriving ticket 29's census — forbidden, it is read, not re-derived. Heavy dispatches capped: orca reads return `LOCATIONS` ≤ 20 or `SUMMARY` ≤ 200 words.
