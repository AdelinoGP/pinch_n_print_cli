# Implementation Plan: 89_benchy-3mf-retirement

## Execution Rules

- One atomic step at a time. Each step ends with a passing narrow verification before the next step begins.
- Each step maps back to `TASK-239`.
- Rename + assertion rewrite + import-path / mod-declaration update for a single test file land in **one Git commit** so `git log --follow` stays accurate.
- TDD discipline: the migrated cube tests must continue to pass after each step. Never run `cargo test --workspace` speculatively — use the narrow per-test commands listed below.
- All `cargo test` output is teed to `target/test-output.log` per `CLAUDE.md` §Test Discipline.

## Steps

### Step 1: Discover residual-reference inventory and confirm derivative-fixture status

- Task IDs:
  - `TASK-239`
- Objective: produce an authoritative file:line list of every reference to `benchy_4color.3mf` or `benchy_painted.3mf` in the workspace, and confirm whether `resources/cube_cilindrical_modifier.3mf` and `resources/cube_rotated_component.3mf` already exist.
- Precondition: working tree clean.
- Postcondition: an in-implementer-notes inventory of all reference sites (file:line + 1-line context) plus binary YES/NO answers for the two possible-derivative fixtures.
- Files allowed to read:
  - None directly. This is a pure-dispatch step.
- Files allowed to edit (≤ 3):
  - None.
- Files explicitly out-of-bounds for this step:
  - Any `.rs` source under `crates/`. Resist the temptation to open files.
- Expected sub-agent dispatches:
  - "Run `rg -nE 'benchy_(4color|painted)\.3mf' crates/ modules/ docs/ .ralph/`; return LOCATIONS (≤ 60 entries) PLUS a per-file count table (`file → ref_count`). If total > 60, the LOCATIONS list MAY truncate but the per-file count table MUST be complete so no migration target is silently dropped" — purpose: full residual inventory. (Total expected ≈ 43+ references across the design.md-listed files; cap was 30 in a prior revision and risked truncation.)
  - "Run `test -f resources/cube_cilindrical_modifier.3mf && echo present || echo absent`; same for `resources/cube_rotated_component.3mf`; return FACT" — purpose: derivative-fixture status check.
- Context cost: `S` (pure dispatch).
- Authoritative docs:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` — read §"P0a — Benchy 3MF retirement" (lines 200-260 approx) — purpose: reconcile reported reference count with the roadmap table.
- OrcaSlicer refs: none.
- Verification:
  - LOCATIONS list non-empty (the migration has work to do); FACT for each derivative fixture is either `present` or `absent`.
- Exit condition: a written inventory and two FACT answers are recorded in the implementer's notes for use in Steps 2–7.

### Step 2: Determine cube-face paint distribution for `cube_4color.3mf` and `cube_fuzzyPainted.3mf`

- Task IDs:
  - `TASK-239`
- Objective: produce the canonical face→`PaintSemantic` mapping for both cube fixtures so Step 3+ can write per-face strengthened assertions.
- Precondition: Step 1 complete.
- Postcondition: a table of (fixture, face axis, expected PaintSemantic value) covering all faces that carry paint in either fixture.
- Files allowed to read:
  - `docs/specs/orca-paint-segmentation-parity.md` — §"Fixture Strategy" only — line range to be determined by the dispatch below.
  - `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` — read only the per-face-assertion test bodies (typically the first few tests in each file).
- Files allowed to edit (≤ 3):
  - None.
- Files explicitly out-of-bounds for this step:
  - The `.3mf` files themselves (binary; never `Read`).
- Expected sub-agent dispatches:
  - "Locate §'Fixture Strategy' (or equivalent) in `docs/specs/orca-paint-segmentation-parity.md`; return LOCATIONS (file:line of the section heading + 1-line context)" — purpose: range-read target.
  - "What `PaintSemantic` value does each of the six faces of `resources/cube_4color.3mf` carry, and the same for `resources/cube_fuzzyPainted.3mf`? Return SUMMARY ≤ 150 words with two tables, one per fixture" — purpose: produce the mapping.
- Context cost: `S`.
- Authoritative docs:
  - `docs/specs/orca-paint-segmentation-parity.md` §"Fixture Strategy" — read lines indicated by the dispatch return; otherwise delegate.
- OrcaSlicer refs: none.
- Verification:
  - SUMMARY return contains both tables; tables list 6 faces × 2 fixtures.
- Exit condition: mapping table recorded in implementer's notes; Step 3 can write specific face-axis assertions without re-asking.

### Step 3: Migrate `benchy_4color_modifier_part_e2e_tdd.rs` → `cube_4color_modifier_part_e2e_tdd.rs`

- Task IDs:
  - `TASK-239`
- Objective: rewrite the 7 modifier-part tests (6 STRUCTURAL + 1 SHAPE-DEPENDENT) to consume `cube_4color.3mf` (or `cube_cilindrical_modifier.3mf` per Step 1's FACT), strengthen the SHAPE-DEPENDENT assertion to a per-face check using Step 2's mapping, and rename the file.
- Precondition: Steps 1 and 2 complete. If Step 1's FACT reported `cube_cilindrical_modifier.3mf` as `absent`, this step also includes authoring that fixture (≤ 100 KB) before edit.
- Postcondition: file renamed; all 7 tests pass against the cube fixture; the SHAPE-DEPENDENT test asserts an exact `PaintSemantic` value on a known cube face.
- Files allowed to read:
  - `crates/slicer-runtime/tests/e2e/benchy_4color_modifier_part_e2e_tdd.rs` — read in full (≤ 300 lines expected).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/e2e/benchy_4color_modifier_part_e2e_tdd.rs` (renamed during edit).
  - `crates/slicer-runtime/tests/e2e.rs` — update `mod` declaration.
  - Optionally `resources/cube_cilindrical_modifier.3mf` (only if authoring is required).
