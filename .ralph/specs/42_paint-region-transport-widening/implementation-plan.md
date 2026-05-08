# Implementation Plan: paint-region-transport-widening

## Execution Rules

- One atomic step at a time.
- Each step maps back to `TASK-130c` (the new backlog row this packet registers).
- TDD first, then implementation, then narrowest falsifying validation.
- Each step honors the context-discipline preamble. The fields below are the budget contract, not optional metadata.
- The macro `PrePass::PaintSegmentation` arm at `crates/slicer-macros/src/lib.rs:1760-1788` is **out of bounds** for every step in this packet; that is Packet 43's territory.

## Steps

### Step 0: Lock the open design decisions via FACT dispatches (no edits)

- Task IDs:
  - `TASK-130c`
- Objective: resolve the three Step-0 design questions before any non-trivial edit so the rest of the plan is deterministic.
- Precondition: Packet 06 is `implemented` (verified at packet authoring).
- Postcondition: the implementer has FACT-confirmed answers, recorded inline in this file as a one-paragraph "Step 0 Notes" addendum, for:
  1. Whether `wit/deps/ir-types.wit` already declares a `paint-value-input` (or equivalently shaped) variant.
  2. Whether `PaintRegionEntry::paint_order` is read anywhere besides the host-harvest enumeration index.
  3. Whether `slicer_ir::ExPolygon` can be re-exported (public fields + Clone/Debug derives) or if a wrapper `ExPolygonView` is required.
  4. Whether the harvest's Custom mapping needs an additive `PaintValue::Custom(String)` IR variant or whether the Custom payload rides solely on `PaintSemantic::Custom`.
  5. The exact insertion line in `docs/07_implementation_status.md` for TASK-130c (sibling of 130b at line 70) and the blocker-list line (~180).
  6. Whether `test-guests/build-test-guests.sh` runs on the local Windows toolchain or requires WSL / CI dispatch.
- Files allowed to read: none directly; this step is **pure dispatch**.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds for this step: every source file. This step is FACT-only.
- Expected sub-agent dispatches:
  - "Search `wit/deps/ir-types.wit` for `paint-value-input` (and `paint-value`, `paint-value-view`); return SNIPPETS (≤ 30 lines) of any matching variant declarations." — return format: SNIPPETS.
  - "Find every reader of `PaintRegionEntry::paint_order` (the SDK field) across `crates/slicer-sdk/`, `crates/slicer-host/`, `modules/core-modules/`. Return LOCATIONS." — return format: LOCATIONS.
  - "Show the `slicer_ir::ExPolygon` struct definition + derives (search `crates/slicer-ir/src/`); return ≤ 15 lines." — return format: SNIPPETS.
  - "Show the IR `PaintValue` enum (`crates/slicer-ir/src/slice_ir.rs`); return the enum definition + derives, ≤ 15 lines." — return format: SNIPPETS.
  - "Open `docs/07_implementation_status.md` to lines 65-75 and lines 175-185; return SNIPPETS." — return format: SNIPPETS.
  - "Run `which wasm32-wasi cargo-component`; return FACT (which paths exist or 'not found'). If 'not found', recommend the WSL path or a CI-dispatch alternative." — return format: FACT.
- Context cost: `S` (pure dispatch).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: the Step 0 Notes paragraph appended to this file MUST contain a binary answer for each of the six FACT items above.
- Exit condition: each Step-0 question has a recorded binary answer.

### Step 1: Register TASK-130c + DEV-025 mismatches 4 and 5 in docs (precision edits only)

- Task IDs:
  - `TASK-130c`
- Objective: add the backlog row + the deviation entries so subsequent steps reference real registered work.
- Precondition: Step 0 Notes addendum present.
- Postcondition: `docs/07_implementation_status.md` carries a TASK-130c row near 130a/130b and a TASK-130c entry in the blocker list at line ~180; `docs/DEVIATION_LOG.md` DEV-025 carries mismatches 4 and 5 with status `open` (will close at packet acceptance); `docs/14_deviation_audit_history.md` DEV-025 row references TASK-130c.
- Files allowed to read:
  - `docs/07_implementation_status.md` lines 65-75 and 175-185 (already retrieved in Step 0)
  - `docs/DEVIATION_LOG.md` (whole file — keep < 500 lines or delegate; per packet authoring scan it is small)
  - `docs/14_deviation_audit_history.md` (DEV-025 row only)
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
- Files explicitly out-of-bounds for this step: every source file.
- Expected sub-agent dispatches:
  - "Re-read `docs/07_implementation_status.md` lines 65-72 and 175-185 after edit; return the full edited region as SNIPPETS to confirm TASK-130c is positioned correctly." — return format: SNIPPETS.
