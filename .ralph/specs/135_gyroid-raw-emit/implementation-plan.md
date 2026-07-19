# Implementation Plan: 135_gyroid-raw-emit

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 0: Pre-activation dependency check

- Task IDs: none (verification only).
- Objective: confirm preconditions for activation. Three FACT dispatches, no code changes.
  - `rg -q 'TASK-256.*\[[xX]\]' docs/07_implementation_status.md` (TASK-256 closed —
    per-region config delivery is in).
  - `rg -c 'fn (clip_polyline_to_expolygon|point_in_expolygon|point_in_polygon|polygon_bbox_mm)' modules/core-modules/gyroid-infill/src/lib.rs` returns 4 today (the functions to delete).
  - `rg 'claim:sparse-fill' modules/core-modules/gyroid-infill/gyroid-infill.toml` returns 1
    (the sole current claim; manifest gains three).
- Precondition: clean tree.
- Postcondition: a one-line PASS/FAIL note recorded.
- Files allowed to edit: none (read-only verification step).
- Context cost: `S`.
- Exit condition: PASS. If FAIL, file a deviation or refuse activation.

### Step 1: RED — author the AC tests + claims addition

- Task IDs:
  - `TASK-260`
- Objective: add the manifest's three solid claims (AC-5 goes green immediately); author the
  new tests (`square_10mm_z_0p2_emits_raw_waves`,
  `rotated_square_45_matches_unrotated_after_inverse`, `align_to_grid_snaps_bbox_min`,
  `expand_factor_is_10x_spacing`, `default_holders_gyroid_sparse_only`) — RED against current
  behavior; survey which existing tests pin the rotation block and the broken clipper
  output. No point-in-polygon tests exist (FACT I 2026-07-19) so the spec's "delete
  point-in-polygon tests" is moot.
- Precondition: packet 131 closed (verified Step 0).
- Postcondition: manifest updated; new tests RED; the 11 existing tests stay green; guest
  rebuild adjudicated (manifest edit feeds the guest build).
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
- Exit condition: claims in; new tests RED; rotation-block-affected test inventory recorded
  in the test file header.

### Step 2: GREEN — rotation-order fix + deletions + align_to_grid + expand

- Task IDs:
  - `TASK-260`
- Objective: replace the rotation block (lib.rs:344) with the polygon-first ordering
  (FillGyroid.cpp:300-376); delete `clip_polyline_to_expolygon` (lib.rs:611) /
  `point_in_expolygon` (lib.rs:570) / `point_in_polygon` (lib.rs:585) / `polygon_bbox_mm`
  (lib.rs:551); add `align_to_grid` (grid constant via FACT dispatch) and the 10× expand
  (lib.rs:259: `4.0` → `10.0`); per-region density via the 131 accessor; emit raw waves.
- Precondition: Step 1 RED state.
- Postcondition: AC-1…AC-4, AC-6, AC-N1 green; wave-core byte-identical at
  `gyroid_f` (lib.rs:394), `make_one_period` (lib.rs:430), `make_wave` (lib.rs:491).
- Files allowed to read: own module only.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/gyroid-infill/src/lib.rs`
  - `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` (fixture tweaks +
    rotation-block-affected test rewrites)
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
- Exit condition: full module suite green; deletions grep-clean; wave-core byte-identical
  (diff of lib.rs:394, lib.rs:430, lib.rs:491 is empty).

### Step 3: Gates + carve delta

- Task IDs:
  - `TASK-260`
- Objective: rebuild guests; run module + workspace gates; append newly-affected goldens to
  the 131 carve list (recorded deviation). The 5 `carved: infill-parity D6` markers in
  `crates/slicer-runtime/tests/executor/cube_4color_*` are NOT touched here — those are
  packet 136's restore-and-bless scope; this packet only appends to the carve list if the
  rewrite breaks a non-infill test.
- Precondition: Step 2 exit condition.
- Postcondition: gates green; carve delta recorded (likely empty for this packet).
- Files allowed to read: carve list.
- Files allowed to edit (≤ 3):
  - `.ralph/specs/131_per-region-config-delivery/carve-list.md` (append-only, if needed)
- Files explicitly out-of-bounds for this step: every other packet's surface.
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
- Exit condition: all packet ACs green; carve delta recorded (likely empty).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | pre-activation FACT |
| Step 1 | M | claims + RED suite + surveys |
| Step 2 | M | the fixes + deletions |
| Step 3 | S | gates + carve delta (likely empty) |

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
