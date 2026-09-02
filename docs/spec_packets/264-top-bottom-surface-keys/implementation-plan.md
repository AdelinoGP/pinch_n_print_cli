# Implementation Plan: top-bottom-surface-keys

## Execution Rules

- Steps are ordered. Do not start a step before its predecessor's exit condition is met.
- `design.md` § Code Change Surface is the authoritative files-in-scope list; § Out-of-Bounds Files must not be edited or loaded.
- Every OrcaSlicer read is a delegated dispatch. Every cargo/xtask run is delegated with a `FACT pass/fail` return.
- Every test invocation tees to `target/test-output.log` and is inspected by reading that file, never by re-running.
- Ledger facts (module counts, deviation row counts) are re-derived from disk at the moment of use, never copied from this document.
- **FORWARD-DEP gate.** Packet **262b is `draft` as of this packet's authoring** and must reach `status: implemented` before Step 1 begins — re-derive its status from `docs/spec_packets/262b-infill-pattern-holder-mapping/packet.spec.md` at that moment rather than trusting this line. It creates `monotonic-infill` and the pattern→holder derivation this packet extends; every reference in this plan to `monotonic-infill` or to "262b's derivation helper" is a forward dependency, not a shipped symbol.

## Steps

### Step 1: Learn 262b's derivation shape and scaffold the six crates

- **Task IDs:** — (queue packet)
- **Objective:** create the six module crates as compiling no-op `LayerModule`s at `Layer::Infill` holding `claim:top-fill` + `claim:bottom-fill`, and register them in all four registration surfaces.
- **Preconditions:** 262b landed; `modules/core-modules/monotonic-infill/` exists.
- **Allowed reads:** an existing single-claim filler crate (`modules/core-modules/lightning-infill/**`) as the crate-shape template; `docs/adr/0056-integrated-modules-native-dispatch.md`.
- **Edits (≤ 3 logical units):** the six new crate trees; root `Cargo.toml`; `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}` + `crates/pnp-cli/Cargo.toml`.
- **Out of bounds:** everything in `design.md` § Out-of-Bounds Files.
- **Dispatches:** `FACT` ≤ 5 lines — the name of 262b's pattern→holder derivation helper in `crates/slicer-scheduler/src/config_resolution.rs`. `FACT pass/fail` — `cargo check --workspace --all-targets`.
- **Cost:** M
- **Authorities:** ADR-0056; `docs/03_wit_and_manifest.md` manifest schema.
- **Verification:** `cargo check --workspace --all-targets`, delegated, `FACT pass/fail`.
- **Exit / falsifying condition:** the workspace compiles with six new members and `cargo xtask dist --edition integrated` does not complain about a missing passthrough feature. If any crate cannot be registered without editing a file outside the change surface, stop and report.

### Step 2: Manifest ingestion — claims, stage, and the module count

- **Objective:** make the six manifests load cleanly and move the module-count assertion.
- **Preconditions:** Step 1 exit met.
- **Allowed reads:** `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`.
- **Edits:** `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`; the six manifests if a field is missing.
- **Dispatches:** `FACT` ≤ 5 lines — the currently asserted module count, re-derived from disk **now** (262b and 263 also move it).
- **Cost:** S
- **Verification:** `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-10 green — six more modules, each with `Layer::Infill` and both fill claims, zero `Error` diagnostics. A claim-conflict `Error` at load falsifies `design.md` invariant 4 and must stop the packet.

### Step 3: `monotonicline-infill`

- **Objective:** port canonical `FillMonotonicLines` — monotonic sweep ordering with no contour connectors.
- **Preconditions:** Step 2 exit met.
- **Allowed reads:** own crate; `crates/slicer-sdk/src/views.rs` accessors.
- **Edits:** `modules/core-modules/monotonicline-infill/src/lib.rs`, its manifest, `tests/monotonicline_infill_tdd.rs`.
- **Dispatches:** `SUMMARY` ≤ 200 words + ≤ 3 snippets ≤ 30 lines — `FillMonotonicLines::fill_surface`, `fill_surface_by_lines`' monotonic branch, `connect_segment_intersections_by_contours`.
- **Cost:** M
- **Verification:** `cargo test -p monotonicline-infill --test monotonicline_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-6's monotonic-order and zero-connector assertions pass. If the emitted lines are not in monotonic sweep order, the port of the ordering branch is wrong — fix it here, do not weaken the assertion.

### Step 4: `alignedrectilinear-infill`

