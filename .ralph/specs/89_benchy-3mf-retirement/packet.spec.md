---
status: active
packet: 89
task_ids: [TASK-239]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 89 — Benchy 3MF Retirement

## Goal

Retire `resources/benchy_4color.3mf` (2.6 MB) and `resources/benchy_painted.3mf` (2.5 MB) as test fixtures by migrating every consuming test to the engineered cube fixtures (`cube_4color.3mf`, `cube_fuzzyPainted.3mf`) landed in cherry-pick `5c272ef970fee2b861081799169a3ddb87e179c9`, strengthening assertions to per-face deterministic checks where the cube's engineered semantics allow, and deleting the two benchy 3MF files plus the accompanying README so no test in the workspace continues to parse a multi-megabyte 3MF whose paint geometry is opaque.

## Scope Boundaries

This packet rewrites tests but does not change any production paint pipeline behavior; it is purely a fixture migration. Two tiny derivative cube fixtures (`cube_with_modifier_part.3mf`, `cube_rotated_component.3mf`) may be authored if and only if existing cube assets cannot satisfy a SHAPE-DEPENDENT or STRUCTURAL test that was previously satisfied by benchy. Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: cherry-pick `5c272ef970fee2b861081799169a3ddb87e179c9` already landed (provides cube fixtures + RED tests).
- Unblocks: P0b (`benchy.stl` STL swap) is independent — both can land in either order; P1a–P1c onwards are independent of this packet.
- Activation blockers: none.

## Acceptance Criteria

### AC-1 — `resources/benchy_4color.3mf`, `resources/benchy_painted.3mf`, `resources/benchy_painted.README.md` are deleted

**Given** the migration target,
**When** the resources directory is inspected,
**Then** none of the three files exists.

| `test ! -f resources/benchy_4color.3mf && test ! -f resources/benchy_painted.3mf && test ! -f resources/benchy_painted.README.md`

### AC-2 — Zero references to the deleted benchy 3MF basenames survive in the tree

**Given** the deletions in AC-1,
**When** the workspace is grepped,
**Then** no file under `crates/`, `modules/`, `docs/`, or `.ralph/` (excluding **all** spec packets under `.ralph/specs/**`, since sibling packets 90–95 may legitimately reference the retired fixture names in their own scope/roadmap narratives) mentions `benchy_4color.3mf` or `benchy_painted.3mf`.

| `rg -n --glob '!.ralph/specs/**' 'benchy_4color\.3mf|benchy_painted\.3mf' crates/ modules/ docs/ .ralph/ ; test $? -eq 1`

### AC-3 — Every test in `crates/slicer-runtime/tests/e2e/benchy_4color_modifier_part_e2e_tdd.rs` is migrated and renamed; assertions strengthen on per-face paint

**Given** the 7 tests in `benchy_4color_modifier_part_e2e_tdd.rs`,
**When** the file is replaced by `cube_4color_modifier_part_e2e_tdd.rs`,
**Then** all 7 tests use `resources/cube_4color.3mf` (or `resources/cube_with_modifier_part.3mf` if authored), each test that was STRUCTURAL continues to assert the same structural property (e.g. modifier-part presence, sidecar classification routing), and each test that was SHAPE-DEPENDENT now asserts on a specific cube face's known `PaintSemantic::Material(ToolIndex)` value rather than a generic "some color was applied" check. The renamed test file passes.

| `mkdir -p target && cargo test -p slicer-runtime --test e2e cube_4color_modifier_part 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed'`

### AC-4 — `benchy_painted_e2e_tdd.rs` and `benchy_painted_overrides_e2e_tdd.rs` migrated to cube fixtures

**Given** the 2 tests in `benchy_painted_e2e_tdd.rs` and 1 test in `benchy_painted_overrides_e2e_tdd.rs`,
**When** they are renamed to `cube_painted_e2e_tdd.rs` and `cube_painted_overrides_e2e_tdd.rs`,
**Then** the CLI-SHAPE test (`painted_slice_succeeds`) uses `resources/cube_4color.3mf` and still asserts exit 0 + output written; the SHAPE-DEPENDENT test (`painted_gcode_differs_from_unpainted`) compares the cube_4color g-code to a plain unpainted cube g-code and asserts a non-trivial diff in tool-change / temperature blocks; the overrides test uses `resources/cube_4color.3mf` and asserts the per-face override the cube paints support.

