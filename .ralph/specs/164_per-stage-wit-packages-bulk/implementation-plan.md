# Implementation Plan: 164_per-stage-wit-packages-bulk

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-146c`.
- Use TDD where a step adds/changes a guard, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently.
- **The tree does not compile between Step 2 and the end of Step 6.** That is by design (six layers move together); no step before Step 7 has a compile exit. Do not "fix" a red `cargo check` mid-sequence by touching files outside the current step.

## Steps

### Step 0: Verify 163 landed; inventory the post-163 seams

- Task IDs: `TASK-146c`
- Objective: prove the tree contains 163's exports before any edit, and pin the fixture/doc inventories this packet needs.
- Precondition: clean working tree.
- Postcondition: a ≤15-bullet note recording: (a) `qualified_export_for_stage_id`, `StageGlueKind`, `resolve_stage_glue`, `stage_wit_mtime`, `GuestSpec.stage_id`, and the three pilot `bindgen!` mods all exist (each verified by one `rg` hit); (b) the deviation row owned by `164_per-stage-wit-packages-bulk` — its DEV id + status (re-derived, never assumed); (c) the `LOCATIONS` inventories for `docs/03` tier-world/`wit-world` mentions and for test files writing `wit-world`/calling `.wit_world()`; (d) whether `emit_world_preamble` nests all flat `deps/*.wit` unconditionally.
- Files allowed to read: none directly beyond `rg` output windows (±10).
- Files allowed to edit: none.
- Files explicitly out of bounds: everything else.
- Expected sub-agent dispatches: the four inventory dispatches listed in `design.md` §Expected Sub-Agent Dispatches (docs/03 LOCATIONS; wit-world fixture LOCATIONS; prepass-guest consumer FACT; emit_world_preamble FACT).
- Context cost: `S`
- Authoritative docs: `.ralph/specs/163_per-stage-wit-packages-pilot/design.md` §"Exports handed to packet #3" — direct read.
- OrcaSlicer refs: none.
- Verification: `rg -n 'qualified_export_for_stage_id' crates/slicer-schema/src/lib.rs` returns ≥1 hit; `rg -n '164_per-stage-wit-packages-bulk' docs/DEVIATION_LOG.md` returns exactly one `| DEV-` row.
- Exit condition: all Postcondition bullets recorded; if any (a) probe misses, **STOP — 163 has not landed; this packet cannot start.**

### Step 1: Author the 12 stage packages + prepass-types; delete the two tier worlds

- Task IDs: `TASK-146c`
- Objective: create `crates/slicer-schema/wit/deps/<pkg>/<pkg>.wit` for the 12 packages per `design.md`'s name/content tables (types and signatures moved verbatim; funcs renamed `run`; prepass resources in imported `<iface>-types`; shared views in new flat `deps/prepass-types.wit`), then delete `deps/world-layer/` and `deps/world-prepass/`.
- Precondition: Step 0 exit passed.
- Postcondition: AC-1's command prints `PASS`. Tree does not compile (expected).
- Files allowed to read, with ranges:
  - `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` — whole (30 lines)
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — whole (149 lines)
  - `crates/slicer-schema/wit/deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit` — whole (163's template)
  - `crates/slicer-schema/wit/deps/types.wit` — the `geometry` interface header region only
- Files allowed to edit (creation-heavy, one mechanical family): the 13 new `.wit` files; the two dir deletions.
- Files explicitly out of bounds: everything under `crates/`, `modules/`, `docs/`.
- Expected sub-agent dispatches: none.
- Context cost: `M`
- Authoritative docs: `docs/adr/0045-…` §"Decision", §"The naive shape inverts resource ownership" — ranged.
- OrcaSlicer refs: none.
- Verification: AC-1's python command — FACT PASS/FAIL.
- Exit condition: AC-1 PASS; any `resource` inside an exported interface is a step failure, not a style choice.

### Step 2: slicer-schema columns + guards

- Task IDs: `TASK-146c`
- Objective: fill the 12 rows' `wit_dir`/`wit_package`/`wit_interface`/`wit_world` + `wit_export: "run"`; set the `PrePass::PaintSegmentation` row's five WIT columns to `""` with the host-built-in comment; relax `stage_and_world_lookups_are_consistent` to the "all rows except the documented host-built-in row" shape (TDD: adjust the test first, watch it fail against the old table); correct `WORLD_*` doc comments to tier-vocabulary wording. Do **not** delete `SUPPORTED_WIT_WORLDS` here (Step 8 owns retirement, keeping migration and retirement failures separable).
- Precondition: Step 1 done.
- Postcondition: `cargo test -p slicer-schema` passes (schema is a leaf crate; it compiles even while the workspace is red).
- Files allowed to read:
  - `crates/slicer-schema/src/lib.rs` — `STAGES` region + lookup fns (locate by symbol; ~lines 40-360 pre-163, re-verify)
  - `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` — whole
- Files allowed to edit (≤3): `crates/slicer-schema/src/lib.rs`, `crates/slicer-schema/tests/export_for_stage_id_tdd.rs`, `crates/slicer-schema/src/` unit-test module (same file if inline).
- Files explicitly out of bounds: `host.rs`, `dispatch.rs`, macros.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs: `docs/adr/0006-…` — whole.
- Verification: `cargo test -p slicer-schema 2>&1 | tee target/test-output.log | rg '^test result'` — FACT.
- Exit condition: schema suite green; `rg '"world-(layer|prepass)"' crates/slicer-schema/src/lib.rs` returns nothing.

### Step 3: Macro glue split (12 builders)

- Task IDs: `TASK-146c`
- Objective: expand `StageGlueKind` to 15 per-stage variants; map the 12 stage ids in `resolve_stage_glue` and delete the `Some("LayerModule")`/`Some("PrepassModule")` fallbacks; replace `build_layer_world_glue`/`build_prepass_world_glue` with 12 builders copying `build_postpass_gcode_glue`'s shape (per-package `include_str!`, world names from the table); switch `stage_export`/`__slicer_wit_exports()` metadata to the qualified spelling for all 15 WASM stages.
- Precondition: Step 1 done (package files exist for `include_str!`).
- Postcondition: AC-5's python command prints `PASS` (text-level; compile deferred to Step 7).
- Files allowed to read, with ranges (re-verify all hints post-163):
  - `crates/slicer-macros/src/lib.rs` — `resolve_stage_glue`/`StageGlueKind` region; one pilot builder; `build_layer_world_glue` + `build_prepass_world_glue` bodies; `emit_world_preamble`; the metadata-assembly region. Locate each by `rg -n`, read ±60. **Never whole (~2900 lines).**
  - `crates/slicer-macros/tests/all_worlds_glue_tdd.rs`, `binding_surface_tdd.rs` — matched assertion lines ±10 only.
- Files allowed to edit (≤3): `crates/slicer-macros/src/lib.rs`, `crates/slicer-macros/tests/all_worlds_glue_tdd.rs`, `crates/slicer-macros/tests/binding_surface_tdd.rs`.
- Files explicitly out of bounds: `crates/slicer-macros/src/lib.rs` regions belonging to the three pilot builders (copy, don't edit).
- Expected sub-agent dispatches: Step 0's `emit_world_preamble` FACT consumed here (if flat deps are not nested unconditionally, add `prepass-types.wit` to the nest list — one edit, inside lib.rs).
- Context cost: `M`
- Authoritative docs: 163 `design.md` §"Exports handed to packet #3" (glue template contract) — direct read.
- Verification: AC-5's python command — FACT.
- Exit condition: AC-5 PASS; `rg -c 'benign' crates/slicer-macros/src/lib.rs` shows no new padding-arm comments in the 12 builders.

### Step 4: Host bindgen — 12 mods, canonical `with:` definer moves to `layer_perimeters`

- Task IDs: `TASK-146c`
- Objective: replace `pub mod layer` / `pub mod prepass` with the 12 per-stage `bindgen!` mods (five-key `with:` block each); make `layer_perimeters` the defining mod for the five dep interfaces and re-point **all** other mods' `with:` values (including 163's three pilot mods) at its generated paths; define `slicer:prepass-types/prepass-types` in `prepass_seam_planning` and alias it in `prepass_support_geometry`; re-point the prepass resource `impl Host*` blocks and any layer-type helper paths at the new mods.
- Precondition: Step 1 done.
- Postcondition: AC-3's python command prints `PASS`; `cargo check -p slicer-wasm-host` may still be red only for `dispatch.rs` references (Step 5's surface), not for `host.rs` itself.
- Files allowed to read, with ranges:
  - `crates/slicer-wasm-host/src/host.rs` — the `layer`/`prepass` mods, one pilot mod, the prepass/layer resource impl regions; locate by `rg -n 'pub mod |impl .*Host'`, read ±80. **Never whole (~4100 lines).**
- Files allowed to edit (≤3): `crates/slicer-wasm-host/src/host.rs` (single file; large but one surface).
- Files explicitly out of bounds: `dispatch.rs` (Step 5), everything else.
- Expected sub-agent dispatches: none.
- Context cost: `M`
- Authoritative docs: `docs/adr/0002-…` — whole.
- Verification: AC-3's python command — FACT; `cargo check -p slicer-wasm-host 2>&1 | rg -c 'host\.rs' || true` as a bounded error probe (errors localized to dispatch.rs are expected and deferred).
- Exit condition: AC-3 PASS. A `wrong type` linker/compile error naming a dep interface means a `with:` path mismatch — fix the path, never fork the type.

### Step 5: Dispatch rewiring (layer + prepass)

- Task IDs: `TASK-146c`
- Objective: restructure `dispatch_layer_call`/`call_layer_export` and `dispatch_prepass_call`/its router so the stage match selects the per-stage world (per-arm `add_to_linker` + `instantiate` + interface-accessor `call_run`), moving each arm's existing marshalling body unmodified; add 163's qualified-export `reason` to every `TypedInstantiation` arm; leave the `MissingComponent` conversion arms (DEV-087) byte-untouched.
- Precondition: Step 4 done.
- Postcondition: `dispatch.rs` has zero `host::LayerModule`/`host::PrepassModule` (AC-4's grep half).
- Files allowed to read, with ranges:
  - `crates/slicer-wasm-host/src/dispatch.rs` — `dispatch_layer_call` (head at ~`:246` pre-163 — re-verify), `call_layer_export`, `dispatch_prepass_call` (~`:662`), plus `dispatch_postpass_gcode_call` as the pilot pattern; locate by symbol, read ±80. **Never whole (~2500 lines).**
- Files allowed to edit (≤3): `crates/slicer-wasm-host/src/dispatch.rs`.
- Files explicitly out of bounds: the pilot runners (pattern source only), `binding.rs`, `pool.rs`.
- Expected sub-agent dispatches: none.
- Context cost: `M` (largest step)
- Authoritative docs: ADR-0045 §"Why this works" (the seam note: "dispatch.rs already does `match stage_id.as_str()`") — ranged.
- Verification: AC-4's python grep half — FACT (test halves deferred to Step 7+).
- Exit condition: grep PASS and every new `TypedInstantiation` arm includes `qualified_export_for_stage_id`.

### Step 6: Test guests + xtask compile closure

- Task IDs: `TASK-146c`
- Objective: retarget the four hand-written guests (`prepass-guest` per Step 0's consumer FACT — narrow or split, mirroring 163's `postpass-guest` precedent; `layer-infill-guest`; `infill-postprocess-echo-guest`; `path-optimization-multi-read`) to their per-stage worlds with interface-grouped `Guest` impls and `fn run`.
- Precondition: Steps 1-5 done.
- Postcondition: guest sources reference no tier world.
- Files allowed to read: the four guests' `src/lib.rs` (whole — each small); 163's retargeted `postpass-guest/src/lib.rs` as template.
- Files allowed to edit (≤4 small files — one mechanical family): the four guests' `src/lib.rs`.
- Files explicitly out of bounds: `sdk-*` guests and core-module sources (macro regenerates them).
- Expected sub-agent dispatches: none (consumer FACT already held).
- Context cost: `S`
- Authoritative docs: none new.
- Verification: `rg -n 'world-layer|world-prepass' crates/slicer-wasm-host/test-guests/*/src/lib.rs` returns nothing — FACT.
- Exit condition: grep clean.

### Step 7: Compile gate + full guest rebuild + executor/baseline green

- Task IDs: `TASK-146c`
- Objective: bring the workspace green: fix residual compile fallout **within the files already edited in Steps 2-6 plus test files that reference renamed paths** (delegate the error inventory first); then `cargo xtask build-guests` (full rebuild — this packet invalidates every guest), then the executor suite, dispatch contract tests (add AC-N1's layer case to `stage_miss_is_fatal_at_instantiation` — TDD it in first), and the AC-N4 baseline.
- Precondition: Steps 1-6 done.
- Postcondition: `cargo check --workspace --all-targets` passes; AC-4, AC-6, AC-N1, AC-N4 commands print PASS/green.
- Files allowed to read: compile-error windows (±20) only; `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs` — `stage_miss_is_fatal_at_instantiation` ±60.
- Files allowed to edit (≤3 per fix iteration): the erroring file, `dispatch_protocol_tdd.rs`, plus one test file per iteration.
- Files explicitly out of bounds: anything not named in a compile error or an AC.
- Expected sub-agent dispatches: `cargo check --workspace --all-targets` FACT (+SNIPPETS on failure); `cargo xtask build-guests` FACT (STALE list / success).
- Context cost: `M`
- Authoritative docs: `CLAUDE.md` §"Guest WASM Staleness" — direct.
- Verification: AC-4 full command; AC-6; AC-N1; AC-N4 — each FACT.
- Exit condition: all four PASS. A red `perimeter_parity`/`legacy_zero` here is caused by this packet — diagnose, never regolden.

### Step 8: `wit-world` retirement

- Task IDs: `TASK-146c`
- Objective: delete the key end to end — `manifest.rs` field/accessor/builder/parse + `validate_wit_world`; `SUPPORTED_WIT_WORLDS` in `slicer-schema`; the 20 manifest lines (sweep; one line each); `module_new.rs` scaffold + `wit_world_for_stage` + `wit_world_mapping` test; replace `wit_world_mismatch_rejects_invalid_package_name` and `versioned_wit_world_is_rejected_with_actionable_diagnostic` (the real names in `manifest_ingestion_tdd.rs` — docs/07's TASK-146 row carries stale ones) with `wit_world_key_is_ignored` (TDD: write it first against the old parser — it fails on `required_string` — then delete the parse); fix the fixture helpers from Step 0's LOCATIONS inventory.
- Precondition: Step 7 green (so retirement fallout is cleanly attributable).
- Postcondition: AC-7 and AC-N3 commands print PASS.
- Files allowed to read: `crates/slicer-scheduler/src/manifest.rs` — `rg -n 'wit_world'` windows ±20; `crates/pnp-cli/src/module_new.rs` — scaffold + `wit_world_for_stage` regions; fixture files from the inventory, matched lines ±10.
- Files allowed to edit: `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-schema/src/lib.rs`, `crates/pnp-cli/src/module_new.rs`, plus two mechanical sweep families with per-file one-line contracts: the 20 `modules/core-modules/*/*.toml` (delete the `wit-world` line) and the fixture/test files inventoried in Step 0 (drop `wit_world` params/asserts).
- Files explicitly out of bounds: dispatch/host/macros (done), WIT files.
- Expected sub-agent dispatches: `cargo check --workspace --all-targets` FACT after the sweep.
- Context cost: `M`
- Authoritative docs: ADR-0044 — delegated SUMMARY.
- Verification: AC-7 command; AC-N3 command — FACT each.
- Exit condition: both PASS; `rg -l 'wit-world' modules/ crates/` returns nothing outside docs.

### Step 9: Contract guards + binding tests + isolation evidence

- Task IDs: `TASK-146c`
- Objective: update `wit_single_source_tdd.rs` / `wit_drift_detection_tdd.rs` to the 15-world surface (AC-9); sweep the core-module `slicer_module_binding_tdd.rs` expectations — derive the file set at point of use with `ls modules/core-modules/*/tests/slicer_module_binding_tdd.rs` (14 layer/prepass files at authoring time; per-file: `__slicer_stage_export_name() == "run"`, qualified `stage_export` per the name table) — and **create** the missing `modules/core-modules/arachne-perimeters/tests/slicer_module_binding_tdd.rs` (copy `classic-perimeters`' shape; same stage, same expected strings); run xtask staleness tests; run AC-N2's isolation script (the ADR's headline claim, on the motivating tier).
- Precondition: Step 8 done.
- Postcondition: AC-2, AC-9, AC-N2 commands print PASS; `cargo test -p xtask -- stage_wit` green.
- Files allowed to read: the two contract test files (whole — they are the edit surface); one binding test as the sweep template.
- Files allowed to edit: the two contract test files + the binding-test sweep family (the `ls`-derived set, 14 at authoring time, plus the new arachne-perimeters file).
- Files explicitly out of bounds: `xtask/src/build_guests.rs` (163's; must pass unchanged — if it doesn't, the bug is in this packet's schema rows, not in xtask).
- Expected sub-agent dispatches: AC-N2's script as a FACT dispatch (it rebuilds guests twice; bounded output).
- Context cost: `M`
- Authoritative docs: none new.
- Verification: AC-2; AC-9; AC-N2; `(cargo test -p xtask -- stage_wit 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo FAIL`; `(cargo test -p arachne-perimeters --test slicer_module_binding_tdd 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: new arachne binding test absent or 0 ran'` — FACT each.
- Exit condition: all PASS.

### Step 10: Docs, deviation closure, backlog

- Task IDs: `TASK-146c`
- Objective: execute `packet.spec.md` §Doc Impact (docs/03 restructure incl. deleting the fictional `run-paint-segmentation` listing and the `wit-world` sections; CONTEXT.md Module tier / Stage contract rewrites + Stage interface "not yet implemented" drop; wit README Layout; docs/07 TASK-146c row); close the deviation row per AC-8 (re-derive its ID first).
- Precondition: Steps 1-9 green (docs describe a landed state).
- Postcondition: every Doc Impact grep passes; AC-8 PASS.
- Files allowed to read: `CONTEXT.md` — the three glossary entries ±10 each; `crates/slicer-schema/wit/README.md` — whole (short); `docs/03_wit_and_manifest.md` — only the ranges from Step 0's LOCATIONS; `docs/DEVIATION_LOG.md` — the owned row only.
- Files allowed to edit: `docs/03_wit_and_manifest.md`, `CONTEXT.md`, `crates/slicer-schema/wit/README.md`, `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` (docs family; each edit anchored by its grep).
- Files explicitly out of bounds: all code.
- Expected sub-agent dispatches: docs/07 update via worker dispatch, never a full backlog read; deviation-row FACT re-derivation.
- Context cost: `S`
- Authoritative docs: `CLAUDE.md` §"Ledger Facts Must Be Re-derived, Not Quoted" — direct.
- Verification: every grep in §Doc Impact; AC-8's command — FACT each.
- Exit condition: all greps + AC-8 PASS.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | inventories via dispatch |
| Step 1 | M | 13 WIT files, verbatim moves |
| Step 2 | S | leaf crate, compiles independently |
| Step 3 | M | ranged reads in a ~2900-line file |
| Step 4 | M | `with:` definer move is the risk center |
| Step 5 | M | largest step; marshalling arm moves |
| Step 6 | S | 4 small guest files |
| Step 7 | M | compile gate + full guest rebuild |
| Step 8 | M | wide-but-shallow sweep |
| Step 9 | M | guards + 15-file sweep + isolation script |
| Step 10 | S | docs + closure |

Aggregate `M`. No step is L; Steps 4-5 are the ones to watch — if either exceeds its band, split at the layer/prepass boundary (do layer, gate, then prepass).

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Deviation row closed (AC-8); no reopened/superseded transitions pending.
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and gate command.
- `cargo xtask test --summary --workspace` dispatched to a sub-agent, `FACT` pass/fail + failing names only (CLAUDE.md §Test Discipline; the guest-freshness gate must run — `xtask test`, not bare `cargo test`).
- Record remaining packet-local risk (expected: none beyond DEV-087, which is untouched by design).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where applicable so test, bench, and example targets compile.