- **Objective:** port canonical `FillAlignedRectilinear` — rectilinear with the per-layer angle pinned to 0.
- **Preconditions:** Step 3 exit met.
- **Allowed reads:** own crate; `modules/core-modules/rectilinear-infill/src/lib.rs` scan-line helpers as the reference implementation.
- **Edits:** the crate's `src/lib.rs`, manifest, `tests/alignedrectilinear_infill_tdd.rs`.
- **Dispatches:** `SUMMARY` ≤ 200 words — `FillAlignedRectilinear` and how it differs from `FillRectilinear::fill_surface`.
- **Cost:** S
- **Verification:** `cargo test -p alignedrectilinear-infill --test alignedrectilinear_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-7 green — first-line angle identical across three consecutive layer indices, and demonstrably different for `rectilinear-infill` on the same fixture. If the rectilinear comparison arm does not alternate, the fixture does not exercise per-layer angle and the AC is vacuous — fix the fixture.

### Step 5: `concentric-infill`

- **Objective:** port canonical `FillConcentric::_fill_surface_single` — nested closed loops by successive inward offset.
- **Preconditions:** Step 4 exit met.
- **Allowed reads:** own crate; the SDK offset helper used elsewhere in `modules/core-modules/`.
- **Edits:** the crate's `src/lib.rs`, manifest, `tests/concentric_infill_tdd.rs`.
- **Dispatches:** `SUMMARY` ≤ 200 words + ≤ 1 snippet ≤ 30 lines — `FillConcentric::_fill_surface_single`.
- **Cost:** M
- **Verification:** `cargo test -p concentric-infill --test concentric_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-8 green on the annulus fixture — all paths closed, strictly nested, loop count equal to the number of non-empty successive offsets. An open path or a straight scan line falsifies the port.

### Step 6: the three `FillPlanePath` fillers

- **Objective:** port `FillHilbertCurve`, `FillArchimedeanChords`, `FillOctagramSpiral`, each emitting one continuous clipped curve.
- **Preconditions:** Step 5 exit met.
- **Allowed reads:** the three own crates.
- **Edits:** `hilbert-curve-infill`, `archimedean-chords-infill`, `octagram-spiral-infill` — `src/lib.rs`, manifest, and test file each.
- **Dispatches:** `SUMMARY` ≤ 200 words + ≤ 3 snippets ≤ 30 lines — the three point generators and `FillPlanePath::fill_surface`'s `dont_connect() || density > 0.5` short-circuit.
- **Cost:** M
- **Authorities:** `docs/08_coordinate_system.md` — the curves are generated in mm and converted once at the boundary.
- **Verification:** `cargo test -p hilbert-curve-infill --test hilbert_curve_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`, then the same form for `archimedean-chords-infill` and `octagram-spiral-infill`.
- **Exit / falsifying condition:** AC-9 green for all three — one continuous polyline each, contained in the region, and the three structural discriminators (lattice membership, monotonic radius, octagram turn-angle set) each hold for exactly their own curve. If any filler emits more than one polyline for a solid region, the `dont_connect` short-circuit was not ported.

### Step 7: `monotonic-infill` gains `claim:bottom-fill`

- **Objective:** give canonical's `bottom_surface_pattern` default (`monotonic`) a holder.
- **Preconditions:** Step 6 exit met.
- **Allowed reads:** `modules/core-modules/monotonic-infill/src/lib.rs`.
- **Edits:** `modules/core-modules/monotonic-infill/monotonic-infill.toml` (append `"claim:bottom-fill"` to `holds`); `modules/core-modules/monotonic-infill/src/lib.rs` (the `BottomSolidInfill` emission arm).
- **Out of bounds:** `docs/spec_packets/262b-infill-pattern-holder-mapping/**` — never edit another packet's directory.
- **Cost:** S
- **Verification:** `cargo test -p monotonic-infill --test monotonic_infill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and the AC-10 command.
- **Exit / falsifying condition:** `monotonic-infill` emits `BottomSolidInfill` when it holds `claim:bottom-fill` and nothing when it does not; 262b's own tests stay green.

### Step 8: the pattern→holder derivation, global and per-object

- **Objective:** make both pattern keys resolve to holders on both resolution paths, rejecting unknown values.
- **Preconditions:** Step 7 exit met — all eight target modules exist, so no mapping entry points at a missing module.
- **Allowed reads:** `crates/slicer-scheduler/src/config_resolution.rs` — `resolve_global_config`, `apply_overlay`, and 262b's derivation helper.
- **Edits:** `crates/slicer-scheduler/src/config_resolution.rs`; net-new `crates/slicer-scheduler/tests/integration/top_bottom_pattern_holder_tdd.rs` + its `mod` line in `tests/integration/main.rs`.
- **Cost:** M
- **Authorities:** `docs/04_host_scheduler.md` § Claim Resolution.
- **Verification:** `cargo test -p slicer-scheduler --test scheduler_integration top_bottom_pattern_holder 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-1, AC-2, AC-3, AC-N3 all green. A derivation that satisfies AC-1/AC-2 but not AC-3 means it was added only to `resolve_global_config` — not done.

