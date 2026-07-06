# Implementation Plan: 147-arachne-cross-cutting-closure

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`none` — provenanced by the cross-cutting closure policies in `docs/specs/arachne-parity-N1-N13-plan.md`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.

## Steps

### Step 1: Confirm A1–E are `status: implemented` + diagnose e2e closure gate residual

- Task IDs:
  - `none` (chain closure — provenanced by `docs/specs/arachne-parity-N1-N13-plan.md` cross-cutting policies)
- Objective: Confirm all 6 chain packets (A1 `141`, A2 `142`, B `143`, C `144`, D `145`, E `146`) are `status: implemented` (their red tests green, their per-packet fixtures re-baselined, their deviation-log entries present). Run the e2e closure gate (`cube_4color_arachne_outer_walls_close_end_to_end`) and the 7 N1–N4 red tests to confirm the chain's acceptance oracles are green. If the e2e gate is still red, diagnose the residual via `pnp_cli slice --instrument-stderr` (per `docs/17_agent_debugging.md` + the `debug-pipeline` skill) and decide: cross-cutting integration issue (F fixes in-scope) or finding-level divergence (F files a follow-up packet).
- Precondition: A1–E are expected to be `status: implemented` (F is the closure gate; if any is still `draft`/`active`, F cannot close).
- Postcondition: Either (a) the e2e gate is green (AC-1 passes) and F proceeds to Step 2, OR (b) the e2e gate is red, the residual is diagnosed, and F either fixes it in-scope (cross-cutting integration issue) or files a follow-up packet (finding-level divergence) and records the decision in its commit message. The 7 N1–N4 red tests are confirmed green (the chain's acceptance oracles).
- Files allowed to read (with line-range hints when > 300 lines):
  - `.ralph/specs/141-arachne-beading-propagation-and-junction-bands/packet.spec.md` — frontmatter only (confirm `status: implemented`).
  - `.ralph/specs/142-arachne-canonical-connectjunctions-emission/packet.spec.md` — frontmatter only.
  - `.ralph/specs/143-arachne-transition-ends-and-extra-ribs/packet.spec.md` — frontmatter only.
  - `.ralph/specs/144-arachne-angle-fudge-and-noncentral-regions/packet.spec.md` — frontmatter only.
  - `.ralph/specs/145-arachne-local-maxima-and-construction-epilogue/packet.spec.md` — frontmatter only.
  - `.ralph/specs/146-arachne-postprocess-order-and-remove-small-simplify/packet.spec.md` — frontmatter only.
  - `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145-1229` — the e2e gate test (AC-1 oracle).
  - `docs/17_agent_debugging.md` — read for the `pnp_cli slice --instrument-stderr` diagnostic protocol (if the e2e gate is red).
- Files allowed to edit (≤ 3):
  - None (Step 1 is a confirmation/diagnosis step; no edits unless the e2e gate is red and F fixes a cross-cutting integration issue in-scope — in which case the edit is the specific integration fix, scoped narrowly).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/*` (A1–E's scope — F does not touch `slicer-core` unless diagnosing a cross-cutting integration residual)
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` (Step 2's scope — the cross-crate fixtures)
  - `OrcaSlicerDocumented/...` (delegate any diagnostic reads)
- Expected sub-agent dispatches:
  - "Read `.ralph/specs/141-arachne-beading-propagation-and-junction-bands/packet.spec.md` frontmatter; return FACT status (draft/active/implemented)" — purpose: confirm A1 is `status: implemented`. (Batch with 142, 143, 144, 145, 146 — 6 dispatches, or one batched SUMMARY returning all 6 statuses.)
  - "Run `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT pass/fail + the `failures.len()/total_checked` summary line" — purpose: validate AC-1 (e2e closure gate).
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — all 7 N1-N4 red tests green)" — purpose: confirm the chain's acceptance oracles.
  - If the e2e gate is RED: "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/f-diagnose.gcode --instrument-stderr 2>&1 | tail -50`; return SNIPPETS (the last 50 lines of the instrumented stderr)" — purpose: diagnose the residual (per `docs/17_agent_debugging.md`).
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/arachne-parity-N1-N13-plan.md` — cross-packet policies (e2e record-only→block-in-F).
  - `docs/17_agent_debugging.md` — the `pnp_cli slice --instrument-stderr` diagnostic protocol (if the e2e gate is red).
- OrcaSlicer refs:
  - None (F owns no new refs; any diagnostic reads are delegated per the `orca-delegation` contract).
- Verification:
  - `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 | tee target/test-output-f-step1-ac1.log` — FACT pass/fail.
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-f-step1-red-suite.log` — FACT pass (expected).
- Exit condition: A1–E confirmed `status: implemented`; 7 N1–N4 red tests green; e2e gate either green (AC-1 passes, proceed to Step 2) or red with a diagnosed residual + a decision (fix in-scope or file follow-up packet). If the e2e gate is red and the residual is a finding-level divergence F cannot fix in-scope, F stays `draft` and files the follow-up packet.

### Step 2: Re-baseline cross-crate `perimeter_parity` fixtures + deviation-log closure + ADR 0035 + `cargo xtask test --workspace --summary` closure ceremony

- Task IDs:
  - `none` (chain closure — cross-cutting artifacts)
- Objective: Re-record the cross-crate `slicer-runtime` `perimeter_parity` fixtures via the `#[ignore]`d `record_*` functions (`perimeter_parity.rs:1101-1854`): `record_tapered_wedge`, `record_narrow_strip_widening`, `record_max_bead_count_cap`, `record_complex_multi_feature`, `record_cube_4color_arachne`. Add `D-147-CHAIN-CLOSURE` deviation-log entry + addenda on `D-141` through `D-146`. Author ADR `0035-arachne-faithful-emission-and-transitions.md`. Add any `CONTEXT.md` glossary gaps A1–E didn't carry. Run `cargo xtask test --workspace --summary` (AC-N1 — the closure ceremony).
- Precondition: Step 1 is green (e2e gate green OR residual diagnosed + fixed in-scope / follow-up filed; 7 N1–N4 red tests green).
- Postcondition: AC-1 (e2e gate) green. AC-2 (cross-crate `perimeter_parity` fixtures) green. AC-N1 (`cargo xtask test --workspace --summary`) PASS. `D-147-CHAIN-CLOSURE` present. ADR 0035 present. `CONTEXT.md` glossary complete.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — range-read the `record_*` function signatures (`:1101-1854`); do NOT full-read.
  - `docs/DEVIATION_LOG.md` — range-read the `D-11X-*` entries (A1–E's) + the `D-147-CHAIN-CLOSURE` insertion point.
  - `docs/adr/0034-arachne-faithful-graph-construction.md` — full (short); ADR 0035 follows it.
  - `CONTEXT.md` — range-read the existing glossary entries (confirm A1–E's additions; identify gaps).
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md` (closure entry + addenda only — no in-place edits to A1–E's narratives)
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (NEW)
  - `CONTEXT.md` (glossary additions for any gaps A1–E didn't carry)
- (Secondary edits not counted against the ≤ 3: `crates/slicer-runtime/tests/fixtures/perimeter_parity/*/expected_perimeter_ir.json` — re-recorded via the `#[ignore]`d `record_*` functions; the JSONs are regenerated, not hand-edited. `docs/07_implementation_status.md` — updated via worker dispatch, never by loading the full backlog.)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/*` (A1–E's scope)
  - `crates/slicer-core/tests/fixtures/arachne/*.json` (A1–E's per-packet scope)
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/*/expected_perimeter_ir.json` (re-record via `record_*`; NEVER read directly)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "Run the `#[ignore]`d `record_*` functions in `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (`record_tapered_wedge`, `record_narrow_strip_widening`, `record_max_bead_count_cap`, `record_complex_multi_feature`, `record_cube_4color_arachne`); return FACT pass/fail (the fixtures are regenerated)" — purpose: re-baseline the cross-crate fixtures (AC-2). (Run via `cargo test -p slicer-runtime --test integration -- perimeter_parity --ignored --nocapture` or the documented `record_*` invocation pattern.)
  - "Run `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1`; return FACT pass/fail" — purpose: validate AC-2 (re-baselined fixtures green).
  - "Run `cargo xtask test --workspace --summary 2>&1`; return FACT pass/fail + the `PASS`/`FAIL` verdict + the per-binary `test result:` line count" — purpose: validate AC-N1 (closure ceremony). The full output is on disk at `target/test-output.log` for drill-down.
  - "Run `cargo xtask build-guests --check`; return FACT clean / STALE list" — purpose: guest WASM coherence (mandatory if E added WIT record fields; run unconditionally per the closure ceremony).
  - "Run `rg -q 'D-147-CHAIN-CLOSURE' docs/DEVIATION_LOG.md`; return FACT pass/fail" — purpose: confirm deviation-log closure entry.
  - "Run `rg -q '0035-arachne-faithful-emission-and-transitions' docs/adr/0035-arachne-faithful-emission-and-transitions.md`; return FACT pass/fail" — purpose: confirm ADR 0035.
  - "Run `rg -q '### Rib edge\|### Junction fan\|### BeadingPropagation\|### Transition end\|### Local maximum' CONTEXT.md`; return FACT pass/fail" — purpose: confirm CONTEXT.md glossary.
  - "Update `docs/07_implementation_status.md` for the chain closure (M2 Real Arachne N1–N13 parity complete); return FACT pass/fail" — purpose: record the chain closure in the backlog (via worker dispatch, never by loading the full backlog).
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/arachne-parity-N1-N13-plan.md` — cross-packet policies (deviation-log supersession, ADR 0035, `cargo xtask test --workspace --summary` closure ceremony).
  - `docs/adr/0034-arachne-faithful-graph-construction.md` — ADR 0035 follows it.
- OrcaSlicer refs:
  - None (F owns no new refs).
- Verification:
  - `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 | tee target/test-output-f-step2-ac1.log` — FACT pass (AC-1).
  - `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | tee target/test-output-f-step2-ac2.log` — FACT pass (AC-2).
  - `cargo xtask test --workspace --summary 2>&1 | tee target/test-output-f-step2-neg1.log` — FACT PASS (AC-N1).
  - `rg -q 'D-147-CHAIN-CLOSURE' docs/DEVIATION_LOG.md` — FACT pass.
  - `rg -q '0035-arachne-faithful-emission-and-transitions' docs/adr/0035-arachne-faithful-emission-and-transitions.md` — FACT pass.
  - `cargo xtask build-guests --check` — FACT clean.
- Exit condition: AC-1, AC-2, AC-N1 pass; `D-147-CHAIN-CLOSURE` present; ADR 0035 present; `CONTEXT.md` glossary complete; `cargo xtask build-guests --check` clean; `docs/07_implementation_status.md` updated for the chain closure.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (confirm A1–E + diagnose e2e residual) | M | Heaviest dispatch: the e2e gate test + the 6 A1–E status checks + (conditional) the `--instrument-stderr` diagnosis. |
| Step 2 (cross-crate fixtures + deviation-log + ADR + closure ceremony) | M | Heaviest dispatch: `cargo xtask test --workspace --summary` (~11 minutes; `--summary` keeps the digest compact). |

Aggregate: M + M = M (Step 2 shares Step 1's chain context). If the sum exceeds M aggregate in practice, hand off after Step 1.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1, AC-2, AC-N1 dispatched and returned PASS).
- ALL 7 N1–N4 red tests green (the chain's acceptance oracles).
- A1–E are ALL `status: implemented`.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask build-guests --check` returns clean.
- `D-147-CHAIN-CLOSURE` present in `docs/DEVIATION_LOG.md` with addenda on `D-141` through `D-146`.
- ADR `0035-arachne-faithful-emission-and-transitions.md` present in `docs/adr/`.
- `CONTEXT.md` glossary complete (any A1–E gaps F closed).
- Cross-crate `perimeter_parity` fixtures re-baselined via `record_*` (never read directly).
- `docs/07_implementation_status.md` updated for the chain closure (M2 Real Arachne N1–N13 parity complete) via worker dispatch.
- `packet.spec.md` ready to move to `status: implemented`.
- The Arachne parity N1–N13 chain is COMPLETE.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1, AC-2, AC-N1).
- Confirm packet-level verification commands are green.
- Confirm ALL 7 N1–N4 red tests are green (the chain's acceptance oracles).
- Confirm A1–E are ALL `status: implemented`.
- Run `cargo xtask test --workspace --summary` as the closure ceremony (AC-N1); record the `PASS`/`FAIL` verdict + per-binary `test result:` line count. The full output is on disk at `target/test-output.log` for drill-down (never re-run).
- Record the e2e closure gate's `failures.len()/total_checked` summary explicitly (should be `0/N` — all sub-loops close).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.
- The Arachne parity N1–N13 chain is COMPLETE; record this in `docs/07_implementation_status.md`.