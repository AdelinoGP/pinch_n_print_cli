# Implementation Plan: 242-support-family-orca-closure

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Every cargo invocation tees combined output to `target/test-output.log`; results are read from
  the file, never re-run for more output.
- Before attributing any guest/parity/module-dispatch failure:
  `cargo xtask build-guests --check` (exit 0 fresh / 1 stale → rebuild / 3 infra).

## Steps

### Step 1: Register TASK-429..440 and amend TASK-335 in docs/07

- Task IDs: `TASK-429`
- Objective: allocate this packet's twelve rows in `docs/07_implementation_status.md` and amend
  the TASK-335 row to record "closes at packet 242 (this packet); see
  docs/spec_packets/242-support-family-orca-closure/". TASK-335's final `[x]` flip happens in
  Step 8 only after the gate evidence exists.
- Precondition: re-derive the next free ID immediately before writing (`grep -oE "TASK-[0-9]{3}"
  docs/07_implementation_status.md | sort -u | tail -1` → must be ≤ TASK-428; if it exceeds
  TASK-428 because another packet registered first, stop and re-map IDs before proceeding).
- Postcondition: twelve open rows TASK-429..TASK-440 attributed to packet 242 exist; TASK-335 row
  carries the pending-closure pointer; no other row changed.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - tail range only (last ~80 lines of the task ledger),
    located by grep; never a full read.
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - every other doc, packet dir, and source file
- Blast-radius discipline: n/a (doc rows, no struct/schema change).
- Expected sub-agent dispatches:
  - Question: "Insert twelve open rows TASK-429..TASK-440 attributed to packet
    242-support-family-orca-closure and append the pending-closure sentence to the TASK-335
    row"; scope: `docs/07_implementation_status.md`; return: FACT (row IDs inserted + amended
    line excerpt).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §14 rule 2 (fresh ID allocation).
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'TASK-429' docs/07_implementation_status.md && rg -q 'TASK-440' docs/07_implementation_status.md && rg -q 'TASK-335' docs/07_implementation_status.md && echo STEP1_REGISTERED`
- Exit condition: FACT confirms all thirteen rows (12 new + 1 amended) present.

### Step 2: Cross-packet disposition pre-audit (read-only)

- Task IDs: `TASK-430`
- Objective: produce the disposition FACT table — for each G-row its routing destination and the
  owner packet's closure state; for DEV-141..146 their current DEVIATION_LOG state; for each of
  the eight divergence sections the consuming packet. This step decides nothing; it feeds
  Steps 3-5.
- Precondition: all seven dependency packets implemented (activation gate).
- Postcondition: one FACT table covering G-01..G-24 × {owner, candidate verdict}, six deviation
  states, eight divergence verdicts — recorded in the working notes, not yet in design.md.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/support-parity-gap-register.md` - full read (~70 lines)
  - `docs/spec_packets/237..241` packet dirs - delegated per-packet FACT surveys
- Files allowed to edit (at most 3): none (read-only step).
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` body beyond the DEV-141..146 rows (ranged grep reads only)
  - any production source
- Expected sub-agent dispatches:
  - Per-owner FACT surveys as listed in `design.md §Expected Sub-Agent Dispatches`.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §10 (supersession), §12 briefs of
    the seven packets (what each was supposed to close).
- OrcaSlicer refs: none.
- Verification:
  - `test "$(grep -cE '^\| G-[0-9]+ ' docs/specs/support-parity-gap-register.md)" -eq 24 && echo PREAUDIT_INPUT_COMPLETE`
- Exit condition: FACT table complete; every row has an owner + candidate verdict. Falsifying
  exit: any routed destination missing its implementation → `[BLOCK]`, route back, do not write
  a fake CLOSED.

### Step 3: Write the gap-register disposition ledger and mirror tokens

- Task IDs: `TASK-431`, `TASK-432`
- Objective: author `design.md ## Gap Register Disposition Ledger (242)` with 24 tokened rows,
  then mirror each token into the corresponding register evidence cell.
- Precondition: Step 2 table complete.
- Postcondition: AC-8 and AC-N3 commands pass; tokens follow the grammar
  `[CLOSED <packet> <date>]` / `[WAIVED <date>: <justification>]` / `[CARRIED -> <owner>:
  <reason>]`; G-14/G-15/G-20 carry explicit waiver/register-only tokens.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/support-parity-gap-register.md` - full read
- Files allowed to edit (at most 3):
  - `docs/spec_packets/242-support-family-orca-closure/design.md`
  - `docs/specs/support-parity-gap-register.md`
- Files explicitly out of bounds:
  - owner packets' files; `docs/DEVIATION_LOG.md`
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches: none required.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §13 T10 (noise/literal debt are
    waivers, not fixes), Ruling 2 (parity bar).
- OrcaSlicer refs: none.
- Verification:
  - AC-8 command verbatim (`packet.spec.md`) - FACT pass/fail with both counts printed.
- Exit condition: both counts equal 24 and equal each other. Falsifying exit: a row whose true
  state is open → route back / `[BLOCK]`; writing CLOSED without owner evidence is forbidden.

### Step 4: Deviation and divergence dispositions

- Task IDs: `TASK-433`, `TASK-434`
- Objective: author `design.md ## Deviation Dispositions` (six lines, verbs CLOSED/CARRIED set
  by auditing 238b/238c outcomes) and `## Divergence Dispositions` (eight DISPOSITIONED lines).
