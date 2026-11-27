# Proptest/OpenRSCAD rollout evidence ledger (G-ROLLOUT)

## 1. Header and provenance

This is the durable, checked closure index for the Proptest/OpenRSCAD rollout. It is written by
packet `proptest-openrscad_17_pbt-ci-closure`
(`docs/spec_packets/proptest-openrscad_17_pbt-ci-closure/packet.spec.md`). It indexes evidence; it
is **not** itself a claimed pass. A row is `PASS` only when it links an actual archived run.

| Field | Value |
|---|---|
| Source plan | `docs/specs/proptest-openrscad-adoption.md` (§10 host matrix, §11 module matrix, §12 runner/CI, §13 measurement gates, §16 handoff) |
| Registry | `test-properties.toml` — **93** `[[property]]` records, 93 unique `id`s |
| Charters | `docs/testing/properties.md` — **93** `## <ID>` charter headings; the heading set equals the registry ID set (symmetric difference empty) |
| Re-derivation command | `grep -cE '^\[\[property\]\]' test-properties.toml` and `grep -cE '^## [A-Z]+-[A-Z0-9-]+\s*$' docs/testing/properties.md`; set equality checked with a `tomllib` load of the registry against `re.findall(r'^## ([A-Z]+-[A-Z0-9-]+)\s*$', …)` over the charter file |
| `git rev-parse HEAD` at drafting | `6f3e2b7f3f65cd341769fecf86226c17dc2d2fb7` (branch `feature/proptest-openrscad`) |
| Working tree during implementation | Dirty until the status commit; the final provenance row below records the shipped heads. Local lanes ran at `f952bd07` (13 dirty paths); final-head coverage is the PR run at `312e5278` and the dispatch runs at `fd9d5463` |
| Predecessor status | #1–#16 `packet.spec.md` front matter all read `status: implemented`; #17 reads `status: draft` (re-derived with `grep -n '^status' docs/spec_packets/proptest-openrscad_*/packet.spec.md`) |
| Final provenance | PR head `312e5278` (merged to `master` as `fd9d5463`); dispatch runs at `fd9d5463`. Lane tokens and digests are in §6.1; gate logs under `target/evidence/proptest-rollout/gates/`; CI artifacts under `target/ci-evidence/`. Runs: 2026-11-26/27, Windows, rustc 1.96.0. The final status commit is docs-only and follows the green workspace ceremony. |

**Closure verdict: COMPLETE.** Every gate is satisfied: the 21 control aliases, both routine and
both extended lanes (89/89 core and 93/93 SCAD-inclusive records), both SCAD geometry lanes, the
`H-PAINT-OVERHANG` extended measurement, both replay pairs, `GRAPH`, all 55 predecessor forward
items, the PR-hosted Windows checks, the required-check ruleset (id 28214978), both manual
dispatches (extended PASS; failure-probe failed as designed with its artifact), and the gated
workspace ceremony (`VERDICT: PASS`, exit 0). The packet status is `implemented`.

## 2. Verdict and closure gaps

Every row below is a gate; all G1–G13 are resolved or explicitly closed. The table keeps each
disposition and its evidence URL for traceability. AC-N2 is satisfied: nothing is left as optional
coverage.

| # | Gate | Current state | Impact | Owner | Next action |
|---|---|---|---|---|---|
| G1 | Required PR status (AC-3) | **Resolved:** ruleset `master required checks` (id `28214978`) is active on `refs/heads/master` and requires `Properties (Windows)`, `Scripted SCAD properties (Windows)`, `SCAD adapter (Windows)` and `SCAD ingestion (Windows)`; the API response is archived at `target/evidence/proptest-rollout/ruleset-created.json`. An admin-role bypass (`bypass_mode: always`) is configured for docs-only final commits; the docs-only status commit `85f0d9c4` used it, and its push CI run `45961720670` is in progress at the time of writing (the code-head push run `45950316945` was green) | Closed | Repo admin | Confirm `45961720670` green; consider `bypass_mode: pull_request` |
| G2 | Manual extended + failure-probe dispatch (AC-4) | **Resolved:** `extended` run 45950327847/job 140344427469 passed both complete selections (89/93 end events, artifacts at `target/ci-evidence/extended-45950327847/`); `failure-probe` run 45950340929/job 140344467445 failed as designed with downloadable `pbt-ci-failure-probe` (marker + real `property_executor_*` status controls, artifact at `target/ci-evidence/failure-probe-45950340929/`) | Closed | #17 | None |
| G3 | `H-PAINT-OVERHANG` extended tier | **Resolved:** `H-PAINT-OVERHANG-extended` passed (1 record, 256 cases, census 51); the first attempt failed only because the evidence-copy harness filled the disk, and the re-run after freeing duplicates passed | Closed for this record | #11 (executed by #17) | Recorded in §5 |
| G4 | Classic post-offset degeneracy | **Resolved by classification:** the canonical `process_classic` and `process_arachne` (`src/libslic3r/PerimeterGenerator.cpp`) each contain the comment "collapse too narrow infill areas" and do not re-screen every offset perimeter ring for degeneracy; the PnP classic module follows that contract, and the exact band oracle plus deterministic witnesses cover ring validity. No SUT defect | Closed; no fix | #12 | None |
| G5 | Gyroid upstream grid-origin recheck | **Resolved:** canonical `make_gyroid_waves` (`src/libslic3r/Fill/FillGyroid.cpp`) runs `bb.merge(align_to_grid(bb.min, Point(2*M_PI*distance, 2*M_PI*distance)));` after the comment "align bounding box to a multiple of our grid module"; `gyroid-infill::align_to_grid` mirrors `floor(val/grid)*grid`. The selected checkout is authoritative for this recheck | Closed | #13 | None |
| G6 | Wave-overhang fork delegated read | **Resolved:** the selected canonical tree has no wave-overhang fork; the nearest `wave` code is `Algorithm/RegionExpansion` propagation, a different algorithm. The PnP module is a product implementation and makes no canonical parity claim | Closed | #13 | None |
| G7 | #15 cold compile and hosted lane | **Resolved:** cold compile of the four #15 owner crates' property-test targets = **135 s** (`CARGO_TARGET_DIR=target/cold-probe-15`, exit 0, `target/evidence/proptest-rollout/cold-15/compile.log`). The PR `Properties (Windows)` job run 45946601128 executes all 89 core records, including the 11 #15 IDs, on head `312e5278` | Closed | #15 / #17 | None; the PR run is recorded in §5.1 |
| G8 | Hosted Windows PR runs at closure head | **Resolved:** PR run `45946601128` at head `312e5278` passed all four required Windows jobs (properties, scripted SCAD, adapter, ingestion) plus all Ubuntu jobs, including `Test` (conclusion success); artifacts are downloaded under `target/ci-evidence/pr-45946601128/` (89 core end events / census 51/0; 93 SCAD end events / census 54/0; ingestion 12 tests; adapter 3 tests) | Closed | #17 | None |
| G9 | Closure command set (§5) | The 21 control aliases, every `P-*`/`R-*`/`H-PAINT-OVERHANG-extended` lane and `GRAPH` passed. The current `summary.txt` ends `batch=controls failed_aliases=0`, `batch=routine failed_aliases=0` and `batch=replay failed_aliases=0`. First attempts of `X`, `GCODE`, `BRIM`, `P-core-routine` and `P-scad-routine` exited 101 on the pre-fix merged tree and were re-run green after the fixture/cap fixes | All direct closure commands, both dispatches and the hosted checks are evidenced | #17 | None |
| G10 | Dependency isolation (`GRAPH`, §9) | **Resolved:** all 11 host graphs exit 0; the workspace/default graphs have 0 `openrscad|manifold|proptest` hits; 60/60 guest manifests have 0 leaks; the enabled SDK/adapter graphs reach the engine/property stack as positive controls | Closed for the measured graphs | #17 | Evidence in §9 |
| G11 | Scanner blind-spot finding (§10) | **Resolved:** the outer `GeometryDomain` waiver and 10 waiver placements are fixed; `cargo xtask check-literals` (0 violations), `check-test-quality --report` (0 findings) and the per-file review in §10 are recorded | Closed for the gate, with the recorded low/medium test-quality findings in §10.1 | #17 | Track the §10.1 findings with their owners |
| G12 | Final workspace ceremony | **Resolved:** `cargo xtask test --summary --workspace` returned `VERDICT: PASS` / exit 0 on the merged tree after the seam-placer and SDK property-context fixture fixes. The gated runner applies its pre-existing built-in `--skip arachne_parity_pipeline_concentric_infill_uses_arachne` (not a packet change) | Closed | #17 | Evidence in §10.2 |
| G13 | `M-LAYERS-SEQ`/`M-LAYERS-MERGE` oracle independence | **Resolved by explicit disposition:** the merge-plane builders are a documented characterisation pin (`// test-quality:` waiver) and the two charters now state the limitation, crediting the independent native-top model and the deterministic A/B, raft, mixed and exact-end witnesses instead | Closed as a recorded limitation, not as independent correctness | #15 | None; re-run the lanes on the final tree |

**Master-side AC-6 change.** The PR merge brought `8821b30f` (ticket 162), which replaced
`lexical_overlay_witness` with `authority_precedence_witness` and rewrote the H-REGIONMAP charter.
The PR run tested the merged tree; `CHAIN-core` and `CHAIN-runtime` were re-run on the final head
`85f0d9c4` and each passes (`target/evidence/proptest-rollout/CHAIN-core/final-head.log`,
`CHAIN-runtime/final-head.log`: 1 passed, 0 failed).

**First-attempt failures resolved.** On the first merged-tree pass, `X`, `GCODE`, `BRIM`,
`P-core-routine` and `P-scad-routine` exited 101: the SDK/gcode/skirt fixtures still used the
pre-merge config keys and the G-code oracle lacked the merged wipe-tower cap. Each was fixed and
re-run green; the current `summary.txt` contains only `batch=controls`, `batch=routine` and
`batch=replay`, all with `failed_aliases=0`.

## 3. Family and inventory matrix

ID prefix convention used by this ledger (a convention note, not a registry rule): `H-` = a
source-plan §10 host family; `M-` = a §11 module row; `D-` = an SDK direct/scripted-domain helper
record. Some `H-` records are owned by module packages where the host family explicitly delegates
to the native module kernel: `H-LIGHTNING-*` → `lightning-infill`; `H-COOLING` → `part-cooling`;
`H-OVERHANG-SPEED` → `overhang-classifier-default`; `H-GCODE-TEMPLATE` → `machine-gcode-emit`;
`H-SKIRT-BRIM` → `skirt-brim`; `H-WIPE-TOWER` → `wipe-tower`.

Charter anchors are `docs/testing/properties.md#<id-lowercase>` (for example
`docs/testing/properties.md#h-ir-anchors`).

### 3.1 Source-plan §10 host families