| `mkdir -p target && cargo test -p slicer-runtime --test e2e cube_painted 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed'`

### AC-5 — `threemf_fixture_e2e_tdd.rs` lines 540–924 region migrated to cube fixture

**Given** the 13 references to `benchy_4color.3mf` clustered in `crates/slicer-runtime/tests/e2e/threemf_fixture_e2e_tdd.rs:540-924`,
**When** the file is edited,
**Then** every reference becomes `resources/cube_4color.3mf` (or the smaller derivative if the test requires it), each affected test still passes, and the file contains zero `benchy_4color` substrings.

| `mkdir -p target && cargo test -p slicer-runtime --test e2e threemf_fixture 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed' && ! rg -q 'benchy_4color' crates/slicer-runtime/tests/e2e/threemf_fixture_e2e_tdd.rs`

### AC-6 — `threemf_paint_drop_on_modifier_tdd.rs` migrated using `cube_with_modifier_part.3mf` (authored if absent)

**Given** the 1 STRUCTURAL test in `threemf_paint_drop_on_modifier_tdd.rs`,
**When** the test runs,
**Then** it loads a cube fixture containing both painted volumes and a modifier-part volume (authored as `resources/cube_with_modifier_part.3mf` if it does not yet exist), asserts that the loader correctly drops paint data on the modifier volume per the OrcaSlicer-parity rule, and the file contains no `benchy_painted` substring.

| `mkdir -p target && cargo test -p slicer-runtime --test integration threemf_paint_drop_on_modifier 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed' && ! rg -q 'benchy_painted' crates/slicer-runtime/tests/integration/threemf_paint_drop_on_modifier_tdd.rs`

### AC-7 — `threemf_transform_tdd.rs` migrated using `cube_rotated_component.3mf` (authored if absent)

**Given** the 4 references in `threemf_transform_tdd.rs` exercising a rotated-component sub-object,
**When** the file is edited,
**Then** each reference resolves against `resources/cube_rotated_component.3mf` (a small cube derivative whose only feature is a 45° rotated component instance; authored if absent), tests pass, and no `benchy_*\.3mf` substring remains in the file.

| `mkdir -p target && cargo test -p slicer-runtime --test integration threemf_transform 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed' && ! rg -q 'benchy_(4color|painted)' crates/slicer-runtime/tests/integration/threemf_transform_tdd.rs`

### AC-8 — `model_loader_tdd.rs`, `threemf_sidecar_classification_tdd.rs` migrated

**Given** the 6 references in `crates/slicer-model-io/tests/model_loader_tdd.rs` and 9 references in `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`,
**When** both files are edited,
**Then** all references become `resources/cube_4color.3mf` (or the smaller derivatives where the test demands), both files pass under `cargo test -p slicer-model-io`, and neither file contains `benchy_4color` or `benchy_painted`.

| `mkdir -p target && cargo test -p slicer-model-io --test model_loader_tdd 2>&1 | tee target/test-output.log && cargo test -p slicer-model-io --test threemf_sidecar_classification_tdd 2>&1 | tee -a target/test-output.log && ! rg -q 'benchy_(4color|painted)' crates/slicer-model-io/tests/`

### AC-9 — Test-fixture cache (`crates/slicer-runtime/tests/common/model_cache.rs`) header comment updated; no benchy entries in any cache key

**Given** the cache documentation at `crates/slicer-runtime/tests/common/model_cache.rs:5-8` explicitly names `benchy_4color.3mf` and `benchy_painted.3mf` as motivating examples,
**When** the cache header doc-comment is rewritten,
**Then** it names `cube_4color.3mf` and `cube_fuzzyPainted.3mf` as the canonical motivating examples and the actual sizes (37 KB and 27 KB respectively) replace the obsolete sizes (2.6 MB / 2.5 MB), and no test inserts a benchy path into the cache via `cached_load_model` or `cached_run`.

