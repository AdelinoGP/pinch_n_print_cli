# Implementation Plan: 238a-support-pattern-config-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never
  write "see Step N".
- E4/T4: run `cargo xtask build-guests --check` before attributing any guest-facing
  failure; this packet's surface feeds guest builds (`modules/**`, `crates/slicer-ir/**`).
- Invariant 16/T2: every verification asserts a non-zero matched-test count in-run
  (`test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` after tee).
- E6/T5: every slicer-core test invocation carries `--features host-algos`.
- T8: every manifest `[config.schema]` change and its
  `cargo xtask gen-config-docs && cargo xtask gen-config-docs --check` land in the SAME
  step/commit.
- Tee every cargo test to `target/test-output.log`; read the file, never re-run.

## Steps

### Step 1: Red guest-dispatch tests for the undeclared tree-planner keys

- Task IDs: `TASK-363`
- Objective: author failing tests in the runtime executor bucket proving that
  `max_bridge_length`, `support_branch_merge_distance_mm`, and
  `support_max_branches_per_layer` supplied through config actually reach the compiled tree
  planner today (`max_bridge_length_config_reaches_tree_planner` is the AC-2 name; the
  merge-distance/branch-cap twins document current silent-default behavior). Tests assert
  observable plan deltas (e.g. sample spread under a short bridge length), not artifact
  existence. They stay red until Step 2 declares the keys. The new file is registered in
  `crates/slicer-runtime/tests/executor/main.rs` via
  `mod support_config_surface_tdd;` (the bucket is one `[[test]]` binary; unregistered
  files silently compile to nothing).
- Precondition: clean tree on `parity/support-planners-clean`;
  `cargo xtask build-guests --check` exit 0.
- Postcondition: named tests exist and fail for the right reason (config value dropped by
  the filtered view); zero unrelated failures.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs` -
    lines 200–320 only (the `default_planner_config_map` injector pattern)
  - `crates/slicer-runtime/tests/executor/main.rs` - lines 80–108 only (the `mod`
    registration block)
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines 95–160 (constants) and
    ~3570–3610 (`sample_contact_points`, verified ~:3587) only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/support_config_surface_tdd.rs` (new file)
  - `crates/slicer-runtime/tests/executor/main.rs` (add
    `mod support_config_surface_tdd;` to the registration block)
- Files explicitly out of bounds:
  - all manifests this step; all `src/` production code; other crates
- Expected sub-agent dispatches:
  - Question: confirm the executor bucket's `main.rs` registration convention (plain
    `mod <file>;` lines, no `path` attribute) before adding the module line; scope:
    `crates/slicer-runtime/tests/executor/main.rs`; return: `FACT`; purpose: register
    `mod support_config_surface_tdd;` correctly.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` - row G-16 (small; direct read)
- OrcaSlicer refs:
  - none this step (no canonical read needed for red plumbing tests)
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test executor support_config_surface_tdd::max_bridge_length_config_reaches_tree_planner -- --exact 2>&1 | tee target/test-output.log && grep -cE "^test .+ FAILED|^error\[E" target/test-output.log` -
    expect ≥1 (red state proven)
- Exit condition: log shows exactly the authored test failing; no collateral failures;
  `cargo check -p slicer-runtime --all-targets` clean.

### Step 2: Tree-planner manifest declarations (G-16/div 3.1)