| §10 row | Registry IDs | Owner package(s) | Predecessor | Evidence document |
|---|---|---|---|---|
| H-COORD | `H-COORD` | `slicer-ir` | #6 | `docs/testing/proptest-ir-config-evidence.md` |
| H-IDS | `H-IDS` | `slicer-ir` | #6 | `docs/testing/proptest-ir-config-evidence.md` |
| H-IR | `H-IR-ANCHORS` (validator; #1), `H-IR-SERDE` (#6) | `slicer-ir` | #1, #6 | `docs/testing/proptest-foundation-evidence.md`, `docs/testing/proptest-ir-config-evidence.md` |
| H-CONFIG | `H-CONFIG-INTERN`, `H-CONFIG-BOUNDS` | `slicer-ir`, `slicer-scheduler` | #6 | `docs/testing/proptest-ir-config-evidence.md` |
| H-CHAINS | `H-CHAINS` (enumerator order only) | `slicer-ir` | #6 | `docs/testing/proptest-ir-config-evidence.md` |
| H-DAG | `H-DAG` | `slicer-scheduler` | #7 | `docs/testing/proptest-scheduler-evidence.md` |
| H-CLAIMS | `H-CLAIMS` | `slicer-scheduler` | #7 | `docs/testing/proptest-scheduler-evidence.md` |
| H-REGISTRY | `H-REGISTRY` | `slicer-scheduler` | #7 | `docs/testing/proptest-scheduler-evidence.md` |
| H-RUNTIME | `H-RUNTIME-DISPATCH`, `H-RUNTIME-SELECTION`, `H-RUNTIME-MODIFIERS`, `H-RUNTIME-PARTITION` | `slicer-runtime` | #8 | `docs/testing/proptest-runtime-evidence.md` |
| H-BOOL | `H-BOOL-SET`, `H-BOOL-CLIP`, `H-BOOL-SCAD` (scripted enrichment) | `slicer-core` | #9 | `docs/testing/proptest-geometry-evidence.md` |
| H-OFFSET | `H-OFFSET-MITER`, `H-OFFSET-SMOOTH` | `slicer-core` | #9 | `docs/testing/proptest-geometry-evidence.md` |
| H-MESH | `H-MESH-SLICE`, `H-MESH-QUERY`, `H-MESH-SCAD` (scripted enrichment) | `slicer-core` | #9 | `docs/testing/proptest-geometry-evidence.md` |
| slicer-helpers | `H-HELPERS-SPLIT` | `slicer-helpers` | #9 | `docs/testing/proptest-geometry-evidence.md` |
| H-ARACHNE | `H-ARACHNE-BEADING`, `H-ARACHNE-GRAPH`, `H-ARACHNE-PIPELINE` (+ §11 `M-ARACHNE-*`) | `slicer-core` | #10 | `docs/testing/proptest-arachne-evidence.md` |
| H-REGIONMAP | `H-REGIONMAP` (distinct from `H-CHAINS`/`H-REGISTRY`) | `slicer-core` | #11 | `docs/testing/proptest-prepass-evidence.md` |
| H-PAINT | `H-PAINT-COMPOSE`, `H-PAINT-DRIVER`, `H-PAINT-OVERHANG` | `slicer-core` | #11 | `docs/testing/proptest-prepass-evidence.md` |
| H-SUPPORT (host) | `H-SUPPORT-SCHEDULE`, `H-SUPPORT-CONTACT`, `H-SUPPORT-ROLES` (`slicer-core`); `H-SUPPORT-ANALYSIS` (`slicer-runtime`); `H-SUPPORT-TERRITORY` (`slicer-wasm-host`) | as listed | #11 (host analysis/territory; distinct from #14 planners/renderers) | `docs/testing/proptest-prepass-evidence.md` |
| H-LIGHTNING | `H-LIGHTNING-SEEDS`, `H-LIGHTNING-TREES` | `lightning-infill` (native generator) | #11 | `docs/testing/proptest-prepass-evidence.md` |
| H-IO | `H-IO-ID`, `H-IO-EXTENT`, `H-IO-PLACEMENT` | `slicer-model-io` | #16 | `docs/testing/proptest-io-gcode-evidence.md` |
| H-GCODE | `H-GCODE-FEEDRATE`, `H-GCODE-EMIT`, `H-GCODE-SERIALIZE` (`slicer-gcode`); `H-GCODE-TEMPLATE` (`machine-gcode-emit`) | as listed | #16 | `docs/testing/proptest-io-gcode-evidence.md` |
| SDK direct domains | `D-SCALARS`, `D-GRAPHS`, `D-IR`, `D-GEOMETRY`, `D-MESH` (helper validation; do not discharge host/module charters) | `slicer-sdk` | #2 | `docs/testing/proptest-domain-evidence.md` |
| SDK scripted domains | `D-SCAD-2D`, `D-SCAD-3D` | `slicer-sdk` | #4 | `docs/testing/proptest-scripted-evidence.md` |
| SCAD adapter prerequisite | none (product/adapter contract tests) | `slicer-scad` | #3 | `docs/scad/windows-feasibility.md` |
| SCAD ingestion | none (feature-off/on product acceptance) | `slicer-model-io`, `pnp-cli` | #5 | `docs/scad/ingestion-evidence.md` |

### 3.2 Structural-coverage dispositions (closure disposition, owned by this ledger)

Source plan §10 ("Review the schema tables, macro expansion tests, integrated registry, and CLI
locator…") and AC-1 require a reasoned disposition. No generative property is added for a fixed
enumeration.

| Surface | Owning test surface | Disposition |
|---|---|---|
| Schema tables | `docs/02_ir_schemas.md` IR schema tables; owned by schema/contract tests and compiler exhaustiveness | Fixed enumerations; a random sampler adds no signal. No property. |
| Macro expansion tests | `crates/slicer-macros/tests/` — `module_test_tdd.rs`, `slicer_module_tdd.rs`, `smoke.rs`, `binding_surface_tdd.rs`, `all_worlds_glue_tdd.rs`, `postpass_text_glue_tdd.rs`, `variant_chain_boundary_tdd.rs` | Expansion shapes are fixed per input; structural tests own them. `slicer-macros` has no `property_testing` feature. No property. |
| Integrated registry | `integrated_registrations` (`crates/slicer-integrated-modules/src/lib.rs`), consumed by `cli_integrated_registrations` (`crates/pnp-cli/src/main.rs`); unit test `integrated_registrations_are_empty_by_default` (`crates/slicer-integrated-modules/src/lib.rs`) | Fixed registration list; structural/unit tests own it. No property. |
| CLI locator | `crates/pnp-cli-locator` — the `pnp_cli` locator (`workspace_root`, `newest_source_mtime`, `staleness_reason`, `pnp_cli_bin`), governed by ADR-0054 (std-only, dev-dependency only, host-side only) and consumed by the freshness/staleness path | Deterministic path resolution and mtime/staleness comparison over the real checkout; a generative property would need a synthetic filesystem and add no signal over the existing deterministic checks. No property. |

### 3.3 Source-plan §11 module rows

| Module | Registry IDs | Predecessor | Evidence document |
|---|---|---|---|
| `classic-perimeters` | `M-CLASSIC-WALL-STRUCTURE`, `M-CLASSIC-BAND-SELECTION` | #12 | `docs/testing/proptest-walls-evidence.md` |
| `arachne-perimeters` | `M-ARACHNE-CLASSIFY`, `M-ARACHNE-BUILD`, `M-ARACHNE-SEQUENCE` | #10 | `docs/testing/proptest-arachne-evidence.md` |
| `fuzzy-skin` | `M-FUZZY-REPEATABILITY`, `M-FUZZY-SPAN-PRESERVATION`, `M-FUZZY-DISPLACEMENT` | #12 | `docs/testing/proptest-walls-evidence.md` |
| `rectilinear-infill` | `M-RECTILINEAR-SCAN`, `M-RECTILINEAR-SPACING` | #13 | `docs/testing/proptest-infill-evidence.md` |
| `gyroid-infill` | `M-GYROID-PHASE`, `M-GYROID-CLIP` | #13 | `docs/testing/proptest-infill-evidence.md` |
| `lightning-infill` | `H-LIGHTNING-SEEDS`, `H-LIGHTNING-TREES` (shared with §10; no duplicate `M-` record) | #11 | `docs/testing/proptest-prepass-evidence.md` |
| `infill-linker` | `M-LINKER-GRAPH`, `M-LINKER-CONNECT`, `M-LINKER-CHAIN` | #13 | `docs/testing/proptest-infill-evidence.md` |
| `wave-overhangs` | `M-WAVE-TRIM` | #13 | `docs/testing/proptest-infill-evidence.md` |
| `top-surface-ironing` | `M-IRONING-ZIGZAG` | #13 | `docs/testing/proptest-infill-evidence.md` |
| `traditional-support-planner` | `M-SUPPORT-RASTER`, `M-SUPPORT-HOLES`, `M-SUPPORT-GRID`, `M-SUPPORT-TERRITORY` | #14 | `docs/testing/proptest-support-modules-evidence.md` |
| `tree-support-planner` | `M-TREE-SMOOTH`, `M-TREE-ROLES` | #14 | `docs/testing/proptest-support-modules-evidence.md` |
| `traditional-support` | `M-TRADITIONAL-SCAN` | #14 | `docs/testing/proptest-support-modules-evidence.md` |
| `tree-support` | `M-TREE-SCAN`, `M-TREE-RENDER` | #14 | `docs/testing/proptest-support-modules-evidence.md` |
| `support-surface-ironing` | `M-SUPPORT-IRONING` | #14 | `docs/testing/proptest-support-modules-evidence.md` |
| `layer-planner-default` | `M-LAYERS-SEQ`, `M-LAYERS-MERGE` | #15 | `docs/testing/proptest-layers-seams-evidence.md` |
| `seam-planner-default` | `M-SEAM-PLANNER-PICK`, `M-SEAM-PLANNER-PAINT`, `M-SEAM-PLANNER-SPLINE` | #15 | `docs/testing/proptest-layers-seams-evidence.md` |
| `seam-placer` | `M-SEAM-PLACER-SELECT`, `M-SEAM-PLACER-PROJECT`, `M-SEAM-PLACER-ROTATE` | #15 | `docs/testing/proptest-layers-seams-evidence.md` |
| `path-optimization-default` | `M-PATH-NEAREST`, `M-PATH-LOCKS`, `M-PATH-GROUPS` | #15 | `docs/testing/proptest-layers-seams-evidence.md` |
| `part-cooling` | `H-COOLING` | #16 | `docs/testing/proptest-io-gcode-evidence.md` |
| `overhang-classifier-default` | `H-OVERHANG-SPEED` | #16 | `docs/testing/proptest-io-gcode-evidence.md` |
| `machine-gcode-emit` | `H-GCODE-TEMPLATE` | #16 | `docs/testing/proptest-io-gcode-evidence.md` |
| `skirt-brim` | `H-SKIRT-BRIM` | #16 | `docs/testing/proptest-io-gcode-evidence.md` |
| `wipe-tower` | `H-WIPE-TOWER` | #16 | `docs/testing/proptest-io-gcode-evidence.md` |

All 23 module packages above own at least one registry record. Nonmember experiments and the
external community-module policy (`docs/14_submodule_programming_languages.md`) are not shipped
modules and are not counted.

### 3.4 Traceable ID/shape dispositions

| ID | Change | Disposition |
|---|---|---|
| `H-PAINT-OVERHANG` | Packet #11's record table (`docs/spec_packets/proptest-openrscad_11_pbt-prepass/packet.spec.md`) and its implementation plan name `proptest_overhang_adjacent_layer_bands`; the registry (`test-properties.toml`) and `crates/slicer-core/tests/property_prepass.rs` use `proptest_overhang_adjacent_layer_areas`. | **Accepted rename.** The ticket-142 rewrite removed the producer-side four-band partition, so the test name now reflects the annotation's unsupported-area / predecessor-identity assertion. Stable ID and charter are unchanged. Packet #11's `## Deviations` section does not mention the rename; this row is its traceable record. |
| CI ID roster | At `HEAD`, the `properties-windows` step named 54 IDs and `properties-scad-windows` named 4 (counted from `git show HEAD:.github/workflows/ci.yml`), so 35 registered records were never asserted and the lists duplicated the registry. | **Implemented and verified; the PR and dispatch runs record the pass.** The closure workflow removes both lists. The runner's registry census (`CensusScope`, `xtask/src/properties/executor.rs`: all eligible owners/targets from Cargo metadata, bidirectional discovery vs registry, reconciliation against the approved charter set in `docs/testing/properties.md`) plus per-record controller completion evidence now owns the assertion. |

## 4. Registry inventory

Generated mechanically from `test-properties.toml` (`target/scratch/gen_registry_table.py`); the
runner validates the registry itself on every `--properties` invocation. Re-derived counts:

| Count | Value | Method |
|---|---:|---|
| Records | 93 | `[[property]]` count; 93 unique `id` |
| Core-lane records (`requires_scad = false`) | 89 | `tomllib` load |
| SCAD records (`requires_scad = true`) | 4 — `D-SCAD-2D`, `D-SCAD-3D`, `H-BOOL-SCAD`, `H-MESH-SCAD` | `tomllib` load |
| Owner packages | 32 (23 modules + 9 host crates: `slicer-core`, `slicer-gcode`, `slicer-helpers`, `slicer-ir`, `slicer-model-io`, `slicer-runtime`, `slicer-scheduler`, `slicer-sdk`, `slicer-wasm-host`) | distinct `package` |
| Distinct `(package, target_kind, target_name)` | 40 across all records; 38 with at least one core record (the two SCAD-only targets are `slicer-core/test/property_geometry_scad` and `slicer-sdk/test/property_scripted`) | distinct triples |
| `requires_host_algos = true` | 24 | `tomllib` load |
| `generator_version` | 92 records at `1`; `M-SEAM-PLANNER-PICK` at `2` | `tomllib` load |

**Harness-fixture exception.** `crates/slicer-sdk/tests/property_harness.rs` contains two
`proptest_`-named tests, `proptest_fixture_context_child` and `proptest_fixture_all_rejecting`,
each preceded by the annotation `// property-census: harness-fixture` (the `CENSUS_EXEMPTION`
constant in `xtask/src/properties/executor.rs`). They are xtask boundary-test fixtures launched with
a subprocess-local context, are still discovered by the census, and are deliberately **not**
property records.

| ID | Package | Target | Full test name | Features | SCAD | Routine | Extended | Production symbol |
|---|---|---|---|---|---|---|---|---|
| `M-ARACHNE-BUILD` | `arachne-perimeters` | `lib`:`arachne_perimeters` | `property_tests::proptest_module_arachne_build_walls` | `arachne-perimeters/property_testing` | no | 64 cases, in_process, max_junctions = 32, max_abs_mm_units = 20000, max_spacing_units = 2000, max_height_units = 500 | 1024 cases, in_process, max_junctions = 64, max_abs_mm_units = 40000, max_spacing_units = 8000, max_height_units = 2000 | `arachne_perimeters::ArachnePerimeters::build_walls` |
| `M-ARACHNE-CLASSIFY` | `arachne-perimeters` | `lib`:`arachne_perimeters` | `property_tests::proptest_module_arachne_classify` | `arachne-perimeters/property_testing` | no | 64 cases, in_process, max_inset_idx = 8 | 1024 cases, in_process, max_inset_idx = 32 | `arachne_perimeters::classify_line` |
| `M-ARACHNE-SEQUENCE` | `arachne-perimeters` | `lib`:`arachne_perimeters` | `property_tests::proptest_module_arachne_sequence` | `arachne-perimeters/property_testing` | no | 64 cases, in_process, max_walls = 6 | 1024 cases, in_process, max_walls = 12 | `arachne_perimeters::commit_wall_sequence` |
| `M-CLASSIC-BAND-SELECTION` | `classic-perimeters` | `lib`:`classic_perimeters` | `property_tests::proptest_classic_band_selection` | `classic-perimeters/property_testing` | no | 64 cases, in_process, max_abs_units = 2048, max_vertices = 16, max_holes = 2, max_components = 3, max_shrink_candidates = 32, max_wall_count = 4, max_width_units = 8, max_layer_height_units = 4 | 256 cases, in_process, max_abs_units = 2048, max_vertices = 16, max_holes = 2, max_components = 3, max_shrink_candidates = 32, max_wall_count = 4, max_width_units = 8, max_layer_height_units = 4 | `classic_perimeters::ClassicPerimeters::emit_walls` |
| `M-CLASSIC-WALL-STRUCTURE` | `classic-perimeters` | `lib`:`classic_perimeters` | `property_tests::proptest_classic_wall_structure` | `classic-perimeters/property_testing` | no | 64 cases, in_process, max_abs_units = 2048, max_vertices = 16, max_holes = 2, max_components = 3, max_shrink_candidates = 32, max_wall_count = 4, max_width_units = 8, max_layer_height_units = 4 | 256 cases, in_process, max_abs_units = 2048, max_vertices = 16, max_holes = 2, max_components = 3, max_shrink_candidates = 32, max_wall_count = 4, max_width_units = 8, max_layer_height_units = 4 | `classic_perimeters::ClassicPerimeters::emit_walls` |
| `M-FUZZY-DISPLACEMENT` | `fuzzy-skin` | `lib`:`fuzzy_skin` | `property_tests::proptest_fuzzy_displacement` | `fuzzy-skin/property_testing` | no | 64 cases, in_process, max_abs_mm_units = 20000, max_thickness_units = 3000, max_point_distance_units = 8000, max_points = 8 | 256 cases, in_process, max_abs_mm_units = 20000, max_thickness_units = 3000, max_point_distance_units = 8000, max_points = 8 | `fuzzy_skin::apply_fuzzy_skin` |
| `M-FUZZY-REPEATABILITY` | `fuzzy-skin` | `lib`:`fuzzy_skin` | `property_tests::proptest_fuzzy_repeatability` | `fuzzy-skin/property_testing` | no | 64 cases, in_process, max_abs_mm_units = 20000, max_thickness_units = 3000, max_point_distance_units = 8000, max_points = 8 | 256 cases, in_process, max_abs_mm_units = 20000, max_thickness_units = 3000, max_point_distance_units = 8000, max_points = 8 | `fuzzy_skin::apply_fuzzy_skin` |
| `M-FUZZY-SPAN-PRESERVATION` | `fuzzy-skin` | `lib`:`fuzzy_skin` | `property_tests::proptest_fuzzy_span_preservation` | `fuzzy-skin/property_testing` | no | 64 cases, in_process, max_abs_mm_units = 20000, max_thickness_units = 3000, max_point_distance_units = 8000, max_points = 8 | 256 cases, in_process, max_abs_mm_units = 20000, max_thickness_units = 3000, max_point_distance_units = 8000, max_points = 8 | `fuzzy_skin::apply_fuzzy_skin` |
| `M-GYROID-CLIP` | `gyroid-infill` | `lib`:`gyroid_infill` | `property_tests::proptest_gyroid_raw_wave_clipping` | `gyroid-infill/property_testing` | no | 64 cases, in_process, max_abs_units = 524288, max_vertices = 32, max_holes = 4, max_components = 2, max_shrink_candidates = 32, max_width_units = 6000, min_density_ppm = 100000, max_phase_milliradians = 6284, max_wave_points = 65536 | 256 cases, in_process, max_abs_units = 524288, max_vertices = 32, max_holes = 4, max_components = 2, max_shrink_candidates = 32, max_width_units = 6000, min_density_ppm = 100000, max_phase_milliradians = 6284, max_wave_points = 65536 | `gyroid_infill::GyroidInfill::fill_expolygon` |
| `M-GYROID-PHASE` | `gyroid-infill` | `lib`:`gyroid_infill` | `property_tests::proptest_gyroid_phase_and_grid` | `gyroid-infill/property_testing` | no | 64 cases, in_process, max_phase_milliradians = 6284, max_grid_units = 512 | 256 cases, in_process, max_phase_milliradians = 6284, max_grid_units = 512 | `gyroid_infill::gyroid_f` |
| `M-LINKER-CHAIN` | `infill-linker` | `lib`:`infill_linker` | `property_tests::proptest_linker_chain_and_origins` | `infill-linker/property_testing` | no | 64 cases, in_process, max_abs_units = 400000, max_paths = 4, max_points = 4, max_regions = 4, max_anchor_units = 100000 | 256 cases, in_process, max_abs_units = 400000, max_paths = 4, max_points = 4, max_regions = 4, max_anchor_units = 100000 | `infill_linker::connect::chain_or_connect_infill` |
| `M-LINKER-CONNECT` | `infill-linker` | `lib`:`infill_linker` | `property_tests::proptest_linker_connection_compatibility` | `infill-linker/property_testing` | no | 64 cases, in_process, max_abs_units = 200000, max_paths = 4, max_points = 5, max_anchor_units = 100000 | 256 cases, in_process, max_abs_units = 200000, max_paths = 4, max_points = 5, max_anchor_units = 100000 | `infill_linker::connect::connect_infill` |
| `M-LINKER-GRAPH` | `infill-linker` | `lib`:`infill_linker` | `property_tests::proptest_linker_ring_walks` | `infill-linker/property_testing` | no | 64 cases, in_process, max_abs_units = 200000, max_vertices = 16, max_holes = 2, max_components = 2, max_shrink_candidates = 4096 | 256 cases, in_process, max_abs_units = 200000, max_vertices = 16, max_holes = 2, max_components = 2, max_shrink_candidates = 4096 | `infill_linker::graph::contour_connector` |
| `M-LAYERS-MERGE` | `layer-planner-default` | `lib`:`layer_planner_default` | `property_tests::proptest_layer_merge` | `layer-planner-default/property_testing` | no | 64 cases, in_process, max_objects = 4, max_layer_steps = 16, max_raft_layers = 4, max_z_quanta = 8192 | 256 cases, in_process, max_objects = 4, max_layer_steps = 16, max_raft_layers = 4, max_z_quanta = 8192 | `layer_planner_default::merge_layer_sequences` |
| `M-LAYERS-SEQ` | `layer-planner-default` | `lib`:`layer_planner_default` | `property_tests::proptest_layer_sequence` | `layer-planner-default/property_testing` | no | 64 cases, in_process, max_layer_steps = 16, max_raft_layers = 4, max_z_quanta = 8192 | 256 cases, in_process, max_layer_steps = 16, max_raft_layers = 4, max_z_quanta = 8192 | `layer_planner_default::generate_object_layers` |
| `H-LIGHTNING-SEEDS` | `lightning-infill` | `lib`:`lightning_infill` | `kernel::property_tests::proptest_lightning_seed_regions` | `lightning-infill/property_testing` | no | 64 cases, in_process, max_layers = 3, max_abs_units = 200000, max_radius_units = 20000, max_cells = 32 | 256 cases, in_process, max_layers = 3, max_abs_units = 200000, max_radius_units = 20000, max_cells = 64 | `lightning_infill::kernel::generator::generate_initial_internal_overhangs` |
| `H-LIGHTNING-TREES` | `lightning-infill` | `lib`:`lightning_infill` | `kernel::property_tests::proptest_lightning_tree_structure` | `lightning-infill/property_testing` | no | 64 cases, in_process, max_layers = 3, max_objects = 1, max_regions = 1, max_abs_units = 200000, max_tree_nodes = 20000 | 256 cases, in_process, max_layers = 3, max_objects = 1, max_regions = 1, max_abs_units = 200000, max_tree_nodes = 20000 | `lightning_infill::kernel::generator::Generator::generate_trees` |
| `H-GCODE-TEMPLATE` | `machine-gcode-emit` | `lib`:`machine_gcode_emit` | `property_tests::proptest_placeholder_substitution` | `machine-gcode-emit/property_testing` | no | 64 cases, in_process, max_template_bytes = 64, max_placeholders = 6 | 256 cases, in_process, max_template_bytes = 64, max_placeholders = 6 | `machine_gcode_emit::substitute_placeholders` |
| `H-OVERHANG-SPEED` | `overhang-classifier-default` | `test`:`property_overhang_speed` | `proptest_overhang_speed_sections` | `overhang-classifier-default/property_testing` | no | 64 cases, in_process, max_path_width_mm = 4, max_speed_mm_s = 200, max_interpolation_samples = 8 | 256 cases, in_process, max_path_width_mm = 4, max_speed_mm_s = 200, max_interpolation_samples = 8 | `overhang_classifier_default::calculate_speed` |
| `H-COOLING` | `part-cooling` | `lib`:`part_cooling` | `property_tests::proptest_cooling_decision_table` | `part-cooling/property_testing` | no | 64 cases, in_process, max_layer_index = 8, max_close_layers = 4 | 256 cases, in_process, max_layer_index = 8, max_close_layers = 4 | `part_cooling::PartCooling::cooling_decision_for_event` |
| `M-PATH-GROUPS` | `path-optimization-default` | `lib`:`path_optimization_default` | `property_tests::proptest_path_groups` | `path-optimization-default/property_testing` | no | 64 cases, in_process, max_entities = 16, max_coord_quanta = 8192 | 256 cases, in_process, max_entities = 16, max_coord_quanta = 8192 | `path_optimization_default::PathOptimizationDefault::group_then_nearest_neighbor` |
| `M-PATH-LOCKS` | `path-optimization-default` | `lib`:`path_optimization_default` | `property_tests::proptest_path_locks` | `path-optimization-default/property_testing` | no | 64 cases, in_process, max_entities = 16, max_coord_quanta = 8192 | 256 cases, in_process, max_entities = 16, max_coord_quanta = 8192 | `path_optimization_default::coalesce_locked_candidates` |
| `M-PATH-NEAREST` | `path-optimization-default` | `lib`:`path_optimization_default` | `property_tests::proptest_path_nearest` | `path-optimization-default/property_testing` | no | 64 cases, in_process, max_candidates = 16, max_coord_quanta = 8192 | 256 cases, in_process, max_candidates = 16, max_coord_quanta = 8192 | `path_optimization_default::nearest_neighbor_permutation` |
| `M-RECTILINEAR-SCAN` | `rectilinear-infill` | `lib`:`rectilinear_infill` | `property_tests::proptest_rectilinear_scan_intervals` | `rectilinear-infill/property_testing` | no | 64 cases, in_process, max_abs_units = 200000, max_vertices = 8, max_holes = 1, max_components = 1, max_shrink_candidates = 16, max_spacing_units = 20000, max_rows = 32 | 256 cases, in_process, max_abs_units = 200000, max_vertices = 8, max_holes = 1, max_components = 1, max_shrink_candidates = 16, max_spacing_units = 20000, max_rows = 32 | `rectilinear_infill::scan_expolygon` |
| `M-RECTILINEAR-SPACING` | `rectilinear-infill` | `lib`:`rectilinear_infill` | `property_tests::proptest_rectilinear_solid_spacing` | `rectilinear-infill/property_testing` | no | 64 cases, in_process, max_width_units = 200000, max_spacing_units = 20000 | 256 cases, in_process, max_width_units = 200000, max_spacing_units = 20000 | `rectilinear_infill::adjust_solid_spacing` |
| `M-SEAM-PLACER-PROJECT` | `seam-placer` | `lib`:`seam_placer` | `property_tests::proptest_placer_project` | `seam-placer/property_testing` | no | 64 cases, in_process, max_wall_points = 16, max_walls = 4, max_coord_quanta = 8192 | 256 cases, in_process, max_wall_points = 16, max_walls = 4, max_coord_quanta = 8192 | `seam_placer::project_onto_wall_segment` |
| `M-SEAM-PLACER-ROTATE` | `seam-placer` | `lib`:`seam_placer` | `property_tests::proptest_placer_rotate` | `seam-placer/property_testing` | no | 64 cases, in_process, max_wall_points = 16, max_coord_quanta = 8192 | 256 cases, in_process, max_wall_points = 16, max_coord_quanta = 8192 | `seam_placer::rotate_wall_loop` |
| `M-SEAM-PLACER-SELECT` | `seam-placer` | `lib`:`seam_placer` | `property_tests::proptest_placer_select` | `seam-placer/property_testing` | no | 64 cases, in_process, max_candidates = 16, max_coord_quanta = 8192, max_score_quanta = 4096 | 256 cases, in_process, max_candidates = 16, max_coord_quanta = 8192, max_score_quanta = 4096 | `seam_placer::select_seam_candidate` |
| `M-SEAM-PLANNER-PAINT` | `seam-planner-default` | `lib`:`seam_planner_default` | `property_tests::proptest_planner_paint` | `seam-planner-default/property_testing` | no | 64 cases, in_process, max_candidates = 16, max_coord_quanta = 8192 | 256 cases, in_process, max_candidates = 16, max_coord_quanta = 8192 | `seam_planner_default::region_candidates` |
| `M-SEAM-PLANNER-PICK` | `seam-planner-default` | `lib`:`seam_planner_default` | `property_tests::proptest_planner_pick` | `seam-planner-default/property_testing` | no | 64 cases, in_process, max_candidates = 16, max_coord_quanta = 8192, max_score_quanta = 4096 | 256 cases, in_process, max_candidates = 16, max_coord_quanta = 8192, max_score_quanta = 4096 | `seam_planner_default::comparator::pick_seam_point` |
| `M-SEAM-PLANNER-SPLINE` | `seam-planner-default` | `lib`:`seam_planner_default` | `property_tests::proptest_planner_spline` | `seam-planner-default/property_testing` | no | 64 cases, in_process, max_observations = 16, max_segments = 8, max_coord_quanta = 8192, max_weight_quanta = 4096 | 256 cases, in_process, max_observations = 16, max_segments = 8, max_coord_quanta = 8192, max_weight_quanta = 4096 | `seam_planner_default::align::fit_cubic_bspline` |
| `H-SKIRT-BRIM` | `skirt-brim` | `lib`:`skirt_brim` | `property_tests::proptest_skirt_brim_enclosure` | `skirt-brim/property_testing` | no | 64 cases, in_process, max_abs_mm = 64, max_skirt_distance_mm = 8, max_loops = 3, max_line_width_mm = 2 | 256 cases, in_process, max_abs_mm = 64, max_skirt_distance_mm = 8, max_loops = 3, max_line_width_mm = 2 | `skirt_brim::SkirtBrim::generate` |
| `H-ARACHNE-BEADING` | `slicer-core` | `test`:`property_arachne` | `proptest_arachne_beading_relations` | `slicer-core/host-algos`, `slicer-core/property_testing` | no | 64 cases, in_process, max_beads = 8, max_width_units = 20000, max_excess_units = 8000 | 1024 cases, in_process, max_beads = 16, max_width_units = 40000, max_excess_units = 16000 | `slicer_core::beading::distributed::DistributedBeadingStrategy::compute` |
| `H-ARACHNE-GRAPH` | `slicer-core` | `test`:`property_arachne` | `proptest_arachne_graph_topology` | `slicer-core/host-algos`, `slicer-core/property_testing` | no | 64 cases, in_process, max_abs_units = 400000, max_vertices = 24, max_holes = 2, max_components = 3, max_shrink_candidates = 64 | 1024 cases, in_process, max_abs_units = 1000000, max_vertices = 32, max_holes = 3, max_components = 4, max_shrink_candidates = 256 | `slicer_core::skeletal_trapezoidation::SkeletalTrapezoidationGraph::from_polygons` |
| `H-ARACHNE-PIPELINE` | `slicer-core` | `test`:`property_arachne` | `proptest_arachne_pipeline_structure` | `slicer-core/host-algos`, `slicer-core/property_testing` | no | 64 cases, in_process, max_abs_units = 400000, max_vertices = 24, max_holes = 2, max_components = 3, max_shrink_candidates = 64, max_beads = 10 | 1024 cases, in_process, max_abs_units = 1000000, max_vertices = 32, max_holes = 3, max_components = 4, max_shrink_candidates = 256, max_beads = 16 | `slicer_core::arachne::pipeline::run_arachne_pipeline` |
| `H-BOOL-CLIP` | `slicer-core` | `test`:`property_geometry` | `proptest_clip_polylines_full_segments` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_abs_units = 60000, max_vertices = 32, max_holes = 4, max_components = 3, max_shrink_candidates = 32, max_queries = 64, max_polylines = 12 | 512 cases, in_process, max_abs_units = 120000, max_vertices = 64, max_holes = 8, max_components = 4, max_shrink_candidates = 64, max_queries = 128, max_polylines = 16 | `slicer_core::polygon_ops::clip_polylines` |
| `H-BOOL-SET` | `slicer-core` | `test`:`property_geometry` | `proptest_boolean_set_algebra` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_abs_units = 60000, max_vertices = 32, max_holes = 4, max_components = 3, max_shrink_candidates = 32, max_queries = 64 | 512 cases, in_process, max_abs_units = 120000, max_vertices = 64, max_holes = 8, max_components = 4, max_shrink_candidates = 64, max_queries = 128 | `slicer_core::polygon_ops::union` |
| `H-MESH-QUERY` | `slicer-core` | `test`:`property_geometry` | `proptest_mesh_transform_and_aabb` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_extent_mm = 8, max_translation_mm = 8, max_queries = 8 | 512 cases, in_process, max_extent_mm = 16, max_translation_mm = 16, max_queries = 16 | `slicer_core::transform_point3` |
| `H-MESH-SLICE` | `slicer-core` | `test`:`property_geometry` | `proptest_slice_analytical_solids` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_extent_mm = 8, max_translation_mm = 8, max_planes = 6 | 512 cases, in_process, max_extent_mm = 16, max_translation_mm = 16, max_planes = 10 | `slicer_core::triangle_mesh_slicer::slice_mesh_ex` |
| `H-OFFSET-MITER` | `slicer-core` | `test`:`property_geometry` | `proptest_offset_rectangular_clearance` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_abs_units = 120000, max_side_mm = 8, max_offset_mm = 3 | 512 cases, in_process, max_abs_units = 200000, max_side_mm = 16, max_offset_mm = 5 | `slicer_core::polygon_ops::offset2_ex` |
| `H-OFFSET-SMOOTH` | `slicer-core` | `test`:`property_geometry` | `proptest_smooth_outward_safe_families` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_abs_units = 120000, max_side_units = 40000, max_clip_units = 5000 | 512 cases, in_process, max_abs_units = 200000, max_side_units = 80000, max_clip_units = 8000 | `slicer_core::smooth_outward::smooth_outward_polygon` |
| `H-BOOL-SCAD` | `slicer-core` | `test`:`property_geometry_scad` | `proptest_scripted_boolean_membership` | `slicer-core/property_testing_scad`, `slicer-core/host-algos` | yes | 24 cases, fork, timeout 30000 ms, max_abs_units = 20000, max_vertices = 64, max_holes = 4, max_components = 4, max_shrink_candidates = 32, max_recipe_nodes = 24, max_recipe_depth = 6, max_facets = 16, max_source_bytes = 16384, max_evaluations = 48, max_scad_vertices = 200000, max_scad_triangles = 400000, evaluation_steps = 5000000, worker_stack_bytes = 268435456, test_process_timeout_ms = 120000, max_queries = 32, max_planes = 4 | 96 cases, fork, timeout 60000 ms, max_abs_units = 40000, max_vertices = 64, max_holes = 8, max_components = 8, max_shrink_candidates = 64, max_recipe_nodes = 32, max_recipe_depth = 8, max_facets = 24, max_source_bytes = 32768, max_evaluations = 128, max_scad_vertices = 400000, max_scad_triangles = 800000, evaluation_steps = 20000000, worker_stack_bytes = 268435456, test_process_timeout_ms = 600000, max_queries = 32, max_planes = 6 | `slicer_core::polygon_ops::union` |
| `H-MESH-SCAD` | `slicer-core` | `test`:`property_geometry_scad` | `proptest_scripted_mesh_cross_sections` | `slicer-core/property_testing_scad`, `slicer-core/host-algos` | yes | 24 cases, fork, timeout 30000 ms, max_abs_units = 20000, max_vertices = 64, max_holes = 4, max_components = 4, max_shrink_candidates = 32, max_recipe_nodes = 24, max_recipe_depth = 6, max_facets = 16, max_source_bytes = 16384, max_evaluations = 48, max_scad_vertices = 200000, max_scad_triangles = 400000, evaluation_steps = 5000000, worker_stack_bytes = 268435456, test_process_timeout_ms = 120000, max_queries = 32, max_planes = 4 | 96 cases, fork, timeout 60000 ms, max_abs_units = 40000, max_vertices = 64, max_holes = 8, max_components = 8, max_shrink_candidates = 64, max_recipe_nodes = 32, max_recipe_depth = 8, max_facets = 24, max_source_bytes = 32768, max_evaluations = 128, max_scad_vertices = 400000, max_scad_triangles = 800000, evaluation_steps = 20000000, worker_stack_bytes = 268435456, test_process_timeout_ms = 600000, max_queries = 32, max_planes = 6 | `slicer_core::triangle_mesh_slicer::slice_mesh_ex` |
| `H-PAINT-COMPOSE` | `slicer-core` | `test`:`property_prepass` | `proptest_paint_composition_cells` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_layers = 2, max_abs_units = 64, max_semantics = 2, max_cells = 16 | 256 cases, in_process, max_layers = 2, max_abs_units = 128, max_semantics = 3, max_cells = 16 | `slicer_core::algos::paint_segmentation::compose_variants::compose_variants` |
| `H-PAINT-DRIVER` | `slicer-core` | `test`:`property_prepass` | `proptest_paint_driver_source_isolation` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_layers = 1, max_objects = 2, max_regions = 1, max_abs_units = 200000, max_facets = 16 | 256 cases, in_process, max_layers = 1, max_objects = 3, max_regions = 1, max_abs_units = 200000, max_facets = 16 | `slicer_core::algos::paint_segmentation::execute_paint_segmentation` |
| `H-PAINT-OVERHANG` | `slicer-core` | `test`:`property_prepass` | `proptest_overhang_adjacent_layer_areas` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_layers = 2, max_abs_units = 200000, max_band_width_units = 12000, max_cells = 32 | 256 cases, in_process, max_layers = 2, max_abs_units = 200000, max_band_width_units = 24000, max_cells = 64 | `slicer_core::algos::overhang_annotation::annotate_overhangs` |
| `H-REGIONMAP` | `slicer-core` | `test`:`property_prepass` | `proptest_region_mapping_expansion` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_layers = 2, max_objects = 2, max_regions = 2, max_semantics = 3, max_values_per_semantic = 3, max_chains = 64 | 256 cases, in_process, max_layers = 2, max_objects = 3, max_regions = 2, max_semantics = 4, max_values_per_semantic = 3, max_chains = 256 | `slicer_core::algos::region_mapping::execute_region_mapping_with_cap` |
| `H-SUPPORT-CONTACT` | `slicer-core` | `test`:`property_prepass` | `proptest_support_contact_rectangles` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_abs_units = 200000, max_cells = 4 | 256 cases, in_process, max_abs_units = 200000, max_cells = 4 | `slicer_core::algos::overhang_annotation::detect_support_contacts` |
| `H-SUPPORT-ROLES` | `slicer-core` | `test`:`property_prepass` | `proptest_support_role_partition` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_abs_units = 200000, max_cells = 8 | 256 cases, in_process, max_abs_units = 200000, max_cells = 16 | `slicer_core::support_regularize::regularize_entry_roles` |
| `H-SUPPORT-SCHEDULE` | `slicer-core` | `test`:`property_prepass` | `proptest_support_schedule_and_sources` | `slicer-core/property_testing`, `slicer-core/host-algos` | no | 64 cases, in_process, max_layers = 4, max_objects = 2, max_regions = 1, max_abs_units = 200000 | 256 cases, in_process, max_layers = 4, max_objects = 3, max_regions = 1, max_abs_units = 200000 | `slicer_core::algos::support_geometry::execute_support_geometry` |
| `H-GCODE-EMIT` | `slicer-gcode` | `test`:`property_gcode` | `proptest_emission_state_sequence` | `slicer-gcode/property_testing` | no | 64 cases, in_process, max_layers = 3, max_entities_per_layer = 3, max_points_per_entity = 4, max_abs_mm = 64 | 256 cases, in_process, max_layers = 3, max_entities_per_layer = 3, max_points_per_entity = 4, max_abs_mm = 64 | `slicer_gcode::emit::DefaultGCodeEmitter::emit_gcode` |
| `H-GCODE-FEEDRATE` | `slicer-gcode` | `test`:`property_gcode` | `proptest_feedrate_resolution` | `slicer-gcode/property_testing` | no | 64 cases, in_process, max_base_speed_mm_s = 1000, max_factor_milli = 10000 | 256 cases, in_process, max_base_speed_mm_s = 1000, max_factor_milli = 10000 | `slicer_gcode::emit::DefaultGCodeEmitter::resolve_feedrate` |
| `H-GCODE-SERIALIZE` | `slicer-gcode` | `test`:`property_gcode` | `proptest_serializer_extrusion_state` | `slicer-gcode/property_testing` | no | 64 cases, in_process, max_commands = 12, max_abs_mm = 64, max_abs_e_mm = 100 | 256 cases, in_process, max_commands = 12, max_abs_mm = 64, max_abs_e_mm = 100 | `slicer_gcode::serialize::DefaultGCodeSerializer::serialize_gcode` |
| `H-HELPERS-SPLIT` | `slicer-helpers` | `test`:`property_helpers` | `proptest_helpers_split_components` | `slicer-helpers/property_testing` | no | 64 cases, in_process, max_components = 4, max_faces = 12, max_abs_mm = 8 | 512 cases, in_process, max_components = 8, max_faces = 24, max_abs_mm = 16 | `slicer_helpers::split_connected_components` |
| `H-CHAINS` | `slicer-ir` | `test`:`property_ir` | `proptest_canonical_chain_product` | `slicer-ir/property_testing` | no | 64 cases, in_process, max_semantics = 3, max_values_per_semantic = 3, max_chains = 64, max_text_bytes = 16 | 1024 cases, in_process, max_semantics = 4, max_values_per_semantic = 4, max_chains = 625, max_text_bytes = 32 | `slicer_ir::region_split_registry::enumerate_canonical_chains` |
| `H-CONFIG-INTERN` | `slicer-ir` | `test`:`property_ir` | `proptest_config_equality_hash_and_interning` | `slicer-ir/property_testing` | no | 64 cases, in_process, max_configs = 4, max_list_len = 4, max_list_depth = 2, max_text_bytes = 16, max_abs_config_value = 4096 | 1024 cases, in_process, max_configs = 8, max_list_len = 8, max_list_depth = 3, max_text_bytes = 32, max_abs_config_value = 16384 | `slicer_ir::RegionMapIR::intern_config` |
| `H-COORD` | `slicer-ir` | `test`:`property_ir` | `proptest_coord_conversion_envelopes` | `slicer-ir/property_testing` | no | 64 cases, in_process, max_abs_mm = 25, max_abs_units = 250000 | 1024 cases, in_process, max_abs_mm = 100, max_abs_units = 1000000 | `slicer_ir::mm_to_units` |
| `H-IDS` | `slicer-ir` | `test`:`property_ir` | `proptest_entity_and_modifier_ids` | `slicer-ir/property_testing` | no | 64 cases, in_process, max_allocations = 256, max_id_bytes = 64, max_footprint_points = 8 | 1024 cases, in_process, max_allocations = 4096, max_id_bytes = 256, max_footprint_points = 32 | `slicer_ir::LayerEntityIdGen::next` |
| `H-IR-ANCHORS` | `slicer-ir` | `test`:`property_ir` | `proptest_ir_travel_anchors_membership` | `slicer-ir/property_testing` | no | 64 cases, in_process, max_entities = 32, max_travels = 64 | 1024 cases, in_process, max_entities = 256, max_travels = 512 | `slicer_ir::validate_travel_anchors` |
| `H-IR-SERDE` | `slicer-ir` | `test`:`property_ir` | `proptest_ir_serialization_and_defaults` | `slicer-ir/property_testing` | no | 64 cases, in_process, max_entities = 8, max_travels = 16, max_points = 4, max_layers = 4 | 1024 cases, in_process, max_entities = 16, max_travels = 32, max_points = 8, max_layers = 8 | `slicer_ir::LayerCollectionIR` |
| `H-IO-EXTENT` | `slicer-model-io` | `test`:`property_io` | `proptest_object_world_z_extent` | `slicer-model-io/property_testing` | no | 64 cases, in_process, max_vertices = 16, max_abs_mm = 64 | 256 cases, in_process, max_vertices = 16, max_abs_mm = 64 | `slicer_model_io::loader::object_world_z_extent` |
| `H-IO-PLACEMENT` | `slicer-model-io` | `test`:`property_io` | `proptest_place_bare_mesh_on_bed` | `slicer-model-io/property_testing` | no | 64 cases, in_process, max_objects = 4, max_vertices_per_object = 8, max_abs_mm = 64, max_translation_mm = 200 | 256 cases, in_process, max_objects = 4, max_vertices_per_object = 8, max_abs_mm = 64, max_translation_mm = 200 | `slicer_model_io::loader::place_bare_mesh_on_bed` |
| `H-IO-ID` | `slicer-model-io` | `lib`:`slicer_model_io` | `loader::property_tests::proptest_object_id_and_basename_collisions` | `slicer-model-io/property_testing` | no | 64 cases, in_process, max_basename_bytes = 32, max_paths = 8, max_index = 64 | 256 cases, in_process, max_basename_bytes = 32, max_paths = 8, max_index = 64 | `slicer_model_io::loader::path_object_id` |
| `H-SUPPORT-ANALYSIS` | `slicer-runtime` | `test`:`property_prepass` | `proptest_support_analysis_layered_scenes` | `slicer-runtime/property_testing` | no | 64 cases, in_process, max_layers = 3, max_objects = 2, max_regions = 2, max_abs_units = 200000, max_cells = 16 | 256 cases, in_process, max_layers = 3, max_objects = 2, max_regions = 2, max_abs_units = 200000, max_cells = 32 | `slicer_runtime::builtins::support_analysis_producer::commit_support_analysis_builtin` |
| `H-RUNTIME-DISPATCH` | `slicer-runtime` | `test`:`property_runtime` | `proptest_runtime_semantic_dispatch` | `slicer-runtime/property_testing` | no | 64 cases, in_process, max_layers = 3, max_regions = 3, max_semantics = 4, max_chain_len = 3, max_text_bytes = 32 | 512 cases, in_process, max_layers = 4, max_regions = 4, max_semantics = 8, max_chain_len = 4, max_text_bytes = 64 | `slicer_wasm_host::selection::paint_allows_layer` |
| `H-RUNTIME-SELECTION` | `slicer-runtime` | `test`:`property_runtime` | `proptest_runtime_selection_authority` | `slicer-runtime/property_testing` | no | 64 cases, in_process, max_layers = 3, max_regions = 3, max_modules = 2, max_chain_len = 3 | 512 cases, in_process, max_layers = 4, max_regions = 4, max_modules = 4, max_chain_len = 4 | `slicer_runtime::layer_executor::build_and_commit_layer_selection_index` |
| `H-RUNTIME-MODIFIERS` | `slicer-runtime` | `lib`:`slicer_runtime` | `region_partition::property_tests::proptest_runtime_modifier_transactions` | `slicer-runtime/property_testing` | no | 64 cases, in_process, max_regions = 3, max_footprints = 3, max_abs_units = 512, max_cells = 512 | 512 cases, in_process, max_regions = 4, max_footprints = 4, max_abs_units = 4096, max_cells = 2048 | `slicer_runtime::region_partition::split_regions_by_modifier_footprints` |
| `H-RUNTIME-PARTITION` | `slicer-runtime` | `lib`:`slicer_runtime` | `region_partition::property_tests::proptest_runtime_fill_partition` | `slicer-runtime/property_testing` | no | 64 cases, in_process, max_regions = 3, max_abs_units = 512, max_cells = 512 | 512 cases, in_process, max_regions = 4, max_abs_units = 4096, max_cells = 2048 | `slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice` |
| `H-CONFIG-BOUNDS` | `slicer-scheduler` | `test`:`property_config` | `proptest_config_scalar_bounds` | `slicer-scheduler/property_testing` | no | 64 cases, in_process, max_declarations = 4, max_list_len = 4, max_abs_bound = 1000 | 1024 cases, in_process, max_declarations = 8, max_list_len = 8, max_abs_bound = 100000 | `slicer_scheduler::config_resolution::ConfigBoundsIndex::check` |
| `H-CLAIMS` | `slicer-scheduler` | `test`:`property_scheduler` | `proptest_scheduler_claim_models` | `slicer-scheduler/property_testing` | no | 64 cases, in_process, max_modules = 4, max_claims = 4, max_observations = 8, max_layers = 3, max_text_bytes = 36 | 1024 cases, in_process, max_modules = 7, max_claims = 8, max_observations = 16, max_layers = 4, max_text_bytes = 64 | `slicer_scheduler::validation::validate_startup_dag` |
| `H-DAG` | `slicer-scheduler` | `test`:`property_scheduler` | `proptest_scheduler_dag_models` | `slicer-scheduler/property_testing` | no | 64 cases, in_process, max_nodes = 5 | 1024 cases, in_process, max_nodes = 7 | `slicer_scheduler::topological_sort` |
| `H-REGISTRY` | `slicer-scheduler` | `test`:`property_scheduler` | `proptest_scheduler_region_split_registry` | `slicer-scheduler/property_testing` | no | 64 cases, in_process, max_modules = 4, max_declarations = 8, max_text_bytes = 36 | 1024 cases, in_process, max_modules = 7, max_declarations = 16, max_text_bytes = 64 | `slicer_scheduler::region_split::aggregate_region_splits` |
| `D-GEOMETRY` | `slicer-sdk` | `test`:`property_domains` | `proptest_direct_geometry_domains` | `slicer-sdk/property_testing` | no | 64 cases, in_process, max_abs_units = 1000, max_vertices = 32, max_holes = 4, max_components = 4, max_shrink_candidates = 64 | 1024 cases, in_process, max_abs_units = 10000, max_vertices = 64, max_holes = 8, max_components = 8, max_shrink_candidates = 256 | `slicer_sdk::test_support::property_testing::domains::geometry_cases` |
| `D-GRAPHS` | `slicer-sdk` | `test`:`property_domains` | `proptest_direct_graph_domains` | `slicer-sdk/property_testing` | no | 64 cases, in_process, max_nodes = 16 | 1024 cases, in_process, max_nodes = 32 | `slicer_sdk::test_support::property_testing::domains::graphs` |
| `D-IR` | `slicer-sdk` | `test`:`property_domains` | `proptest_direct_ir_domains` | `slicer-sdk/property_testing` | no | 64 cases, in_process, max_entities = 16, max_travels = 32, max_points = 8, max_layers = 8 | 1024 cases, in_process, max_entities = 64, max_travels = 128, max_points = 16, max_layers = 16 | `slicer_sdk::test_support::property_testing::domains::anchor_cases` |
| `D-MESH` | `slicer-sdk` | `test`:`property_domains` | `proptest_direct_mesh_domains` | `slicer-sdk/property_testing` | no | 64 cases, in_process, max_extent_mm = 128, max_translation_mm = 128 | 1024 cases, in_process, max_extent_mm = 1000, max_translation_mm = 1000 | `slicer_sdk::test_support::property_testing::domains::analytical_meshes` |
| `D-SCALARS` | `slicer-sdk` | `test`:`property_domains` | `proptest_direct_scalar_domains` | `slicer-sdk/property_testing` | no | 64 cases, in_process, max_scalar_units = 10000, max_id_bytes = 64 | 1024 cases, in_process, max_scalar_units = 100000, max_id_bytes = 256 | `slicer_sdk::test_support::property_testing::domains::finite_f32` |
| `D-SCAD-2D` | `slicer-sdk` | `test`:`property_scripted` | `proptest_scripted_contour_domains` | `slicer-sdk/property_testing_scad` | yes | 24 cases, fork, timeout 30000 ms, max_abs_units = 20000, max_vertices = 64, max_holes = 4, max_components = 4, max_shrink_candidates = 32, max_recipe_nodes = 24, max_recipe_depth = 6, max_facets = 16, max_source_bytes = 16384, max_evaluations = 48, max_scad_vertices = 200000, max_scad_triangles = 400000, evaluation_steps = 5000000, worker_stack_bytes = 268435456, test_process_timeout_ms = 120000 | 96 cases, fork, timeout 60000 ms, max_abs_units = 40000, max_vertices = 96, max_holes = 8, max_components = 8, max_shrink_candidates = 64, max_recipe_nodes = 32, max_recipe_depth = 8, max_facets = 24, max_source_bytes = 32768, max_evaluations = 128, max_scad_vertices = 400000, max_scad_triangles = 800000, evaluation_steps = 20000000, worker_stack_bytes = 268435456, test_process_timeout_ms = 600000 | `slicer_sdk::test_support::property_testing::scripted::scripted_cases` |
| `D-SCAD-3D` | `slicer-sdk` | `test`:`property_scripted` | `proptest_scripted_solid_domains` | `slicer-sdk/property_testing_scad` | yes | 24 cases, fork, timeout 30000 ms, max_abs_units = 20000, max_vertices = 64, max_holes = 4, max_components = 4, max_shrink_candidates = 32, max_recipe_nodes = 24, max_recipe_depth = 6, max_facets = 16, max_source_bytes = 16384, max_evaluations = 48, max_scad_vertices = 200000, max_scad_triangles = 400000, evaluation_steps = 5000000, worker_stack_bytes = 268435456, test_process_timeout_ms = 120000 | 96 cases, fork, timeout 60000 ms, max_abs_units = 40000, max_vertices = 96, max_holes = 8, max_components = 8, max_shrink_candidates = 64, max_recipe_nodes = 32, max_recipe_depth = 8, max_facets = 24, max_source_bytes = 32768, max_evaluations = 128, max_scad_vertices = 400000, max_scad_triangles = 800000, evaluation_steps = 20000000, worker_stack_bytes = 268435456, test_process_timeout_ms = 600000 | `slicer_sdk::test_support::property_testing::scripted::scripted_cases` |
| `H-SUPPORT-TERRITORY` | `slicer-wasm-host` | `test`:`property_prepass` | `proptest_support_territory_clearance` | `slicer-wasm-host/property_testing` | no | 64 cases, in_process, max_objects = 1, max_regions = 3, max_abs_units = 200000, max_clearance_units = 8000, max_cells = 16 | 256 cases, in_process, max_objects = 1, max_regions = 3, max_abs_units = 200000, max_clearance_units = 8000, max_cells = 32 | `slicer_wasm_host::support_territory::TerritoryClipper::clip` |
| `M-SUPPORT-IRONING` | `support-surface-ironing` | `lib`:`support_surface_ironing` | `property_tests::proptest_support_ironing_fill` | `support-surface-ironing/property_testing` | no | 64 cases, in_process, max_abs_units = 100000, max_vertices = 8, max_holes = 1, max_components = 1, max_shrink_candidates = 1, max_spacing_units = 10000, max_rows = 10 | 256 cases, in_process, max_abs_units = 500000, max_vertices = 8, max_holes = 1, max_components = 1, max_shrink_candidates = 1, max_spacing_units = 10000, max_rows = 128 | `support_surface_ironing::SupportSurfaceIroning::fill_expolygon` |
| `M-IRONING-ZIGZAG` | `top-surface-ironing` | `lib`:`top_surface_ironing` | `property_tests::proptest_ironing_full_paths` | `top-surface-ironing/property_testing` | no | 64 cases, in_process, max_abs_units = 80000, max_vertices = 16, max_holes = 2, max_components = 1, max_shrink_candidates = 16, max_spacing_units = 15000, max_rows = 64 | 256 cases, in_process, max_abs_units = 80000, max_vertices = 16, max_holes = 2, max_components = 1, max_shrink_candidates = 16, max_spacing_units = 15000, max_rows = 64 | `top_surface_ironing::generate_zigzag_strokes_for_polygon` |
| `M-TRADITIONAL-SCAN` | `traditional-support` | `lib`:`traditional_support` | `property_tests::proptest_traditional_support_fill` | `traditional-support/property_testing` | no | 64 cases, in_process, max_abs_units = 200000, max_vertices = 16, max_holes = 2, max_components = 1, max_shrink_candidates = 1, max_spacing_units = 10000, max_rows = 16 | 256 cases, in_process, max_abs_units = 200000, max_vertices = 16, max_holes = 2, max_components = 1, max_shrink_candidates = 1, max_spacing_units = 10000, max_rows = 16 | `traditional_support::TraditionalSupport::fill_expolygon` |
| `M-SUPPORT-GRID` | `traditional-support-planner` | `lib`:`traditional_support_planner` | `property_tests::proptest_support_grid_extraction` | `traditional-support-planner/property_testing` | no | 64 cases, in_process, max_abs_units = 40000, max_pixel_units = 5000, max_grid_width = 18, max_grid_height = 8, max_blocks = 2, max_cells = 144 | 256 cases, in_process, max_abs_units = 40000, max_pixel_units = 5000, max_grid_width = 18, max_grid_height = 8, max_blocks = 2, max_cells = 144 | `traditional_support_planner::agg_raster::SupportGrid::new` |
| `M-SUPPORT-HOLES` | `traditional-support-planner` | `lib`:`traditional_support_planner` | `property_tests::proptest_support_hole_fill` | `traditional-support-planner/property_testing` | no | 64 cases, in_process, max_grid_width = 6, max_grid_height = 5, max_cells = 25 | 256 cases, in_process, max_grid_width = 6, max_grid_height = 5, max_cells = 25 | `traditional_support_planner::agg_raster::fill_grid_holes` |
| `M-SUPPORT-RASTER` | `traditional-support-planner` | `lib`:`traditional_support_planner` | `property_tests::proptest_support_raster_cells` | `traditional-support-planner/property_testing` | no | 64 cases, in_process, max_abs_units = 80, max_pixel_units = 10, max_grid_width = 8, max_grid_height = 8, max_cells = 64 | 256 cases, in_process, max_abs_units = 80, max_pixel_units = 10, max_grid_width = 8, max_grid_height = 8, max_cells = 64 | `traditional_support_planner::agg_raster::rasterize_polygons` |
| `M-SUPPORT-TERRITORY` | `traditional-support-planner` | `lib`:`traditional_support_planner` | `property_tests::proptest_support_planner_territory` | `traditional-support-planner/property_testing` | no | 64 cases, in_process, max_abs_units = 100000, max_clearance_units = 10000, max_cells = 121 | 256 cases, in_process, max_abs_units = 100000, max_clearance_units = 10000, max_cells = 121 | `traditional_support_planner::SupportPlanner::clip_to_territory` |
| `M-TREE-RENDER` | `tree-support` | `lib`:`tree_support` | `property_tests::proptest_tree_support_walls_and_fill` | `tree-support/property_testing` | no | 64 cases, in_process, max_abs_units = 100000, max_vertices = 8, max_holes = 1, max_components = 1, max_shrink_candidates = 1, max_width_units = 5000, max_walls = 2, max_rows = 10 | 256 cases, in_process, max_abs_units = 100000, max_vertices = 8, max_holes = 1, max_components = 1, max_shrink_candidates = 1, max_width_units = 5000, max_walls = 2, max_rows = 10 | `tree_support::TreeSupport::render_polygon_with_wall_count` |
| `M-TREE-SCAN` | `tree-support` | `lib`:`tree_support` | `property_tests::proptest_tree_support_scan` | `tree-support/property_testing` | no | 64 cases, in_process, max_abs_units = 200000, max_vertices = 8, max_holes = 1, max_components = 1, max_shrink_candidates = 1, max_spacing_units = 25000, max_rows = 10 | 256 cases, in_process, max_abs_units = 200000, max_vertices = 8, max_holes = 1, max_components = 1, max_shrink_candidates = 1, max_spacing_units = 25000, max_rows = 10 | `tree_support::TreeSupport::scan_fill_region` |
| `M-TREE-ROLES` | `tree-support-planner` | `lib`:`tree_support_planner` | `property_tests::proptest_tree_carve_and_roles` | `tree-support-planner/property_testing` | no | 64 cases, in_process, max_abs_units = 300000, max_regions = 6, max_cells = 289, max_batch_items = 3 | 256 cases, in_process, max_abs_units = 1000000, max_regions = 10, max_cells = 625, max_batch_items = 4 | `tree_support_planner::build_roles_batched` |
| `M-TREE-SMOOTH` | `tree-support-planner` | `lib`:`tree_support_planner` | `property_tests::proptest_tree_smoothing_graph` | `tree-support-planner/property_testing` | no | 64 cases, in_process, max_abs_units = 100000, max_nodes = 24, max_layers = 12, max_width_units = 10000 | 256 cases, in_process, max_abs_units = 1000000, max_nodes = 64, max_layers = 32, max_width_units = 20000 | `tree_support_planner::smooth_nodes` |
| `M-WAVE-TRIM` | `wave-overhangs` | `lib`:`wave_overhangs` | `property_tests::proptest_wave_trim_and_coverage` | `wave-overhangs/property_testing` | no | 32 cases, in_process, max_abs_units = 120000, max_spacing_units = 6000, max_width_units = 12500, max_iterations = 3, max_cells = 9216 | 128 cases, in_process, max_abs_units = 120000, max_spacing_units = 6000, max_width_units = 12500, max_iterations = 3, max_cells = 9216 | `wave_overhangs::generator::generate` |
| `H-WIPE-TOWER` | `wipe-tower` | `lib`:`wipe_tower` | `property_tests::proptest_generate_purge_paths` | `wipe-tower/property_testing` | no | 64 cases, in_process, max_abs_mm = 64, max_scan_lines = 64, max_line_width_mm = 2, max_purge_volume_mm3 = 64 | 256 cases, in_process, max_abs_mm = 64, max_scan_lines = 64, max_line_width_mm = 2, max_purge_volume_mm3 = 64 | `wipe_tower::WipeTower::generate_purge_paths` |

## 5. Execution evidence

Archive root: `target/evidence/proptest-rollout/<alias>/` (created by the closure runs). Each row
records the exact command from `packet.spec.md` § Verification; a `PENDING (closure run)` row would
mean the result was not available and is **not** a pass, but the closure rows below are all filled.
Provenance cells are filled from each run's `context.json` (`source_revision`, `dirty_paths`,
`source_state_digest`).

| Alias | Exact command | Status | Archive | Head / dirty | Result |
|---|---|---|---|---|---|
| `C1` | `run_logged cargo test -p xtask --bin xtask -- property_closure_tests::closure_registry_census_rejects_unregistered_target --exact --nocapture` | PASS | `target/evidence/proptest-rollout/C1/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 286 s (merged-tree run); control status=0; negative status=1 with `unregistered_property_test` |
| `C2` | `run_logged cargo test -p xtask --bin xtask -- property_closure_tests::closure_registry_census_rejects_missing_registered_property --exact --nocapture` | PASS | `target/evidence/proptest-rollout/C2/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 38 s (merged-tree run); control status=0; negative `missing_property_execution: registered test proptest_absent_fixture not discovered` |
| `X` | `run_logged cargo test -p xtask --bin xtask -- property_ --nocapture` | PASS | `target/evidence/proptest-rollout/X/` | HEAD `f952bd07` (merged, dirty) | PASS — 25 passed, 0 failed, 1568 s (merged-tree run); includes the full SCAD census (`lane=scad targets=54 status=0`) |
| `CHAIN-runtime` | `run_logged cargo test -p slicer-runtime --features property_testing --test property_prepass_controls -- prepass_paint_map_identity_integration --exact --nocapture` | PASS | `target/evidence/proptest-rollout/CHAIN-runtime/` | HEAD `f952bd07`; final-head re-run `85f0d9c4` | PASS — 1 passed, 0.00 s; final-head re-run `final-head.log` passes |
| `CHAIN-core` | `run_logged cargo test -p slicer-core --features property_testing,host-algos --test property_prepass_controls -- prepass_regionmap_order_and_cap_controls --exact --nocapture` | PASS | `target/evidence/proptest-rollout/CHAIN-core/` | HEAD `f952bd07`; final-head re-run `85f0d9c4` | PASS — 1 passed, 0.00 s; final-head re-run `final-head.log` passes |
| `GCODE` | `run_logged cargo test -p slicer-gcode --features property_testing --test property_gcode -- gcode_serializer_controls --exact --nocapture` | PASS | `target/evidence/proptest-rollout/GCODE/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.34 s |
| `SPEED` | `run_logged cargo test -p overhang-classifier-default --features property_testing --test property_overhang_speed -- overhang_speed_controls --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SPEED/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.10 s |
| `BRIM` | `run_logged cargo test -p skirt-brim --features property_testing --lib -- property_tests::skirt_brim_rounding_boundary_control --exact --nocapture` | PASS | `target/evidence/proptest-rollout/BRIM/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.13 s |
| `SCAD-controls` | `run_logged cargo test -p slicer-sdk --features property_testing_scad --test property_scripted_contracts -- scripted_abort_and_timeout_keep_pre_execution_case --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-controls/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.36 s |
| `SCAD-off-adapter` | `run_logged cargo test -p slicer-scad --no-default-features --test scad_feature_contract -- scad_backend_feature_contract --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-off-adapter/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.00 s |
| `SCAD-on-adapter` | `run_logged cargo test -p slicer-scad --features openrscad --test scad_feature_contract -- scad_backend_feature_contract --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-on-adapter/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.02 s |
| `SCAD-off-model` | `run_logged cargo test -p slicer-model-io --no-default-features --test scad_ingest_feature_contract -- scad_model_feature_routing --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-off-model/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.00 s |
| `SCAD-off-cli` | `run_logged cargo test -p pnp-cli --no-default-features --test scad_cli_feature_contract -- scad_cli_feature_routing --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-off-cli/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.83 s |
| `SCAD-on-model` | `run_logged cargo test -p slicer-model-io --no-default-features --features scad --test scad_ingest_feature_contract -- scad_model_feature_routing --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-on-model/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.00 s |
| `SCAD-on-cli` | `run_logged cargo test -p pnp-cli --no-default-features --features scad --test scad_cli_feature_contract -- scad_cli_feature_routing --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-on-cli/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 2.56 s |
| `SCAD-adapter` | `run_logged cargo test -p slicer-scad --features openrscad --test scad_adapter_contracts -- scad_rejects_partial_approximate_degraded_and_unknown --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-adapter/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.06 s |
| `SCAD-process` | `run_logged cargo test -p slicer-scad --features openrscad --test scad_process_limits -- scad_process_supervision_retains_repro_and_joins --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-process/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 20.73 s |
| `SCAD-model-all` | `run_logged cargo test -p slicer-model-io --features scad --test scad_model_ingest -- scad_model_ --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-model-all/` | HEAD `f952bd07` (merged, dirty) | PASS — 5 passed, 0.22 s (all selected `scad_model_*` executed) |
| `SCAD-cli-all` | `cargo xtask test --summary -p pnp-cli --features pnp-cli/scad --test scad_cli_ingest -- scad_cli_ --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-cli-all/` | HEAD `f952bd07` (merged, dirty) | PASS — 10 passed, 5.86 s (all selected `scad_cli_*` executed) |
| `SCAD-product` | `cargo xtask test --summary -p pnp-cli --features pnp-cli/scad --test scad_cli_ingest -- scad_cli_slice_places_and_emits --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-product/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 1.43 s |
| `SCAD-import` | `cargo xtask test --summary -p pnp-cli --features pnp-cli/scad --test scad_cli_ingest -- scad_cli_import_preserves_source_coordinates --exact --nocapture` | PASS | `target/evidence/proptest-rollout/SCAD-import/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 passed, 0.26 s |
| `P-core-routine` | `cargo xtask test --summary --properties --tier routine` | PASS | `target/evidence/proptest-rollout/P-core-routine/` | HEAD `f952bd07` (merged, dirty) | PASS — VERDICT: PASS; 89 controller end event(s); 89 unique property ID(s); census lane=core targets=51 status=0 |
| `P-scad-routine` | `cargo xtask test --summary --properties --tier routine --with-scad` | PASS | `target/evidence/proptest-rollout/P-scad-routine/` | HEAD `f952bd07` (merged, dirty) | PASS — VERDICT: PASS; 93 controller end event(s); 93 unique property ID(s); census lane=scad targets=54 status=0 |
| `P-core-extended` | `cargo xtask test --summary --properties --tier extended` | PASS | `target/evidence/proptest-rollout/P-core-extended/` | HEAD `f952bd07` (merged, dirty) | PASS — VERDICT: PASS; 89 controller end event(s); 89 unique property ID(s); census lane=core targets=51 status=0 |
| `P-scad-extended` | `cargo xtask test --summary --properties --tier extended --with-scad` | PASS | `target/evidence/proptest-rollout/P-scad-extended/` | HEAD `f952bd07` (merged, dirty) | PASS — VERDICT: PASS; 93 controller end event(s); 93 unique property ID(s); census lane=scad targets=54 status=0 |
| `P-scad-geometry-routine` | `cargo xtask test --summary --properties --property H-BOOL-SCAD --property H-MESH-SCAD --tier routine --with-scad` | PASS | `target/evidence/proptest-rollout/P-scad-geometry-routine/` | HEAD `f952bd07` (merged, dirty) | PASS — VERDICT: PASS; 2 controller end event(s); 2 unique property ID(s); census lane=scad targets=54 status=0 |
| `P-scad-geometry-extended` | `cargo xtask test --summary --properties --property H-BOOL-SCAD --property H-MESH-SCAD --tier extended --with-scad` | PASS | `target/evidence/proptest-rollout/P-scad-geometry-extended/` | HEAD `f952bd07` (merged, dirty) | PASS — VERDICT: PASS; 2 controller end event(s); 2 unique property ID(s); census lane=scad targets=54 status=0 |
| `R-core` run 1 / run 2 | `cargo xtask test --summary --properties --property H-IR-ANCHORS --tier routine --seed 42` | PASS | `target/evidence/proptest-rollout/R-core-1/`, `…/R-core-2/` | HEAD `f952bd07` (merged, dirty) | PASS twice; **REPLAY MATCH** — 64/64 accepted initial observations identical, first input identical, context identical (seed 42, same source digest) |
| `R-scad` run 1 / run 2 | `cargo xtask test --summary --properties --property D-SCAD-2D --tier routine --with-scad --seed 42` | PASS | `target/evidence/proptest-rollout/R-scad-1/`, `…/R-scad-2/` | HEAD `f952bd07` (merged, dirty) | PASS twice; **REPLAY MATCH** — 24/24 accepted initial observations identical (source text and engine provenance included), context identical |
| `GRAPH` | see §9 | PASS | `target/evidence/proptest-rollout/GRAPH/` | HEAD `f952bd07` (merged, dirty) | PASS — 11/11 host graphs exit 0; 60/60 guest manifests 0 leaks; positive controls present |
| `H-PAINT-OVERHANG` extended (G3) | `cargo xtask test --summary --properties --property H-PAINT-OVERHANG --tier extended` | PASS | `target/evidence/proptest-rollout/H-PAINT-OVERHANG-extended/` | HEAD `f952bd07` (merged, dirty) | PASS — 1 record, 256 cases, census 51 (an earlier pre-merge attempt hit `Out of diskspace`; the merged-tree run is the recorded one) |

### 5.1 Windows CI

| Job | Trigger | Closure-head run | Status |
|---|---|---|---|
| `properties-windows` | `pull_request`, `push` | PR run [`45946601128`](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45946601128) / [job 140332608523](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45946601128/job/140332608523), head `312e5278` | PASS |
| `properties-scad-windows` | `pull_request`, `push` | PR run [`45946601128`](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45946601128) / [job 140332608584](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45946601128/job/140332608584), head `312e5278` | PASS |
| `scad-adapter-windows` | `pull_request`, `push` | PR run [`45946601128`](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45946601128) / [job 140332608659](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45946601128/job/140332608659), head `312e5278` | PASS |
| `scad-ingest-windows` | `pull_request`, `push` | PR run [`45946601128`](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45946601128) / [job 140332608599](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45946601128/job/140332608599), head `312e5278` | PASS |
| `properties-extended-windows` (`property_run = extended`) | `workflow_dispatch` only | [run 45950327847](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45950327847), [job 140344427469](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45950327847/job/140344427469), head `fd9d5463` | **PASS** — both complete extended selections (`extended-core` 89 end events / census 51/0; `extended-scad` 93 / census 54/0); artifacts archived at `target/ci-evidence/extended-45950327847/` |
| `properties-extended-windows` (`property_run = failure-probe`) | `workflow_dispatch` only | [run 45950340929](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45950340929), [job 140344467445](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/45950340929/job/140344467445), head `fd9d5463` | **FAILED as designed**; artifact `pbt-ci-failure-probe` downloaded to `target/ci-evidence/failure-probe-45950340929/` (marker `pbt-ci-failure-probe`; real `property_executor_*` status controls passed). Never completion evidence |

Historical predecessor hosted runs (these predate the census/roster replacement and are **not**
closure evidence): `scad-adapter-windows` runs 42535615636, 43113651822, 43131961268
(`docs/scad/windows-feasibility.md`); ingestion run 43369601815 (`docs/scad/ingestion-evidence.md`);
43523810446 (`docs/testing/proptest-ir-config-evidence.md`); 43556307245
(`docs/testing/proptest-scheduler-evidence.md`); 43628720540, 43629202926
(`docs/testing/proptest-runtime-evidence.md`); 44182873399 (`docs/testing/proptest-geometry-evidence.md`);
44434672409 (`docs/testing/proptest-arachne-evidence.md`); 44976563310
(`docs/testing/proptest-support-modules-evidence.md`).

The PR evidence artifacts are downloaded under `target/ci-evidence/pr-45946601128/`:
`properties-windows-evidence` (89 controller end events; `property_census: lane=core targets=51
status=0`), `properties-scad-windows-evidence` (93 end events; `lane=scad targets=54 status=0`),
`scad-ingest-windows-evidence` (12 `bed_placement_tdd` tests among the retained logs) and
`scad-adapter-windows` (3 tests).

Lanes with no recorded hosted Windows run in their *predecessor* evidence were #12 and #15 (local
lanes only). Both are now covered by the closure-head PR run `45946601128` (head `312e5278`), whose
`Properties (Windows)` job executes all 89 core records including #12's five and #15's eleven
IDs; the downloaded evidence log contains 89 controller end events and `lane=core targets=51
status=0`.