| `rg -q 'cube_4color\.3mf|cube_fuzzyPainted\.3mf' crates/slicer-runtime/tests/common/model_cache.rs && ! rg -q 'benchy_(4color|painted)' crates/slicer-runtime/tests/common/`

### AC-10 — Workspace clippy clean and full e2e + integration buckets green

**Given** the migrations in AC-3 through AC-9 (e2e tests are migrated in AC-3/AC-4/AC-5; integration tests are migrated in AC-6/AC-7 — both buckets must clear),
**When** clippy and both `slicer-runtime` test buckets run,
**Then** all three succeed with zero warnings / zero failures.

| `mkdir -p target && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log && cargo test -p slicer-runtime --test e2e 2>&1 | tee -a target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed' && cargo test -p slicer-runtime --test integration 2>&1 | tee -a target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed'`

## Negative Test Cases

### AC-N1 — No test in the workspace skips a paint-semantic assertion that used to be asserted on benchy

**Given** the migration may have weakened an assertion through inattention,
**When** the migrated tests are grepped for sentinel patterns associated with weak assertions (`assert!\(.*\.is_some\(\)\)` over `PaintSemantic` types, `assert_eq!\(.*\.len\(\), 0\)` on paint-data vectors that used to be non-zero),
**Then** the grep produces an inventory of candidates; the implementer reviews each entry and records a verdict in the `## Closure Log` section below (`### Weakened-assertion review`) — every entry is either (a) **JUSTIFIED**, accompanied by a one-line comment in the test source explaining why the cube fixture produces an empty/optional value where benchy produced a populated value, or (b) **REWRITTEN** as a stronger cube-specific assertion. AC-N1 passes when every grep hit has a matching verdict line in the Closure Log AND each JUSTIFIED hit carries the explanatory comment in source. A zero-hit inventory is the easy path and is recorded as such.

The grep below is the machine half of the gate — it MUST run to populate the inventory, but a clean (zero-hit) result short-circuits the manual phase.

| `mkdir -p target && rg -nE '(\.is_some\(\)|\.len\(\) == 0|\.len\(\), 0)' crates/slicer-runtime/tests/e2e/cube_4color_modifier_part_e2e_tdd.rs crates/slicer-runtime/tests/e2e/cube_painted_e2e_tdd.rs crates/slicer-runtime/tests/e2e/cube_painted_overrides_e2e_tdd.rs 2>&1 | tee target/test-output.log`

### AC-N2 — Any new fixture authored is ≤ 100 KB

**Given** the goal of reclaiming the 5.2 MB benchy 3MF footprint,
**When** `cube_with_modifier_part.3mf` and/or `cube_rotated_component.3mf` are authored (if either is authored),
**Then** each file is ≤ 100 KB on disk; if a candidate fixture exceeds this it must be regenerated with a coarser mesh or rejected as a candidate (use the existing cube fixture instead and adjust assertions).

