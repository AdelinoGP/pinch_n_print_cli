# Implementation Plan: 189-per-point-speed-factor-carrier

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Steps 3 and 4 exceed the usual three-file edit limit by design. `.claude/skills/spec-packet-generator/SKILL.md` §Packet Safety requires the step that adds a struct field to own the struct-literal blast radius rather than leaving it for a follow-up `cargo check`; those two steps are that ownership, split by crate group so neither rates `L`.

## Steps

### Step 0: Pin the pre-packet schema version and re-derive the sweep sizing

- Task IDs: `TASK-308`
- Objective: capture the two figures `AC-2` and Steps 3-4 are graded against **as measurements taken now**, not as numbers quoted from this document. (a) Write the live `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` triple to `target/pin-layer-collection-schema-before.txt` in bare `MAJOR.MINOR.PATCH` form; `AC-2` asserts an additive minor bump **relative to that pin** rather than against a hardcoded SemVer, because the constant is mutable shared state that any sibling packet adding a `LayerCollectionIR` field will move. (b) Re-run the `LayerCollectionIR` literal-site census and record the per-crate counts in the swarm log. Both outputs are **ledger facts** and both live outside tracked files on purpose — the pin in `target/` (untracked, so it cannot rot into the repo), the census in the log.
- Precondition: tree is at `deviations-fix` HEAD; `target/` exists (`mkdir -p target`).
- Postcondition: `target/pin-layer-collection-schema-before.txt` exists and holds exactly one `\d+\.\d+\.\d+` line. No tracked file is modified by this step.
- Files to read (with any range limits):
  - `crates/slicer-ir/src/slice_ir.rs` (very long) - locate `pub const CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` only; do not read the file whole
- Files allowed to edit:
  - `target/pin-layer-collection-schema-before.txt` (scratch, untracked)
- Files explicitly out of bounds:
  - every tracked file in the workspace — this step measures and writes scratch only
- Expected sub-agent dispatches:
  - none; both commands are one-liners and their output is the deliverable