- Precondition: Step 2 audits; 238b/238c actually landed their DEV corrections.
- Postcondition: AC-9 and AC-10 commands pass; `cargo xtask check-deviations` still passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - DEV-141..DEV-146 rows only (grep-located ranged read)
  - `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` - section
    headers + delegated per-section summaries
- Files allowed to edit (at most 3):
  - `docs/spec_packets/242-support-family-orca-closure/design.md`
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` (premise corrections belong to owning packets; if an audit finds a
    still-wrong premise, that is a route-back/[BLOCK], not an edit here)
- Expected sub-agent dispatches:
  - Question: "SUMMARY each divergence section's consuming-packet outcome"; scope:
    `docs/spec_packets/23{7,8a,8b,8c}*/`; return: FACT per section.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §10 (DEV-145 corrected premise),
    T11 (never resurrect disproved premises).
- OrcaSlicer refs: none.
- Verification:
  - AC-9 command verbatim; AC-10 command verbatim - FACT pass/fail each.
- Exit condition: 6/6 deviation lines, 8/8 divergence lines, check-deviations green.

### Step 5: Absorbed-218 e2e support-marker test (red-first)

- Task IDs: `TASK-435`, `TASK-436`
- Objective: add `gcode_support_type_markers_render_alongside_layer_images` to
  `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`: inline G-code carrying
  `;TYPE:Support` and `;TYPE:Support interface` segments beside `;TYPE:Outer wall`, driven
  through the file's existing helper pattern (`write_gcode`/`gcode_request`/`manifest_at`),
  asserting support/interface-marked moves render as layer images coexisting with the other
  roles. Write the test first, watch it fail or pass for the right reason; only touch
  `crates/pnp-cli/src/visual_debug_gcode.rs` if the failure exposes a real parser/renderer gap.
- Precondition: Step 1 done; helpers understood from a bounded read.
- Postcondition: AC-7 command prints `test result: ok. 1 passed`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - helpers ~lines 42-100 plus one
    existing test body as template
  - `crates/slicer-gcode/src/emit.rs` - `orca_type_label` vicinity only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
  - `crates/pnp-cli/src/visual_debug_gcode.rs` (conditional, justified failure only)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (the mapping already exists; changing it is emitter work,
    not e2e evidence work)
- Blast-radius discipline: n/a (new test only; no struct/schema change). If the conditional
  renderer edit happens, run `cargo clippy --workspace --all-targets -- -D warnings` in-step.
- Expected sub-agent dispatches:
  - Only on red: the visual_debug_gcode FACT dispatch from `design.md`.
- Context cost: `S` (green path) / `M` (renderer fix needed)
- Authoritative docs:
  - `docs/19_visual_debug.md` - delegated summary if manifest fields are unclear.
- OrcaSlicer refs: none (markers verified against `emit.rs`, in-tree).
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_gcode_renderer_tdd -- gcode_support_type_markers_render_alongside_layer_images --exact 2>&1 | tee target/test-output.log | grep -E "^test result: ok\. 1 passed"` - FACT pass/fail
- Exit condition: 1 passed. Falsifying exit: renderer fix attempted and the test still red →
  `[BLOCK]` with the failure SNIPPETS (≤20 lines) recorded in design.md Open Questions.

### Step 6: Re-prove the inherited suite and write the two inspection records

- Task IDs: `TASK-437`, `TASK-438`
- Objective: render the four bundles (two PnP model-source requests + two standalone-Orca
  requests against FRESH references), run the eight-name suite with asserted count, then write
  `design.md ## Matched-Height Inspection Record (242)` and `## Differential Inspection Record
  (242)` plus the `## TASK-163b and TASK-335 Disposition` re-confirmation — per family × five
  axes, each verdict naming layer + tap (E2), including 239's branch outcome note ([FWD] in
  design.md).
- Precondition: fresh references under `tmp/` verified by direct listing (T1);
  `cargo xtask build-guests --check` exit 0.
