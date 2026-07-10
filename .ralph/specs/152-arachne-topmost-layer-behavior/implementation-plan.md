# Implementation Plan: 152-arachne-topmost-layer-behavior

## Execution Rules

- One atomic step at a time; each maps to gaps G3/G10 from
  `docs/18_arachne_parity_audit.md`.
- TDD first (red gap tests exist; G3 part 2 gets packet-authored tests).
- WIT/pipeline plumbing lands before any module test is trusted (stale guests
  mask both gaps).
- Honor the context-discipline preamble; per-step fields are the budget contract.

## Steps

### Step 1: Extend arachne-params WIT record + Rust mirror + adapter (G10 plumbing)

- Gaps: G10 (plumbing), G3 (shared flags).
- Objective: add `is-bottom-layer` / `is-topmost-layer` to `common.wit`
  `arachne-params`; mirror in `ArachneParams` (+ `Default = false`); add adapter
  arms in `slicer-sdk`/`slicer-macros`; rebuild guests.
- Precondition: packet active (150 + 151 implemented).
- Postcondition: AC-4 (`rg` finds `is-topmost-layer` in `common.wit`); workspace
  + guests build; no boundary type-identity mismatch.
- Files read: `crates/slicer-schema/wit/deps/common.wit:22-50`,
  `crates/slicer-core/src/arachne/pipeline.rs:145-208`,
  `crates/slicer-sdk/src/host.rs` (ArachneParams adapter region).
- Files edit (≤3 per touch): `common.wit`, `pipeline.rs`, then
  `host.rs`+`slicer-macros/src/lib.rs` (adapter) as a second touch.
- Out-of-bounds: generated `*/wit-guest/` bindgen.
- Dispatches: "Find every reference to `arachne-params`/`ArachneParams` across
  `wit_host.rs`, `dispatch.rs`, adapter modules; LOCATIONS."; "Run `cargo build
  --tests`; FACT."; "Run `cargo xtask build-guests`; FACT success + failing
  guests."
- Context cost: `M`.
- Docs: `CLAUDE.md` §"WIT/Type Changes Checklist" (load); `docs/03` (delegate
  arachne-params section).
- OrcaSlicer refs: `PerimeterGenerator.cpp:2153-2154` (flag derivation).
- Verification: AC-4 `rg`; `cargo build --tests`; guests rebuild clean.
- Exit condition: AC-4 green; all guests fresh with the two new fields.

### Step 2: removeSmallLines top/bottom exception (G10)

- Gaps: G10.
- Objective: `remove_small_lines` keys the lenient `min_width/2` threshold on
  `is_bottom || is_topmost`; `run_arachne_pipeline` passes the flags from
  `params`. Adapt the G10 gap test's CALL to supply the topmost flag `true`
  (assertion `!surviving.is_empty()` unchanged — see requirements §Step
  Completion). Audit `is_initial_layer` consumers before subsuming it.
- Precondition: Step 1 landed.
- Postcondition: AC-3 (G10 gap test green), AC-N1 (mid-stack strict-drop).
- Files read: `crates/slicer-core/src/arachne/remove_small.rs:42-82`,
  `crates/slicer-core/src/arachne/pipeline.rs:317-321`.
- Files edit (≤3): `remove_small.rs`, `pipeline.rs`,
  `crates/slicer-runtime/tests/arachne_parity_gaps.rs` (G10 CALL only).