- Context cost: `S`
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'mkdir -p target && python3 -c "import io,re,sys; s=io.open(r\"crates/slicer-ir/src/slice_ir.rs\",encoding=\"utf-8\").read(); k=\"CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION: SemVer\"; sys.exit(print(\"FAIL: constant not found\")) if k not in s else None; b=s[s.index(k):s.index(k)+140]; g=lambda t: re.search(t+r\": (\\d+)\",b).group(1); io.open(r\"target/pin-layer-collection-schema-before.txt\",\"w\").write(\"%s.%s.%s\"%(g(\"major\"),g(\"minor\"),g(\"patch\"))); print(\"PASS: pinned \"+io.open(r\"target/pin-layer-collection-schema-before.txt\").read())"'` - FACT (**the `AC-2` pin.** If `python3` resolves to the Microsoft Store alias stub — measured on this tree, where both `python` and `python3` on `PATH` are that stub and print an install prompt — install a real interpreter or read the four lines following the constant and write the file by hand. **Do not respond by hardcoding a SemVer back into `AC-2`.**)
  - `bash -c 'rg -c "LayerCollectionIR\s*\{" --glob "*.rs" crates modules xtask | sort -t: -k2 -rn | head -30'` - FACT (the literal-site census; record the output in the swarm log, **never** in a tracked file)
- Exit condition: the pin file exists and holds a single `MAJOR.MINOR.PATCH` line, and the census output is in the log. Neither figure is copied into `packet.spec.md`, `design.md`, `requirements.md` or this file.

### Step 1: Red tests for the carrier, the applier, and the emitter

- Task IDs: `TASK-308`
- Objective: write the six failing tests that define the contract before any production code exists — `modify_entity_set_point_speed_factors_applies`, `modify_entity_set_point_speed_factors_length_mismatch_errors` and a `speed_profiles`-stays-empty assertion appended to `modify_entity_set_speed_factor_applies` (all in `crates/slicer-sdk/tests/finalization_builder_tdd.rs`), plus `per_point_speed_profile_varies_f_within_one_entity`, `per_point_speed_profile_indexes_original_points_after_simplification` and `unprofiled_entity_in_a_profiled_layer_keeps_whole_entity_speed` (all three in `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`).
- Precondition: tree is at `deviations-fix` HEAD; `cargo test -p slicer-sdk --test finalization_builder_tdd` reports 11 passed and `cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd` reports 9 passed (both measured green on the unfixed tree — re-derive the counts, do not trust these).
- Postcondition: the two test files reference `EntityMutation::SetPointSpeedFactors`, `EntitySpeedProfile` and `speed_profiles`, so both binaries fail to **compile**. A non-compiling test binary is the correct red state for this step; do not stub the types to make it link.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` - locate `modify_entity_set_speed_factor_applies` and `modify_entity_set_flow_factor_applies`, open ±40 lines around each
  - `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` - locate `speed_factor_modulates_role_speed`, open ±60 lines (it is the closest existing shape: it builds a `LayerCollectionIR`, runs `emit_gcode`, and reads `f` off the emitted `Move`s)
  - `crates/slicer-gcode/src/emit.rs` - locate `simplified_points` and `drop_short_segments_mm`, open ±40 lines, to construct a path whose interior point is provably dropped
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs`
  - `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`
- Files explicitly out of bounds:
  - every `src/` file in the workspace (this is a test-only step)
  - `modules/core-modules/**`
- Expected sub-agent dispatches:
  - Question: "What `min_segment_length` and point spacing make `drop_short_segments_mm` drop exactly the second of four points in `emit_gcode`? Return the resolved-config field name and a worked example."; scope: `crates/slicer-gcode/src/emit.rs`, `crates/slicer-core/**` for the helper; return: `FACT` ≤ 5 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/05_module_sdk.md` - delegated grep for the `modify_entity` variant table only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::_extrude` per-segment `F` emission - delegate; never load
- Verification:
  - `bash -c 'cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 | rg -q "cannot find|no variant|E0433|E0599" && echo "RED as expected" || echo "FAIL: tests compiled — the new symbols must not exist yet"'` - FACT
  - `bash -c 'cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd 2>&1 | rg -q "cannot find|no field|E0433|E0609" && echo "RED as expected" || echo "FAIL: tests compiled — the new symbols must not exist yet"'` - FACT
- Exit condition: both binaries fail to compile for the stated missing-symbol reason and for no other reason. If either fails for an unrelated reason, stop and diagnose before proceeding.

### Step 2: Add `EntitySpeedProfile`, the `speed_profiles` field, and the schema bump

- Task IDs: `TASK-308`
- Objective: add `pub struct EntitySpeedProfile { pub entity_id: u64, pub factors: Vec<f32> }` (deriving the same set `TravelMove` derives), add `#[serde(default)] pub speed_profiles: Vec<EntitySpeedProfile>` to `LayerCollectionIR`, add `speed_profiles: Vec::new()` to its explicit `Default` impl, take the **additive minor bump** on `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` — read Step 0's `target/pin-layer-collection-schema-before.txt`, keep `major`, set `minor` to pinned-minor + 1 and `patch` to `0` — with a comment naming this packet. **Do not read the target version out of any packet file; there is deliberately no literal in any of them** (`AC-2` asserts the relation against Step 0's pin, not a hardcoded SemVer). Also and re-export `EntitySpeedProfile` from `crates/slicer-ir/src/lib.rs`. **There are ZERO `LayerCollectionIR { … }` struct literals to fix in `crates/slicer-ir/src/slice_ir.rs` — an earlier revision of this step said "the two literals" and was wrong.** `rg -n 'LayerCollectionIR\s*\{' crates/slicer-ir/src/slice_ir.rs` returns exactly two hits and both are non-literals: `pub struct LayerCollectionIR {` (the definition, which is where the `#[serde(default)] pub speed_profiles` field is added) and `impl Default for LayerCollectionIR {` (whose body is where `speed_profiles: Vec::new()` is added). Both are already the substance of this step's objective above; nothing extra is owed here.
- Precondition: Step 1 complete; both new test binaries are red for missing symbols.
- Postcondition: `cargo check -p slicer-ir` passes; `cargo check --workspace --all-targets` still fails, and every remaining failure is `E0063` (missing field `speed_profiles`) — that failure list is Step 3/4's worklist.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` (2578 lines) - locate `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`, `pub struct TravelMove`, `pub struct LayerCollectionIR`, `impl Default for LayerCollectionIR`; open ±30 lines around each
  - `crates/slicer-ir/src/lib.rs` - locate the `TravelMove` re-export
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**`, `crates/slicer-sdk/**`, `crates/slicer-wasm-host/**`, `crates/slicer-macros/**` (later steps)
  - `docs/**` (Step 8)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - This step adds a field to `LayerCollectionIR` and bumps a public schema constant. The Step 3 command reports **50 hits across 27 files** (a ledger fact — re-derive it; and see Step 3's over-count note, because that raw figure includes non-literal matches). Two of those hits are the definition and the `Default` impl in this step's own file and are handled by this step's objective; the rest are owned by Steps 3 and 4, which are part of the same packet and must not be deferred past it.
  - The **test-assertion fallout** of the schema bump is `crates/slicer-ir/tests/ir_tests.rs`'s `LayerCollectionIR::default().schema_version must equal CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`. It is a *relative* assertion and survives the bump only if the `Default` impl and the constant move together — which is why both edits are in this step. Verified: no test in the tree hard-asserts the literal `1.1.0` for this IR.
  - A `LOCATIONS` worker was dispatched for the literal sites; its result is transcribed inline in Steps 3 and 4.
- Expected sub-agent dispatches:
  - Question: "Which derives does `pub struct TravelMove` carry in `crates/slicer-ir/src/slice_ir.rs`?"; scope: that file; return: `FACT` ≤ 3 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"IR 10 — LayerCollectionIR" - delegated `SNIPPETS` of the struct block and the `default()` contract paragraph; do not load the file
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'cargo check -p slicer-ir 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT pass/fail
  - `bash -c 'rg -q "pub speed_profiles: Vec<EntitySpeedProfile>" crates/slicer-ir/src/slice_ir.rs && rg -q "speed_profiles: Vec::new\(\)" crates/slicer-ir/src/slice_ir.rs && rg -q "EntitySpeedProfile" crates/slicer-ir/src/lib.rs && python3 -c "import io,os,re,sys; p=r\"crates/slicer-ir/src/slice_ir.rs\"; sys.exit(1) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); m=re.search(r\"pub struct EntitySpeedProfile\s*\{(.*?)\}\", s, re.S); sys.exit(1) if not m else None; b=m.group(1); sys.exit(0 if (\"pub entity_id: u64\" in b and \"pub factors: Vec<f32>\" in b) else 1)" && echo PASS || echo "FAIL: EntitySpeedProfile missing, its fields are not exactly entity_id: u64 / factors: Vec<f32>, or the carrier is not on LayerCollectionIR / not re-exported"'` - FACT (**this is AC-1’s current windowed command, verbatim.** The earlier draft of this step ran a file-wide `bash -c 'rg -q "pub speed_profiles: Vec<EntitySpeedProfile>" crates/slicer-ir/src/slice_ir.rs && rg -q "speed_profiles: Vec::new\(\)" crates/slicer-ir/src/slice_ir.rs && rg -q "EntitySpeedProfile" crates/slicer-ir/src/lib.rs && python3 -c "import io,os,re,sys; p=r\"crates/slicer-ir/src/slice_ir.rs\"; sys.exit(1) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); m=re.search(r\"pub struct EntitySpeedProfile\s*\{(.*?)\}\", s, re.S); sys.exit(1) if not m else None; b=m.group(1); sys.exit(0 if (\"pub entity_id: u64\" in b and \"pub factors: Vec<f32>\" in b) else 1)" && echo PASS || echo "FAIL: EntitySpeedProfile missing, its fields are not exactly entity_id: u64 / factors: Vec<f32>, or the carrier is not on LayerCollectionIR / not re-exported"'`, which already matches today on `TravelMove` and `PrintEntity` and therefore **accepts an `EntitySpeedProfile` declared with `entity_id: u32`** — a struct AC-1 rejects. An implementer would have cleared Step 2 and failed only at the completion gate.)
  - `bash -c 'python3 -c "import io,os,re,sys; p=r\"crates/slicer-ir/src/slice_ir.rs\"; q=r\"target/pin-layer-collection-schema-before.txt\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; sys.exit(print(\"FAIL: \"+q+\" missing - Step 0 did not pin the pre-packet schema version\")) if not os.path.exists(q) else None; s=io.open(p,encoding=\"utf-8\").read(); k=\"CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION: SemVer\"; sys.exit(print(\"FAIL: constant not found\")) if k not in s else None; blk=s[s.index(k):s.index(k)+140]; g=lambda t,b: int(re.search(t+r\": (\\d+)\",b).group(1)); now=(g(\"major\",blk),g(\"minor\",blk),g(\"patch\",blk)); was=tuple(int(x) for x in io.open(q,encoding=\"utf-8\").read().strip().split(\".\")); want=(was[0],was[1]+1,0); print(\"PASS: %d.%d.%d -> %d.%d.%d\"%(was+now) if now==want else \"FAIL: expected additive minor bump %d.%d.%d -> %d.%d.%d, found %d.%d.%d\"%(was+want+now))"'` - FACT (this is AC-2's command, verbatim; it asserts the **relation** against Step 0's pin rather than a hardcoded SemVer)
  - `bash -c 'cargo test -p slicer-ir --test ir_tests 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: ir_tests regressed after the LayerCollectionIR schema bump"'` - FACT (this is AC-9's command; the schema bump's test fallout lands in the same step as the bump)
- Exit condition: AC-1, AC-2 and AC-9 all print PASS, and `cargo check --workspace --all-targets` fails only with `E0063` errors naming `speed_profiles`.

### Step 3: Struct-literal sweep — `slicer-ir`, `slicer-macros`, `slicer-gcode` (7 files here; the 8th, `crates/slicer-ir/src/slice_ir.rs`, is owned by Step 2)

- Task IDs: `TASK-308`
- Objective: add `speed_profiles: Vec::new(),` to every `LayerCollectionIR { … }` literal in this group that does not already end in `..Default::default()`. Do not read the files for comprehension — edit the literal and move on.
- Precondition: Step 2 complete; `cargo check --workspace --all-targets` fails only with `E0063`.
- Postcondition: `cargo check -p slicer-ir --all-targets`, `cargo check -p slicer-macros --all-targets` and `cargo check -p slicer-gcode --all-targets` all pass.
- Files allowed to read, with ranges when over 300 lines:
  - each file below - locate `LayerCollectionIR {` and open ±15 lines only
- Files allowed to edit (this step deliberately exceeds three; see §Execution Rules and `design.md` §Code Change Surface):
  - `crates/slicer-ir/tests/entity_id_invariants_tdd.rs` (2 sites)
  - `crates/slicer-ir/tests/ir_validation_tdd.rs` (2 sites)
  - `crates/slicer-macros/src/lib.rs` (1 site — inside generated guest-facing code; this file is a guest-WASM build input). **Handle this site deliberately, not mechanically:** its `LayerCollectionIR` literal hardcodes `schema_version: ::slicer_ir::SemVer { major: 1, minor: 0, patch: 0 }`, which is **already stale** against the live `1.1.0` and will be two minors behind after Step 2. Adding `speed_profiles: Vec::new(),` beside a wrong `schema_version` propagates the staleness. Either switch it to `::slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` (preferred — it cannot rot again) or update the literal to `minor: 2`, and say which in the commit message.
  - `crates/slicer-gcode/tests/gcode_toolchange_wrapping.rs` (3 sites)
  - `crates/slicer-gcode/tests/finalization_aware_travel_tdd.rs` (2 sites)
  - `crates/slicer-gcode/tests/gcode_emit_travel_anchor_tdd.rs` (2 sites)
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (1 site)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` and `modules/**` (Step 4)
  - every `src/` file not listed above
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - **Re-derive the site list rather than trusting the counts above** (they are a ledger fact measured this session and a parallel edit can move them). The `LOCATIONS` command, verified to run on this tree and to print `50 27` followed by `count path` lines:
    `bash -c 'python3 -c "
    import os,re,io
    rx=re.compile(r\"LayerCollectionIR\s*\{\")
    hits={}
    for root in (\"crates\",\"modules\",\"xtask\"):
        for dp,dn,fn in os.walk(root):
            if \"target\" in dp.split(os.sep): continue
            for f in fn:
                if not f.endswith(\".rs\"): continue
                p=os.path.join(dp,f); s=io.open(p,encoding=\"utf-8\",errors=\"replace\").read()
                for m in rx.finditer(s):
                    i=s.index(chr(123),m.start()); d=0; j=i
                    while j<len(s):
                        if s[j]==chr(123): d+=1
                        elif s[j]==chr(125):
                            d-=1
                            if d==0: break
                        j+=1
                    if \"..Default::default()\" not in s[i:j+1]: hits[p]=hits.get(p,0)+1
    print(sum(hits.values()), len(hits))
    for k in sorted(hits): print(hits[k], k.replace(os.sep,chr(47)))
    "'`
  - **The command over-counts, deliberately erring wide — do not treat its output as a list of edits.** Its regex `LayerCollectionIR\s*\{` matches three things that are **not** struct literals: `pub struct LayerCollectionIR {`, `impl Default for LayerCollectionIR {`, and every `-> [path::]LayerCollectionIR {` return-type brace (the brace-matched body of a function returning the type contains no `..Default::default()`, so it survives the filter). Measured examples on this tree: `crates/slicer-ir/src/slice_ir.rs`'s two hits are the definition and the `Default` impl and hold **zero** literals; `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` and `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` are each listed at "1 site" but hold only a `fn … -> LayerCollectionIR {` signature. **The packet is not under-scoped by this** — the file list is a strict superset of the true set, so following it can only cause a wasted look, never a missed edit.
  - The compiler is the oracle: any site missed here surfaces as `E0063` in this step's verification, never as a runtime surprise. Correspondingly, a listed file with no `E0063` was a return-type false positive and needs no edit.
- Expected sub-agent dispatches:
  - Question: "Run the `LayerCollectionIR` literal-site command above and return the `path count` lines for `crates/slicer-ir`, `crates/slicer-macros` and `crates/slicer-gcode` only."; scope: workspace; return: `LOCATIONS` ≤ 20 entries
- Context cost: `S`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'cargo check -p slicer-ir --all-targets 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT
  - `bash -c 'cargo check -p slicer-macros --all-targets 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT
  - `bash -c 'cargo check -p slicer-gcode --all-targets 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT (this will still fail until Step 6 if the Step 1 red tests reference `speed_profiles_by_entity` behaviour; if so, confirm the only remaining errors come from `gcode_feedrate_emission_tdd.rs` and proceed)
- Exit condition: no `E0063` naming `speed_profiles` remains in these three crates.

### Step 4: Struct-literal sweep — `slicer-runtime` and `modules/core-modules` (19 files)

- Task IDs: `TASK-308`
- Objective: same mechanical edit across the remaining group.
- Precondition: Step 3 complete.
- Postcondition: `cargo check --workspace --all-targets` reports **zero** `E0063` errors naming `speed_profiles`.
- Files allowed to read, with ranges when over 300 lines:
  - each file below - locate `LayerCollectionIR {` and open ±15 lines only
- Files allowed to edit (this step deliberately exceeds three; see §Execution Rules):
  - `crates/slicer-runtime/tests/executor/finalization_live_tdd.rs` (4 sites)
  - `crates/slicer-runtime/tests/integration/gcode_skirt_brim_emission_tdd.rs` (3 sites)
  - `crates/slicer-runtime/tests/unit/layer_collection_builder_tdd.rs` (3 sites)
  - `crates/slicer-runtime/tests/unit/tool_ordering_tdd.rs` (3 sites)
  - `crates/slicer-runtime/tests/contract/postpass_gcode_emit_contract_tdd.rs` (2 sites)
  - `crates/slicer-runtime/tests/executor/finalization_mutation_roundtrip_tdd.rs` (2 sites)
  - `crates/slicer-runtime/tests/executor/finalization_world_deep_copy_tdd.rs` (2 sites)
  - `crates/slicer-runtime/tests/executor/layer_finalization_tdd.rs` (2 sites)
  - `crates/slicer-runtime/tests/executor/macro_finalization_deep_copy_tdd.rs` (2 sites)
  - `crates/slicer-runtime/tests/executor/postpass_executor_tdd.rs` (2 sites)
  - `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` (2 sites)
  - `crates/slicer-runtime/tests/visual_debug_postpass_tap_tdd.rs` (1 site)
  - `crates/slicer-runtime/tests/executor/live_seam_path_tdd.rs` (1 site)
  - `modules/core-modules/part-cooling/tests/part_cooling_tdd.rs` (1 site)
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` (1 site)
  - `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` (1 site)
  - `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` (1 site)
  - `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` (1 site)
  - `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` (1 site)
- Files explicitly out of bounds:
  - every `src/` file in `crates/slicer-runtime` and `modules/core-modules` — the sweep touches test files only in this group; if a `src/` file reports `E0063`, stop: the site list has drifted and must be re-derived
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Re-derive with the Step 3 command before starting; the file list above is an over-broad worklist and the per-file counts are not authoritative. **The same over-count applies here as in Step 3** — the command's regex also matches `-> [path::]LayerCollectionIR {` return-type braces, so several files listed at "1 site" (measured: `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`, `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`) contain only a function signature and need no edit. The `E0063` set from this step's verification is the authority on what actually needs editing.
  - Two of these files (`macro_finalization_deep_copy_tdd.rs`, `finalization_world_deep_copy_tdd.rs`) exercise the finalization deep copy. Adding the field there is still mechanical, but their **pass/fail** result after the sweep is the first signal that the side table survives the finalization round trip; treat a failure in either as a design signal, not a sweep miss.
- Expected sub-agent dispatches:
  - Question: "Run the Step 3 `LayerCollectionIR` literal-site command and return the `path count` lines under `crates/slicer-runtime` and `modules/`."; scope: workspace; return: `LOCATIONS` ≤ 20 entries
  - Question: "Does `cargo check --workspace --all-targets` still report any `E0063`? Return only the distinct file paths."; scope: workspace; return: `FACT` plus ≤ 20 paths
- Context cost: `M` (19 files; each edit is one inserted line and no file is read for comprehension)
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'cargo check --workspace --all-targets 2>&1 | rg -q "E0063" && echo "FAIL: E0063 remains" || echo PASS'` - FACT
- Exit condition: zero `E0063` workspace-wide. Remaining errors, if any, must be attributable to the Step 1 red tests only.

### Step 5: Add the `SetPointSpeedFactors` mutation across the five-file channel and the applier

- Task IDs: `TASK-308`
- Objective: add `set-point-speed-factors(list<f32>)` to `variant entity-mutation` in the WIT; add `EntityMutation::SetPointSpeedFactors(Vec<f32>)` to `crates/slicer-sdk/src/traits.rs`; implement the `apply_to` branch (reject a length mismatch with an `Err` naming both lengths, otherwise upsert an `EntitySpeedProfile` row into the owning layer's `speed_profiles` keyed by `entity_id`, **replacing** any existing row — ADR-0052 §Decision 1 makes the replace-not-append rule normative, and packet 191 depends on it); mirror the variant in `WitEntityMutation` and its `fm::EntityMutation` match arm (`crates/slicer-wasm-host/src/host.rs`), in the `dispatch.rs` translation, and in the `crates/slicer-macros/src/lib.rs` guest-side translation. Then rebuild guests.
- **The new `apply_to` arm CANNOT be written in the shape of the existing ones — it will not compile.** "Resolve the entity exactly as the existing arms do" was the earlier wording here and it is wrong. In `FinalizationOutputBuilder::apply_to`'s `MergeOp::ModifyEntity` arm (`crates/slicer-sdk/src/traits.rs`), the existing code does `let layer = layers.iter_mut().find(…);` and then `let entity = layer.and_then(|l| l.ordered_entities.iter_mut().find(…));`. The `and_then` **moves** the `Option<&mut LayerCollectionIR>`, and the resulting `&mut PrintEntity` keeps the `&mut layers` borrow alive for the whole `match entity` block — so an arm that needs to write `layer.speed_profiles` has neither a `layer` binding left nor a free borrow to make one with. `SetSpeedFactor` and `SetFlowFactor` do not hit this because they only ever touch `e.path`.
- **Corrected shape — capture, drop the borrow, then upsert:**
  1. Find the **layer index** by position, not by mutable reference: `let Some(li) = layers.iter().position(|l| l.global_layer_index == layer_idx) else { return Err(…) };`
  2. Take the entity borrow in a scope that ends, and capture only `Copy`/owned data out of it: `let n = { let Some(e) = layers[li].ordered_entities.iter().find(|e| e.entity_id == entity_id) else { return Err(format!("modify_entity: entity_id {} not found in layer {}", entity_id, layer_idx)) }; e.path.points.len() };` — note `iter()`, not `iter_mut()`: the length check needs no mutable access at all.
  3. Length-check `v.len() == n` and return the `Err` naming both lengths if not. **This is `AC-N1`'s error and it must fire before any mutation**, so the operation is atomic.
  4. Only now upsert, with the entity borrow long gone: `let sp = &mut layers[li].speed_profiles; match sp.iter_mut().find(|p| p.entity_id == entity_id) { Some(p) => p.factors = v, None => sp.push(EntitySpeedProfile { entity_id, factors: v }) }`.
  Keep the existing arms untouched; do **not** refactor them to this shape as a tidy-up — `AC-N2` asserts `SetSpeedFactor` still writes no profile row, and touching those arms is how that regresses.
- **`crates/slicer-macros/src/lib.rs`'s arm needs `.clone()`, and the file's own convention will mislead you.** The two existing arms in the generated `MergeOp::ModifyEntity` translation **deref** their payload — `EntityMutation::SetSpeedFactor(v) => EntityMutation::SetSpeedFactor(*v)` and the identical `SetFlowFactor` line — because `op` is matched by reference and `f32` is `Copy`. A `Vec<f32>` is **not** `Copy`, so `SetPointSpeedFactors(v) => EntityMutation::SetPointSpeedFactors(*v)` fails to compile (`cannot move out of `*v` which is behind a shared reference`). Write `v.clone()`. The clone is per mutated entity per layer and bounded by the entity's own point count; do not try to avoid it by changing how `op` is matched, which would perturb the two arms `AC-N2` guards.
- Precondition: Step 4 complete; workspace compiles apart from the Step 1 red tests.
- Postcondition: `cargo test -p slicer-sdk --test finalization_builder_tdd` is green including the three new/extended tests; `cargo xtask build-guests --check` reports no `STALE:`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit` (under 120 lines) - read whole
  - `crates/slicer-sdk/src/traits.rs` - locate `pub enum EntityMutation` and the `MergeOp::ModifyEntity` arm in `apply_to`, open ±40 lines around each
  - `crates/slicer-wasm-host/src/host.rs` - locate `pub enum WitEntityMutation` and the `fm::EntityMutation::SetSpeedFactor` match arm, open ±20 lines around each
  - `crates/slicer-wasm-host/src/dispatch.rs` - locate `host::WitEntityMutation::SetSpeedFactor`, open ±20 lines
  - `crates/slicer-macros/src/lib.rs` - locate `::slicer_sdk::traits::EntityMutation::SetSpeedFactor`, open ±20 lines
  - `CLAUDE.md` §"WIT/Type Changes Checklist" and §"Guest WASM Staleness" - read both sections
- Files allowed to edit (this step edits the five channel files plus the WIT; the chain cannot be split without leaving the workspace non-compiling mid-step):
  - `crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-wasm-host/src/host.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-macros/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**` (Step 6)
  - `modules/core-modules/overhang-classifier-default/**` — read-only in this packet; it is packet 190's surface
  - any other `wit` file under `crates/slicer-schema/wit/`
- Expected sub-agent dispatches:
  - Question: "Run `cargo xtask build-guests --check`. Return only whether any line begins with `STALE:` and, if so, the first five such lines."; scope: workspace; return: `FACT` ≤ 6 lines
  - Question: "Does any other file under `crates/slicer-schema/wit/` or `modules/core-modules/*/wit-guest/` also declare an `entity-mutation` variant that must be kept in sync?"; scope: `crates/slicer-schema/wit/**`, `modules/core-modules/*/wit-guest/**`; return: `LOCATIONS` ≤ 10 entries
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated grep for the `entity-mutation (variant)` bullet only
  - `docs/05_module_sdk.md` - delegated grep for the `modify_entity` variant table only
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'rg -q "SetPointSpeedFactors\(Vec<f32>\)" crates/slicer-sdk/src/traits.rs && rg -q "set-point-speed-factors\(list<f32>\)" crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit && rg -q "SetPointSpeedFactors" crates/slicer-wasm-host/src/host.rs && rg -q "SetPointSpeedFactors" crates/slicer-wasm-host/src/dispatch.rs && rg -q "SetPointSpeedFactors" crates/slicer-macros/src/lib.rs && echo PASS || echo "FAIL: SetPointSpeedFactors missing from at least one of the five channel files"'` - FACT (this is AC-3's command)
  - `bash -c 'cargo test -p slicer-sdk --test finalization_builder_tdd -- modify_entity_set_point_speed_factors_applies --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: modify_entity_set_point_speed_factors_applies did not run or did not pass"'` - FACT (AC-4)
  - `bash -c 'cargo test -p slicer-sdk --test finalization_builder_tdd -- modify_entity_set_point_speed_factors_length_mismatch_errors --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: modify_entity_set_point_speed_factors_length_mismatch_errors did not run or did not pass"'` - FACT (AC-N1)
  - `bash -c 'cargo test -p slicer-sdk --test finalization_builder_tdd -- modify_entity_set_speed_factor_applies --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && python3 -c "import io,sys; s=io.open(r\"crates/slicer-sdk/tests/finalization_builder_tdd.rs\",encoding=\"utf-8\").read(); b=s.index(\"fn modify_entity_set_speed_factor_applies\"); e=s.index(chr(10)+chr(125)+chr(10), b); sys.exit(0 if \"speed_profiles\" in s[b:e] else 1)" && echo PASS || echo "FAIL: the SetSpeedFactor test did not pass, or it still never asserts speed_profiles stays empty"'` - FACT (**AC-N2, verbatim.** This step implements the `apply_to` branch, so it is the step that could accidentally re-implement `SetSpeedFactor` as an expanded per-point profile — the exact failure AC-N2 forbids. AC-N2 was named by **no** step's Verification block before this round: a criterion with zero copies in the plan is invisible to a copy-vs-copy drift check, which is how the orphaning survived a clean run. It is the criterion the whole "absent profile ⇒ byte-identical output" claim rests on.)
  - `bash -c 'cargo xtask build-guests --check > target/guard-ac10-guests.txt 2>&1; rc=$?; if [ $rc -ne 0 ]; then echo "FAIL: build-guests --check exited $rc — see target/guard-ac10-guests.txt"; elif rg -q "STALE:" target/guard-ac10-guests.txt; then echo "FAIL: stale guests — rebuild with cargo xtask build-guests"; else echo PASS; fi'` - FACT (AC-10; rebuild without `--check` if it reports STALE, then re-run)
- Exit condition: AC-3, AC-4, AC-N1, **AC-N2** and AC-10 print PASS. Do not proceed to Step 6 while `--check` reports `STALE:` — every later component or dispatch failure would be unattributable.

### Step 6: Make the emitter resolve `F` per point from the profile

- Task IDs: `TASK-308`
- Objective: in `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`), build `speed_profiles_by_entity: HashMap<u64, &Vec<f32>>` immediately after the existing `travel_moves_by_entity` map; change the simplification `kept` remap so each surviving point carries its **original index**; resolve each `GCodeCommand::Move`'s `f:` as `self.resolve_feedrate(role, profile.and_then(|p| p.get(original_index).copied()).unwrap_or(entity.path.speed_factor))`. Leave `resolve_feedrate`'s signature and body unchanged.
- Precondition: Step 5 complete; `cargo xtask build-guests --check` clean.
- Postcondition: `per_point_speed_profile_varies_f_within_one_entity` and `per_point_speed_profile_indexes_original_points_after_simplification` pass; all nine pre-existing feedrate tests and `golden_emit_tdd` still pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` (1225 lines) - locate `travel_moves_by_entity`, `simplified_points`, `let mut prev_point`, and `DefaultGCodeEmitter::resolve_feedrate`; open ±40 lines around each. Do **not** read the file whole.
  - `crates/slicer-ir/src/slice_ir.rs` - locate `pub struct EntitySpeedProfile` only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/estimator.rs`, `crates/slicer-gcode/src/serialize.rs` — the `F` value they consume comes from `GCodeCommand::Move.f`, which this step already sets; neither needs a change
  - `crates/slicer-sdk/**`, `crates/slicer-wasm-host/**` (Step 5, done)
- Expected sub-agent dispatches:
  - Question: "In `crates/slicer-gcode/src/emit.rs`, quote the `kept` remap loop verbatim (the block that maps `pruned_xy` back onto `points`)."; scope: that file; return: `SNIPPETS` ≤ 1, ≤ 30 lines
- Context cost: `S`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::_extrude`, for the per-segment `F` emission shape - delegate; never load
- Verification:
  - `bash -c 'cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd -- per_point_speed_profile_varies_f_within_one_entity --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: per_point_speed_profile_varies_f_within_one_entity did not run or did not pass"'` - FACT (AC-5)
  - `bash -c 'cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd -- per_point_speed_profile_indexes_original_points_after_simplification --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && python3 -c "import io,sys; s=io.open(r\"crates/slicer-gcode/src/emit.rs\",encoding=\"utf-8\").read(); sys.exit(0 if \"speed_profiles_by_entity\" in s else 1)" && echo PASS || echo "FAIL: the original-index test did not pass, or emit.rs has no speed_profiles_by_entity lookup"'` - FACT (AC-6)
  - `bash -c 'cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: gcode_feedrate_emission_tdd or golden_emit_tdd regressed"'` - FACT (AC-7)
  - `bash -c 'cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd -- unprofiled_entity_in_a_profiled_layer_keeps_whole_entity_speed --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: unprofiled_entity_in_a_profiled_layer_keeps_whole_entity_speed did not run or did not pass"'` - FACT (AC-N3; the mixed profiled/un-profiled layer that packet 190 creates on every slice)
- Exit condition: AC-5, AC-6, AC-7 and AC-N3 print PASS.

### Step 7: Full regression sweep across the blast radius

- Task IDs: `TASK-308`
- Objective: prove the sweep and the emit change together moved nothing, across the three buckets that carry the largest share of the blast radius.
- Precondition: Step 6 complete.
- Postcondition: all four commands below print PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - on failure only, via `Grep` for `FAILED|panicked at|---- .* stdout ----` with `-C 5`; never re-run a test to see more output
- Files allowed to edit (at most 3):
  - none — this is a read-only validation step. If a test fails, stop and open a diagnosis under the failing step's ownership.
- Files explicitly out of bounds:
  - all source files (a green sweep is the goal; fixing a red test here means the design is wrong, not the test)
- Expected sub-agent dispatches:
  - Question: "Run each of the four verification commands below and return only their PASS/FAIL lines."; scope: workspace; return: `FACT` ≤ 5 lines
- Context cost: `M` (four cargo runs; none of their output enters the implementer's context)
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'cargo test -p slicer-gcode 2>&1 | tee target/test-output.log | rg "^test result:" > target/guard-ac8-gcode.txt; rg -q "[1-9][0-9]* failed|^test result: FAILED" target/guard-ac8-gcode.txt && echo "FAIL: see target/test-output.log" || (rg -q "^test result: ok\. [1-9]" target/guard-ac8-gcode.txt && echo PASS || echo "FAIL: zero tests ran")'` - FACT (AC-8)
  - `bash -c 'cargo test -p slicer-runtime --test executor 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo FAIL'` - FACT (finalization deep-copy and mutation-roundtrip bucket)
  - `bash -c 'cargo test -p slicer-runtime --test unit 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo FAIL'` - FACT (`layer_collection_builder_tdd`, `tool_ordering_tdd`)
  - `bash -c 'cargo clippy --workspace --all-targets -- -D warnings 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT
- Exit condition: four PASS lines. Any FAIL is diagnosed against the owning step before the packet advances.

### Step 8: Docs and `TASK-308` registration

- Task IDs: `TASK-308`
- Objective: land every entry in `packet.spec.md` §Doc Impact Statement — the `docs/02_ir_schemas.md` IR-10 edits (struct block, `EntitySpeedProfile` block, additive-bump note, and the normative `default()` contract paragraph), the `docs/05_module_sdk.md` variant-table row, the `docs/03_wit_and_manifest.md` bullet, and the `TASK-308` registration in `docs/07_implementation_status.md` outside the generated block.
- Precondition: Step 7 complete; all code ACs green.
- Postcondition: every doc verification grep in `packet.spec.md` §Doc Impact Statement returns PASS, and `DEV-009` is still `Open`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` (2157 lines) - §"IR 10 — LayerCollectionIR" through the start of §"IR 11 — GCodeIR" **only**
  - `docs/05_module_sdk.md` - the `modify_entity` variant table only, located by grep
  - `docs/03_wit_and_manifest.md` - the `entity-mutation (variant)` bullet only, located by grep
  - `docs/07_implementation_status.md` - **delegate**; only the region around the highest existing `TASK-###` row and the `<!-- BEGIN GENERATED: open-deviations` marker matter
- Files allowed to edit (this step edits four docs; each is a distinct, independently-verified anchor):
  - `docs/02_ir_schemas.md`
  - `docs/05_module_sdk.md`
  - `docs/03_wit_and_manifest.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` — **`DEV-009` must not be touched by this packet**; packets 190 and 191 close its two remaining sub-items
  - everything inside `<!-- BEGIN GENERATED: open-deviations … -->` / `<!-- END GENERATED: open-deviations -->` in `docs/07_implementation_status.md` — regenerated by `cargo xtask check-deviations`, never hand-edited
- Expected sub-agent dispatches:
  - Question: "Re-derive the highest `TASK-###` currently present in `docs/07_implementation_status.md` and confirm `TASK-308` has zero hits."; scope: that file; return: `FACT` ≤ 3 lines
  - Question: "Quote the `pub struct LayerCollectionIR` code block and the `LayerCollectionIR::default()` contract paragraph from `docs/02_ir_schemas.md` verbatim."; scope: that file; return: `SNIPPETS` ≤ 2, ≤ 30 lines each
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - ranged read as above
  - `docs/07_implementation_status.md` - delegated `FACT` only
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'python3 -c "import io,os,re,sys; p=r\"docs/02_ir_schemas.md\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); a=re.search(r\"^## IR 10\b\",s,re.M); b=re.search(r\"^## IR 11\b\",s,re.M); sys.exit(print(\"FAIL: IR-10/IR-11 section headers not found\")) if not (a and b) else None; seg=s[a.start():b.start()]; ok=(\"speed_profiles\" in seg) and (\"EntitySpeedProfile\" in seg) and (\"speed_profiles = vec![]\" in seg); print(\"PASS\" if ok else \"FAIL: IR-10 section missing speed_profiles / EntitySpeedProfile / the default-contract mention\")"'` - FACT (AC-11; located by `^## IR 10` regex, not by the em-dash-bearing literal heading — the literal form is encoding-fragile inside `bash -c 'python3 -c "import io,os,re,sys; p=r\"docs/02_ir_schemas.md\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); a=re.search(r\"^## IR 10\b\",s,re.M); b=re.search(r\"^## IR 11\b\",s,re.M); sys.exit(print(\"FAIL: IR-10/IR-11 section headers not found\")) if not (a and b) else None; seg=s[a.start():b.start()]; ok=(\"speed_profiles\" in seg) and (\"EntitySpeedProfile\" in seg) and (\"speed_profiles = vec![]\" in seg); print(\"PASS\" if ok else \"FAIL: IR-10 section missing speed_profiles / EntitySpeedProfile / the default-contract mention\")"'` on this platform)
  - `bash -c 'rg -q "SetPointSpeedFactors" docs/05_module_sdk.md && rg -q "set-point-speed-factors" docs/03_wit_and_manifest.md && echo PASS || echo "FAIL: docs/05_module_sdk.md or docs/03_wit_and_manifest.md was not updated"'` - FACT
  - `bash -c 'python3 -c "import io,os,sys; p=r\"docs/07_implementation_status.md\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); B=\"<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->\"; E=\"<!-- END GENERATED: open-deviations -->\"; i=s.find(B); j=s.find(E); sys.exit(print(\"FAIL: open-deviations generated markers not found in \"+p)) if (i<0 or j<0 or j<i) else None; outside=s[:i]+s[j+len(E):]; print(\"PASS\" if \"TASK-308\" in outside else \"FAIL: TASK-308 is not registered OUTSIDE the open-deviations generated block\")"'` - FACT (**this is the §Doc Impact Statement `TASK-308` probe, verbatim.** It is split out of the doc-grep chain above and written in `python3` because a bare `rg -q 'TASK-308' docs/07_implementation_status.md` cannot distinguish a row hand-added outside the markers — which this step requires — from one that landed inside the generated block and will be silently destroyed by the next `cargo xtask check-deviations`. Measured: `TASK-156` occurs both inside and outside that block on this tree today, so the whole-file grep is demonstrably non-discriminating for this obligation.)
  - `bash -c 'rg -q "^\| DEV-009 .*Open" docs/DEVIATION_LOG.md && echo PASS || echo "FAIL: DEV-009 was flipped"'` - FACT
- Exit condition: four PASS lines.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Read-only plus one untracked scratch file; pins the pre-packet schema version so `AC-2` asserts a **relative** additive bump instead of a hardcoded SemVer, and re-derives the literal-site census Steps 3-4 are graded against. Both outputs are ledger facts and neither lands in a tracked file |
| Step 1 | S | Two test files; red-by-non-compilation is the expected state |
| Step 2 | S | Two files; carrier type, field, `Default`, schema bump, re-export |
| Step 3 | S | 7 files, 13 sites, one inserted line each, compiler-driven (counts are a ledger fact — re-derive with the Step 3 command; the file list is the authority) |
| Step 4 | M | 19 files, 35 sites, one inserted line each; largest step in the packet (counts are a ledger fact — re-derive with the Step 3 command) |
| Step 5 | M | Five-file mutation chain plus the applier, plus a 34-artifact guest rebuild |
| Step 6 | S | One file; profile lookup, original-index remap, per-point `f` |
| Step 7 | M | Four cargo runs, all delegated; no source edits |
| Step 8 | S | Four docs; all edits anchored by an independently-verified grep |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate: `M`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS — including AC-7, AC-8, AC-9 and AC-10, which are do-not-regress guards that were already PASS before the packet started and whose value is entirely in still being PASS after it.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions: none — this packet supersedes nothing.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Re-run `cargo xtask build-guests --check` immediately before closure; a `STALE:` at closure invalidates every component test result collected during the packet.
- Record remaining packet-local risk: the carrier is unexercised by any live producer until packet 190 lands, so the only proof that it works end-to-end through a real WASM guest is AC-10 plus the `slicer-runtime --test executor` bucket. State that explicitly at closure rather than implying end-to-end coverage.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