- Context cost: `S`.
- Authoritative docs: `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` (all targeted line ranges only).
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-host --test paint_region_transport_widening_tdd docs_07_registers_task_130c -- --exact --nocapture` and `cargo test -p slicer-host --test paint_region_transport_widening_tdd dev_log_extends_dev025_with_4_and_5 -- --exact --nocapture` (both will RED until the TDD file is authored in Step 7; Step 1 acceptance for now is "the doc lines exist and the markdown lints").
- Exit condition: a worker dispatch returns the inserted snippets verbatim and they match the expected layout.

### Step 2: Author RED tests for the SDK-side and host-side acceptance criteria

- Task IDs:
  - `TASK-130c`
- Objective: stand up the two new test files with all positive + negative AC tests in RED state. TDD anchor for the rest of the packet.
- Precondition: Step 0 Notes locked.
- Postcondition: `crates/slicer-sdk/tests/paint_region_transport_widening_tdd.rs` and `crates/slicer-host/tests/paint_region_transport_widening_tdd.rs` both exist; each contains the test names called out in `packet.spec.md` Acceptance Criteria; each test currently FAILS (RED) because the underlying types/fields do not yet exist.
- Files allowed to read:
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` — for SDK-side test patterns (read first 100 lines for harness)
  - `crates/slicer-host/tests/dispatch_tdd.rs` lines 5349-5441 — for host-side direct-wiring patterns
  - `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` — for IR-shape assertion patterns
- Files allowed to edit (≤ 3):
  - `crates/slicer-sdk/tests/paint_region_transport_widening_tdd.rs` (NEW)
  - `crates/slicer-host/tests/paint_region_transport_widening_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step: production source files. The point of TDD is to write the test first.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd -- --nocapture`; return FACT (count of failures + first failure assertion if compile-OK; otherwise compile error snippet)." — return format: FACT or SNIPPETS.
  - "Run `cargo test -p slicer-host --test paint_region_transport_widening_tdd -- --nocapture`; return FACT or SNIPPETS." — return format: FACT or SNIPPETS.
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` PaintRegionIR section — direct read; narrow.
  - `docs/05_module_sdk.md` — delegate SUMMARY.
- OrcaSlicer refs: none directly used in test code; cite in test doc-comments.
- Verification: each test compiles or fails-to-compile with the expected error messages (the SDK tests fail because `ExPolygonView` does not exist; the host tests fail because the WIT generated bindings still have `value: String`).
- Exit condition: every Acceptance Criterion test name from `packet.spec.md` is present, the file compiles or fails with a *predictable* compile error pattern, and the failure pattern is recorded in commit message / step log.

### Step 3: SDK widening — `ExPolygonView` + `PaintRegionEntry` + `push_paint_region`

- Task IDs:
  - `TASK-130c`
- Objective: replace the SDK's hole-blind shape with the typed, hole-bearing shape.
- Precondition: Step 2 RED tests exist and FAIL on SDK-side missing types.
- Postcondition: `crates/slicer-sdk/tests/paint_region_transport_widening_tdd.rs::sdk_paint_region_entry_carries_expolygon_view`, `::sdk_push_paint_region_preserves_holes_and_typed_value`, `::contour_points_api_is_fully_removed` are all GREEN.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — only the ExPolygon, Polygon, Point2 region (Step 0 located line range)
  - `crates/slicer-sdk/src/prepass_builders.rs` — full file (it is < 600 lines per packet authoring scan)
- Files allowed to edit (≤ 3):
  - `crates/slicer-sdk/src/prepass_builders.rs`
  - (optionally one neighboring SDK file if `ExPolygonView` lives in a different module; Step 0 selects)
- Files explicitly out-of-bounds for this step: WIT files, host source, canonical guest, macro source.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd sdk_paint_region_entry_carries_expolygon_view sdk_push_paint_region_preserves_holes_and_typed_value contour_points_api_is_fully_removed -- --exact --nocapture`; return FACT pass/fail." — return format: FACT.
  - "Run `cargo build -p slicer-sdk`; return FACT (compile success + warning count, or first error)." — return format: FACT or SNIPPETS.
- Context cost: `S` to `M`.
- Authoritative docs: `docs/05_module_sdk.md` — delegate SUMMARY for the `PaintSegmentationOutput` section.
- OrcaSlicer refs: none.
- Verification: three named SDK tests GREEN; `cargo build -p slicer-sdk` succeeds.
- Exit condition: SDK builds; SDK-side AC tests for the SDK shape pass.