- Task IDs: `TASK-364`
- Objective: add the four `[config.schema]` tables to
  `modules/core-modules/tree-support-planner/tree-support-planner.toml` —
  `support_branch_merge_distance_mm` (float, default 0.8, min 0), 
  `support_max_branches_per_layer` (int, default 1024, min 1, max 10000),
  `max_bridge_length` (float, default 10.0, min 0), `support_style` (enum with declared
  `values = ["default", "grid", "snug", "organic", "tree_slim", "tree_strong",
  "tree_hybrid"]` — the complete canonical `s_keys_map_SupportMaterialStyle` set including
  `tree_strong` and `tree_hybrid`, default `"default"`) — regenerate
  `docs/15_config_keys_reference.md`, and turn Step 1's red test green.
  `support_style` is declare-only (behavior is 238b); its comment cites canonical
  `SupportParameters.hpp`'s style normalization and `TreeSupport.cpp`'s `is_strong`
  (`is_tree && style == smsTreeStrong`) as the behavior surface 238b will key on.
  Additionally wire the ONE missing consumer: add a `config.get("max_bridge_length")`
  read beside `DEFAULT_MAX_BRIDGE_LENGTH_MM` in `src/lib.rs`. Manifest declaration alone
  cannot satisfy AC-2's override semantics — no read site exists today (verified at
  preflight, 2026-08-24): without the added read, a config-supplied value would still be
  silently ignored.
- Precondition: Step 1 landed (red tests exist); blast-radius dispatch completed.
- Postcondition: AC-1 greps pass (all four tables individually, `tree_strong`/`tree_hybrid`
  present in the declared value set); AC-2 command PASSes with the reader wired
  (`config.get("max_bridge_length")` present in `src/lib.rs`); historical injectors (AC-N4
  suite) still green; gen-config-docs `--check` exits 0.
- Blast-radius discipline (mandatory — manifest shape changes):
  - Dispatch a `LOCATIONS` worker BEFORE editing for every test/fixture asserting this
    manifest's table set or count (`manifest` greps under `crates/**/tests`,
    `xtask/src/gen_config_docs.rs` expectations, any golden TOML dumps). Cite the result
    inline in this step's working notes; every surfaced site is fixed HERE.
  - Known injectors to keep green (verified live): 
    `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs`
    (`default_planner_config_map`: 0.8 / 1024),
    `crates/slicer-runtime/tests/integration/support_geometry_config_normalization_tdd.rs`
    (`base_config`: 0.8 / 1024),
    `modules/core-modules/tree-support-planner/tests/diagnostics_tdd.rs`
    (`support_max_branches_per_layer = 1024`). All values in-bounds; expected fallout zero,
    but prove it with AC-N4 rather than assume it.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` - full (188
    lines)
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines 95–160 only
    (constants region; the `DEFAULT_MAX_BRIDGE_LENGTH_MM` edit site ~:103)
  - `docs/15_config_keys_reference.md` - generated; regenerate, never hand-edit
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml`
  - `docs/15_config_keys_reference.md` (regenerated output only)
  - `modules/core-modules/tree-support-planner/src/lib.rs` (single-site
    `max_bridge_length` read; see Objective)
- Files explicitly out of bounds:
  - `modules/core-modules/tree-support-planner/src/lib.rs` reader BODIES beyond the one
    `max_bridge_length` read site above (planner behavior is 238b). The existing readers
    for `support_line_width`, `support_max_branches_per_layer`, and `support_style`
    already accept those keys; `max_bridge_length` uniquely has NO read site today —
    that one read is in scope here.
- Expected sub-agent dispatches:
  - Question: manifest-shape assertions inventory (see blast-radius discipline); return:
    `LOCATIONS` ≤20.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §13 T8, §12 brief item
    "G-16 + divergence 3.1"
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `init_fff_params`
    (`max_bridge_length` coFloat default 10) and `s_keys_map_SupportMaterialStyle`; delegate
- Verification:
  - AC-1 grep command (packet.spec.md) prints PASS
  - AC-2 command prints PASS; AC-N4 command prints PASS
  - `rg -q 'config\.get\("max_bridge_length"\)' modules/core-modules/tree-support-planner/src/lib.rs && echo PASS || echo FAIL` -
    reader wired (AC-2 override path real)
  - `mkdir -p target && cargo xtask build-guests --check; echo "exit=$?"` — record exit
    code; if 1, rebuild without `--check` before proceeding (T4: manifest edits feed the
    fingerprint closure)
- Exit condition: all three PASS lines plus recorded fresh-or-rebuilt guests.