The five `slicer-sdk --test property_domains` tests that packet #9 reported as pre-existing
workspace failures now assert `missing_property_context` and skip loudly in a bare workspace run;
the registry lane executes them (D-SCALARS re-run PASS). The workspace ceremony is green.

## 6. Profiles, seeds and replay

### 6.1 Accepted profiles

The per-record measured profiles, pilots and methods are in `docs/testing/properties.md`
§ Measured profiles and each charter's profile paragraph; they are not duplicated here. Registry-wide
distribution, re-derived from `test-properties.toml`:

| Key | routine | extended |
|---|---|---|
| `cases` | 64 ×88, 24 ×4 (SCAD), 32 ×1 (`M-WAVE-TRIM`) | 256 ×56, 1024 ×21, 512 ×11, 96 ×4 (SCAD), 128 ×1 (`M-WAVE-TRIM`) |
| `isolation` | `in_process` ×89, `fork` ×4 (SCAD) | same |
| `timeout_ms` | 0 ×89, 30000 ×4 | 0 ×89, 60000 ×4 |
| `max_local_rejects` / `max_global_rejects` | 0/0 ×89; 32/128 ×4 | 0/0 ×89; 128/512 ×4 |
| `max_flat_map_regens` | 0 ×93 | 0 ×93 |
| `max_shrink_iters` | 64 ×93 | 256 ×78, 128 ×15 (the #8 runtime and #11 prepass records) |
| `max_shrink_time_ms` | 0 ×93 | 0 ×93 |
| `max_default_size_range` | 32 ×93 | 256 ×21, 128 ×72 |
| `rng_algorithm` | `chacha` ×93 | `chacha` ×93 |
| SCAD `bounds.test_process_timeout_ms` | 120000 ×4 | 600000 ×4 |

Observed resolved values from the closure runs (archived `context.json` per record):

| Lane | Contexts | Unique seeds | Dirty paths | `source_state_digest` |
|---|---:|---:|---:|---|
| `P-core-routine` | 89 | 89 | 13 | `18647e4f8e…` |
| `P-scad-routine` | 93 | 93 | 13 | `18647e4f8e…` |
| `P-core-extended` | 89 | 89 | 13 | `18647e4f8e…` |
| `P-scad-extended` | 93 | 93 | 13 | `18647e4f8e…` |
| `P-scad-geometry-routine` | 2 | 2 | 13 | `18647e4f8e…` |
| `P-scad-geometry-extended` | 2 | 2 | 13 | `18647e4f8e…` |
| `H-PAINT-OVERHANG-extended` | 1 | 1 | 13 | `18647e4f8e…` |

The digest is a SHA-256 over `git rev-parse HEAD`, the full tracked binary diff and the bytes of
relevant untracked files (`source_state_at`, `xtask/src/properties.rs`). The untracked evidence
ledger is itself a relevant `.md` file, so the digests changed **within** `P-core-extended` and
`P-scad-routine` while this ledger was being written; the 13 dirty paths and the tracked diff were
unchanged. The replay pairs were run after the ledger was frozen; their contexts are identical.

Windows/toolchain: rustc 1.96.0, `dev` profile, `CARGO_TARGET_DIR` unset, Proptest 1.11.0.
SCAD records resolved `isolation = fork`, routine `timeout_ms = 30000` /
`bounds.test_process_timeout_ms = 120000`, extended `timeout_ms = 60000` /
`bounds.test_process_timeout_ms = 600000`; the scripted observations carry the engine identity
(`revision = 1a45bd33f2886c32bda32fe6e8358a67ab03f5ef`, `patch_digest = c0297079…`,
`backend = manifold_cpp`) and the generated `source.scad` text. Per-record compile, execution,
generation/render and shrink costs were not separately instrumented beyond the lane walls and the
per-record child status; test-call counters are not accepted cases.

Shrink completion is reported `null`/unknown for pure Proptest records (Proptest 1.11 cannot
report whether minimization finished; `docs/testing/properties.md` § Measured profiles), and is
reported only where the harness itself can establish it (for example the scripted evaluation-budget
case reports `false`). Known shrink-limit gap: routine `max_shrink_iters = 64` truncates
`H-BOOL-SET` seeded-fault minimisation, which needed 110–167 iterations
(`docs/testing/proptest-geometry-evidence.md`); recorded as a profile limitation (owner #9); the extended tier uses
`max_shrink_iters = 256`, so no acceptance gate depends on the routine 64-iteration value.

### 6.2 Replay

| Replay | Command (run twice) | Retained seed | Observation source | Status |
|---|---|---|---|---|
| `R-core` | `cargo xtask test --summary --properties --property H-IR-ANCHORS --tier routine --seed 42` | 42 | `target/evidence/proptest-rollout/R-core-1/artifacts/.../observations.jsonl`, `case.txt`, `context.json` | PASS twice; 64/64 accepted initial observations identical (the 128-line `observations.jsonl` has a `started` + `passed` record per attempt) |
| `R-scad` | `cargo xtask test --summary --properties --property D-SCAD-2D --tier routine --with-scad --seed 42` | 42 | `target/evidence/proptest-rollout/R-scad-1/artifacts/.../observations.jsonl`, `case.txt`, `context.json`, engine identity | PASS twice; 24/24 accepted initial observations identical (48-line journal, same `started`/`passed` pairing) |

Acceptance compares the concrete pre-shrink `case.txt` sequence and observations of run 1 and run 2
under unchanged `generator_version`, `source_state_digest` and engine identity; equal seed text alone
is not replay evidence. Fresh OS-entropy sampling (`resolve_seed`, `xtask/src/properties.rs`) is the
default for every other run. Retain each run's `context.json`, `case.txt`, `failure.json` (when
present) and its reproduction command before the next invocation.

## 7. Candidate defects and regressions

Compiled from the forward-item audits of #1–#16 (§8). Source inspection is not reproduction.

| Packet | Candidate | Reproduced? | Fix / regression test | Status |
|---|---|---|---|---|
| #2 | Three test/oracle defects | yes | fixed in domain helpers; `docs/testing/proptest-domain-evidence.md` | resolved |
| #3 | Ten adapter defects | yes (review) | fixed; `docs/scad/windows-feasibility.md` | resolved |
| #3 | `indexed_queries == 0` from thread-local `DIAGNOSTIC_STATE` vs rayon | observed | diagnostic counter only; no property or oracle depends on it; owner #3 | recorded limitation (owner #3) |
| #5 | Stale cached `world_z_extent` | yes | fixed; `docs/scad/ingestion-evidence.md` | resolved |
| #5 | `non_finite_coordinate` from public input | not reproduced end-to-end | public validating APIs reject non-finite values; constructed-envelope coverage in #3; owner #5 | recorded gap (owner #5, no acceptance impact) |
| #6 | Wire-layout break from removed `ResolvedConfig` field | yes | serialization carrier; `docs/testing/proptest-ir-config-evidence.md` | resolved |
| #7 | Typed claim-scope / region-stability weaknesses in `validate_claim_conflicts` (`crates/slicer-scheduler/src/validation.rs`) | yes (two SUT defects) | typed `ClaimScopeKey`; `delimiter_rich_object_and_region_ids_never_share_a_claim_scope`, `non_transitionable_claim_rejects_holder_change_across_regions_of_one_object` | resolved |
| #8 | Hook error path losing the taken slice | yes (pre-fix log `target/evidence/step4-regression-prefix.log`, git-ignored) | `runtime_modifier_collision_restores_taken_slice` (`crates/slicer-runtime/tests/common/property_runtime_modifiers.rs`) | resolved |
| #9 | XOR redundant-collinear spike / needle excursion | yes | `xor_redundant_collinear_vertex_emits_no_backtracking_spike`, `needle_excursion_returning_to_its_start_leaves_no_duplicate_vertex` (`crates/slicer-core/tests/polygon_ops_tdd.rs`) | resolved; `polygons_to_expolygons` bypasses normalization, no spike observed |
| #9 / #4 | Combined G-GEOMETRY decision | — | `docs/testing/proptest-scripted-evidence.md` § 12 → **`hybrid_sufficient`**; `bounded_direct_gap_filled` unused; `general_generator_blocked` not recorded | resolved |
| #10 | Canonical widening / local-maximum micro-loop / at-cap / winding | yes | `limited_transition_thickness_at_cap_matches_canonical_floor`, `arachne_local_maximum_pre_stitch_is_open`, three `build_walls_winding_witness_*` | resolved; `getRegionOrder` not adjudicated (no dependent property) |
| #11 | Ordered-chain identity (ADR-0018 / DEV-199) | source-derived correction (no red pre-fix test) | ADR-0018 amendment "Shared priority/name chain identity" (2026-11-19, `docs/adr/0018-region-split-priority-registry-and-canonical-order.md`); DEV-199 reconciliation note (`docs/DEVIATION_LOG.md`); controls `prepass_regionmap_order_and_cap_controls`, `prepass_paint_map_identity_integration` | documentary resolved; behavioral controls `CHAIN-core` / `CHAIN-runtime` PASS |
| #11 | Distinct-footprint modifier collision / unsupported parent | real, no defect | `prepass_regionmap_rejection_controls`; painted-ID panic path removed | resolved (painted-ID change not labelled a confirmed SUT defect) |
| #12 | Fuzzy / capacity witnesses | executed | 1 mm / 10-band witness emits exactly band `{0}` (`docs/testing/proptest-walls-evidence.md` § Step 4) | no SUT defect |
| #12 | Classic post-offset degeneracy | canonical classification | `process_classic`/`process_arachne` collapse narrow infill/gap areas but do not re-screen each offset ring; PnP follows the same contract; exact band oracle + witnesses | resolved (G4) |
| #13 | Over-cap solid spacing; ironing hole crossing | yes | `solid_spacing_over_cap_uses_canonical_1_2_factor`, `hole_crossing_regression_no_ironing_segment_enters_the_hole`; DEV-209 rows | resolved |
| #13 | Gyroid / linker oracle defects | yes | fixed in property oracles | resolved |
| #13 | `clip_batched` vs `clip` | candidate | both points stay on the inflated boundary; no retention defect observed; owner #13 | none-found |
| #13 | Wave-overhang fork parity | canonical read | no wave-overhang fork in the selected checkout; `Algorithm/RegionExpansion.cpp` is a different wave algorithm; no parity claim | resolved (G6) |
| #14 | Tree vertical-scan guard; traditional fallback translation | yes | `vertical_scan_fill_spans_the_full_y_extent_regression`, `narrow_translated_tip_fallback_is_placed_at_the_bbox_centre_regression`; DEV-209 amendment | resolved; `CLIPPER_ALLOW = 2` rounding rule source-comment only |
| #15 | Random-empty, open-path projection, XY closure, lock split | yes | `random_empty_candidates_degrade_without_panic`, `open_polyline_projection_preserves_open_geometry`, `cross_role_and_cross_tool_locked_blocks_stay_adjacent_and_forward`; DEV-201 | resolved; `role_group` `_ => 9` wildcard characterized, unchanged |
| #16 | Absolute-mode retract/unretract must emit and update absolute E | yes | named regression in `docs/testing/proptest-io-gcode-evidence.md`; control `GCODE` PASS (1 passed, 0.34 s) | resolved |
| #16 | `calculate_speed` last boundary = `min(round(last_section_speed), original_speed)` (`modules/core-modules/overhang-classifier-default/src/lib.rs`) | yes | fixed to `sections[len-1].1.round().min(original_speed)` with named concrete regression (`docs/testing/properties.md` H-OVERHANG-SPEED charter); control `SPEED` PASS (1 passed, 0.10 s) | resolved |
| #16 | `place_bare_mesh_on_bed` live-transform `build_volume` | yes | `identity_matrix_placement_updates_build_volume_to_the_world_union`, `zero_shift_placement_still_refreshes_build_volume`, `extreme_same_sign_extents_do_not_overflow_the_midpoint`, `non_finite_y_world_returns_none_without_mutation` | resolved |
| #16 | X.500 namespace naming | — | namespace is `6ba7b814-9dad-11d1-80b4-00c04fd430c8` (X.500), not `Uuid::NAMESPACE_OID`; identity bytes unchanged | resolved |
| #16 | `role_equals` | yes | fixed (`docs/testing/proptest-io-gcode-evidence.md`) | resolved |
| #16 | Skirt/brim 1.19 / 0.17 zero-offset boundary | **not reproduced** (4 bands / 8 rings) | control `skirt_brim_rounding_boundary_control`; `BRIM` PASS (1 passed, 0.13 s) | none-found |
| #16 | Thin-tail sliver | candidate | canonical emission of sub-0.0125 mm slivers unmeasured and not claimed; the thin-tail classifier covers the generated domain; owner #16 | recorded limitation (no acceptance impact) |
| #16 | Wipe-tower bounded scan / convex bed | — | bed check in `WipeTower::run_finalization`; concave control oracle-only | resolved |

## 8. Forward-item ledger

55 predecessor `[FWD]` rows: **55 resolved, 0 blocked/partial**. Counts come from the two
independent forward-item audits (01–08: 24 rows; 09–16: 31 rows) prepared for this closure; the
per-row source is the named evidence document. "Approved" marks a human-approved disposition;
blocked rows are unreviewed omissions until their owner acts. Row 34 (`H-PAINT-OVERHANG` extended)
was resolved in this closure by the passing `H-PAINT-OVERHANG-extended` run.

| # | Pkt | Forward item (short) | Status | Resolution / blocker |
|---:|---|---|---|---|
| 1 | 01 | Select Proptest 1.11 + entropy/serialization deps; record lock | RESOLVED | `docs/testing/proptest-foundation-evidence.md` § G-HARNESS; `Cargo.lock` proptest 1.11.0; `property_dependency_isolation` (`xtask/src/properties.rs`). Gap: MSRV 1.91 build not performed |
| 2 | 01 | Measure first-property pilots; choose counts/bounds | RESOLVED | `docs/testing/properties.md` § Measured profiles; raw archives git-ignored under `target/evidence/` |
| 3 | 01 | Fork re-entry/test-name handling; truthful shrink completion | RESOLVED | `property_run_fork_probe_tags_workers`, `property_run_fork_crash_is_not_reported_as_completed` (`crates/slicer-sdk/tests/property_harness.rs`) |
| 4 | 02 | Direct-family pilots; float/arithmetic error policies | RESOLVED | `docs/testing/proptest-domain-evidence.md` § Registry pilots; `docs/testing/properties.md` § Direct-domain pilots |
| 5 | 02 | Exact rational/big-integer deps | RESOLVED | optional `num-bigint`/`num-rational` (`crates/slicer-sdk/Cargo.toml`); `domain_regression_fixture_compiles_and_preserves_bits` |
| 6 | 02 | Direct-only G-GEOMETRY disposition | RESOLVED | `docs/testing/proptest-domain-evidence.md` § Direct G-GEOMETRY disposition |
| 7 | 03 | Pin OpenRSCAD revision/deps/backend/patches; Windows build | RESOLVED | `docs/scad/windows-feasibility.md` § Source provenance; hosted run 43131961268. Vendored patched subset is a documented deviation |
| 8 | 03 | Budget/override/exact-mode, diagnostics, seed/resource seams | RESOLVED | `docs/scad/windows-feasibility.md` § Narrow test results |
| 9 | 03 | Conversion bounds, worker stack, process limits | RESOLVED | `scad_worker_stack_policy_contains_non_tail_recursion`, `scad_process_supervision_retains_repro_and_joins`; no in-process render deadline (supervised child only) |
| 10 | 04 | Measure recipe generation/evaluation/rejection/shrink + containment | RESOLVED | `docs/testing/proptest-scripted-evidence.md` §§ 5–10; `property_scad_supervision_covers_generation_shrink_and_descendants`, `property_scad_job_object_contains_a_descendant_spawned_before_assignment`. Gap: replay logs under `target/pbt-*` not durable |
| 11 | 04 | Checked arithmetic/grid/f32/mm³ envelopes | RESOLVED | `docs/testing/proptest-scripted-evidence.md` § 4 |
| 12 | 04 | Combined G-GEOMETRY disposition | RESOLVED | `hybrid_sufficient` (`docs/testing/proptest-scripted-evidence.md` § 12) |
| 13 | 05 | Bounded Windows import/slice fixtures and costs | RESOLVED | `docs/scad/ingestion-evidence.md` (950 s cold build; hosted run 43369601815). Gap: `non_finite_coordinate` not reproduced end-to-end |
| 14 | 06 | Pilots, profiles, exact reference arithmetic | RESOLVED | `docs/testing/proptest-ir-config-evidence.md` § Measured profiles, § Retained-seed replay |
| 15 | 06 | Audit live config Eq/Hash field families | RESOLVED | `docs/testing/properties.md` § H-CONFIG-INTERN |
| 16 | 06 | #8 modifier collision; #11 caller collision / unsupported parent | RESOLVED | `runtime_modifier_collision_restores_taken_slice`; `prepass_regionmap_rejection_controls` |
| 17 | 07 | Pilots; baseline failures | RESOLVED | `docs/testing/proptest-scheduler-evidence.md` § Committed profiles |
| 18 | 07 | Reproduce typed-scope/region-stability weaknesses | RESOLVED | §7 #7 row |
| 19 | 07 | #11 owns H-REGIONMAP driver audit | RESOLVED | `docs/testing/proptest-prepass-evidence.md` § The ordering correction; ADR-0018 amendment |
| 20 | 07 | #6 H-CHAINS exercises enumerator order, not caller | RESOLVED (documentary) | `docs/testing/properties.md` § H-CHAINS |
| 21 | 07 | #8 owns selection dispatch/modifier transactions | RESOLVED | `docs/testing/proptest-runtime-evidence.md` |
| 22 | 08 | Pilots before profiles; compile/execution separate | RESOLVED | `docs/testing/proptest-runtime-evidence.md` § Compile and execution measurements (extended tier 512) |
| 23 | 08 | Reproduce hook error losing taken slice | RESOLVED | §7 #8 row |
| 24 | 08 | #9/#11 own general correctness; runtime seam | RESOLVED | `docs/testing/proptest-runtime-evidence.md` § Windows CI delivery (run 43629202926) |
| 25 | 09 | Direct Windows pilots; cases/bounds | RESOLVED | `docs/testing/proptest-geometry-evidence.md`; hosted run 44182873399. Gap: §6.1 `H-BOOL-SET` shrink-limit note |
| 26 | 09 | Separate SCAD profiles + bounded mesh reference | RESOLVED | `proptest_scripted_boolean_membership`, `proptest_scripted_mesh_cross_sections` (`crates/slicer-core/tests/property_geometry_scad.rs`); hosted job 134391555486 |
| 27 | 09 | Recheck baselines; classify defects | RESOLVED | §7 #9 row |
| 28 | 10 | Arachne bounds, envelopes, profiles | RESOLVED | `docs/testing/proptest-arachne-evidence.md` |
| 29 | 10 | Canonical audit: open-path direction, invariants | RESOLVED | §7 #10 row |
| 30 | 10 | Coordinate ownership with #12; no new public helper | RESOLVED (structural) | registry ID sets; no explicit evidence-doc sentence |
| 31 | 10 | Re-derive literal-scanner scope | RESOLVED | packet #10 Deviations; scanner still omits module `src/**` and out-of-line modules (§10) |
| 32 | 11 | Coordinate ordering with #7; supersede stale driver wording | RESOLVED | ADR-0018 amendment 2026-11-19; DEV-199 |
| 33 | 11 | Real distinct-footprint collision; painted-ID caller | RESOLVED | `prepass_regionmap_rejection_controls` |
| 34 | 11 | Windows pilots incl. `H-PAINT-OVERHANG` extended | RESOLVED | `H-PAINT-OVERHANG-extended` PASS (1 record, 256 cases, census 51); §5 row records the first-attempt disk-full failure and the green re-run |
| 35 | 11 | Hand analysis/territory to #14, lightning to #13, IDs to #17 | RESOLVED | `docs/testing/proptest-prepass-evidence.md` § Ownership handoffs |
| 36 | 12 | Measure all five wall/fuzzy properties | RESOLVED | `docs/testing/proptest-walls-evidence.md` § Step 7 |
| 37 | 12 | Derive fuzzy displacement tolerance K | RESOLVED | § Step 6 (K = 32) |
| 38 | 12 | 1 mm / 10-band capacity witness | RESOLVED | § Step 4 |
| 39 | 12 | Classify fuzzy metadata and classic post-offset degeneracy | RESOLVED | fuzzy part in `docs/02_ir_schemas.md` § Fuzzy-skin spans; classic part classified against `PerimeterGenerator.cpp` (no per-ring re-screen), PnP contract-conforming, no defect |
| 40 | 12 | Recheck #1/#2 surfaces and Proptest 1.11 APIs | RESOLVED | § Prerequisite surface |
| 41 | 13 | Recheck #1/#2/#9 APIs and #11 chain handoff | RESOLVED | `docs/testing/proptest-infill-evidence.md` § Prerequisite surface |
| 42 | 13 | Pilots; integer/f32/clip/footprint/lattice bounds | RESOLVED | §§ Step 4/6/8 |
| 43 | 13 | Gyroid residual mapping or retain phase/grid/clip; recheck grid origin | RESOLVED | residual deliberately not asserted (correct disposition); grid origin verified against `FillGyroid.cpp` (`align_to_grid(bb.min, 2*pi*distance)`) |
| 44 | 13 | Over-cap spacing and holed/U ironing witnesses | RESOLVED | §7 #13 row |
| 45 | 13 | Wave-overhang fork delegated read | RESOLVED | no canonical wave-overhang fork; `RegionExpansion` is a different algorithm; no parity claim, contract-only module evidence |
| 46 | 14 | Recheck #1/#2/#11 status, exports, territory | RESOLVED | `docs/testing/proptest-support-modules-evidence.md` § 2 |
| 47 | 14 | Support/tree pilots; envelopes | RESOLVED (source-inspected residual) | §§ 9.2–9.5b; `CLIPPER_ALLOW = 2` unverified |
| 48 | 14 | Traditional fallback / vertical tree witnesses | RESOLVED | §7 #14 row |
| 49 | 14 | Re-read canonical source before borrowing a fix | RESOLVED | § 5; DEV-143/165/166/142/144/156 preserved |
| 50 | 15 | Recheck #1/#2 exports, selectors, feature unification | RESOLVED | `docs/testing/proptest-layers-seams-evidence.md` § 2 |
| 51 | 15 | Top-based vs center probe; Random-empty/open/lock/role witnesses | RESOLVED | §7 #15 row; DEV-201 |
| 52 | 15 | Fit/projection envelopes; profiles; hand IDs to #17 | RESOLVED | cold compile measured at 135 s for the four owner crates; the hosted `Properties (Windows)` run 45946601128 executes all 11 records |
| 53 | 16 | Measure all eleven records | RESOLVED | `docs/testing/proptest-io-gcode-evidence.md` § 7 |
| 54 | 16 | Wipe-tower convex bed; concave control oracle-only | RESOLVED | § 5 |
| 55 | 16 | Recheck #1/#2/#6 exports; preserve #11 chain identity | RESOLVED | § 2 |

### 8.1 This packet's own open questions

| #17 `[FWD]` | Current disposition |
|---|---|
| Re-derive statuses, registry/target/feature identities, predecessor `[FWD]` dispositions, workspace membership, scanner scope | Done in this draft: all 16 predecessors `implemented`; 93 records / 93 charters / 32 owners; §8 ledger; §10 scanner scope. Must be re-checked at final write |
| Measure Windows compile/execution/generation/render/shrink; validate profiles and replay; obtain ruleset/check-run and dispatch artifacts | **Closed** — §5, §5.1, §6, §10.2 |
| Resolve #9 scripted enrichment, #4 combined decision, SCAD feasibility/product, collision/candidate follow-ups | #4 `hybrid_sufficient` and #9 `H-BOOL-SCAD`/`H-MESH-SCAD` are implemented with a historical hosted job; SCAD product commands PASS (SCAD-product, SCAD-import, SCAD-cli-all 10/10, SCAD-model-all 5/5); every §7 candidate has a disposition (none-found/recorded limitation) as listed there |

## 9. Dependency and guest isolation (`GRAPH`)

Results below are from the closure run (`target/evidence/proptest-rollout/GRAPH/`). Evaluate
actual graph nodes, not source text. Dev/test graphs are labeled separately and never presented as
production graphs.

| Command | Expected finding | Result |
|---|---|---|
| `cargo tree --workspace --target x86_64-pc-windows-msvc -e normal,build` | negative: no `openrscad`/`manifold`/`proptest` merely because `slicer-scad` is a member | PASS — exit 0; `openrscad|manifold|proptest` hits 0. The graph contains the `slicer-scad` member node (1 line) but not its engine; membership alone does not enable OpenRSCAD. |
| `cargo tree -p pnp-cli --target x86_64-pc-windows-msvc -e normal,build` | negative: no `proptest`, no OpenRSCAD | PASS — exit 0; forbidden `proptest|openrscad|manifold|slicer-scad` hits 0. |
| `cargo tree -p pnp-cli --features scad --target x86_64-pc-windows-msvc -e normal,build` | positive: engine present; negative: no `proptest`. The existing normal `pnp-cli` → `slicer-sdk` (`test`) edge is expected and is not claimed absent | PASS — exit 0; `proptest` hits 0; `slicer-scad` present (1 line) as the enabled positive edge. The normal `pnp-cli` → `slicer-sdk` (`test`) edge remains and is not claimed absent. |
| `cargo tree -p slicer-model-io --features scad --target x86_64-pc-windows-msvc -e normal,build,features` | positive: `slicer-scad` + engine; negative: no `proptest`, `slicer-sdk`, `slicer-runtime` (ADR-0017) | PASS — exit 0; `slicer-scad` present (2 lines); `proptest|slicer-sdk|slicer-runtime` hits 0 (ADR-0017 leaf rule). |
| `cargo tree -p slicer-sdk --no-default-features --target x86_64-pc-windows-msvc -e normal,build,features` | negative: no `proptest`, no OpenRSCAD | PASS — exit 0; `proptest|openrscad|manifold|slicer-scad` hits 0. |
| `cargo tree -p slicer-sdk --features test --target x86_64-pc-windows-msvc -e normal,build,features` | negative: no `proptest`, no OpenRSCAD | PASS — exit 0; `proptest|openrscad|manifold` hits 0. The `test` feature does not activate Proptest. |
| `cargo tree -p slicer-sdk --features property_testing --target x86_64-pc-windows-msvc -e normal,build,features` | positive: `proptest`; negative: no OpenRSCAD | PASS — exit 0; `proptest` present (15 lines); `openrscad|manifold|slicer-scad` hits 0. |
| `cargo tree -p slicer-sdk --features property_testing_scad --target x86_64-pc-windows-msvc -e normal,build,features` | positive control: `proptest` + `slicer-scad` engine | PASS — exit 0; positive control: `proptest` 15 lines and `slicer-scad` 2 lines (engine reachable only when explicitly enabled). |
| `cargo tree -p slicer-scad --no-default-features --target x86_64-pc-windows-msvc -e normal,build,features` | negative: no engine; adapter independent of SDK/runtime | PASS — exit 0; `openrscad|manifold|proptest` hits 0. The root `slicer-scad` package node is the only match; no engine dependency is active. |
| `cargo tree -p slicer-scad --features openrscad --target x86_64-pc-windows-msvc -e normal,build,features` | positive control: engine present; negative: no SDK/runtime/`proptest` | PASS — exit 0; positive control: `openrscad` 11 lines; `proptest|slicer-sdk|slicer-runtime` hits 0. |

**Guests.** `cargo xtask build-guests --list` exited 0 and printed **60** guests. Each was
run with its own manifest, `--target wasm32-unknown-unknown -e normal,build,features`; all 60
exited 0 and every one had **0** hits for `proptest|openrscad|manifold|slicer-scad`. The full
per-guest outputs are under `target/evidence/proptest-rollout/GRAPH/guest-*.txt` and the tab-separated
list at `GRAPH/guests.tsv`. A representative guest was not substituted.

## 10. Scanner and test-quality manual review

**Scope, re-derived from `collect_enforced_files`.** In `xtask/src/check_literals.rs`: crate
`tests/**` and `benches/**` whole-file; crate `src/**` only inside inline `#[cfg(test)]` module
items (`ScanMode::CfgTestOnly`); module `modules/core-modules/*/tests/**` whole-file.
`xtask/src/check_test_quality.rs` walks the same roots (its doc comment: "Same scope as
`check_literals::collect_enforced_files`"). Neither reads `modules/core-modules/*/src/**` or
`xtask/src/**`. A clean scanner result says nothing about those files.

**Blind spots requiring manual review.**

- 22 module `src/**/property_tests.rs` files (glob `modules/core-modules/*/src/**/property_tests.rs`):
  21 at `src/property_tests.rs` in `arachne-perimeters`, `classic-perimeters`, `fuzzy-skin`,
  `gyroid-infill`, `infill-linker`, `layer-planner-default`, `machine-gcode-emit`, `part-cooling`,
  `path-optimization-default`, `rectilinear-infill`, `seam-placer`, `seam-planner-default`,
  `skirt-brim`, `support-surface-ironing`, `top-surface-ironing`, `traditional-support`,
  `traditional-support-planner`, `tree-support`, `tree-support-planner`, `wave-overhangs`,
  `wipe-tower`; plus `lightning-infill/src/kernel/property_tests.rs`. (`overhang-classifier-default`
  uses `tests/property_overhang_speed.rs`, which is scanned.)
- `xtask/src/property_closure_tests.rs` (new) and the other in-`xtask/src` test modules.
- Out-of-line crate-source test module files: `crates/slicer-runtime/src/region_partition/property_tests.rs`
  (declared by `mod property_tests;` in `crates/slicer-runtime/src/region_partition.rs`) — the
  literal scanner does not follow out-of-line declarations (`docs/21_data_defaults_and_fixtures.md` § 7.1).
  `crates/slicer-model-io/src/loader.rs`'s `property_tests` is inline and therefore scanned.
- Macro token trees (`docs/21_data_defaults_and_fixtures.md` § 7.1 Scanner scope, macro range blind spot).

**Manual FRU/waiver pass.** A watched-type scan over module `src/**` property tests (watched = a
`pub struct` with ≥5 named fields under `crates/*/src`; the same rule as
`docs/21_data_defaults_and_fixtures.md` §3) found 58 candidate sites. Fifty-three already used FRU
or a waiver. The remaining five were reviewed by hand; three were false positives and the other two
were real placement/waiver defects:

| Site | Watched type | Finding | Disposition |
|---|---|---|---|
| `planning_objects` (`modules/core-modules/layer-planner-default/src/property_tests.rs`) | `LayerPlanningObject` | Reasoned `// exhaustive:` sat on the line **after** the opening brace, not immediately above the literal | Fixed: marker moved to the line above `.map(|spec| LayerPlanningObject {` |
| `region_at` (same file) | `RegionLayerProposal` | False positive: return type, not a literal | None |
| `model_interpolate` (`modules/core-modules/seam-placer/src/property_tests.rs`) | `Point3WithWidth` | Multi-line waiver whose marker was not the last line before the literal | Fixed: marker placed on the line immediately above |
| `geometry_domain` (`modules/core-modules/top-surface-ironing/src/property_tests.rs`) | `GeometryDomain` (5 fields, `crates/slicer-sdk/src/test_support/property_testing/domains/mod.rs`) | Outer literal had no FRU and no own waiver; the nearby `// exhaustive:` covered only the nested `GeometryBounds` | Fixed: added an outer `// exhaustive:` for the complete topology domain |
| `tree-support-planner` `LayerRecord` literal | local private `LayerRecord` (`modules/core-modules/tree-support-planner/src/lib.rs`) | False positive: 3-field local type, name collision with the watched `slicer_runtime::report::model::LayerRecord` | None |

The same pass also corrected seven further multi-line waiver markers so the `// exhaustive:` line is
immediately above the literal at `classic-perimeters` (`GeometryDomain`, two `GeometryBounds`),
`rectilinear-infill` (`GeometryBounds`, `GeometryDomain`), `seam-placer` (`Point3WithWidth` ×2) and
`traditional-support` (`GeometryDomain`). No FRU was added to a type that already had a reasoned
waiver; no production struct gained a `Default` for test convenience.

**Per-file false-green review.** Each blind-spot file has a predecessor manual review and a closure
re-scan. The closure review fixed or explicitly waived every finding: the lightning-infill
scaffolding/vacuous loop, the gyroid-infill dense population counter, the classic-perimeters
input-derived counters, the infill-linker tautologies, the skirt-brim nesting block, the redundant
self-built selector probes, the path-optimization decorative counters/map mirror, and the
machine-gcode and wipe-tower scene-witness waivers. The layer-planner merge comparison is now an
explicit characterisation pin in its charters, not independent correctness. `CLEAN` means the
file's property closures, oracles, population counters and waivers were inspected and no
production-defect-blind pattern remains; it is not a line-by-line claim beyond those surfaces.

### 10.1 Closure false-green findings

| Severity | File | Finding | Disposition |
|---|---|---|---|
| **Medium** | `layer-planner-default/src/property_tests.rs` | The four `model_*` plane builders transcribe the production merge (`merge_raft_sequences`, `merge_same_height`, `merge_different_heights`, `raft_top`), including the f32 `1e-6` dedup and catch-up guards. `compare_planes` checks the merge against its own copy. | **Resolved by explicit disposition:** `// test-quality:` characterisation-pin waiver added above the model builders; the `M-LAYERS-SEQ`/`M-LAYERS-MERGE` Oracle charters now state the limitation and credit the independent native-top model plus the deterministic A/B, raft, mixed and exact-end witnesses instead. G13 is closed as a recorded limitation, not as independent correctness. |
| Low | `classic-perimeters/src/property_tests.rs` | `mode_counts` and `single_band_cases` counted generated input cases, not nonempty SUT output. | **Fixed:** the counters now increment only inside the `!output.is_empty()` branch. |
| Low | `infill-linker/src/property_tests.rs` | Two terminal assertions (`chain_only + chain_connect + orchestrate == cases`, `queries + empty_graphs > 0`) hold for every successful callback. | **Fixed:** both removed; the deterministic per-mode witnesses and per-case checkers remain. |
| Low ×3 | `path-optimization-default/src/property_tests.rs` | Decorative `witness_*` counters, self-built `.no_shrink()` selector probes with mislabelled waivers, and a probe-bound map mirrored by literal assertions. | **Fixed:** witness counters removed from the assertion loops (still reported); selector probes removed; probe-map literal equality removed while the bound-key presence check remains. The real strategies are covered by `shrink_probe`/`shrink_transitions`. |
| Low | `skirt-brim/src/property_tests.rs` | The `sawtooth` nesting block could never fail and silently skipped a wrong shape. | **Fixed:** on the exact (non-classified-tail) sub-domain the check now asserts `outer > hole` and the band order directly; the classified thin tail remains excluded. |
| Low | `traditional-support-planner/src/property_tests.rs` | `assert_nonshrinking` fed a test-built selector, not the real strategies. | **Fixed:** the redundant probe was removed; the real raster/hole/grid/territory strategies already run through `shrink_probe` in the same test. |
| Low | `tree-support/src/property_tests.rs` | The inline no-shrink block did not cover the real scan/render strategies. | **Fixed:** the redundant block was removed; `probe_shrinking` still runs the real strategies. |
| Low | `tree-support-planner/src/property_tests.rs` | The no-shrink block did not cover the real smooth/roles strategies. | **Fixed:** the redundant block was removed; `probe_tree` still runs the real strategies. |
| Low | `wave-overhangs/src/property_tests.rs` | `assert_no_shrink` ran on test-built selectors, not `wave_strategy`. | **Fixed:** the redundant helper and calls were removed; the per-mode `wave_strategy` shrink loop remains. |
| Low ×2 | `wipe-tower/src/property_tests.rs` | The no-shrink probe did not cover the real strategy; the bed-containment block is built around `tower_rect` and cannot fail on its own. | **Fixed/waived:** the redundant mode-selector probe was removed; the real strategy is covered by the shrink loop, and the bed check carries a `// test-quality:` scene-witness waiver naming the production `run_finalization` bed check. |

All findings are now fixed or explicitly waived; none was a confirmed production defect. The
medium finding is a documented characterisation-pin limitation in the two charters, not a claim of
independent correctness. The final-tree property lanes were re-run after these edits; §5 and §5.1 record the local and hosted
passes.

| File | Predecessor review | Closure finding |
|---|---|---|
| `arachne-perimeters/src/property_tests.rs` | #10 `proptest-arachne-evidence.md` § Manual literal review (AC-8) | CLEAN |
| `classic-perimeters/src/property_tests.rs` | #12 `proptest-walls-evidence.md` § Manual watched-type FRU/waiver review | CLEAN on the FRU pass; 3 waiver markers placed. **FIXED (low):** the mode and single-band counters now count only nonempty SUT output. |
| `fuzzy-skin/src/property_tests.rs` | #12 same | CLEAN (determinism waiver names its surface) |
| `gyroid-infill/src/property_tests.rs` | #13 `proptest-infill-evidence.md` § Manual FRU/test-quality review | **Fixed:** `dense_cases > 0` population assert added |
| `infill-linker/src/property_tests.rs` | #13 same | **FIXED (low):** the two tautological terminal assertions were removed; the per-mode witnesses and per-case checkers remain. |
| `layer-planner-default/src/property_tests.rs` | #15 `proptest-layers-seams-evidence.md` §10 | CLEAN on the FRU pass; 2 waiver markers placed. **WAIVED (medium):** `// test-quality:` characterisation pin; the two charters now state the limitation and credit the independent native-top model plus deterministic witnesses. |
| `machine-gcode-emit/src/property_tests.rs` | #16 `proptest-io-gcode-evidence.md` §8 | **Waived:** `scan_template` is an independent contract model, not a mirror |
| `part-cooling/src/property_tests.rs` | #16 §8 | CLEAN (independent decision table + mutants) |
| `path-optimization-default/src/property_tests.rs` | #15 §10 | **FIXED (low ×3):** witness counters removed from the assertion loops (still reported), redundant selector probes removed, and probe-map literal equality removed while the bound-key presence check remains; the real strategies and mutation controls remain. |
| `rectilinear-infill/src/property_tests.rs` | #13 § Manual FRU/test-quality | CLEAN; 2 waiver markers placed. The spacing oracle is an exact integer form of the canonical rule, anchored by 17 hand-computed rows and a pre-fix control. |
| `seam-placer/src/property_tests.rs` | #15 §10 | CLEAN; 2 waiver markers placed |
| `seam-planner-default/src/property_tests.rs` | #15 §10 | CLEAN |
| `skirt-brim/src/property_tests.rs` | #16 §8 | **FIXED (low):** the nesting/band-order check now asserts `outer > hole` and the band order directly on the exact (non-classified-tail) sub-domain instead of silently skipping. |
| `support-surface-ironing/src/property_tests.rs` | #14 `proptest-support-modules-evidence.md` §9 manual review | CLEAN |
| `top-surface-ironing/src/property_tests.rs` | #13 § Manual FRU/test-quality | **Fixed:** outer `GeometryDomain` waiver added |
| `traditional-support/src/property_tests.rs` | #14 §9 manual review | CLEAN; waiver marker placed |
| `traditional-support-planner/src/property_tests.rs` | #14 §9 manual review | **FIXED (low):** the redundant test-built selector probe was removed; the real raster/hole/grid/territory strategies remain in `shrink_probe`. |
| `tree-support/src/property_tests.rs` | #14 §9 manual review | **FIXED (low):** the redundant inline selector block was removed; `probe_shrinking` still runs the real strategies. |
| `tree-support-planner/src/property_tests.rs` | #14 §9 manual review | CLEAN on the literal/FRU pass (local `LayerRecord` is not the watched type). **FIXED (low):** the redundant selector block was removed; `probe_tree` still runs the real strategies. |
| `wave-overhangs/src/property_tests.rs` | #13 § Manual FRU/test-quality | **FIXED (low):** the redundant test-built selector probe was removed; the per-mode real-strategy shrink loop remains. |
| `wipe-tower/src/property_tests.rs` | #16 §8 | **FIXED/WAIVED (low ×2):** the redundant mode-selector probe was removed; the real strategy remains in the shrink loop, and the bed containment carries a `// test-quality:` scene-witness waiver naming the production `run_finalization` check. |
| `lightning-infill/src/kernel/property_tests.rs` | #11 driver review; closure review | **Fixed and verified:** scaffolding `bound_domain` removed; the simplify/complicate loop now re-expands after each successful `simplify()` and requires a complication. Re-run on the current tree: `test result: ok. 1 passed; 0 failed` (`target/evidence/proptest-rollout/lightning-shrink-control/test-output.log`). |
| `xtask/src/property_closure_tests.rs` (new) | Closure review | CLEAN: both tests drive the real `census_scoped` with the real launcher and assert exact log lines; each has a passing control before its negative |
| `crates/slicer-runtime/src/region_partition/property_tests.rs` (out-of-line) | #8 `proptest-runtime-evidence.md` defect ledger | CLEAN: no new watched-type literal or false-green pattern |

Macro token blind spots remain documented in `docs/21_data_defaults_and_fixtures.md` §7.1; this
packet added no macro-generated watched literal. Gate results for `cargo xtask check-literals` and
`cargo xtask check-test-quality --report`: see §5/§10.2. ADR-0064 and ADR-0065 remain in force;
report-mode findings are neither promoted nor ignored.

The census's ignored-test, duplicate-identity, stale-exemption and zero-target branches are
verified by source review; AC-2 requires the two named negative controls (whole-target removal and
missing registered property), both of which execute the real census. A future fixture for
`stale_property_census_exemption` would make Deviation 1's annotation rule independently
falsifiable; it is recorded here as an optional strengthening, not claimed as executed.

### 10.2 Gate results

| Command | Status | Evidence |
|---|---|---|
| `cargo build --workspace` | PASS — exit 0, 664 s (merged tree) | `target/evidence/proptest-rollout/gates/build.log` |
| `cargo check --workspace --all-targets` | PASS — exit 0, 1247 s (merged tree) | `…/gates/check-all.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0, 186 s (merged tree) | `…/gates/clippy-all.log` |
| `cargo check/clippy -p slicer-sdk --all-targets --features property_testing_scad` | PASS — both exit 0 (50 s / 21 s) | `…/gates/check-sdk-scad.log`, `clippy-sdk-scad.log` |
| `cargo check/clippy -p slicer-core --all-targets --features property_testing_scad,host-algos` | PASS — both exit 0 (44 s / 32 s) | `…/gates/check-core-scad.log`, `clippy-core-scad.log` |
| `cargo check/clippy -p slicer-scad --all-targets --features openrscad` | PASS — both exit 0 (9 s / 2 s) | `…/gates/check-scad.log`, `clippy-scad.log` |
| `cargo check/clippy -p slicer-model-io --all-targets --features scad` | PASS — both exit 0 (7 s / 7 s) | `…/gates/check-model-io-scad.log`, `clippy-model-io-scad.log` |
| `cargo check/clippy -p pnp-cli --all-targets --features scad` | PASS — both exit 0 (331 s / 40 s) | `…/gates/check-pnp-cli-scad.log`, `clippy-pnp-cli-scad.log` |
| `cargo xtask check-literals` | PASS — 0 violations (watchlist 201 types) | `…/gates/check-literals.log` |
| `cargo xtask check-test-quality --report` | PASS — 0 findings | `…/gates/check-test-quality.log` |
| `cargo xtask check-fmt` | PASS — 62 packages clean | `…/gates/check-fmt.log` |
| `cargo xtask build-guests --check` | PASS — exit 0 (fresh; not an infrastructure error) | `…/gates/build-guests-check.log` |
| Post-review fix: `cargo clippy -p lightning-infill --all-targets --features property_testing -- -D warnings` | PASS — exit 0, 14.12 s | `…/gates/clippy-lightning-property.log` |
| Post-review fix: `cargo xtask check-fmt` | PASS — 62 packages clean | `…/gates/check-fmt-post-fix.log` |

The final workspace ceremony (`cargo xtask test --summary --workspace`) is recorded below after the
narrower gates and lanes passed; it does not replace the SCAD/property lanes.

| `cargo xtask test --summary --workspace` | PASS — `VERDICT: PASS`, exit 0 (merged tree) | `target/evidence/proptest-rollout/workspace/console.log`, `workspace/test-output.log` |

## 11. Documentation and contributor handoff

Canonical references: `docs/testing/properties.md` (registry schema, profiles, context/event schema,
execution/replay/failure workflow, every charter); `docs/scad/ingestion.md`;
`docs/scad/openrscad-compatibility.md`; `docs/05_module_sdk.md` § Test Support;
`docs/21_data_defaults_and_fixtures.md`; ADR-0017
(`docs/adr/0017-file-format-io-boundary-separation.md`); ADR-0018
(`docs/adr/0018-region-split-priority-registry-and-canonical-order.md`); DEV-199
(`docs/DEVIATION_LOG.md`).

Features: `slicer-sdk/property_testing` (Proptest, engine-free) and
`slicer-sdk/property_testing_scad` (implies `property_testing`, adds the scripted SCAD surface;
native only). Owner packages declare an empty, default-off `property_testing` (or
`property_testing_scad`) feature.

Adding a property:

1. Write a charter `## <ID>` in `docs/testing/properties.md` with an **independent oracle** (never the
   production function under test) and **non-vacuous witnesses**: mandatory structural cases that the
   generator alone might miss, plus mutation controls the same checker rejects.
2. Place module kernel tests privately in `src/property_tests.rs` behind stacked `#[cfg(test)]` and
   `#[cfg(feature = "property_testing")]` (`docs/05_module_sdk.md`); host tests go in a `tests/`
   target. Name the test `proptest_*`.
3. Register one `[[property]]` record in `test-properties.toml` with exact `package`, `target_kind`,
   `target_name`, full `test_name`, `features` (including `slicer-core/host-algos` where required),
   `production_symbol`/`production_path`, `charter`, and measured `routine`/`extended` profiles. SCAD
   records need a positive `test_process_timeout_ms`. The census fails any discovered `proptest_*`
   without a record and any record without a matching executed test.
4. Measure pilots on Windows before choosing profiles; record cases, bounds, reject/shrink limits,
   isolation, timeout and RNG.
5. Run `cargo xtask test --summary --properties --property <ID> --tier routine` (add `--with-scad` for
   SCAD). Fresh seeds are the default; `--seed <u64>` replays a retained seed.
6. On failure, inspect `target/property-testing/<run_token>/<ID>/` (`context.json`, `case.txt`,
   `observations.jsonl`, `failure.json`). A confirmed defect gets a canonical-correct fix and a **named
   concrete regression** using the concrete input; seeds are diagnostics, never committed history.
7. Review FRU/waivers by hand for any file outside the scanner scope (§10).

## 12. Appendix — artifact paths and provenance method

| Artifact | Path | Producer |
|---|---|---|
| Combined log | `target/test-output.log` (owned by `cargo xtask test`) | `xtask/src/test.rs` |
| Per-run property artifacts | `target/property-testing/<run_token>/<ID>/{context.json,case.txt,observations.jsonl,failure.json}` | `build_property_context` (`xtask/src/properties.rs`); SDK `PropertyRun` |
| Source state | `source-state.json` in the run artifact dir: `source_revision`, `dirty_paths`, `source_state_digest` | `source_state` (`xtask/src/properties.rs`) |
| CI archives | `target/property-ci/extended-core/`, `target/property-ci/extended-scad/`, `target/property-ci/failure-probe/marker.txt` | `properties-extended-windows` (`.github/workflows/ci.yml`) |
| SCAD artifacts | `target/scad-adapter/`, `target/scad-ingest/` | #3 / #5 lanes |
| Closure archive | `target/evidence/proptest-rollout/<alias>/` | this packet (created by the closure runs) |

Digest method: the source-state digest covers the revision, tracked diff bytes and relevant untracked
source/manifest/config bytes, so changing dirty contents changes the digest
(`docs/testing/properties.md` § Context and event schema). Every run records it in `context.json`.

Seeds are diagnostics. Committed regressions are named tests over concrete inputs; source-adjacent
seed persistence is disabled (`failure_persistence: None`) and no `proptest-regressions*` file is
written.