### Step 9: the density wire in `rectilinear-infill`

- **Objective:** replace the hardcoded `SOLID_DENSITY` with the two resolved fractions and add canonical's top-surface zero skip.
- **Preconditions:** Step 8 exit met.
- **Allowed reads:** `modules/core-modules/rectilinear-infill/src/lib.rs` — `LayerModule::from_config`, `run_infill`, `solid_fill_role`, `adjust_solid_spacing`.
- **Edits:** that `src/lib.rs`; `rectilinear-infill.toml` (two `[config.schema]` tables); `rectilinear-infill/Cargo.toml` (`toml = "0.8"` dev-dep, add-if-absent); `tests/top_bottom_fill_tdd.rs`.
- **Blast radius owned by this step:** adding two fields to `RectilinearInfill` puts every test-code literal of that struct under the struct-literal churn gate — each needs a `..` rest or an `// exhaustive: <reason>` waiver. Fix them here; `cargo xtask check-literals` must pass before the step closes.
- **Cost:** M
- **Authorities:** `docs/21_data_defaults_and_fixtures.md`; `docs/08_coordinate_system.md`.
- **Verification:** `cargo test -p rectilinear-infill --test top_bottom_fill_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`, then `cargo xtask check-literals`, then the AC-N1 command.
- **Exit / falsifying condition:** AC-4 and AC-5 green **and** AC-N1 still byte-identical. If the default path shifts by even one path, the percent→fraction normalization is wrong — fix it here; never re-baseline AC-N1.

### Step 10: schema guard and bounds enforcement

- **Objective:** pin the two density tables and their canonical bounds, and pin that no manifest declares either pattern key.
- **Preconditions:** Step 9 exit met.
- **Edits:** net-new `modules/core-modules/rectilinear-infill/tests/top_bottom_surface_config_schema_tdd.rs`; `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`.
- **Cost:** S
- **Verification:** `cargo test -p rectilinear-infill --test top_bottom_surface_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **Exit / falsifying condition:** AC-12, AC-13, AC-N5 green. `bottom_surface_density = 5` **must** be rejected; if it resolves, the min of 10 was not carried from canonical.

### Step 11: claim resolution, docs, guests, closure gates

- **Objective:** prove end-to-end holder selection, land the hand-maintained docs, regenerate the generated one, and refresh the guests.
- **Preconditions:** Step 10 exit met.
- **Edits:** `crates/slicer-runtime/tests/contract/` (`native_infill_claim_resolution` arms); `docs/04_host_scheduler.md` § Claim Resolution; `docs/03_wit_and_manifest.md` § Known claim IDs; `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`; the eight guest `.wasm` artifacts.
- **Cost:** M
- **Verification:** `cargo test -p slicer-runtime --test contract native_infill_claim_resolution 2>&1 | tee target/test-output.log | grep -E "^test result"`; the AC-14 and AC-15 commands; `cargo xtask build-guests --check; echo "exit=$?"` (exit 0 required — never grep for `STALE:`); `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs | grep -cE "^[+-][^+-]"` must print `0`.
- **Exit / falsifying condition:** AC-11, AC-14, AC-15, AC-N2, AC-N4 green and `build-guests --check` exits 0.

## Per-Step Budget Roll-Up

| Step | Cost | Primary surface |
| --- | --- | --- |
| 1 | M | six new crates + four registration surfaces |
| 2 | S | manifest ingestion + module count |
| 3 | M | `monotonicline-infill` |
| 4 | S | `alignedrectilinear-infill` |
| 5 | M | `concentric-infill` |
| 6 | M | three `FillPlanePath` crates |
| 7 | S | `monotonic-infill` bottom claim |
| 8 | M | pattern→holder derivation |
| 9 | M | density wire + literal blast radius |
| 10 | S | schema guard + bounds |
| 11 | M | claims, docs, guests, gates |

Aggregate: **L**. No single step is L, so no split is required before activation.

## Packet Completion Gate

All of the following, each delegated with a `FACT pass/fail` return:

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask check-literals`
4. `cargo xtask build-guests --check; echo "exit=$?"` — exit 0
5. Every AC command in `requirements.md` § Verification Matrix, green
6. The two map gates re-checked by the closing agent: (a) the disposition table lists zero declaration-only keys; (b) every key has at least one AC asserting a behaviour change at a non-default value

## Acceptance Ceremony

`cargo test --workspace` is **not** required by this packet's ACs and must not be run as an AC command. If the closing agent judges the whole-suite run necessary because eight guests and a shared config-resolution path changed, it runs `cargo xtask test --summary --workspace` (never bare `cargo test --workspace`) and is dispatched to a sub-agent returning `FACT pass/fail` only — after every narrower command above has already passed.