### Step 3: Traditional-planner pattern-spacing declaration (G-03 key half)

- Task IDs: `TASK-365`
- Objective: declare `support_base_pattern_spacing` (float, default 2.5, min 0.1, max 10)
  in `traditional-support-planner.toml`; extend the `support_base_pattern` comment block to
  name the canonical value set `default|rectilinear|rectilinear-grid|honeycomb|lightning|hollow`
  (canonical `s_keys_map_SupportMaterialPattern`) and note the reference profile uses
  rectilinear at spacing 2. Regenerate doc 15 in the same commit.
- Precondition: Step 2 merged (doc-regen tooling warm); blast-radius dispatch for THIS
  manifest's shape assertions completed.
- Postcondition: AC-6 greps pass; manifest parses; doc 15 carries the key.
- Blast-radius discipline: same LOCATIONS sweep scoped to
  `traditional-support-planner` shape assertions before editing.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` -
    full (<120 lines)
  - `modules/core-modules/traditional-support-planner/src/lib.rs` - lines 30–130 only
    (`DEFAULT_BASE_PATTERN`, `from_config` reads)
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`
  - `docs/15_config_keys_reference.md` (regenerated)
- Files explicitly out of bounds:
  - pattern-generator implementation (renderer half belongs to 238c); planner algorithm
    bodies
- Expected sub-agent dispatches:
  - Question: confirm no consumer needs `support_base_pattern_spacing` wired module-side
    TODAY beyond declaration (the renderer half is 238c); scope:
    `modules/core-modules/traditional-support-planner/src/lib.rs`; return: `FACT`;
    purpose: keep this step declare-only honestly.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 brief item G-03
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `init_fff_params`
    (`support_base_pattern_spacing` coFloat 2.5),
    `s_keys_map_SupportMaterialPattern`; delegate
- Verification:
  - AC-6 command prints PASS
  - `mkdir -p target && cargo run --package pnp-cli -- module config-schema --module-dir modules/core-modules 2>/dev/null | rg -q 'support_base_pattern_spacing' && echo PASS || echo FAIL` -
    proves the declared key surfaces through the CLI schema view
- Exit condition: PASS lines; `cargo xtask gen-config-docs --check` exit 0.

### Step 4: Eleven typed host keys incl. line-width retype (G-04/G-05/G-08 host half)

- Task IDs: `TASK-366`
- Objective: add to `declare_resolved_config!` (`crates/slicer-ir/src/resolved_config.rs`)
  the typed host fields: `support_expansion` (f32 0.0), `support_top_z_distance` (f32 0.2),
  `support_bottom_z_distance` (f32 0.2), `support_threshold_overlap` (float_or_percent,
  default 50 percent), `support_line_width` (float_or_percent, default 0 = auto),
  `bridge_no_support` (bool false), `enforce_support_layers` (int 0),
  `support_critical_regions_only` (bool false), `support_remove_small_overhang` (bool
  true), `support_object_first_layer_gap` (f32 0.2), `support_sharp_tails` (bool
  true). Route them through
  `docs/config/host-keys.toml`. Retype `support_line_width` float→float_or_percent in BOTH
  declaring manifests (tree-support-planner, traditional-support-planner if present there)
  keeping plain-mm accepted; migrate the tree-planner reader
  (`SupportPlanner::from_config`) to percent-aware resolution where the sdk ConfigView
  offers it. Record the divergence-5.4 decision comment at the field site.
- Precondition: Steps 2–3 merged. Percent-transport precedent understood via SNIPPETS
  dispatch (`classic-perimeters` float_or_percent + `ConfigView::get_abs_value`).
- Postcondition: AC-3 greps pass; `ResolvedConfig::default()` carries the eleven values;
  existing percent machinery compiles clean workspace-wide.