### Step 4: WIT widening — `paint-value-input` variant + `paint-region-entry.value` retype + inline-WIT mirror

- Task IDs:
  - `TASK-130c`
- Objective: change the WIT contract surface in canonical and inline forms.
- Precondition: Step 3 SDK GREEN.
- Postcondition: `wit/world-prepass.wit` and `crates/slicer-macros/src/lib.rs` lines 1283-1314 declare identical `paint-region-entry` records (modulo whitespace), with `value: paint-value-input`. `wit/deps/ir-types.wit` declares `paint-value-input` (or the existing equivalent is reused per Step 0).
- Files allowed to read:
  - `wit/world-prepass.wit`
  - `wit/deps/ir-types.wit`
  - `crates/slicer-macros/src/lib.rs` — only lines 1283-1314 (inline-WIT block); use Grep to land precisely
- Files allowed to edit (≤ 3):
  - `wit/world-prepass.wit`
  - `wit/deps/ir-types.wit`
  - `crates/slicer-macros/src/lib.rs` (inline-WIT block only — line range 1283-1314)
- Files explicitly out-of-bounds for this step: macro arm bodies (lib.rs:1730-1788), host source, canonical guest, SDK.
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; return FACT (compile-only pass) or first compile-error snippet." — return format: FACT or SNIPPETS. (Build will fail until Step 5 wires the host side; that's expected.)
  - "Diff the `paint-region-entry` records in `wit/world-prepass.wit` vs `crates/slicer-macros/src/lib.rs:1283-1314`; return SNIPPETS of any non-whitespace difference." — return format: SNIPPETS.
- Context cost: `S`.
- Authoritative docs: `docs/03_wit_and_manifest.md` — delegate SUMMARY for WIT version-bumping rule (the deviation rationale is already locked in design.md).
- OrcaSlicer refs: none.
- Verification: WIT diff returns "no differences"; `cargo build --workspace` fails at the host wit_host.rs / dispatch.rs sites (the next steps fix these — failure is expected here).
- Exit condition: WIT files updated; canonical and inline byte-match modulo whitespace.

### Step 5: Host widening — `wit_host.rs` validator + `dispatch.rs` harvest 1:1 typed mapping

- Task IDs:
  - `TASK-130c`
- Objective: update host-side validation + drop `parse_value`, replacing with typed match. Migrate `dispatch_tdd.rs::paint_segmentation_output_rejects_invalid_entries`.
- Precondition: Step 4 WIT GREEN; build is RED at host sites only.
- Postcondition: `cargo build --workspace` succeeds; `cargo test -p slicer-host --test dispatch_tdd paint_segmentation_output_rejects_invalid_entries` passes; `cargo test -p slicer-host --test paint_region_transport_widening_tdd host_harvest_drops_string_parsing wit_paint_region_entry_value_is_typed_variant` passes.
- Files allowed to read:
  - `crates/slicer-host/src/wit_host.rs` — only lines 1371-1376 (paint_region_entries field) and 4074-4102 (HostPaintSegmentationOutput::push_paint_region) and 2300-2330 (existing typed-variant helpers for naming convention)
  - `crates/slicer-host/src/dispatch.rs` — only lines 1954-2045 (`harvest_paint_segmentation_ir`)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/dispatch_tdd.rs` (lines 5349-5441 only)
- Files explicitly out-of-bounds for this step: macro source, canonical guest, SDK, WIT files (already done in Step 4).
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; return FACT pass/fail." — return format: FACT.
  - "Run `cargo test -p slicer-host --test dispatch_tdd paint_segmentation_output_rejects_invalid_entries -- --exact --nocapture`; return FACT pass/fail." — return format: FACT.
  - "Run `cargo test -p slicer-host --test paint_region_transport_widening_tdd host_harvest_drops_string_parsing wit_paint_region_entry_value_is_typed_variant inline_and_canonical_wit_match -- --nocapture`; return FACT pass/fail." — return format: FACT.
  - "Grep `crates/slicer-host/src/dispatch.rs` within `harvest_paint_segmentation_ir` for `parse_value`, `parse::<u32>()`, `parse::<f32>()`; return FACT (zero matches expected)." — return format: FACT.
- Context cost: `M`.
- Authoritative docs: `docs/02_ir_schemas.md` PaintRegionIR section — direct, narrow.
- OrcaSlicer refs: none.
- Verification: all four named tests pass; harvest contains zero string-parse calls; workspace builds.
- Exit condition: host-side AC tests GREEN; build green.

### Step 6: Canonical guest migration

- Task IDs:
  - `TASK-130c`
- Objective: migrate `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs::run_paint_segmentation` to construct typed `paint-region-entry`.
- Precondition: Step 5 host-side GREEN.
- Postcondition: `cargo test -p paint-segmentation -- --nocapture` passes; the file no longer contains `value: entry.value.clone()` (the String-typed assignment); the guest's emit constructs `value: PaintValueInput::ToolIndex(...)` (or the parsed equivalent depending on the input config grammar).
- Files allowed to read:
  - `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` — full file (≤ 500 lines per packet authoring scan)
  - The neighboring `modules/core-modules/paint-segmentation/src/lib.rs` if needed for context (read-only)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs`
  - (optionally) `modules/core-modules/paint-segmentation/wit-guest/Cargo.toml` if the bindgen target needs adjustment