- Postcondition: AC-1, AC-2, AC-3, AC-4, AC-6 commands pass; records name source, layer, tap,
  verdict per family.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` - full read
  - 224 design.md - §Orca reference profile + §Orca Inspection Checklist ranges (delegated SUMMARY)
- Files allowed to edit (at most 3):
  - `docs/spec_packets/242-support-family-orca-closure/design.md`
- Files explicitly out of bounds:
  - any test source (the suite is inherited, not edited); `tmp/*_Orca.gcode` may be rendered by
    tools but never read into context or asserted on by tests
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches:
  - The cargo runs themselves delegated per context-discipline; renders return FACT (bundle path
    + manifest entry count).
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E2/E3, §8, §9.
- OrcaSlicer refs: per `requirements.md` §OrcaSlicer Reference Obligations (only if a verdict
  must name a canonical function).
- Verification:
  - AC-1 command verbatim; AC-2 command verbatim; AC-3 command verbatim; AC-4 command verbatim;
    AC-6 command verbatim - FACT pass/fail each.
- Exit condition: five FACTs pass and both records exist with per-axis verdicts. Falsifying exit:
  any suite member red → diagnose against freshness (E4) first, then route back to the owning
  packet — the suite is the tripwire, not something to patch here.

### Step 7: Supersessions — records and the 224 flip

- Task IDs: `TASK-439`
- Objective: author `requirements.md ## Supersession Records (242)` (213/TASK-329 with the
  degenerate-disk exclusion; 215→240, 216→220/224+238c residue, 217→220/224, 218→242 absorption
  mapping; 224 itself) and flip
  `docs/spec_packets/224-support-family-orca-closure/packet.spec.md` YAML to
  `status: superseded` + `superseded_by: 242-support-family-orca-closure`.
- Precondition: AC-1..AC-4, AC-6..AC-10 green (the flip vouches for existing evidence — never
  earlier).
- Postcondition: AC-5 command passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/224-support-family-orca-closure/packet.spec.md` - YAML header + Notes
    ranges
- Files allowed to edit (at most 3):
  - `docs/spec_packets/242-support-family-orca-closure/requirements.md`
  - `docs/spec_packets/224-support-family-orca-closure/packet.spec.md`
- Files explicitly out of bounds:
  - every other predecessor packet directory (210a/210b/211/213/214 stay untouched superseded
    provenance)
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §10 absorption mapping (verbatim
    source of the mapping used here).
- OrcaSlicer refs: none.
- Verification:
  - AC-5 command verbatim - FACT pass/fail.
- Exit condition: records present; 224 flipped; exactly two lines changed in 224's packet.spec.md.

### Step 8: Closure ceremony — whole-suite green run and human-gate record

- Task IDs: `TASK-440`
- Objective: run `cargo xtask test --summary --workspace -- --no-fail-fast` (E5), confirm every
  binary green from `target/test-output.log`, re-dispatch every pipe-suffixed AC command once,
  then fill the Human Validation Gate checklist artifacts and present for sign-off. Flip
  TASK-335's docs/07 row to closed ONLY when the gate signs.
- Precondition: Steps 1-7 exits green; guest freshness confirmed within this session.
- Postcondition: whole-suite PASS digest; Human Validation Gate holds artifact paths + checklist
  status; sign-off pending until the human records date + verdict.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - grep-ranged reads only (`^test result`, FAILED patterns)
- Files allowed to edit (at most 3):
  - `docs/spec_packets/242-support-family-orca-closure/packet.spec.md` (Human Validation Gate
    section only)
  - `docs/07_implementation_status.md` (TASK-335 flip, via delegated dispatch, post-sign-off)
- Files explicitly out of bounds:
  - everything else; no golden regeneration; no tolerance edits (E3)
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches:
  - The workspace run itself dispatched with FACT return (digest + failing-binary count).
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E5, §8 gate contract, §9
    reference freshness precondition.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask test --summary --workspace -- --no-fail-fast` - FACT PASS/FAIL from
    `target/test-output.log` (never re-run for more output; use
    `cargo xtask test --summary-from target/test-output.log` to re-digest).
- Exit condition: digest PASS + all prior AC FACTs still standing. Falsifying exit: any failing
  binary → attribute per E4/T4/T5 rules; a real regression routes back to its owning packet and
  this packet stays open. The gate does NOT close on a red suite under any waiver.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | delegated docs/07 registration |
| Step 2 | M | cross-packet FACT surveys |
| Step 3 | S | ledger + mirror tokens |
| Step 4 | S | deviation/divergence dispositions |
| Step 5 | S/M | red-first e2e test; M only if renderer fix needed |
| Step 6 | M | suite re-proof + written inspection records |
| Step 7 | S | supersession records + 224 flip |
| Step 8 | M | ceremony + gate record |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (224 → superseded; TASK-335 → closed at gate
  sign-off).
- `packet.spec.md` is ready for `status: implemented` only after the Human Validation Gate
  sign-off line is recorded.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