| `for f in resources/cube_with_modifier_part.3mf resources/cube_rotated_component.3mf; do [ ! -f "$f" ] || [ $(wc -c < "$f") -le 102400 ] || { echo "OVERSIZE: $f"; exit 1; }; done`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log` (all migrated e2e tests pass)
4. `cargo test -p slicer-runtime --test integration 2>&1 | tee -a target/test-output.log` (migrated integration tests for AC-6/AC-7 pass)
5. `cargo test -p slicer-model-io 2>&1 | tee -a target/test-output.log`
6. `rg -n --glob '!.ralph/specs/**' 'benchy_4color\.3mf|benchy_painted\.3mf' crates/ modules/ docs/ .ralph/ ; test $? -eq 1` (the `.ralph/specs/**` exclusion covers sibling packets 90–95 which may reference the retired basenames in their own scope narratives)

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0a — Benchy 3MF retirement" — the source of this packet's scope (~50 lines; read directly).
- `docs/specs/orca-paint-segmentation-parity.md` — context on why cube fixtures were authored; read §9 "Test Strategy" only (lines ~942-980) — there is no §"Fixture Strategy" heading (delegate if file is > 300 lines).
- `crates/slicer-runtime/tests/common/model_cache.rs` — the cache module whose header comment must be updated (47 lines; read in full).

## Doc Impact Statement

A list of specific doc sections that this packet modifies:

- `crates/slicer-runtime/tests/common/model_cache.rs` lines 5-8 doc-comment block — `rg -q 'cube_4color\.3mf' crates/slicer-runtime/tests/common/model_cache.rs && ! rg -q 'benchy_4color\.3mf' crates/slicer-runtime/tests/common/model_cache.rs`

No `docs/*.md` change is required — this packet is a test-fixture migration that does not change any pipeline contract.

## Closure Log

This section is the authoritative record for AC-N1 verdicts, the wall-clock measurement required by the Packet Completion Gate, any newly-authored-fixture provenance, and any in-flight packet-doc deviations applied. The implementer fills these subsections in as the migration progresses; **all four subsections must be populated (even if only to record "not applicable") before `status: implemented` is set.**

### Packet-doc deviations applied during implementation

Discovered during Step 1/Step 2 of swarm execution; applied with user approval before code edits began.

- **AC-6, AC-7, Step 5 plan, design.md §Files in Scope**: `threemf_paint_drop_on_modifier_tdd.rs` and `threemf_transform_tdd.rs` live in `crates/slicer-runtime/tests/integration/`, not `tests/e2e/`. Verification commands updated from `--test e2e` to `--test integration`; file-path references updated to `tests/integration/`. Without this fix the gate would silently match `test result: ok. 0 passed; 0 failed`.
- **design.md §Locked Assumption — `cube_4color.3mf` face mapping**: replaced with ground-truth mapping derived from `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs:6-12`. The prior table (`-X=2, +Y=3, -Y=4, ±Z unpainted`) only matched `+X=1`; the actual fixture is more heterogeneous (banded -Y, mixed +Z/-Z, hex-subdivided -X). Strengthened per-face assertions in Steps 3-4 use the corrected mapping.
- **Authoritative-docs pointer**: §"Fixture Strategy" of `docs/specs/orca-paint-segmentation-parity.md` does not exist by that name. Pointer updated to §9 "Test Strategy" (lines ~942-980).
- **design.md Open Question `[FWD]`**: resolved by Step 1 FACT — both `resources/cube_with_modifier_part.3mf` and `resources/cube_rotated_component.3mf` are `absent` at packet start; Steps 3 and 5 will decide whether to author or substitute.

### Weakened-assertion review (AC-N1)

For each entry returned by the AC-N1 inventory grep, record one line in the format:

`<file>:<line> — <JUSTIFIED|REWRITTEN> — <one-line reason if JUSTIFIED, or replacement assertion summary if REWRITTEN>`

If the inventory grep returned zero hits, record exactly: `No weakened-assertion candidates surfaced; AC-N1 trivially satisfied.`

_(Implementer to fill in.)_

### Wall-clock measurement (Packet Completion Gate)

Capture `time cargo test -p slicer-runtime` before and after the migration on the same machine; the expected outcome is a multi-minute reduction driven by the 5.0 MB → 64 KB fixture-size delta.

- Before (commit `<sha>`): `<m>m<s>s`
- After  (commit `<sha>`): `<m>m<s>s`
- Reduction: `<absolute>` (`<percent>%`)

_(Implementer to fill in.)_

### Authored-fixture provenance (AC-N2)

If `resources/cube_with_modifier_part.3mf` and/or `resources/cube_rotated_component.3mf` were authored, record the deterministic authoring command and on-disk size for each so the fixture is reproducible. If neither was authored, record exactly: `Both derivative fixtures unnecessary; existing cube assets sufficed.`

- `resources/cube_with_modifier_part.3mf`: `<command>` — size `<bytes>` — ≤ 100 KB: YES/NO
- `resources/cube_rotated_component.3mf`: `<command>` — size `<bytes>` — ≤ 100 KB: YES/NO

_(Implementer to fill in.)_

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- None. This packet is a fixture migration internal to pinch_n_print and exercises no OrcaSlicer-parity behavior beyond what the cube fixtures already encode.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