- Files explicitly out-of-bounds for this step: every other module, the macro, the host, the SDK.
- Expected sub-agent dispatches:
  - "Run `cargo test -p paint-segmentation -- --nocapture`; return FACT pass/fail or first failing-test SNIPPETS." — return format: FACT or SNIPPETS.
  - "Grep `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` for the literal `value: entry.value.clone()`; return FACT (zero expected)." — return format: FACT.
- Context cost: `S` to `M`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: canonical guest tests GREEN; grep confirms removal.
- Exit condition: paint-segmentation crate builds and tests pass.

### Step 7: Rebuild `prepass-guest.component.wasm` + IR-side regression sweep

- Task IDs:
  - `TASK-130c`
- Objective: ensure the pre-built test guest (used by `macro_paint_region_roundtrip_tdd`) is consistent with the new WIT shape, and that all IR-level paint-region tests pass.
- Precondition: Steps 5 + 6 GREEN.
- Postcondition: `test-guests/prepass-guest.component.wasm` is rebuilt; `macro_paint_region_roundtrip_tdd`, `paint_segmentation_executor_tdd`, `slice_postprocess_paint_annotation_tdd`, `paint_annotation_integration_tdd` all GREEN.
- Files allowed to read:
  - `test-guests/build-test-guests.sh` — for command preview
- Files allowed to edit (≤ 3):
  - `test-guests/prepass-guest.component.wasm` (artifact regeneration; not a manual edit)
- Files explicitly out-of-bounds for this step: every source file. This step is build + test only.
- Expected sub-agent dispatches:
  - "Run `./test-guests/build-test-guests.sh`; return FACT (success line + new size of `prepass-guest.component.wasm`). If toolchain missing, return FACT including the missing-tool name + recommendation (WSL or CI dispatch)." — return format: FACT.
  - "Run `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd -- --nocapture`; return FACT pass/fail." — return format: FACT.
  - "Run `cargo test -p slicer-host --test paint_segmentation_executor_tdd --test slice_postprocess_paint_annotation_tdd --test paint_annotation_integration_tdd -- --nocapture`; return FACT pass/fail." — return format: FACT.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all four named tests GREEN; component .wasm size differs from pre-rebuild.
- Exit condition: pre-built guest rebuilt + all IR-level paint-region tests GREEN.

### Step 8: Acceptance ceremony — full AC sweep + DEV-025 status reconciliation

- Task IDs:
  - `TASK-130c`
- Objective: run every Acceptance Criterion command from `packet.spec.md`; mark DEV-025 mismatches 4 and 5 closed-by-Packet-42 with the closure date in `docs/DEVIATION_LOG.md` and `docs/14_deviation_audit_history.md`; transition `packet.spec.md` to `status: implemented`.
- Precondition: Steps 0-7 GREEN.
- Postcondition: every AC test passes; DEV-025 entry shows mismatches 4 + 5 closed; mismatch 3 remains open; `packet.spec.md` `status: implemented`.
- Files allowed to read:
  - `packet.spec.md` — for the full AC list
  - `docs/DEVIATION_LOG.md` and `docs/14_deviation_audit_history.md`
- Files allowed to edit (≤ 3):
  - `.ralph/specs/42_paint-region-transport-widening/packet.spec.md` (status flip)
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
- Files explicitly out-of-bounds for this step: every source file (acceptance ceremony does not touch source).
- Expected sub-agent dispatches:
  - "Run each pipe-suffixed verification command from `packet.spec.md` Acceptance Criteria + Negative Test Cases. Return one FACT per command (pass/fail)." — return format: FACT list.
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail." — return format: FACT.
  - "Run `cargo test --workspace` once; return FACT (pass/fail + failing test count). This is the closure gate; not for use during iterations." — return format: FACT.