- Files explicitly out-of-bounds for this step:
  - Any other test file (those are Steps 4–6).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test e2e cube_4color_modifier_part`; return FACT pass/fail, SNIPPETS on failure (≤ 20 lines around `FAILED` / `panicked at`)" — purpose: validate exit.
- Context cost: `M` (7 test bodies, one new fixture authoring possibly required).
- Authoritative docs:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0a" — confirm the test-classification entries for this file.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test e2e cube_4color_modifier_part 2>&1 | tee target/test-output.log` → all 7 tests pass.
- Exit condition: AC-3 satisfied.

### Step 4: Migrate `benchy_painted_e2e_tdd.rs` and `benchy_painted_overrides_e2e_tdd.rs`

- Task IDs:
  - `TASK-239`
- Objective: rewrite the 3 painted-e2e tests across two files to consume `cube_4color.3mf`; rename both files; preserve the painted-vs-unpainted assertion semantics (strengthening where possible).
- Precondition: Step 3 green.
- Postcondition: both files renamed; all 3 tests pass.
- Files allowed to read:
  - `crates/slicer-runtime/tests/e2e/benchy_painted_e2e_tdd.rs` (small; ≤ 100 lines).
  - `crates/slicer-runtime/tests/e2e/benchy_painted_overrides_e2e_tdd.rs` (small; ≤ 100 lines).
- Files allowed to edit (≤ 3):
  - The two files above (renamed during edit).
  - `crates/slicer-runtime/tests/e2e.rs` — update `mod` declarations.
- Files explicitly out-of-bounds for this step:
  - Other test files.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test e2e cube_painted`; return FACT pass/fail, SNIPPETS on failure" — purpose: validate exit.
- Context cost: `S`.
- Authoritative docs: roadmap §"P0a".
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test e2e cube_painted 2>&1 | tee target/test-output.log` → 3 tests pass.
- Exit condition: AC-4 satisfied.

### Step 5: Migrate `threemf_fixture_e2e_tdd.rs` lines 540-924; `integration/threemf_paint_drop_on_modifier_tdd.rs`; `integration/threemf_transform_tdd.rs`

- Task IDs:
  - `TASK-239`
