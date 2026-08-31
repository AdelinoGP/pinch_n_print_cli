# Implementation Plan: brim-type-and-brim-keys

## Execution Rules

- Work one atomic step at a time; every step below is a context-budget contract, independently stated.
- Use TDD, then implementation, then the narrowest falsifying validation; `skirt-brim` is a module crate — narrow runs via `cargo test -p skirt-brim --test <file>` are correct (no non-default features exist on this crate).
- Test output always tees to `target/test-output.log`; read the log, never re-run to "see more".
- This packet carries no `docs/07` task IDs (queue precedent, `task_ids: []`); implementation is recorded against wayfinder ticket 12.

## Steps

### Step 1: Manifest declaration + schema guard

- Task IDs: none (wayfinder ticket 12).
- Objective: declare the five brim keys in `skirt-brim.toml` `[config.schema]` exactly per AC-1, and author `tests/brim_config_schema_tdd.rs` asserting them.
- Precondition: tree green at `cargo check -p skirt-brim`; `skirt-brim.toml` parses.
- Postcondition: the guard proves the five tables (types, defaults, bounds, 7-value enum order, `Skirt/Brim` group); `brim_ears` (bool) remains undeclared — its absence is not an error.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/skirt-brim/skirt-brim.toml` - full (82 lines)
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (~120 lines; pattern source)
  - `modules/core-modules/part-cooling/Cargo.toml` - dev-dependencies block - the `toml = "0.8"` dev-dep the pattern requires
  - `docs/spec_packets/257-brim-type-and-brim-keys/requirements.md` - §Per-Key Canonical Evidence table (the exact manifest values)
- Files allowed to edit (at most 3):
  - `modules/core-modules/skirt-brim/skirt-brim.toml`
  - `modules/core-modules/skirt-brim/tests/brim_config_schema_tdd.rs` (new)
  - `modules/core-modules/skirt-brim/Cargo.toml` (dev-dependencies only: add `toml = "0.8"` — the schema guard parses the manifest directly; part-cooling carries the identical dev-dep. Host deps unchanged.)
- Files explicitly out of bounds:
  - `modules/core-modules/skirt-brim/src/lib.rs` (Step 2 owns it)
  - any other module's manifest; `crates/**`
- Blast-radius discipline: no struct fields or schema constants change in this step; the manifest's `[config.schema]` gains tables only. `config_bounds_enforcement_tdd.rs` and the runtime CONFIG_BLOCK tests do not enumerate `skirt-brim` keys today (verified), so there is no hard-assert fallout from the additions.
- Expected sub-agent dispatches:
  - Question: "does `cargo test -p skirt-brim --test brim_config_schema_tdd` pass with AC-1's exact assertions?"; scope: the two edited files; return: `FACT pass/fail`; purpose: step exit.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - delegated; NOT read (generated; Step 4 regenerates it)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; declarations already captured in `requirements.md` §Per-Key Canonical Evidence (do not re-read)
- Verification:
  - `cargo test -p skirt-brim --test brim_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code (manifest is a fingerprint input; rebuild without `--check` if exit 1, then re-verify)
- Exit condition: guard green naming all five keys AND `build-guests --check` exit 0 after rebuild.

### Step 2: Wire the `no_brim` gate

- Task IDs: none (wayfinder ticket 12).
- Objective: add `BrimType` (7 variants, canonical order), a `brim_type` field read in `from_config` (`ConfigValue::String` arm, `auto_brim` fallback), and gate both brim arms on `brim_width > 0.0 && brim_type != BrimType::NoBrim`; author the three invariants (AC-2 suppression, AC-3 default identity, AC-N1 width-gate precedence).
- Precondition: Step 1 merged in-tree (guard green); `lib.rs` currently gates the brim arm on `self.brim_width > 0.0` at two sites (`run_finalization` live path; legacy `process()`).
- Postcondition: `brim_type = "no_brim"` suppresses exactly the brim entities (skirt untouched) on both paths; absent/`auto_brim` value leaves output identical to pre-packet.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/skirt-brim/src/lib.rs` - full (~421 lines) - the only structural read this packet performs
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` - full - existing fixture/driver shape to extend
- Files allowed to edit (at most 3):
  - `modules/core-modules/skirt-brim/src/lib.rs`
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs`
  - `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` (only if the legacy-path identity assertion belongs there; otherwise skip — the file is the `process()` home)
- Files explicitly out of bounds:
  - `crates/**`; other modules; the manifest (Step 1 owns it)
- Blast-radius discipline: `BrimType`/field additions are module-private (no pub-struct-literal blast radius beyond `SkirtBrim`'s own two constructors, both in this file). No public schema/version constant changes. If `slicer_module_binding_tdd.rs` asserts on struct introspection surface, it is unaffected (macro surface keys off `tier_id`/`stage_name`/`wit_exports`, not config fields — verified by subagent read).
- Expected sub-agent dispatches:
  - Question: "does the full `cargo test -p skirt-brim` (three test files) pass with the new invariants and unchanged legacy assertions?"; scope: `modules/core-modules/skirt-brim`; return: `FACT pass/fail + failing test names`; purpose: step exit.
- Context cost: `S`
- Authoritative docs:
  - none beyond the packet files
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Print.hpp` - `has_brim` interplay - delegate, already summarized in `requirements.md`; no re-read
- Verification:
  - `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code (src edits feed the guest build); rebuild if exit 1, then re-run the two test files before trusting them
- Exit condition: all three skirt-brim test binaries green; guest freshness exit 0.

### Step 3: Scheduler bounds arm + CONFIG_BLOCK reachability

- Task IDs: none (wayfinder ticket 12).
- Objective: prove host-side plumbing — (a) real-manifest enum/bounds rejection arm for `brim_type`/`brim_object_gap` in the scheduler integration test; (b) single-emission CONFIG_BLOCK assertion for an explicit `brim_type` in the runtime integration test.
- Precondition: Steps 1–2 green; `skirt-brim` manifest loads via `load_module_from_paths` + `ConfigBoundsIndex::from_modules` (existing pattern in the test file's real-manifest arms).
- Postcondition: AC-4 and AC-5 demonstrable by one command each.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - the real-manifest arms only (`manifest_declared_bound_rejects_out_of_range_value`, `rejects_unknown_support_style_value`) plus the file's module-loading helpers
  - `crates/slicer-gcode/src/serialize.rs` - lines 315–545 only - the `emit_config_kv` dedup + padding loop context for the AC-5 assertion
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - full (the driver `run_pipeline_with_raw_config` and existing key assertions)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/**` (no padding edits, no serializer changes); `crates/slicer-scheduler/src/**`; module dirs
- Blast-radius discipline: test-only edits; no production surface. If the integration buckets require test registration in `main.rs` registries (both files are registry-driven — verify at read time), add the two test modules to the registries in the same step (registry files are then within this step's edit set, still ≤ 3 edits per file).
- Expected sub-agent dispatches:
  - Question: "does the scheduler arm reject `brim_type = "elephant"` and `brim_object_gap = 3.0` from the real manifest, and does the runtime arm show exactly one `; brim_type = <value>` line?"; scope: the two edited test files; return: `FACT pass/fail + counts`; purpose: step exit.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract - delegated SUMMARY only (already captured in packet spec; do not re-dispatch unless a worker disputes the dedup ruling)
- OrcaSlicer refs:
  - none in this step
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: both binaries green with the new arms; no padding-table edit diff exists in `git diff --stat` for `crates/slicer-gcode`.

### Step 4: Doc regeneration + workspace gates

- Task IDs: none (wayfinder ticket 12).
- Objective: regenerate `docs/15` generated tables; run the workspace-level closure gates.
- Precondition: Steps 1–3 green and guests fresh.
- Postcondition: `gen-config-docs --check` exit 0; workspace check+clippy green with `--all-targets`.
- Files allowed to read, with ranges when over 300 lines:
  - none directly (all delegated)
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (via `cargo xtask gen-config-docs` only — never hand-edited)
- Files explicitly out of bounds:
  - any `crates/**` or `modules/**` source
- Expected sub-agent dispatches:
  - Question: "`cargo xtask gen-config-docs --check` exit code after regeneration; five brim keys present with canonical defaults?"; scope: xtask + doc 15 diff; return: `FACT exit code + SNIPPETS ≤ 10 lines of doc diff`; purpose: step exit.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated; verify via `--check` only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask gen-config-docs --check; echo "exit=$?"` - FACT exit code
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: all three FACT green; `git diff --stat` shows only the packet's owned files.

### Step 5: Preflight (authoring gate)

- Task IDs: none (wayfinder ticket 12).
- Objective: run `/spec-review docs/spec_packets/257-brim-type-and-brim-keys --preflight` and reach `PREFLIGHT PASS`.
- Precondition: Steps 1–4 exits green.
- Postcondition: preflight verdict PASS; any BLOCKED finding is fixed and rerun, or (if unfixable) recorded verbatim in this packet's design questions with `status: draft` retained and reported to the wayfinder ticket.
- Files allowed to read, with ranges when over 300 lines:
  - this packet directory - full
- Files allowed to edit (at most 3):
  - any packet file, bounded by the finding
- Files explicitly out of bounds:
  - everything outside `docs/spec_packets/257-brim-type-and-brim-keys/`
- Expected sub-agent dispatches:
  - Question: preflight; scope: packet dir; return: `FACT PREFLIGHT PASS|BLOCKED + findings`; purpose: the gate.
- Context cost: `S`
- Authoritative docs: none
- OrcaSlicer refs: none
- Verification:
  - spec-review preflight verdict - FACT
- Exit condition: `PREFLIGHT PASS`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest + guard; guest freshness rides here |
| Step 2 | S | gate + invariants; guest rebuild if src fingerprint trips |
| Step 3 | S | two narrow test arms |
| Step 4 | S | generated docs + workspace gates |
| Step 5 | S | preflight |

Aggregate: S+M margin — no split needed.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS (AC-1..AC-5, AC-N1, AC-N2).
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — **re-derive the crosswalk question at completion time**; this packet is a queue packet with `task_ids: []`, so the `docs/07` crosswalk is expected N-A (255/256 precedent) — confirm, don't assume.
- No reopened/superseded packets exist for this slice.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command; record FACT lines.
- Record remaining packet-local risk (the degraded-mode tradeoff in design §Risks is the standing one).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.