# Implementation Plan: 135_gyroid-raw-emit

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: RED — author the AC tests + claims addition

- Task IDs:
  - `TASK-260`
- Objective: add the manifest's three solid claims (AC-5 goes green immediately); author the
  new tests (`square_10mm_z_0p2_emits_raw_waves`,
  `rotated_square_45_matches_unrotated_after_inverse`, `align_to_grid_snaps_bbox_min`,
  `expand_factor_is_10x_spacing`, `default_holders_gyroid_sparse_only`) — RED against current
  behavior; delete the point-in-polygon tests together with a stub-level survey of which
  existing tests pin clipped output.
- Precondition: packet 134 closed; clean tree.
- Postcondition: manifest updated; new tests RED; wave-core tests still green; guest rebuild
  adjudicated (manifest edit feeds the guest build).
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/gyroid-infill/src/lib.rs` — full (695 lines, one read)
  - `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` — full
- Files allowed to edit (≤ 3):
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml`
  - `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs`
- Files explicitly out-of-bounds for this step: production `lib.rs` (RED first).
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE"
  - "Run `cargo test -p gyroid-infill 2>&1 | tee target/test-output.log | grep -E '^test
    |^test result'`; FACT per-test" — RED confirmation
- Context cost: `M`
- Authoritative docs: ADR-0027 (full), spec §Phase 3.
- OrcaSlicer refs: none this step.
- Verification:
  - AC-5 rg one-liner — FACT
- Exit condition: claims in; new tests RED; clipped-output test inventory recorded in the
  test file header.

### Step 2: GREEN — rotation-order fix + deletions + align_to_grid + expand

- Task IDs:
  - `TASK-260`
- Objective: replace the rotation block with the polygon-first ordering
  (FillGyroid.cpp:300-376); delete `clip_polyline_to_expolygon` / `point_in_expolygon` /
  `point_in_polygon` / `polygon_bbox_mm` and the short-filter/chaining if present; add
  `align_to_grid` (grid constant via FACT dispatch) and the 10× expand; per-region density
  via the 131 accessor; emit raw waves.
- Precondition: Step 1 RED state.
- Postcondition: AC-1…AC-4, AC-6, AC-N1 green; wave-core diff empty.
- Files allowed to read: own module only.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/gyroid-infill/src/lib.rs`
  - `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` (fixture tweaks +
    clipped-output test rewrites)
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the FillGyroid.cpp:300-376 SUMMARY + SNIPPETS dispatch (design §Expected Sub-Agent
    Dispatches)
  - the align_to_grid grid-constant FACT
  - "Run `cargo test -p gyroid-infill …`; FACT + counts; SNIPPETS ≤20 on failure"
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` (delegate).
- OrcaSlicer refs: FillGyroid.cpp:300-376, :322, :326 — delegate.
- Verification:
  - `cargo test -p gyroid-infill 2>&1 | tee target/test-output.log | grep "^test result"` — FACT
  - AC-6 rg one-liner — FACT
- Exit condition: full module suite green; deletions grep-clean; wave-core byte-identical.

### Step 3: Gates + carve delta

- Task IDs:
  - `TASK-260`
- Objective: rebuild guests; run module + workspace gates; append newly-affected goldens to
  the 131 carve list (recorded deviation).
- Precondition: Step 2 exit condition.
- Postcondition: gates green; carve delta recorded (possibly empty).
- Files allowed to read: carve list.
- Files allowed to edit (≤ 3):
  - `.ralph/specs/131_per-region-config-delivery/carve-list.md` (append-only, if needed)
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE"
  - "Run `cargo clippy -p gyroid-infill --all-targets -- -D warnings` + `cargo check
    --workspace --all-targets`; FACT each"
  - "Run the non-carved golden subset; FACT; list newly red" — carve delta
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - the §Verification gates — FACT each
- Exit condition: all packet ACs green; carve delta recorded.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | claims + RED suite + surveys |
| Step 2 | M | the fixes + deletions |
| Step 3 | S | gates + carve delta |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-260 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
