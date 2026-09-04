# Implementation Plan: top-fill-order-and-calibration-order

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: SDK ordering kernel — `SurfaceFillOrder` and `order_center_based_fragments`

- Task IDs: none (wayfinder-driven packet; backlog source is ticket 33)
- Objective: add `crates/slicer-sdk/src/surface_fill_order.rs` with the `SurfaceFillOrder` enum, its strict `&str` parser, and the four-mode fragment ordering kernel ported from canonical `FillPlanePath::fill_surface`.
- Precondition: `crates/slicer-sdk/src/order_lock.rs` exists with `OrderLockAllocator` and `remap_order_locks_to_global`.
- Postcondition: AC-1, AC-2, and AC-3 pass; nothing else in the tree calls the new module yet.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/order_lock.rs` — whole file (61 lines)
  - `crates/slicer-sdk/src/lib.rs` — the `pub mod` block only
  - `crates/slicer-sdk/src/prelude.rs` — whole file
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/surface_fill_order.rs` (new)
  - `crates/slicer-sdk/src/lib.rs`
  - `crates/slicer-sdk/tests/surface_fill_order_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `modules/**`, `OrcaSlicerDocumented/**`, `docs/spec_packets/264-*/**`
- Blast-radius discipline: no struct field and no schema constant is added in this step. New test literals of `ExtrusionPath3D` appear in Step 3, not here — this step's kernel operates on point lists, not IR paths.
- Expected sub-agent dispatches:
  - Question: how does `FillPlanePath::fill_surface` pick and orient the center spiral in the `is_flow_calib` branch, and what does `restore_source_path_order` do to fragments in the non-Default branch?; scope: the canonical oracle named in the wayfinder map's Notes, files `Fill/FillPlanePath.cpp` and `ClipperUtils.cpp`; return: `SUMMARY`
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — delegated SUMMARY of the mm↔unit boundary rule only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillPlanePath.cpp` — delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp` — delegate; never load
- Verification:
  - `cargo test -p slicer-sdk --test surface_fill_order_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail or bounded failure SNIPPETS
- Exit condition: `SurfaceFillOrder::Default` with `calibration_special_order = false` returns the input in nearest-neighbour order with no fragment reversed, and the three other modes each produce a *different* order on the annulus fragment set. If any two modes produce the same order on that fixture, the kernel is wrong and the step is not done.

### Step 2: Host — widen order-lock remap and tag allocation to all four `InfillRegion` vectors

- Task IDs: none (wayfinder-driven packet; backlog source is ticket 33)
- Objective: change `remap_infill_order_locks_from` and `next_global_infill_tag` (`crates/slicer-runtime/src/layer_executor.rs`) to walk `sparse_infill`, `solid_infill`, `ironing`, and `internal_bridge_infill`, in that order, with global tags unique across all four.
- Precondition: the packet-244 exclusion dispatch below has returned "no AC asserts sparse-only restriction".
- Postcondition: AC-5 and AC-N3 pass; AC-N1 still passes (no module emits a lock yet, so the widening is observationally inert).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` — symbol-located ±40-line windows on `remap_infill_order_locks_from`, `next_global_infill_tag`, `slicer_runtime_order_lock_remap`, and `assemble_ordered_entities_with_support_identities` only. Never open whole.
  - `crates/slicer-ir/src/slice_ir.rs` — `pub struct InfillRegion`, ±40 lines
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/tests/executor/order_lock_tdd.rs` — the packet-244 order-lock tests live here (test binary `executor`, aggregated by `crates/slicer-runtime/tests/executor/main.rs`); append cases, do not create a parallel file, and no new `mod` registration is needed
- Files explicitly out of bounds:
  - `crates/slicer-sdk/**`, `modules/**`, `docs/spec_packets/244-*/**` beyond the single dispatched FACT
