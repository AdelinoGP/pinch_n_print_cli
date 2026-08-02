# Implementation Plan: 193-overhang-distance-prepass-carrier

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Steps 3 through 9 are the struct-literal sweep and each deliberately exceeds the usual three-file edit limit.** `.claude/skills/spec-packet-generator/SKILL.md` §Packet Safety requires the step that adds a struct field to own the struct-literal blast radius rather than leaving it for a follow-up `cargo check`. This packet's blast radius is several times packet 189's `LayerCollectionIR` sweep, so it is split into **seven** steps by crate group — the same technique packet 189 uses for its two, applied at the scale this field actually has. No step rates `L`.
- **Do not transcribe a sweep figure from anywhere.** Step 0 re-derives the sum, the file count and the per-crate-group breakdown, and Steps 3-9 are graded against that run, not against any number written in this packet.
- Steps 1-2 and 4-5 change signatures and wire formats. Steps 3 and 6-9 are blind mechanical insertions. **Do not interleave them**: mixing a signature error class into an `E0063` sweep makes the compiler's output unreadable as a worklist, which is the sweep's only oracle.

## Steps

### Step 0: Pin the schema versions, re-derive the sweep, and record the baselines

- Task IDs: `TASK-314`
- Objective: write the two pin files `AC-2` reads, re-derive the struct-literal blast radius that Steps 3-9 are sized against, and capture the do-not-regress baselines.
- Precondition: tree is at `deviations-fix` HEAD, working tree clean for `crates/slicer-core/src/algos/overhang_annotation.rs` (`AC-N2`'s probe compares against `HEAD`).
- Postcondition: `target/pin-perimeter-schema-before.txt` and `target/pin-surface-schema-before.txt` exist and hold the pre-packet `major.minor.patch`; `AC-2` prints its "no additive minor bump" FAIL (not its "pin file missing" FAIL); the sweep breakdown is recorded in the swarm log and **frozen into no file**.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` (very long) - locate `CURRENT_PERIMETER_IR_SCHEMA_VERSION` and `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` only, ±10 lines each
- Files allowed to edit (at most 3):
  - none — read-only discovery step. The two pin files are `target/` scratch, not tracked content.
- Files explicitly out of bounds:
  - everything tracked; this step edits nothing in the repo
- Expected sub-agent dispatches:
  - Question: "Run `rg -c 'dist_to_top_mm:' --glob '*.rs' crates modules xtask`; return the total occurrence count, the file count, and the per-crate-group breakdown grouping by `crates/<crate>` and `modules/core-modules`."; scope: workspace; return: `LOCATIONS` ≤ 20 group rows — **not** the 100-plus-row file list
  - Question: "Run the four baseline commands below and return only their `^test result:` lines."; scope: workspace; return: `FACT` ≤ 6 lines
- Context cost: `S`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'mkdir -p target && python3 -c "import io,re; s=io.open(r\"crates/slicer-ir/src/slice_ir.rs\",encoding=\"utf-8\").read(); pin=lambda k,o: (lambda b: io.open(o,\"w\",encoding=\"utf-8\").write(re.search(r\"major:\s*(\d+)\",b).group(1)+\".\"+re.search(r\"minor:\s*(\d+)\",b).group(1)+\".\"+re.search(r\"patch:\s*(\d+)\",b).group(1)))(s[s.index(k):s.index(k)+200]); pin(\"CURRENT_PERIMETER_IR_SCHEMA_VERSION: SemVer\",\"target/pin-perimeter-schema-before.txt\"); pin(\"CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION: SemVer\",\"target/pin-surface-schema-before.txt\"); print(\"PASS: pinned \"+io.open(\"target/pin-perimeter-schema-before.txt\").read()+\" / \"+io.open(\"target/pin-surface-schema-before.txt\").read())"'` - FACT (**this is the mechanism `AC-2` depends on and it must run before any edit.** Verified on this tree: it writes `1.0.0` for the perimeter constant, and `AC-2` then prints `FAIL: CURRENT_PERIMETER_IR_SCHEMA_VERSION did not take an additive minor bump - before=(1, 0, 0) now=(1, 0, 0)` — the correct change-proving red. If `target/` is cleaned later, re-pin from `git show HEAD:crates/slicer-ir/src/slice_ir.rs` rather than from the working tree, or the assertion becomes vacuous.)
  - `bash -c 'rg -c "dist_to_top_mm:" --glob "*.rs" crates modules xtask | awk -F: "{s+=\$NF; n++} END {print \"occurrences=\" s, \"files=\" n}"'` - FACT ≤ 1 line (the sweep sizing; **a ledger fact — record it in the swarm log, never in a file**)
  - `bash -c 'rg -q "overhang_distance_mm" crates modules xtask && echo "FAIL: overhang_distance_mm already exists - another packet has landed it" || echo PASS'` - FACT
  - `bash -c 'cargo test -p slicer-core 2>&1 | rg "^test result:" | rg -q "[1-9][0-9]* failed|FAILED" && echo "FAIL: slicer-core is not green before the packet starts" || echo PASS'` - FACT (baseline)
  - `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | rg "^test result:"'` - FACT (baseline; the golden must not move, and this packet has no consumer so it must not)
- Exit condition: both pin files exist, `AC-2` prints the bump FAIL rather than the pin-missing FAIL, `overhang_distance_mm` has zero hits, and the sweep breakdown is in the swarm log.

### Step 1: Red tests for the carrier, the contract, and the two stamping sites

- Task IDs: `TASK-314`
- Objective: write the six failing tests that define the contract before any production code exists — `overhang_distance_is_signed_and_boundary_offset_normalised`, `expolygon_to_path3d_stamps_signed_distance_and_none_on_empty_boundary`, `no_previous_layer_stamps_none_not_zero` and `quartile_stamping_is_unchanged_by_the_distance_carrier` (all in the new `crates/slicer-core/tests/overhang_distance_carrier_tdd.rs`); `absent_overhang_distance_deserializes_as_none` (new `crates/slicer-ir/tests/point3_overhang_distance_roundtrip.rs`); `arachne_stamps_distance_for_regions_with_no_overhang_bands` (new `modules/core-modules/arachne-perimeters/tests/overhang_distance_tdd.rs`).
- Precondition: Step 0 complete.
- Postcondition: all three new binaries fail to **compile** (`overhang_distance_mm`, `signed_distance_to_boundary` and `expolygon_to_path3d`'s new parameter do not exist). A non-compiling test binary is the correct red state; do not stub the field to make it link.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/tests/point3_overhang_quartile_roundtrip.rs` - read whole; it is the shape the new roundtrip test mirrors, including how it constructs an absent-field payload
  - `crates/slicer-core/src/perimeter_utils.rs` - locate `expolygon_to_path3d`, open ±60 lines; its doc-comment states the winding-number rule the distance stamp sits beside
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (long) - locate `region.overhang_quartile_polygons()` and `if !overhang_bands.is_empty()`, open ±40 lines around each. Do **not** read whole.
- Files allowed to edit (three new test files; each is a distinct binary and none can be folded into another crate):
  - `crates/slicer-core/tests/overhang_distance_carrier_tdd.rs`
  - `crates/slicer-ir/tests/point3_overhang_distance_roundtrip.rs`
  - `modules/core-modules/arachne-perimeters/tests/overhang_distance_tdd.rs`
- Files explicitly out of bounds:
  - every `src/` file in the workspace and both `.wit` files (this is a test-only step)
  - `modules/core-modules/overhang-classifier-default/**`
- Expected sub-agent dispatches:
  - Question: "In `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp`, quote the two lines of `estimate_points_properties` that assign `distance + boundary_offset`, and state the four template arguments used by the G-code speed path's instantiation."; scope: that file; return: `SNIPPETS` ≤ 1 of ≤ 10 lines plus ≤ 40 words. **`AC-4`'s test must be written from this quote, not from memory** — signedness and the offset are the two things this packet exists to pin.
  - Question: "Does `modules/core-modules/arachne-perimeters` have a `tests/` directory today, and what test harness attribute do its existing tests use (`#[test]` vs `#[module_test]`)?"; scope: that crate; return: `FACT` ≤ 3 lines. **This matters:** sibling module crates in this tree use `#[module_test]`, and a `#[test]` in such a crate is silently not collected — measured in packet 190's own §Step 1 for `overhang-classifier-default`, where the file carries `#[module_test]` 6× and `#[test]` 0×.
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - `estimate_points_properties` - delegate; never load
- Verification:
  - `bash -c 'cargo test -p slicer-core --test overhang_distance_carrier_tdd 2>&1 | rg -q "cannot find|no field|E0061|E0425|E0433|E0609" && echo "RED as expected" || echo "FAIL: tests compiled — the new symbols must not exist yet"'` - FACT
  - `bash -c 'cargo test -p slicer-ir --test point3_overhang_distance_roundtrip 2>&1 | rg -q "cannot find|no field|E0433|E0609" && echo "RED as expected" || echo "FAIL: tests compiled — overhang_distance_mm must not exist yet"'` - FACT
  - `bash -c 'cargo test -p arachne-perimeters --test overhang_distance_tdd 2>&1 | rg -q "cannot find|no field|E0433|E0609" && echo "RED as expected" || echo "FAIL: tests compiled — the stamping must not exist yet"'` - FACT
- Exit condition: all three binaries fail to compile for the stated missing-symbol reason and no other. If any fails for an unrelated reason, stop and diagnose before proceeding.

### Step 2: Add `overhang_distance_mm` to `Point3WithWidth` and the WIT record, and bump the perimeter schema

- Task IDs: `TASK-314`
- Objective: add `#[serde(default)] pub overhang_distance_mm: Option<f32>` to `Point3WithWidth` (`crates/slicer-ir/src/slice_ir.rs`) with a doc-comment carrying the **signed** convention and the `+ boundary_offset` normalisation verbatim from `design.md` §Data and Contract Notes; add `overhang-distance-mm: option<f32>` to `record point3-with-width` (`crates/slicer-schema/wit/deps/types.wit`); take the additive minor bump on `CURRENT_PERIMETER_IR_SCHEMA_VERSION` with a comment naming this packet. **`record seam-point3-with-width` two lines below in the same file gets NOTHING** — see `design.md` §Data and Contract Notes.
- Precondition: Step 1 complete; the three new test binaries are red for missing symbols.
- Postcondition: `AC-1`, `AC-2`, `AC-3` and `AC-N3` print PASS; `cargo check --workspace --all-targets` now fails, and every remaining failure is `E0063` (missing field `overhang_distance_mm`) — that failure list is Steps 3-9's worklist.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` (very long) - locate `pub struct Point3WithWidth` and `CURRENT_PERIMETER_IR_SCHEMA_VERSION`, open ±30 lines around each
  - `crates/slicer-schema/wit/deps/types.wit` - read whole (short)
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/deps/types.wit`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/slice_ir.rs`'s `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` — **packet 189's line.** See `design.md` §Architecture Constraints for why this is the plausible wrong answer and not this packet's.
  - `crates/slicer-core/**`, `crates/slicer-runtime/**`, `modules/**` (later steps)
  - `docs/**` (Step 10)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - This step adds a field to a type constructed exhaustively across the workspace and bumps a public schema constant. **Re-derive the site list rather than trusting anything written here.** Step 0's command is the derivation; Steps 3-9 are its consumers.
  - The proxy `dist_to_top_mm:` **over-reports** and does so for a knowable reason: the `Point3WithWidth` definition itself (this step's own file) and the three conversion files under `crates/slicer-wasm-host/src/marshal/` also name the field without being literals that take a blind inserted line. The marshal files get a **real** edit in Step 5 and are excluded from every blind-sweep step.
  - The compiler is the oracle. Any site missed surfaces as `E0063`, never as a runtime surprise; correspondingly, a listed file with no `E0063` was a proxy false positive and needs no edit.
  - The **test-assertion fallout** of the schema bump is `AC-N3`'s new roundtrip test plus whatever in `crates/slicer-ir/tests/` pins a `Default().schema_version` against its constant. Land the bump and the field in this one step; do not defer it.
- Expected sub-agent dispatches:
  - Question: "Which derives and serde attributes does `pub struct Point3WithWidth` carry, and does any test in `crates/slicer-ir/tests/` hard-assert a literal `PerimeterIR` schema version string?"; scope: `crates/slicer-ir/**`; return: `FACT` ≤ 5 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"IR 7 — PerimeterIR" - delegated `SNIPPETS` of the struct block only; do not load the file
- OrcaSlicer refs:
  - none for this step
- Verification:
  - the `AC-1` command - FACT (verified to print its precise FAIL on the unfixed tree)
  - the `AC-2` command - FACT (reads the Step 0 pin file; verified to print `FAIL: … did not take an additive minor bump - before=(1, 0, 0) now=(1, 0, 0)` before this step)
  - the `AC-3` command - FACT (windowed with a `(?<!seam-)` lookbehind; verified to print its FAIL on the unfixed tree)
  - `bash -c 'cargo test -p slicer-ir --test point3_overhang_distance_roundtrip 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: point3_overhang_distance_roundtrip did not run or did not pass"'` - FACT (`AC-N3`; the additive-bump proof lands in the same step as the bump)
- Exit condition: `AC-1`, `AC-2`, `AC-3` and `AC-N3` print PASS, and `cargo check --workspace --all-targets` fails **only** with `E0063` naming `overhang_distance_mm`.

### Step 3: Struct-literal sweep, group A — `slicer-ir`, `slicer-core`, `slicer-macros`

- Task IDs: `TASK-314`
- Objective: add `overhang_distance_mm: None,` to every exhaustive `Point3WithWidth { … }` literal in these three crates. Do not read the files for comprehension — edit the literal and move on.
- Precondition: Step 2 complete; `cargo check --workspace --all-targets` fails only with `E0063`.
- Postcondition: `cargo check -p slicer-ir --all-targets`, `cargo check -p slicer-core --all-targets` and `cargo check -p slicer-macros --all-targets` report no `E0063`.
- Files allowed to read, with ranges when over 300 lines:
  - each file in the group - locate `Point3WithWidth {` and open ±15 lines only
- Files allowed to edit (this step deliberately exceeds three; see §Execution Rules):
  - every `.rs` file under `crates/slicer-ir/`, `crates/slicer-core/` and `crates/slicer-macros/` that Step 0's re-derivation lists
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/overhang_annotation.rs`'s `const BAND_BOUNDARY_MULTIPLIERS` declaration (`AC-N2`)
  - `crates/slicer-wasm-host/**` (Step 5 — **real** edits, not blind ones)
  - every other crate and `modules/**` (Steps 4, 6-9)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Re-derive the group's file list with Step 0's command filtered to these three crates; the compiler's `E0063` set is the authority on which listed files actually need an edit.
  - `crates/slicer-macros/src/lib.rs` is a **guest-WASM build input** and its literal sits inside generated guest-facing code. Handle it deliberately: a blind insertion there is fine for this field (the value is `None` everywhere), but confirm the surrounding generated block still compiles for a guest target rather than only for the host.
  - `crates/slicer-core/src/lib.rs`'s `interpolate_point` is **not** a bare literal — it interpolates every field between two points and already carries `overhang_quartile: start.overhang_quartile`. The new field must follow the same "inherit from `start`" convention there, not be set to `None`, or every interpolated point silently loses its distance. This is the one non-mechanical site in group A and it is called out so a blind sweep does not get it wrong.
- Expected sub-agent dispatches:
  - Question: "Run Step 0's re-derivation command restricted to `crates/slicer-ir`, `crates/slicer-core` and `crates/slicer-macros`, and return the `count path` lines."; scope: those three crates; return: `LOCATIONS` ≤ 25 entries
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'cargo check -p slicer-ir --all-targets 2>&1 | rg -q "E0063" && echo FAIL || echo PASS'` - FACT
  - `bash -c 'cargo check -p slicer-core --all-targets 2>&1 | rg -q "E0063" && echo FAIL || echo PASS'` - FACT
  - `bash -c 'cargo check -p slicer-macros --all-targets 2>&1 | rg -q "E0063" && echo FAIL || echo PASS'` - FACT
  - `bash -c 'rg -q "overhang_distance_mm: start.overhang_distance_mm" crates/slicer-core/src/lib.rs && echo PASS || echo "FAIL: interpolate_point sets the new field to None instead of inheriting from start - interpolated points would lose their distance"'` - FACT
- Exit condition: four PASS lines. No `E0063` naming `overhang_distance_mm` remains in these three crates.

### Step 4: Carry the previous-layer boundary from the producer to `SurfaceClassificationIR`, and add the distance helper

- Task IDs: `TASK-314`
- Objective: add `#[serde(default)] pub prev_layer_boundaries: HashMap<u32, Vec<ExPolygon>>` to `SurfaceClassificationIR` keyed by **global** layer index; take the additive minor bump on `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION`; have `annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`) return the previous-layer contours it already computes for the diff; have `commit_overhang_annotation_builtin` (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`) populate the map; add `signed_distance_to_boundary` to `crates/slicer-core/src/perimeter_utils.rs` with its unit convention stated in its doc-comment, and extend `expolygon_to_path3d` with the boundary parameter and the stamped field, updating **every** caller in the same step.
- Precondition: Step 3 complete.
- Postcondition: `AC-4`, `AC-5` and `AC-N1` print PASS; `AC-N2`'s `BAND_BOUNDARY_MULTIPLIERS` conjunct still prints PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/overhang_annotation.rs` (long) - locate `pub fn annotate_overhangs` and the diff it performs; open ±50 lines. Do **not** touch the `const BAND_BOUNDARY_MULTIPLIERS` declaration.
  - `crates/slicer-core/src/perimeter_utils.rs` - locate `expolygon_to_path3d`, open ±60 lines
  - `crates/slicer-ir/src/slice_ir.rs` (very long) - locate `pub struct SurfaceClassificationIR`, `pub struct QuartileBand` and `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION`, ±30 lines each
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` - read whole; short and it is the commit point
  - `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs` (long) - locate its `expolygon_to_path3d` mirror helper only; its own doc-comment records that it drives the **real** function, so the helper must move with the signature
- Files allowed to edit (this step edits the producer chain; splitting it leaves the workspace non-compiling mid-step):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-core/src/algos/overhang_annotation.rs`
  - `crates/slicer-core/src/perimeter_utils.rs`
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`
  - `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs` (caller update only)
- Files explicitly out of bounds:
  - `const BAND_BOUNDARY_MULTIPLIERS` and every band-geometry expression (`AC-N2`)
  - `crates/slicer-schema/wit/**` (Step 5)
  - `modules/**` (Step 6)
- Expected sub-agent dispatches:
  - Question: "In `crates/slicer-core/src/algos/overhang_annotation.rs`, does `annotate_overhangs` already hold the previous layer's region polygons at the point it computes the diff, and in what coordinate units?"; scope: that file; return: `FACT` ≤ 5 lines
  - Question: "List every caller of `slicer_core::perimeter_utils::expolygon_to_path3d` in `crates/` and `modules/`."; scope: workspace; return: `LOCATIONS` ≤ 10 entries
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0031-overhang-classification-in-prepass.md` - read whole; its in-body amendment explicitly preserves "the `SurfaceClassificationIR` extension shape", which is the clause this step extends
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - `ExtrusionQualityEstimator::estimate_extrusion_quality`, for the single fact that `unscaled_prev_layer` is built from the previous layer's **slice boundary** and not its extrusion paths - delegate; never load
- Verification:
  - the `AC-4` command - FACT (the signedness contract; **the packet's primary criterion**)
  - the `AC-5` command - FACT
  - the `AC-N1` command - FACT (`None`, never `Some(0.0)`/`Some(-1.0)`/`Some(f32::MAX)`)
  - the `AC-N2` command - FACT (the quartile must not move and the band declaration must match `HEAD`)
- Exit condition: four PASS lines. If `AC-N2` goes red, the fix is to revert the band-geometry edit — **never** to weaken the probe.

### Step 5: WIT accessor, host marshalling, and the `slicer-wasm-host` real edits

- Task IDs: `TASK-314`
- Objective: add `prev-layer-boundary: func() -> list<ex-polygon>` to `resource slice-region-view` (`crates/slicer-schema/wit/deps/ir-types.wit`), beside the existing `overhang-quartile-polygons`; marshal both it and the new `point3-with-width` field in `crates/slicer-wasm-host/src/marshal/{in_,out,leaf}.rs`. Then rebuild guests.
- Precondition: Step 4 complete.
- Postcondition: `AC-7` and `AC-9` print PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/ir-types.wit` - locate `resource slice-region-view`, read that resource whole
  - `crates/slicer-wasm-host/src/marshal/in_.rs`, `out.rs`, `leaf.rs` - locate every `dist_to_top_mm` and `overhang_quartile_polygons` occurrence and open ±20 lines around each
  - `CLAUDE.md` §"WIT/Type Changes Checklist" and §"Guest WASM Staleness" - read both sections
- Files allowed to edit (the WIT plus its three conversions; the chain cannot be split without leaving the workspace non-compiling mid-step):
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
  - `crates/slicer-wasm-host/src/marshal/out.rs`
  - `crates/slicer-wasm-host/src/marshal/leaf.rs`
- Files explicitly out of bounds:
  - `modules/**` (Step 6)
  - any `.wit` file other than `ir-types.wit` (`types.wit` was Step 2's; do not revisit it here)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - **These three marshal files appear in the sweep's proxy count and are deliberately excluded from every blind step.** They are real WIT↔IR conversions: a mechanical `overhang_distance_mm: None,` inserted here compiles and then **silently drops the field across the guest boundary** in one or both directions. That failure is invisible to `E0063` and to every unit test in this packet — the only thing that would catch it is packet 190 finding `None` on every point. Convert the field in both directions and assert it in the round trip.
  - Whatever `E0063` remains under `crates/slicer-wasm-host/` after this step belongs to test files in that crate and is swept in Step 6, not patched here.
- Expected sub-agent dispatches:
  - Question: "In `crates/slicer-wasm-host/src/marshal/`, which file converts `point3-with-width` in each direction, and which converts `slice-region-view`'s accessors?"; scope: `crates/slicer-wasm-host/src/marshal/**`; return: `LOCATIONS` ≤ 10 entries
  - Question: "Run `cargo xtask build-guests` then `cargo xtask build-guests --check`; report whether `--check` prints any `STALE:` line and, if so, the first five."; scope: workspace; return: `FACT` ≤ 6 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated grep for the `slice-region-view` accessor list only
- OrcaSlicer refs:
  - none for this step
- Verification:
   - `bash -c 'python3 -c "import io,re; p=r\"crates/slicer-ir/src/slice_ir.rs\"; q=r\"target/pin-surface-schema-before.txt\"; s=io.open(p,encoding=\"utf-8\").read(); before=tuple(map(int,io.open(q,encoding=\"utf-8\").read().strip().split(\".\"))); k=\"CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION: SemVer\"; b=s[s.index(k):s.index(k)+200]; now=tuple(int(re.search(name+r\":\\s*(\\d+)\",b).group(1)) for name in (\"major\",\"minor\",\"patch\")); print(\"PASS\" if now[0]==before[0] and now[1]>before[1] and now[2]==0 else \"FAIL: CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION did not take an additive minor bump\")"'` - FACT (compares the live surface constant with the Step-0 pin: same major, strictly greater minor, patch 0)
   - the `AC-7` command - FACT (all six links of the producer→guest path)
  - the `AC-9` command - FACT (rebuild without `--check` if it reports `STALE:`, then re-run)
  - `bash -c 'cargo check -p slicer-wasm-host 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT
 - Exit condition: four PASS lines. **Do not proceed to Step 6 while `--check` reports `STALE:`** — every later component, dispatch or module test failure would be unattributable, and `CLAUDE.md` forbids attributing it to anything else until `--check` is clean.

### Step 6: Struct-literal sweep, group B — `slicer-wasm-host`, `slicer-sdk`, `slicer-gcode`, `pnp-cli`

- Task IDs: `TASK-314`
- Objective: same mechanical `overhang_distance_mm: None,` insertion across this group.
- Precondition: Step 5 complete; `--check` clean.
- Postcondition: `cargo check` on all four crates with `--all-targets` reports no `E0063`.
- Files allowed to read, with ranges when over 300 lines:
  - each file in the group - locate `Point3WithWidth {` and open ±15 lines only
- Files allowed to edit (this step deliberately exceeds three; see §Execution Rules):
  - every `.rs` file under `crates/slicer-wasm-host/`, `crates/slicer-sdk/`, `crates/slicer-gcode/` and `crates/pnp-cli/` that Step 0's re-derivation lists
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/marshal/{in_,out,leaf}.rs` — **already handled with real edits in Step 5.** If any of them still shows `E0063`, stop: it means Step 5's conversion was incomplete, which is a design signal rather than a sweep miss.
  - `crates/slicer-runtime/**` and `modules/**` (Steps 7-9)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Re-derive the group's file list; the `E0063` set is the authority.
  - `crates/slicer-sdk/src/test_support/{fixtures,assert_paths}.rs` are **fixture builders**, not one-off literals. A `None` there is correct, but check whether either exposes a builder method per `Point3WithWidth` field — if so, packets 190 and 191 will need a way to set the new field from a test, and adding that setter here is cheaper than a follow-up.
- Expected sub-agent dispatches:
  - Question: "Run Step 0's re-derivation command restricted to `crates/slicer-wasm-host`, `crates/slicer-sdk`, `crates/slicer-gcode` and `crates/pnp-cli`, and return the `count path` lines."; scope: those four crates; return: `LOCATIONS` ≤ 40 entries
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'for p in slicer-wasm-host slicer-sdk slicer-gcode pnp-cli; do cargo check -p $p --all-targets 2>&1 | rg -q "E0063" && echo "FAIL: $p"; done; echo "swept"'` - FACT (prints one `FAIL: <crate>` line per unswept crate, then `swept`; a lone `swept` is the pass)
  - `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: golden_emit_tdd moved - this packet has no consumer and must move no G-code"'` - FACT
- Exit condition: a lone `swept` line and one PASS. The golden check here is early on purpose: `slicer-gcode` is the first swept crate that could move output, and catching it now localises the cause to this group.

### Step 7: Struct-literal sweep, group C — `slicer-runtime` `executor` bucket and crate `src`

- Task IDs: `TASK-314`
- Objective: same mechanical insertion across `crates/slicer-runtime/src/`, `crates/slicer-runtime/tests/*.rs` (the top-level, un-bucketed test files) and the `crates/slicer-runtime/tests/executor/` bucket — the single densest group in the sweep.
- Precondition: Step 6 complete.
- Postcondition: `cargo check -p slicer-runtime --all-targets` reports no `E0063` originating in these paths.
- Files allowed to read, with ranges when over 300 lines:
  - each file in the group - locate `Point3WithWidth {` and open ±15 lines only
- Files allowed to edit (this step deliberately exceeds three; see §Execution Rules):
  - every `.rs` file under `crates/slicer-runtime/src/`, `crates/slicer-runtime/tests/` (top level) and `crates/slicer-runtime/tests/executor/` that Step 0's re-derivation lists
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/{contract,integration,unit,e2e,common,fixtures}/**` (Step 8)
  - `modules/**` (Steps 9a-9b)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Re-derive the group's file list; the `E0063` set is the authority.
  - `crates/slicer-runtime/src/layer_executor.rs` is **production** code, not a test fixture. Its `Point3WithWidth` constructions already set `overhang_quartile: None` explicitly; `overhang_distance_mm: None` beside them is correct **only** if those constructions are genuinely synthetic points with no classification. Check each one rather than inserting blind — a synthesised point that should have inherited a neighbour's distance is a defect this sweep can introduce and no test in this packet would catch.
- Expected sub-agent dispatches:
  - Question: "Run Step 0's re-derivation command restricted to `crates/slicer-runtime/src`, `crates/slicer-runtime/tests/*.rs` and `crates/slicer-runtime/tests/executor`, and return the `count path` lines."; scope: those paths; return: `LOCATIONS` ≤ 25 entries
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test executor 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the executor bucket did not build or did not pass"'` - FACT
  - `bash -c 'cargo check -p slicer-runtime --all-targets 2>&1 | rg "E0063" | rg -q "tests(/|\\\\)(contract|integration|unit|e2e|common|fixtures)" && echo "REMAINING: only Step 8 paths, as expected" || (cargo check -p slicer-runtime --all-targets 2>&1 | rg -q "E0063" && echo "FAIL: E0063 outside Step 8 paths" || echo PASS)'` - FACT
- Exit condition: the executor bucket is green and any remaining `E0063` in the crate lies only in Step 8's paths.

### Step 8: Struct-literal sweep, group D — `slicer-runtime` `contract`, `integration`, `unit`, `e2e`, `common`, `fixtures` buckets

- Task IDs: `TASK-314`
- Objective: same mechanical insertion across the remaining `slicer-runtime` test buckets.
- Precondition: Step 7 complete.
- Postcondition: `cargo check -p slicer-runtime --all-targets` reports **zero** `E0063`.
- Files allowed to read, with ranges when over 300 lines:
  - each file in the group - locate `Point3WithWidth {` and open ±15 lines only
- Files allowed to edit (this step deliberately exceeds three; see §Execution Rules):
  - every `.rs` file under `crates/slicer-runtime/tests/{contract,integration,unit,e2e,common,fixtures}/` that Step 0's re-derivation lists
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs`'s mirror helper — **already updated in Step 4** with the signature. Its remaining literals are in scope here; its helper is not to be re-touched.
  - `crates/slicer-runtime/tests/fixtures/**` recorded JSON baselines — **never re-record a fixture to make a test pass.** The `.rs` files under that directory are sweep targets; the `.json` files are not.
  - `modules/**` (Steps 9a-9b)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Re-derive the group's file list; the `E0063` set is the authority.
  - `crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs` carries an explicit **TRIPWIRE** doc-comment obliging its `mirrored_run_finalization` to track the module's per-entity rule. **This packet does not change that rule** — it adds a field the module does not read — so the mirror needs only the mechanical field insertion, not a rule update. Packets 190 and 191 own the rule update. Say so in the commit message so a reviewer does not read the touched file as an unreported behaviour change.
- Expected sub-agent dispatches:
  - Question: "Run Step 0's re-derivation command restricted to `crates/slicer-runtime/tests/{contract,integration,unit,e2e,common,fixtures}`, and return the `count path` lines."; scope: those paths; return: `LOCATIONS` ≤ 30 entries
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'cargo check -p slicer-runtime --all-targets 2>&1 | rg -q "E0063" && echo "FAIL: E0063 remains in slicer-runtime" || echo PASS'` - FACT
  - `bash -c 'cargo test -p slicer-runtime --test integration -- overhang_pipeline_e2e_tdd:: 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the overhang e2e pair regressed"'` - FACT
- Exit condition: two PASS lines. Zero `E0063` in `slicer-runtime`.

### Step 9a: Struct-literal sweep, group E — `modules/core-modules`, infill and path family

- Task IDs: `TASK-314`
- Objective: same mechanical insertion across `fuzzy-skin`, `gyroid-infill`, `infill-linker`, `lightning-infill`, `overhang-classifier-default`, `part-cooling`, `path-optimization-default` and `rectilinear-infill`.
- Precondition: Step 8 complete.
- Postcondition: `cargo check` on each of these module crates with `--all-targets` reports no `E0063`.
- Files allowed to read, with ranges when over 300 lines:
  - each file in the group - locate `Point3WithWidth {` and open ±15 lines only
- Files allowed to edit (this step deliberately exceeds three; see §Execution Rules):
  - every `.rs` file under those eight module directories that Step 0's re-derivation lists
- Files explicitly out of bounds:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` **beyond the single mechanical literal insertion.** Its algorithm is packet 190's exclusive surface; this step may add the one field initialiser and nothing else. If the file needs any other change to compile, stop — that is a design signal, not a sweep task.
  - the remaining module directories (Step 9b)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Re-derive the group's file list; the `E0063` set is the authority.
  - **These are guest-WASM build inputs.** `cargo xtask build-guests --check` must be re-run after this step and Step 9b together (Step 10 does it); do not interpret any module-dispatch result in between.
- Expected sub-agent dispatches:
  - Question: "Run Step 0's re-derivation command restricted to those eight module directories, and return the `count path` lines."; scope: `modules/core-modules/{fuzzy-skin,gyroid-infill,infill-linker,lightning-infill,overhang-classifier-default,part-cooling,path-optimization-default,rectilinear-infill}`; return: `LOCATIONS` ≤ 20 entries
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'for p in fuzzy-skin gyroid-infill infill-linker lightning-infill overhang-classifier-default part-cooling path-optimization-default rectilinear-infill; do cargo check -p $p --all-targets 2>&1 | rg -q "E0063" && echo "FAIL: $p"; done; echo "swept"'` - FACT (a lone `swept` is the pass)
  - `bash -c 'git diff --stat -- modules/core-modules/overhang-classifier-default/src/lib.rs | rg -q "1 insertion" && echo PASS || echo "FAIL: overhang-classifier-default/src/lib.rs changed by more than the single field initialiser - that file is packet 190 exclusive surface"'` - FACT
- Exit condition: a lone `swept` line and one PASS.

### Step 9b: Struct-literal sweep, group F — `modules/core-modules`, seam, support and tower family

- Task IDs: `TASK-314`
- Objective: same mechanical insertion across `seam-placer`, `seam-planner-default`, `skirt-brim`, `support-planner`, `support-surface-ironing`, `top-surface-ironing`, `traditional-support`, `tree-support` and `wipe-tower`; plus the two perimeter modules' literals (their *stamping* changes are Step 6-adjacent and land here only as literals if any remain).
- Precondition: Step 9a complete.
- Postcondition: `cargo check --workspace --all-targets` reports **zero** `E0063` workspace-wide — `AC-8` prints PASS.
- Files allowed to read, with ranges when over 300 lines:
  - each file in the group - locate `Point3WithWidth {` and open ±15 lines only
- Files allowed to edit (this step deliberately exceeds three; see §Execution Rules):
  - every `.rs` file under those module directories that Step 0's re-derivation lists
- Files explicitly out of bounds:
  - `modules/core-modules/overhang-classifier-default/**` (Step 9a, already done and capped)
  - every `src/` file already swept in Steps 3-9a
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Re-derive; the `E0063` set is the authority. `AC-8` — a **workspace-wide** zero-`E0063` assertion — is this step's exit and the sweep's completion signal.
  - `modules/core-modules/seam-placer/` carries the largest file count in this group and its literals sit in test fixtures constructing wall loops. A `None` is correct there; the field is not read by seam placement.
- Expected sub-agent dispatches:
  - Question: "Run Step 0's re-derivation command restricted to the remaining `modules/core-modules` directories, and return the `count path` lines."; scope: `modules/core-modules`; return: `LOCATIONS` ≤ 30 entries
  - Question: "Does `cargo check --workspace --all-targets` still report any `E0063`? Return only the distinct file paths."; scope: workspace; return: `FACT` plus ≤ 20 paths
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - the `AC-8` command - FACT (workspace-wide zero `E0063`; the sweep's completion signal)
  - `bash -c 'cargo check --workspace --all-targets 2>&1 | rg -q "^error" && echo "FAIL: workspace check is red for some reason other than E0063" || echo PASS'` - FACT
- Exit condition: two PASS lines. Zero `E0063` workspace-wide.

### Step 10: Stamp the distance at both perimeter sites, rebuild guests, and sweep the regression wall

- Task IDs: `TASK-314`
- Objective: pass the previous-layer boundary through `modules/core-modules/classic-perimeters/src/lib.rs` into `expolygon_to_path3d`, and stamp `overhang_distance_mm` in `modules/core-modules/arachne-perimeters/src/lib.rs` **outside** its `if !overhang_bands.is_empty()` guard; rebuild the guests; run the full regression sweep including clippy.
- Precondition: Step 9b complete; `AC-8` green.
- Postcondition: `AC-6`, `AC-9` and `AC-10` print PASS and `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/classic-perimeters/src/lib.rs` (long) - locate `region.overhang_quartile_polygons()` and the `expolygon_to_path3d` call, ±30 lines each
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (long) - locate `region.overhang_quartile_polygons()` and `if !overhang_bands.is_empty()`, ±30 lines each
  - `target/test-output.log` - on failure only, via `Grep` for `FAILED|panicked at|---- .* stdout ----` with `-C 5`; never re-run a test to see more output
- Files allowed to edit (at most 3):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
- Files explicitly out of bounds:
  - every file already swept; a red test here is a design signal for Step 4 or 5, not something to patch in place
  - `crates/slicer-runtime/tests/fixtures/**` recorded baselines — never re-record to make a test pass
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - The `AC-6` structural conjunct is the point of this step: the assignment must **not** be nested inside the band guard. The two signals have different availability, and a region with a boundary but no bands — a region where nothing overhangs — is exactly the population packet 190 must interpolate a fast speed for. A test alone cannot see the nesting; the probe measures byte offsets against the guard's matched braces.
- Expected sub-agent dispatches:
  - Question: "Run `cargo xtask build-guests` then `cargo xtask build-guests --check`; report whether `--check` prints any `STALE:` line."; scope: workspace; return: `FACT` ≤ 3 lines
  - Question: "Run the five verification commands below and return only their PASS/FAIL lines."; scope: workspace; return: `FACT` ≤ 6 lines
- Context cost: `M` (a full guest rebuild plus five cargo runs; none of their output enters the implementer's context)
- Authoritative docs:
  - `CLAUDE.md` §"Guest WASM Staleness" - the mandatory `--check` procedure
- OrcaSlicer refs:
  - none for this step
- Verification:
  - the `AC-6` command - FACT (test half plus the not-nested-inside-the-guard structural half)
  - the `AC-9` command - FACT (rebuild without `--check` if it reports `STALE:`, then re-run)
  - the `AC-10` command - FACT (`slicer-core` + `slicer-ir` + the three largest runtime buckets)
  - `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the golden moved - this packet has no consumer and must move no G-code"'` - FACT
  - `bash -c 'cargo clippy --workspace --all-targets -- -D warnings 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT
- Exit condition: five PASS lines. Do not interpret any module-dispatch or component failure before `--check` is clean.

### Step 11: Docs, the new `DEV-###` row, and the TASK registration

- Task IDs: `TASK-314`
- Objective: land every entry in `packet.spec.md` §Doc Impact Statement — the `docs/02_ir_schemas.md` IR-7 and `SurfaceClassificationIR` edits, the `docs/03_wit_and_manifest.md` entries, the `docs/05_module_sdk.md` accessor row, one new `DEV-###` row for the per-point-vs-per-path `boundary_offset` divergence, and the TASK registration in `docs/07_implementation_status.md` outside the generated block.
- Precondition: Step 10 complete; all code ACs green.
- Postcondition: every doc verification command in `packet.spec.md` §Doc Impact Statement returns PASS, and `DEV-009` is still `Open`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` (long; a line count is a ledger fact — do not pin one) - §"IR 7 — PerimeterIR" through the start of §"IR 8", plus the `SurfaceClassificationIR` block, located by grep
  - `docs/03_wit_and_manifest.md` - the `point3-with-width` entry and the `slice-region-view` accessor list only, located by grep
  - `docs/05_module_sdk.md` - the "SliceRegionView accessors" section only, located by grep
  - `docs/DEVIATION_LOG.md` - **delegate**; only the highest `DEV-###` and confirmation that `DEV-009` is still `Open`
  - `docs/07_implementation_status.md` - **delegate**; only the highest `TASK-###` and the generated-block markers
- Files allowed to edit (this step edits five docs; each is a distinct, independently-verified anchor):
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - the `DEV-009` row itself — **must not be touched by this packet**; packets 190 and 191 own its progress paragraph and its closure
  - everything inside `<!-- BEGIN GENERATED: open-deviations … -->` / `<!-- END GENERATED: open-deviations -->` in `docs/07_implementation_status.md` — regenerated by `cargo xtask check-deviations`, never hand-edited
- Expected sub-agent dispatches:
  - Question: "Re-derive the highest `DEV-###` with `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`; re-derive the highest `TASK-###` with BOTH `rg -o 'TASK-[0-9]{3}' docs/07_implementation_status.md | sort -u | tail -1` and `rg -o 'TASK-[0-9]{3}' .ralph/specs --no-filename | sort -u | tail -1`, and report both."; scope: those paths; return: `FACT` ≤ 5 lines. **Re-derive at the moment of writing.** The two TASK sources disagree by design — the specs tree runs ahead of `docs/07` because several packets in this batch allocated ids they have not registered. Take the next free number above the higher. A `DEV-###` captured earlier in the session will collide: sibling packets in this queue file rows concurrently.
  - Question: "Quote the `pub struct Point3WithWidth` code block and the `SurfaceClassificationIR` struct block from `docs/02_ir_schemas.md` verbatim."; scope: that file; return: `SNIPPETS` ≤ 2, ≤ 30 lines each
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - ranged read as above
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` - delegated `FACT` only
- OrcaSlicer refs:
  - none for this step
- Verification:
  - the `AC-11` command - FACT (IR-7 must name `overhang_distance_mm`, `signed` and `boundary_offset`; **`signed` alone is non-discriminating** — measured, it already occurs in that section today, which is why the probe requires all three)
  - `bash -c 'rg -q "prev_layer_boundaries" docs/02_ir_schemas.md && rg -q "overhang-distance-mm" docs/03_wit_and_manifest.md && rg -q "prev-layer-boundary" docs/03_wit_and_manifest.md && rg -q "prev_layer_boundary" docs/05_module_sdk.md && echo PASS || echo "FAIL: one of the four doc anchors is missing"'` - FACT
  - `bash -c 'rg -q "boundary_offset" docs/DEVIATION_LOG.md && rg -q "variable-width" docs/DEVIATION_LOG.md && echo PASS || echo "FAIL: no row records the per-point vs per-path boundary_offset divergence"'` - FACT
  - the §Doc Impact Statement TASK-registration probe - FACT (**splits on the generated-block markers rather than grepping the whole file.** A bare grep cannot tell an outside row from one that landed inside the block and will be destroyed by the next `cargo xtask check-deviations`; measured, `TASK-156` occurs both inside and outside that block on this tree today.)
  - `bash -c 'rg -q "^\| DEV-009 .*Open" docs/DEVIATION_LOG.md && echo PASS || echo "FAIL: DEV-009 was flipped; this packet must not close it"'` - FACT
- Exit condition: five PASS lines.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Read-only; writes two `target/` pin files and re-derives the sweep sizing (a ledger fact — swarm log only) |
| Step 1 | M | Three new test binaries; red-by-non-compilation is the expected state |
| Step 2 | S | Two files; the field, the WIT record, the perimeter bump |
| Step 3 | M | Sweep group A — `slicer-ir`, `slicer-core`, `slicer-macros`; one non-mechanical site (`interpolate_point`) |
| Step 4 | M | The producer chain plus the distance helper and the `expolygon_to_path3d` signature — the design core |
| Step 5 | M | WIT accessor plus three **real** marshal edits; full guest rebuild |
| Step 6 | M | Sweep group B — `slicer-wasm-host`, `slicer-sdk`, `slicer-gcode`, `pnp-cli` |
| Step 7 | M | Sweep group C — `slicer-runtime` `src` + `executor`; the densest single group |
| Step 8 | M | Sweep group D — the remaining `slicer-runtime` buckets |
| Step 9a | M | Sweep group E — infill/path module family |
| Step 9b | M | Sweep group F — seam/support/tower module family; `AC-8` is its exit |
| Step 10 | M | Both stamping sites, guest rebuild, regression sweep, clippy |
| Step 11 | S | Five docs, each with an independently-verified anchor |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate: `M`, **at the top of the band** — `design.md` §Open Questions records the 193a/193b split seam and states that splitting there is the correct escalation if a swarm run hits the context band before Step 9b, rather than compressing scope.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS — including the do-not-regress guards `AC-9` and `AC-10`, which were already PASS before the packet started and whose value is entirely in still being PASS after it.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read. Re-derive the TASK id from **both** sources first (see Step 11).
- Reconcile reopened/superseded status transitions: **none from this packet directly.** It supersedes nothing and reopens nothing; it is additive on both schemas and has no consumer. The supersession this packet *enables* — `ADR-0053`'s amendment of ADR-0031, ADR-0032 and ADR-0008 under the maintainer's option (C) ruling — is filed by packets 190 and 191, not here. Do not file an amendment row for this packet; do not let its absence be read as the set being empty.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Re-run `cargo xtask build-guests --check` immediately before closure; a `STALE:` at closure invalidates every component and module test result collected during the packet, and this packet edited two files under `crates/slicer-schema/wit/`.
- Confirm `target/pin-perimeter-schema-before.txt` and `target/pin-surface-schema-before.txt` still hold the **pre-packet** values before grading `AC-2` at closure. If `target/` was cleaned mid-packet, re-pin from `git show HEAD~<n>:crates/slicer-ir/src/slice_ir.rs` at the packet's base commit — **not** from the working tree, which would make the assertion vacuous by construction.
- Record remaining packet-local risk explicitly: the carrier is **unexercised by any live consumer** until packet 190 lands, so nothing in this packet proves the field survives the WASM round trip in anger beyond `AC-7`, `AC-9` and the runtime buckets in `AC-10`. State that at closure rather than implying end-to-end coverage. Record also that the per-point-vs-per-path `boundary_offset` divergence is a filed `DEV-###`, not a resolved question.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson — and if the escalation was triggered by the sweep, record the 193a/193b split seam as the concrete remedy.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