- Context cost: `S` (pure dispatch + small doc edits).
- Authoritative docs: `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md`.
- OrcaSlicer refs: none.
- Verification: AC sweep all GREEN; `cargo clippy` clean; `cargo test --workspace` clean.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Pure FACT dispatch — no edits. |
| Step 1 | S | Three doc edits. |
| Step 2 | M | Two new test files; many test stubs. |
| Step 3 | S/M | One SDK file, possibly two. |
| Step 4 | S | WIT mirror; small line ranges. |
| Step 5 | M | Two host files + one test fixture migration. |
| Step 6 | S/M | One canonical guest file. |
| Step 7 | S | Build + test sweep, no source edits. |
| Step 8 | S | Acceptance ceremony, FACT dispatches. |

Aggregate: **M**. No single step is L. If during execution any step trends toward L, split it before continuing.

## Packet Completion Gate

- All steps complete.
- Every step exit condition met.
- Every Acceptance Criterion command from `packet.spec.md` returned PASS via dispatch.
- `docs/07_implementation_status.md` updated for TASK-130c.
- `docs/DEVIATION_LOG.md` DEV-025 mismatches 4 + 5 closed; mismatch 3 still open (closes in Packet 43).
- `docs/14_deviation_audit_history.md` DEV-025 row references TASK-130c closure of mismatches 4 + 5.
- No reopened/superseded packets to reconcile.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (Acceptance Criteria + Negative Test Cases sections).
- Confirm packet-level Verification commands are GREEN (one workspace `cargo build`, one `cargo clippy --workspace`, one `cargo test --workspace` — the latter only at this ceremony).
- Confirm the implementer's peak context usage stayed under 70%; if not, log a packet-authoring lesson for future spec-packet-generator runs (e.g., "wit_host.rs line-range hints insufficient — split Step 5").
- Record any remaining packet-local risk (most likely: any test that relied on the old string-coerced fallback path that the regression sweep didn't surface).

## Step 0 Notes (locked 2026-05-08)

The six Step-0 FACT dispatches returned the following binary answers:

1. **paint-value-input variant in `wit/deps/ir-types.wit`**: NOT_FOUND. The existing `paint-value` (lines 38-42) and `paint-value-view` (lines 216-220) declare only 3 cases (`flag` / `scalar` / `tool-index`); neither has `custom`. **Decision**: ADD a new variant `paint-value-input { flag(bool), scalar(f32), tool-index(u32), custom(string) }` — do not extend the existing read-side variants.
2. **`PaintRegionEntry::paint_order` readers**: NOT droppable. Production readers exist at `crates/slicer-host/src/paint_segmentation.rs:166,191,262` (conflict detection + deterministic sort). **Decision**: KEEP `paint_order` parameter on `push_paint_region` and field on `PaintRegionEntry`.
3. **`slicer_ir::ExPolygon` re-export viability**: cannot re-export. `ExPolygon` stores `Vec<Polygon>`/`Vec<Point2{i64,i64}>` in 100nm units; SDK boundary uses `Vec<[f64;2]>` in mm. **Decision**: define a wrapper `ExPolygonView { contour: Vec<[f64;2]>, holes: Vec<Vec<[f64;2]>> }` in `crates/slicer-sdk/src/prepass_builders.rs`.
4. **`PaintValue::Custom(String)` IR variant**: missing in `slicer-ir`. `PaintSemantic::Custom(String)` already exists, but the value channel needs structured Custom payload to satisfy AC-5 / NEG-2 (Custom must round-trip without coercion to `ToolIndex(0)`). **Decision**: ADD `PaintValue::Custom(String)` as an additive IR variant; document the mapping `paint-value-input::custom(s)` → `PaintValue::Custom(s)` in `harvest_paint_segmentation_ir` as a top-of-function doc comment.
5. **docs/07_implementation_status.md insertion lines**: insert TASK-130c row after line 70 (sibling of 130a/130b); append `TASK-130c` to the blocker list at line 180.
6. **wasm32 / wasm-tools toolchain**: wasm32 target is installed; `cargo-component` is NOT installed but `build-test-guests.sh` does not need it (uses `cargo build --target wasm32-unknown-unknown` + `wasm-tools component new`). bash from Git for Windows is available; wasm-tools availability is being verified in parallel by the planner. If wasm-tools is missing, Step 7 dispatches the rebuild via WSL or records a CI handoff.