- Blast-radius discipline: new `ExtrusionPath3D` struct literals in the test fixtures fall under the check-literals gate — each must use a `..` rest (FRU) or carry an `// exhaustive: <reason>` waiver per `docs/21_data_defaults_and_fixtures.md`. Dispatch the `LOCATIONS` sweep below before writing them and follow whatever convention the existing packet-244 fixtures use.
- Expected sub-agent dispatches:
  - Question: does any acceptance criterion in packet 244 assert that order-lock remap or validation is *restricted* to `sparse_infill`?; scope: `docs/spec_packets/244-order-locked-extrusion-sequences/packet.spec.md`; return: `FACT`
  - Question: which existing test files construct `ExtrusionPath3D` literals, and do they use FRU or an exhaustive waiver?; scope: `crates/slicer-runtime/tests/**`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegated SUMMARY of the `order_lock` / schema `1.4.0` paragraph
  - `docs/21_data_defaults_and_fixtures.md` — ranged direct read of the struct-literal gate rule
  - `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` — direct read (short). Its "remaps … at every output boundary" clause is the authority for this step; the widening is ADR conformance, not an amendment, so no `ADR-AMENDED` deviation may be filed for it.
- OrcaSlicer refs:
  - None. This step has no canonical counterpart; `no_sort` is an in-memory flag in canonical, not a remapped tag.
- Verification:
  - `cargo test -p slicer-runtime --test executor order_lock 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo xtask check-literals` — FACT exit code
- Exit condition: a fixture with locked blocks in all four vectors of two regions yields globally unique tags, and `next_global_infill_tag` on the *remapped* IR returns a value strictly greater than every tag it contains. If `next_global_infill_tag` still scans one vector, the step is not done even if the remap test passes.

### Step 3: Host — widen cross-module order-lock validation to all four vectors