- Objective: in-place edit three test files (no rename) to swap benchy references for cube references. If Step 1 reported `cube_rotated_component.3mf` absent, author it (≤ 100 KB) before editing `threemf_transform_tdd.rs`.
- Precondition: Step 4 green.
- Postcondition: all three files have zero `benchy_*\.3mf` substrings; each file's tests pass.
- Files allowed to read:
  - `crates/slicer-runtime/tests/e2e/threemf_fixture_e2e_tdd.rs` — RANGED read lines 540-924 only (file is larger; do not load in full).
  - `crates/slicer-runtime/tests/integration/threemf_paint_drop_on_modifier_tdd.rs` (small).
  - `crates/slicer-runtime/tests/integration/threemf_transform_tdd.rs` (small).
- Files allowed to edit (≤ 3):
  - The three files above.
  - Optionally `resources/cube_rotated_component.3mf` (authored only if required by Step 1's FACT).
- Files explicitly out-of-bounds for this step:
  - Test files in `crates/slicer-model-io/` (Step 6's territory).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test e2e threemf_fixture`; return FACT pass/fail" — purpose: validate.
  - "Run `cargo test -p slicer-runtime --test integration threemf_paint_drop_on_modifier`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test integration threemf_transform`; return FACT pass/fail".
- Context cost: `M` (3 files; possibly 1 fixture authoring).
- Authoritative docs: roadmap §"P0a" table.
- OrcaSlicer refs: none.
- Verification:
  - Each dispatched test command returns PASS.
  - `! rg -q 'benchy_(4color|painted)' crates/slicer-runtime/tests/e2e/threemf_fixture_e2e_tdd.rs crates/slicer-runtime/tests/integration/threemf_paint_drop_on_modifier_tdd.rs crates/slicer-runtime/tests/integration/threemf_transform_tdd.rs` — no residual substrings.
- Exit condition: AC-5, AC-6, AC-7 satisfied.

### Step 6: Migrate `crates/slicer-model-io/tests/` references

- Task IDs:
  - `TASK-239`
- Objective: in-place edit `model_loader_tdd.rs` (6 refs) and `threemf_sidecar_classification_tdd.rs` (9 refs) to consume cube fixtures.
- Precondition: Step 5 green.
- Postcondition: both files pass; both have zero `benchy_*\.3mf` substrings.
- Files allowed to read:
  - `crates/slicer-model-io/tests/model_loader_tdd.rs` — open at the relevant reference sites (use the inventory from Step 1 to target line ranges if file is > 300 lines).
  - `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` — same approach.
- Files allowed to edit (≤ 3):
  - Both files above.
- Files explicitly out-of-bounds for this step:
  - Other tests; production source.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-model-io --test model_loader_tdd`; return FACT pass/fail" — purpose: validate.
  - "Run `cargo test -p slicer-model-io --test threemf_sidecar_classification_tdd`; return FACT pass/fail".
- Context cost: `S`.
- Authoritative docs: roadmap §"P0a" table.
- OrcaSlicer refs: none.
- Verification:
  - Both tests pass; both files free of `benchy_*` substrings.
- Exit condition: AC-8 satisfied.

### Step 7: Update `crates/slicer-runtime/tests/common/model_cache.rs` header doc-comment

- Task IDs:
  - `TASK-239`
- Objective: rewrite the doc-comment block at lines 5-8 to reference `cube_4color.3mf` and `cube_fuzzyPainted.3mf` with their actual sizes (37 KB, 27 KB) as the motivating cache example.
- Precondition: Step 6 green.
- Postcondition: doc-comment names the cube fixtures; no `benchy_*` substring remains in `crates/slicer-runtime/tests/common/`.
- Files allowed to read:
  - `crates/slicer-runtime/tests/common/model_cache.rs` — full (47 lines).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/common/model_cache.rs` (doc-comment only; code unchanged).
- Files explicitly out-of-bounds for this step:
  - Any other cache file (`slicer_cache.rs`, etc.) — those have their own ownership.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail" — purpose: confirm the comment edit didn't break the file.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --all-targets` passes.
  - `rg -q 'cube_4color\.3mf' crates/slicer-runtime/tests/common/model_cache.rs && ! rg -q 'benchy_(4color|painted)' crates/slicer-runtime/tests/common/`.
- Exit condition: AC-9 satisfied.

### Step 8: Delete benchy 3MF fixtures and residual-reference sweep

- Task IDs:
  - `TASK-239`
- Objective: delete `resources/benchy_4color.3mf`, `resources/benchy_painted.3mf`, `resources/benchy_painted.README.md`; verify zero residual references across the workspace.
- Precondition: Steps 3-7 all green.
- Postcondition: three files deleted; AC-1 and AC-2 hold.
- Files allowed to read:
  - None.
- Files allowed to edit (≤ 3):
  - The three files above (delete only).
- Files explicitly out-of-bounds for this step:
  - Any test source. If a residual reference is found, Step 8 fails; do NOT patch it in this step — go back to whichever per-file step missed it and re-run.
- Expected sub-agent dispatches:
  - "Run `rg -n --glob '!.ralph/specs/**' 'benchy_4color\.3mf|benchy_painted\.3mf' crates/ modules/ docs/ .ralph/`; return LOCATIONS or empty" — purpose: residual sweep. Note: the exclusion glob covers **all** sibling spec packets (90–95), not just this packet's own folder, because the retired basenames may legitimately appear in those packets' scope/roadmap narratives without being live consumers.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - Three files deleted (`test ! -f` for each).
  - Residual-grep returns empty.
- Exit condition: AC-1, AC-2 satisfied.

### Step 9: Final acceptance ceremony — full e2e + integration buckets + clippy

- Task IDs:
  - `TASK-239`
- Objective: confirm AC-10 by running the workspace gate commands.
- Precondition: Steps 3–8 all green.
- Postcondition: clippy clean; full `slicer-runtime` e2e bucket green; full `slicer-runtime` integration bucket green (the migrated `threemf_paint_drop_on_modifier` / `threemf_transform` tests live here, not in `e2e`); full `slicer-model-io` test crate green.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files explicitly out-of-bounds for this step: any. This is a gate-only step.
- Expected sub-agent dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: gate.
  - "Run `cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log`; return FACT pass/fail with overall test count" — purpose: e2e bucket gate.
  - "Run `cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log`; return FACT pass/fail with overall test count" — purpose: integration bucket gate (covers AC-6/AC-7 migrated files).
  - "Run `cargo test -p slicer-model-io 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: model-io crate gate.
- Context cost: `S` (dispatch-only).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all four dispatches return PASS.
- Exit condition: AC-10 satisfied; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Pure dispatch — residual-ref inventory + derivative-fixture status. |
| Step 2 | S | Pure dispatch — cube face mapping. |
| Step 3 | M | 7 test bodies; possibly 1 fixture authoring. |
| Step 4 | S | 3 test bodies across 2 files. |
| Step 5 | M | 3 files; possibly 1 fixture authoring. |
| Step 6 | S | 2 files in `slicer-model-io/tests/`. |
| Step 7 | S | Doc-comment-only edit. |
| Step 8 | S | Deletions + residual sweep. |
| Step 9 | S | Pure dispatch — workspace gate. |

Aggregate: M (largest single step is M; no L).

## Packet Completion Gate

- All 9 steps complete; each step's exit condition satisfied.
- AC-1 through AC-10 + AC-N1, AC-N2 verified.
- `docs/07_implementation_status.md` updated to record `TASK-239` as implemented and link to `.ralph/specs/89_benchy-3mf-retirement/` (delegate the edit — never load the full backlog file).
- `packet.spec.md` §"Closure Log" fully populated, including all three subsections:
  - `### Weakened-assertion review (AC-N1)` — one verdict line per AC-N1 grep hit, or the zero-hit sentinel.
  - `### Wall-clock measurement` — `time cargo test -p slicer-runtime` before and after, with both commit SHAs and the absolute/percent reduction.
  - `### Authored-fixture provenance (AC-N2)` — deterministic authoring command + on-disk size for each fixture authored, or the "both unnecessary" sentinel.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and confirm each returns PASS.
- Confirm `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, and the e2e bucket + model-io bucket are green via sub-agent FACT.
- Populate `packet.spec.md` §"Closure Log" → "Weakened-assertion review", "Wall-clock measurement", and "Authored-fixture provenance" (expect: no surprises — this is a fixture migration).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future runs.