- Blast-radius discipline (mandatory — macro-generated struct gains fields):
  - `ResolvedConfig` literals are covered by the macro's `Default`-based construction and
    FRU-rest fixtures per `docs/21_data_defaults_and_fixtures.md`; STILL dispatch
    `LOCATIONS` for exhaustive `ResolvedConfig { ... }` literal sites outside slicer-ir
    (production stays exhaustive by policy — enumerate them) and for equality/bitwise
    assertion helpers (`to_bits` chains) that must grow the new fields. Fix all surfaced
    sites in THIS step.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines 840–1010 (macro invocation) and
    1040–1110 (equality impl) only
  - `docs/config/host-keys.toml` - full
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines 1390–1445 only
    (`from_config` width read)
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` (line-width
    retype) — if the sweep shows the retype cannot share the commit cleanly, split the
    manifest edit into Step 5a's budget instead of exceeding 3 files here
- Files explicitly out of bounds:
  - consumer behavior wiring (`overhang_annotation.rs` already consumes expansion;
    producer plumbing exists) — Steps 5a/5b touch consumers only for de-hardcoding
- Expected sub-agent dispatches:
  - Question: float_or_percent extractor mechanics + exhaustive-literal sites (see blast
    radius); scope: `crates/slicer-ir/src/resolved_config.rs`, `crates/**` producers;
    return: `SNIPPETS` ≤30 + `LOCATIONS` ≤20.
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - struct-literal churn gate (ranged read)
  - `docs/ORCA_CONFIG_REFERENCE.md` - rows for the eleven keys (ranged lookups)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `init_fff_params` (all eleven);
    `Flow.cpp` - `auto_extrusion_width` / `opt_key_to_flow_role` (auto-width semantics
    behind default 0); delegate
- Verification:
  - AC-3 command prints PASS
  - `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tail -5` — clean
  - `mkdir -p target && cargo test -p slicer-ir --lib resolved_config 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` -
    resolver tests live in the owning crate slicer-ir; the E6 host-algos form applies only to slicer-core targets
  - `mkdir -p target && cargo xtask build-guests --check; echo "exit=$?"` — slicer-ir edits
    stale ALL guests; rebuild without `--check` before any later test run (T4)
- Exit condition: AC-3 PASS; workspace check clean; guests rebuilt fresh; divergence-5.4
  decision comment present at the `support_line_width` field.

### Step 5a: De-hardcode support distances in the geometry builtin (G-05)

- Task IDs: `TASK-367` (first part)
- Objective: in `execute_support_geometry` (`crates/slicer-core/src/algos/
  support_geometry.rs`) replace `DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM` and the
  `support_layer_height_mm: 0.0` literal with resolved region values (the region's
  `resolved_config` is already reachable per-`ActiveRegion`; plumb the resolved top-z
  distance through the builtin caller chain in
  `crates/slicer-runtime/src/builtins/support_geometry_producer.rs`).
- Precondition: Step 4 merged (typed fields exist).
- Postcondition: no remaining hardcoded support-distance literals in
  `support_geometry.rs`; resolved values flow through the caller chain.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/support_geometry.rs` - full (~340 lines)
  - `crates/slicer-runtime/src/builtins/support_geometry_producer.rs` - full (~60 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/support_geometry.rs`
  - `crates/slicer-runtime/src/builtins/support_geometry_producer.rs`
- Files explicitly out of bounds:
  - planner bottom-z semantics (238b/238c); WIT; marshal legs (Step 6)
- Expected sub-agent dispatches:
  - Question: does any other caller invoke `execute_support_geometry` beyond
    `commit_support_geometry_builtin`; scope: `crates/**`; return: `LOCATIONS` ≤10;
    purpose: complete caller plumbing.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 brief item G-05
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `init_fff_params`
    (`support_top_z_distance`/`support_bottom_z_distance` coFloat 0.2); delegate
- Verification:
  - `rg -n "DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM|support_layer_height_mm: 0\.0" crates/slicer-core/src/algos/support_geometry.rs || echo PASS` -
    literals gone
- Exit condition: literal sweep echo PASS.

### Step 5a-2: New gated target carrying AC-4 (G-05 proof)

- Task IDs: `TASK-367` (second part)
- Objective: create the NEW host-algos-gated test target
  `support_geometry_config_surface_tdd`
  (`crates/slicer-core/tests/support_geometry_config_surface_tdd.rs` + its
  `required-features = ["host-algos"]` `[[test]]` registration in
  `crates/slicer-core/Cargo.toml`) and author + pass
  `support_geometry_ir_carries_resolved_distances` (AC-4), red first against the
  pre-5a tree state, green after.
- Precondition: Step 5a merged (literals gone).
- Postcondition: AC-4 PASS under `--features host-algos`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/algo_support_geometry_tdd.rs` - lines 1–110 only
    (fixture-builder pattern to mirror)
  - `crates/slicer-core/Cargo.toml` - `[[test]]` block region only
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/support_geometry_config_surface_tdd.rs` (new)
  - `crates/slicer-core/Cargo.toml` (`[[test]]` registration)
- Files explicitly out of bounds:
  - production sources (Step 5a owned them); WIT; marshal legs
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 brief item G-05
- OrcaSlicer refs: none (in-tree assertion).
- Verification:
  - AC-4 command prints PASS
- Exit condition: AC-4 PASS line; target registered in Cargo.toml.

### Step 5b: Source the serializer's support width from config (G-08 consumer)

- Task IDs: `TASK-367` (third part)
- Objective: in `DefaultGCodeSerializer` (`crates/slicer-gcode/src/serialize.rs`) source
  `support_line_width` from resolved config (delete the 0.35 hardcode; the resolved
  float_or_percent value is resolved to absolute mm against `nozzle_diameter` upstream,
  auto = nozzle per the divergence-5.4 mapping) and replace the dead
  `("support_bottom_z_distance", "0.2")` config-block table literal with the resolved
  value. Author + pass `support_line_width_header_sources_resolved_value` (AC-7) in the
  file's existing `#[cfg(test)]` module, red first.
- Precondition: Step 4 merged. LOCATIONS dispatch for serializer construction sites done
  (design dispatch list).
- Postcondition: AC-7 PASS; the 0.35 hardcode and the dead bottom-z literal are gone.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` - lines 60–130, 275–310, 500–530, 680–710,
    860–980 only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/serialize.rs`
  - the single construction site the dispatch names (expected under
    `crates/slicer-runtime/src/` — pass the resolved width through the existing
    builder-method pattern, e.g. a `with_support_line_width` chain method)
- Files explicitly out of bounds:
  - flow-model work (deviation recorded, not implemented); marshal legs (Step 6)
- Expected sub-agent dispatches:
  - Question: who constructs `DefaultGCodeSerializer` and what resolved config is in scope
    there; scope: `crates/slicer-runtime/src/**`, `crates/pnp-cli/src/**`; return:
    `LOCATIONS` ≤20; purpose: wire width sourcing without breaking constructors.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 brief item G-08
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` - `auto_extrusion_width(frSupportMaterial,
    nozzle_diameter)` and `opt_key_to_flow_role` (the mapping the auto sentinel encodes);
    delegate
- Verification:
  - AC-7 command prints PASS
  - `rg -n "support_line_width: f32 = 0\.35|support_bottom_z_distance\", \"0\.2" crates/slicer-gcode/src/serialize.rs || echo PASS` -
    literals gone
- Exit condition: AC-7 PASS line; literal sweep echo PASS.

### Step 6: One canonical layer-height transport rule (G-09)

- Task IDs: `TASK-367` (fourth part)
- Objective: extract `canonical_effective_layer_height(plan, global_index)` (MAX across
  participating objects at the index, identical tie-break to the wasm leg today) into the
  marshal module; switch `project_layer_plan_view` (`in_.rs`) onto it (behavior-preserving)
  then flip `build_native_prepass_request` (`native.rs`) onto it (first-match branch
  deleted). RC-11 untouched.
- Precondition: Step 5a merged (5b may land in parallel — disjoint files); FACT dispatch
  confirming no other first-match duplicates.
- Postcondition: exactly ONE derivation site remains (grep-countable).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - lines 60–100 only
  - `crates/slicer-wasm-host/src/marshal/native.rs` - lines 270–295 only
  - `crates/slicer-wasm-host/src/marshal/mod.rs` - full (helper home)
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/marshal/mod.rs`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
  - `crates/slicer-wasm-host/src/marshal/native.rs`
- Files explicitly out of bounds:
  - consumers of `effective_layer_height` (RC-11 walk-Z sites stay as-is); test files
    (Step 6-2 owns them)
- Expected sub-agent dispatches:
  - Question: any other layer-height derivation besides the two cited; scope:
    `crates/slicer-wasm-host/src/marshal/**`; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/spec_packets/224-support-family-orca-closure/design.md` - §RC-11 only
- OrcaSlicer refs:
  - none (transport-internal fix)
- Verification:
  - `rg -c "effective_layer_height.*max_by|find\(\|reference\| reference.global_layer_index == layer.index" crates/slicer-wasm-host/src/marshal/native.rs || echo CLEAN` -
    first-match derivation gone
- Exit condition: derivation-site count is exactly one (mod.rs helper).

### Step 6-2: Contract test for the canonical layer height (G-09 proof)

- Task IDs: `TASK-367` (fifth part)
- Objective: create
  `crates/slicer-wasm-host/tests/contract/layer_height_transport_tdd.rs`, register it in
  `crates/slicer-wasm-host/tests/contract/main.rs` via `mod layer_height_transport_tdd;`
  (the contract bucket is one `[[test]]` binary; unregistered files silently compile to
  nothing), and author + pass
  `native_and_wasm_layer_views_share_canonical_layer_height` (AC-5), red first against a
  pre-Step-6 native leg, green after.
- Precondition: Step 6 merged (helper exists on both legs).
- Postcondition: AC-5 PASS; contract test registered.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/tests/contract/main.rs` - lines 1–35 only (mod block)
  - one existing contract test file as pattern (reviewer-named at dispatch time)
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/tests/contract/layer_height_transport_tdd.rs` (new)
  - `crates/slicer-wasm-host/tests/contract/main.rs` (`mod` registration)
- Files explicitly out of bounds:
  - marshal production sources (Step 6 owned them)
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/spec_packets/224-support-family-orca-closure/design.md` - §RC-11 only
- OrcaSlicer refs: none (transport-internal fix)
- Verification:
  - AC-5 command prints PASS
  - `rg -q '^mod layer_height_transport_tdd;' crates/slicer-wasm-host/tests/contract/main.rs && echo PASS || echo FAIL` -
    registration present
- Exit condition: AC-5 PASS; registration grep PASS.

### Step 7: Bounds negatives + cross-leg regression net (invariant-16 sweep)

- Task IDs: `TASK-367` (final third)
- Objective: author the three OutOfRange negatives in the `scheduler_integration` bucket —
  `rejects_max_bridge_length_below_min` (AC-N1), 
  `rejects_support_max_branches_per_layer_zero` (AC-N2),
  `rejects_negative_support_branch_merge_distance` (AC-N3) — building their
  `ConfigBoundsIndex` from the REAL parsed manifest schemas where a loader helper exists
  (dispatch verifies; otherwise `BoundsDeclaration` mirrors mirroring the declared ranges,
  with the discrepancy noted in the test comment). Re-run AC-N4's executor regression net.
- Precondition: Steps 2–6 merged (5b may land in parallel with 5a/6 — disjoint files);
  236 composition spot-check (FORWARD dependency: if 236
  landed conflicting scheduler changes since authoring, reconcile here before closing).
- Postcondition: AC-N1/N2/N3/N4/N5 all PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - lines
    1–120 (precedent helpers)
  - `crates/slicer-scheduler/src/config_resolution.rs` - lines 140–270 only
    (`from_declarations` ~:150, `check_value` ~:265)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `crates/slicer-scheduler/tests/integration/main.rs` (module registration if the bucket
    requires it)
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/validation.rs` (236-owned, read-only)
- Expected sub-agent dispatches:
  - Question: does a manifest→BoundsDeclaration loader exist for integration tests, or is
    manual construction the established pattern; scope:
    `crates/slicer-scheduler/{src,tests}`; return: `LOCATIONS` ≤10.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §6 invariant 16, §13 T2
- OrcaSlicer refs:
  - none
- Verification:
  - AC-N1, AC-N2, AC-N3 commands each print PASS
  - AC-N4 and AC-N5 commands print PASS
- Exit condition: five PASS lines; no scheduler-unit regressions in the log.

### Step 8: Packet-owned closure — registration, deviation row, human gate artifacts

- Task IDs: `TASK-368`
- Objective: register TASK-363..368 in `docs/07_implementation_status.md` via a worker
  dispatch (never a full backlog read); file the divergence-5.4 DEVIATION_LOG row (key-based
  `support_line_width` mapping; auto = nozzle_diameter; no flow model); produce the human
  gate artifacts of packet.spec.md §Human Validation Gate (three slices + visual-debug
  bundle + evidence file recording removed-hardcode provenance — the deleted
  `DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM` literal was 5.0 vs canonical default 0.2 — alongside
  block-count deltas) and request sign-off; final gates: check/clippy `--all-targets`,
  `gen-config-docs --check`, `build-guests --check`.
- Precondition: Steps 1–7 complete and green (5a/5b/6 included).
- Postcondition: Doc Impact Statement greps all pass; sign-off line pending human verdict;
  packet ready for review toward `status: implemented`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - tail only, via dispatch summary
  - `docs/DEVIATION_LOG.md` - header + last rows only (derive next DEV id at point of use;
    ledger fact — never quote a stale ID)
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (registration rows only)
  - `docs/DEVIATION_LOG.md` (this packet's single new row)
  - `tmp/238a-human-validation.md` (evidence file; tmp is gitignored)
- Files explicitly out of bounds:
  - plan queue table, stub file, other packets' dirs, gap register (already routed at
    authoring)
- Expected sub-agent dispatches:
  - Question: derive next free DEV id + write the registration rows; scope:
    `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md`; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §14 rule 7 (doc hygiene)
- OrcaSlicer refs:
  - none (closure bookkeeping)
- Verification:
  - `cargo check --workspace --all-targets` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean
  - `cargo xtask gen-config-docs --check && cargo xtask build-guests --check; echo "gates-exit=$?"` -
    both exit 0
  - all Doc Impact Statement greps from packet.spec.md return matches
- Exit condition: gates exit 0; greps pass; evidence file lists artifact paths + measured
  deltas; sign-off line present and pending.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | red guest-dispatch tests + executor main.rs registration |
| Step 2 | S | four declarations + regen; blast radius pre-baked |
| Step 3 | S | spacing declaration + enum doc |
| Step 4 | M | eleven host keys; macro blast radius owned here |
| Step 5a | S | de-hardcode distances in geometry builtin |
| Step 5a-2 | S | new gated target `support_geometry_config_surface_tdd` + AC-4 |
| Step 5b | S | serializer width sourcing; dead literals removed |
| Step 6 | S | G-09 helper + both legs flipped to MAX rule |
| Step 6-2 | S | contract test `layer_height_transport_tdd.rs` + registration |
| Step 7 | S | bounds negatives + regression net |
| Step 8 | S | closure, deviation row, human-gate artifacts |

Aggregate M; no L step.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full
  backlog read.
- Human Validation Gate section carries its sign-off line (human-provided) before
  `status: implemented`.
- Reconcile the 236 FORWARD dependency: composition checks re-run after 236 reaches
  `implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and gate command.
- Record remaining packet-local risk (declaration-takes-effect surprise profile keys;
  multi-object-layer native-leg height change).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged
  swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification
commands must use `--all-targets` so the test, bench, and example targets compile.