- Task IDs: none (wayfinder-driven packet; backlog source is ticket 33)
- Objective: change `validate_infill_order_locks` (`crates/slicer-runtime/src/layer_executor.rs`) so its block-preservation check runs over all four `InfillRegion` vectors, including the dropped-region guard that today only inspects `sparse_infill`.
- Precondition: Step 2 landed; `remap_infill_order_locks_from` writes tags into all four vectors.
- Postcondition: AC-6 passes for `solid_infill`, `ironing`, and `internal_bridge_infill`, with the existing `sparse_infill` cases still green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` — symbol-located ±60-line window on `validate_infill_order_locks` only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/tests/executor/order_lock_tdd.rs` — the same file as Step 2; already registered in `crates/slicer-runtime/tests/executor/main.rs`
- Files explicitly out of bounds:
  - `crates/slicer-sdk/**`, `modules/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: no struct field or constant added. New test literals follow the FRU/waiver convention established in Step 2.
- Expected sub-agent dispatches:
  - None. The function is self-contained and read via a symbol-located window.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — already summarized in Step 2; do not re-read
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p slicer-runtime --test executor order_lock 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
- Exit condition: a locked two-path block in `solid_infill` that the replacement splits produces `Err` naming the tag, and the same block preserved contiguously produces `Ok`. Repeat for `ironing` and `internal_bridge_infill`. Refactoring the four vectors through one shared helper is preferred over four copies; either way all four must be covered by a test that fails if its vector is removed from the walk.

### Step 4: SDK block locking + manifest declarations on `archimedean-chords-infill`

- Task IDs: none (wayfinder-driven packet; backlog source is ticket 33)
- Objective: add `lock_block` to `crates/slicer-sdk/src/surface_fill_order.rs`, and declare `top_surface_fill_order`, `bottom_surface_fill_order`, and `calib_flowrate_topinfill_special_order` in `archimedean-chords-infill.toml`'s `[config.schema]`.
- Precondition: packet 264 reads `status: implemented` in its own `packet.spec.md` frontmatter — re-derive it, do not trust any earlier statement. Steps 1–3 complete.
- Postcondition: AC-4, AC-7, and AC-N2 pass. The keys are declared but not yet read — that is Step 5, and this step must not claim the keys are covered.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` — as the `[config.schema]` shape reference
  - `crates/slicer-sdk/src/order_lock.rs` — whole file (61 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/surface_fill_order.rs`
  - `modules/core-modules/archimedean-chords-infill/archimedean-chords-infill.toml`
  - `modules/core-modules/archimedean-chords-infill/tests/surface_fill_order_config_schema_tdd.rs` (new)
- Files explicitly out of bounds:
  - `docs/spec_packets/264-*/design.md` and `implementation-plan.md`; `OrcaSlicerDocumented/**`
- Blast-radius discipline: no struct field or constant added.
- Expected sub-agent dispatches:
  - Question: what is the exact `[config.schema]` TOML shape for a string key with an allowed-value list and for a bool key?; scope: `docs/03_wit_and_manifest.md`; return: `SNIPPETS` (≤2, ≤30 lines)
  - Question: what are `archimedean-chords-infill`'s crate name, manifest filename, and top/bottom solid emission function?; scope: `docs/spec_packets/264-top-bottom-surface-keys/packet.spec.md`; return: `SUMMARY`
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — delegated SNIPPETS as above
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — delegate; the three keys' types, enum values, defaults, and modes
- Verification:
  - `cargo test -p slicer-sdk --test surface_fill_order_tdd lock_block 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo test -p archimedean-chords-infill --test surface_fill_order_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
- Exit condition: `lock_block` assigns one shared local tag across a slice and distinct tags across two calls, and the three manifest keys parse with canonical's defaults (`"default"`, `"default"`, `false`). A declared key with no reader is **not** a completed step — Step 5 is mandatory.

### Step 5: Wire the ordering and the lock into the three center-based modules

- Task IDs: none (wayfinder-driven packet; backlog source is ticket 33)
- Objective: call `order_center_based_fragments` and `lock_block` from `archimedean-chords-infill` (all three keys) and from `concentric-infill` and `octagram-spiral-infill` (the two fill-order keys), and add the annulus fixture and behaviour tests.
- Precondition: Steps 1–4 complete; packet 264's three modules exist and emit top/bottom solid paths.
- Postcondition: AC-8, AC-9, AC-10, AC-11, AC-14, and AC-N4 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/archimedean-chords-infill/src/lib.rs` — whole file if under 300 lines, else the emission function ±60 lines
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — `run_infill`'s solid-emission path only, as the shape reference for `push_solid_path`
- Files allowed to edit (at most 3 per sub-commit; the step is three mechanical repetitions of one change):
  - `modules/core-modules/archimedean-chords-infill/src/lib.rs` + `modules/core-modules/archimedean-chords-infill/tests/calibration_order_tdd.rs` (new)
  - then `modules/core-modules/concentric-infill/{src/lib.rs,concentric-infill.toml}`
  - then `modules/core-modules/octagram-spiral-infill/{src/lib.rs,octagram-spiral-infill.toml}`
  - and, as their own sub-commits, the two new aggregated test files with their `mod` registrations: `crates/slicer-runtime/tests/contract/solid_infill_order_lock_tdd.rs` + `mod solid_infill_order_lock_tdd;` in `crates/slicer-runtime/tests/contract/main.rs`; `crates/slicer-scheduler/tests/integration/per_object_calibration_order_tdd.rs` + its `mod` line in `crates/slicer-scheduler/tests/integration/main.rs`. **A new file under either aggregator with no `mod` line never compiles and the run reports "0 tests" as a false pass** — register it in the same sub-commit.
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (Steps 2–3 already landed there), `docs/spec_packets/264-*/**` beyond a SUMMARY dispatch
- Blast-radius discipline: the new module tests construct `ExtrusionPath3D` literals — apply the FRU/waiver convention from Step 2 and re-run `cargo xtask check-literals`.
- Expected sub-agent dispatches:
  - Question: how does canonical order the loop sequence for `ipConcentric` under a non-Default `top_surface_fill_order`, given `FillConcentric::_fill_surface_single` emits closed loops rather than clipped fragments?; scope: the canonical oracle's `Fill/FillConcentric.cpp` and `Fill/Fill.cpp`; return: `SUMMARY`. This resolves `design.md`'s second `[FWD]`; do not guess.
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — delegated SUMMARY of § Claim Resolution only, to confirm how the region config reaches a fill module
  - `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` — direct read (short). Emitting a lock makes these paths self-clipping and triggers the linker's swept-footprint carve over untagged fill of the same region; AC-14 asserts both obligations. If the carve turns out not to be a no-op against sparse infill, stop and re-scope — an ordering key must not change sparse geometry.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — delegate; the pattern gate on the two fill-order keys
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillConcentric.cpp` — delegate; the `[FWD]` above
- Verification:
  - `cargo test -p archimedean-chords-infill --test calibration_order_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo test -p slicer-runtime --test contract solid_infill_order_lock_survives_path_optimization 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` — FACT exit code; 0 required (three guests edited)
- Exit condition: on the annulus fixture, `calib_flowrate_topinfill_special_order = true` emits the longest fragment last, inside-out, with every path sharing one `order_lock` tag; the default `false` emits `order_lock: None` in the helper's Default order; and `top_surface_fill_order` `"outward"` vs `"inward"` vs `"default"` yield three distinct orders. All ordering assertions must run on the annulus, never on a convex square — a convex region clips the spiral to one fragment and makes every assertion vacuous.

### Step 6: Docs, deviation row, and gates

- Task IDs: none (wayfinder-driven packet; backlog source is ticket 33)
- Objective: correct `docs/02_ir_schemas.md`'s order-lock scope wording, regenerate the config-key reference, and file the recorded-divergence row.
- Precondition: Steps 1–5 complete and green.
- Postcondition: AC-12 and AC-13 pass; `cargo xtask check-deviations` and `cargo xtask gen-config-docs --check` both return 0.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` — the `order_lock` paragraph only, located by grep
  - `docs/DEVIATION_LOG.md` — the last few rows only, via the dispatch below
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/15_config_keys_reference.md` (generated — produced by `cargo xtask gen-config-docs`, never hand-edited)
- Files explicitly out of bounds:
  - `docs/specs/orca-feature-gap/**` — the wayfinder map and its tickets are updated by the wayfinder session that resolves ticket 33, not by this packet
- Blast-radius discipline: no struct field or constant added.
- Expected sub-agent dispatches:
  - Question: what is the highest `DEV-###` currently in the deviation log?; scope: `docs/DEVIATION_LOG.md`; return: `FACT`. Run `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1` at this moment; never carry a number from the packet text.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — the edited paragraph
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — delegate; the pattern-agnostic lock the divergence row describes
- Verification:
  - `cargo xtask gen-config-docs --check` — FACT exit code
  - `cargo xtask check-deviations` — FACT exit code
  - `rg -q 'internal_bridge_infill' docs/02_ir_schemas.md; echo "exit=$?"` — FACT exit code
- Exit condition: the deviation row exists with a freshly derived ID and names `Fill::fill_surface_extruded` by function, not by line number; `docs/02_ir_schemas.md` no longer describes the remap as sparse-only; and the generated key table carries all three keys with the right owners.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Self-contained SDK module; one delegated canonical SUMMARY |
| Step 2 | M | Three coupled host functions plus fixtures; the packet's largest step |
| Step 3 | S | One function, symbol-located window |
| Step 4 | S | One SDK function plus one manifest |
| Step 5 | M | Three modules, but the same change repeated; one open `[FWD]` resolved by dispatch |
| Step 6 | S | Docs and gates only |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is **not** updated by this packet: it carries no `TASK-###` IDs, its backlog source is the wayfinder map, and packet 264 sets the same precedent. If a reviewer requires a `docs/07` row, dispatch a worker to add one — never read the full backlog.
- No reopened or superseded packet: packet 244 is widened, not reopened, and packet 264 is a forward dependency that stays untouched.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run `cargo xtask build-guests --check` and confirm exit 0 before any conclusion about a failing module test.
- Record remaining packet-local risk: the `[FWD]` on concentric loop ordering, if it was resolved by judgement rather than by a canonical read.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