- Out-of-bounds: the module (Step 3+).
- Dispatches: "Find all readers of `ArachneParams.is_initial_layer`; LOCATIONS.";
  "Find all callers of `remove_small_lines`/`run_arachne_pipeline`; LOCATIONS.";
  "Run the G10 gap test + `cargo test -p slicer-core --lib arachne::remove_small
  -- non_top_layer_strict`; FACT."
- Context cost: `M`.
- Docs: none beyond Step 1.
- OrcaSlicer refs: `WallToolPaths.cpp:684-700`.
- Verification: G10 gap test + AC-N1 test.
- Exit condition: AC-3 + AC-N1 green; `is_initial_layer` disposition recorded.

### Step 3: Topmost detection + only_one_wall_top single wall (G3 part 1)

- Gaps: G3.
- Objective: read `SliceRegionView::top_shell_index` in the module; set the WIT
  `is_topmost_layer`/`is_bottom_layer` flags; when `only_one_wall_top=true` and
  the region is topmost (`top_shell_index == Some(0)`), force a single wall.
- Precondition: Steps 1-2 landed (flags exist and flow).
- Postcondition: AC-1 (G3 gap test green) and AC-N2 (key off → full count).
- Files read: `arachne-perimeters/src/lib.rs:293,305-306` + emission region,
  `crates/slicer-sdk/src/views.rs:184-210`,
  `crates/slicer-runtime/src/slice_postprocess_prepass.rs:144-149`.
- Files edit (≤3): `modules/core-modules/arachne-perimeters/src/lib.rs`.
- Out-of-bounds: the host prepass (read-only — confirm population only).
- Dispatches: "Run the G3 gap test + `cargo test -p arachne-perimeters --lib --
  only_one_wall_top_disabled`; FACT."; "Run `cargo xtask build-guests --check`;
  FACT."
- Context cost: `M`.
- Docs: `docs/02` (delegate SliceRegionView top-shell section).
- OrcaSlicer refs: `PerimeterGenerator.cpp:2140-2144`.
- Verification: G3 gap test (AC-1) + AC-N2.
- Exit condition: AC-1 + AC-N2 green.

### Step 4: Second WallToolPaths pass for non-topmost top surfaces (G3 part 2)

- Gaps: G3.
- Objective: for a NON-topmost region with a top sub-area, run the second pass —
  top-area derivation (prefer PnP `top_solid_fill` if it matches Orca's diff;
  else derive), bridge exclusion, `min_width_top_surface` filter via packet-150
  `get_abs_value(perimeter_width)`, `offset2_ex` shrink/expand, second
  `WallToolPaths` over the non-top area with `inner_loop_number + 1` walls,
  `inset_idx += 1` renumbering, merge, empty-top fallback rerun.
- Precondition: Step 3 landed.
- Postcondition: AC-2 — top sub-area emits one wall; merged inner walls have
  `inset_idx` incremented by 1 vs a naive single pass.
- Files read: `arachne-perimeters/src/lib.rs` (emission + params region);
  the Orca SUMMARY (delegated).
- Files edit (≤3): `modules/core-modules/arachne-perimeters/src/lib.rs`,
  `modules/core-modules/arachne-perimeters/tests/*` (packet-authored test), or an
  in-module `#[cfg(test)]` module for `only_one_wall_top_second_pass`.
- Out-of-bounds: `OrcaSlicerDocumented/**` (SUMMARY only); beading engine.
- Dispatches: "Summarize `PerimeterGenerator.cpp:2160-2246` second-pass
  algorithm; SUMMARY ≤200 words."; "Run `cargo test -p arachne-perimeters --lib
  -- only_one_wall_top_second_pass`; FACT or SNIPPETS."
- Context cost: `M`.
- Docs: `docs/08` (offset unit conversions).
- OrcaSlicer refs: `PerimeterGenerator.cpp:2160-2246`.
- Verification: AC-2 packet-authored test asserting single top wall +
  renumbering.
- Exit condition: AC-2 green; any top-area-source divergence recorded as a
  deviation.

### Step 5: Docs, deviation closure, guest freshness

- Gaps: bookkeeping.
- Objective: document the two WIT fields (docs/03), `only_one_wall_top` now
  behavioral + `min_width_top_surface` consumed (docs/15), mark G3/G10 closed
  (docs/18), close D-104d (DEVIATION_LOG).
- Precondition: Steps 1-4 green.
- Postcondition: every Doc Impact grep hits.
- Files read/edit (docs only, sequential): docs/03, docs/15, docs/18,
  DEVIATION_LOG.md.
- Dispatches: "Run each Doc Impact grep; FACT all-hit / misses."
- Context cost: `S`.
- Verification: Doc Impact grep suite; `cargo xtask build-guests --check` clean.
- Exit condition: all greps hit; guests fresh.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | WIT record + mirror + adapter + guest rebuild |
| Step 2 | M | removeSmallLines flag + G10 call adaptation |
| Step 3 | M | topmost detection + single-wall force |
| Step 4 | M | second-pass port (largest; SUMMARY-gated) |
| Step 5 | S | docs + closure |

Aggregate: `M` (one M step at a time; no L — the second pass is bounded by a
delegated SUMMARY, not a raw load).

## Packet Completion Gate

- All 5 steps complete; every exit condition met.
- G3 + G10 gap tests green; packet-authored G3-part-2/negative tests green; all
  other gap tests (G1/G2/G4-G9) already green from 150/151 and unchanged.
- 14 `arachne_parity.rs` locks green, incl. the `only_one_wall_top` source-read
  lock.
- `cargo check`/`clippy --workspace --all-targets` clean;
  `cargo xtask build-guests --check` clean (mandatory — WIT change).
- Doc Impact greps hit; D-104d closed. With this packet, gaps G1–G10 are all
  closed (G11 excluded by decision).

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Workspace gate via sub-agent: `cargo xtask test --summary --workspace` — FACT
  PASS/FAIL + failing-test list only.
- Record the G10 test-call adaptation, the topmost-area-source decision, and the
  `is_initial_layer` disposition explicitly before `status: implemented`.
- Confirm implementer peak context < 70%; log if not.
