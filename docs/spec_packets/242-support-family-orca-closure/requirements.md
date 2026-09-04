# Requirements: 242-support-family-orca-closure

## Packet Metadata

- Grouped task IDs: `TASK-429`..`TASK-440` — a **backfill into an unused gap**, not an append at
  the tip of docs/07's ledger; registered into docs/07 by this packet's own closure step (Step 1),
  which must verify TASK-429..TASK-440 are *absent* before writing (see `implementation-plan.md`
  Step 1 for the re-derivation command and its expected non-tip result)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 224-support-family-orca-closure closed prematurely twice (2026-08-17 retracted; the
2026-08-20 close left a zero-match `--exact` filter, a deleted-but-recreated negative test, and
two vacuous evidence tests, all amended in-session). Its amended ACs, the gap register
(`docs/specs/support-parity-gap-register.md`), the parity audit, and
`docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` are inherited
by this packet, which supersedes 224 and closes the sequence for real: the eleven dependency
packets named in `packet.spec.md`'s frontmatter (237, 238a, 238b, 238c, 239a, 239b, 239c, 239d,
240a, 240b, 241 — the former 239 is superseded by 239a/239b/239c/239d, and the former 240 was
split into 240a + 240b) each change support behavior, so every inherited
closure claim must be re-proven against the post-dependency tree, every routed gap/deviation/
divergence must come back dispositioned in writing, and the absorbed 218 e2e `;TYPE:` evidence
must finally exist. TASK-335 closes here and only here. This is one coherent slice because
closure claims stand or fall together: an un-dispositioned register row invalidates the
differential inspection that the final gate signs.

## In Scope

- Re-run the inherited eight-name closure suite (bare wrappers in
  `crates/slicer-runtime/tests/integration/main.rs` delegating to `support_family_closure::*`)
  as eight chained single-name `--exact` commands, each asserting exactly
  `test result: ok. 1 passed` (invariant 16; per-name isolation, authoritative form in
  packet.spec.md AC-1).
- Matched-height artefact-presence precondition + dual-family visual-debug renders + the written
  E2 inspection record (AC-2) against fresh Orca references (plan §9).
- Differential evidence: `differential_evidence` PnP-side structural invariants + written
  per-family/per-axis inspection record (AC-3), five behavioral axes only.
- Final G-code role markers `;TYPE:Support` / `;TYPE:Support interface` (AC-4).
- Supersession records: 213/TASK-329, deleted drafts 215/216/217/218 with absorption mapping
  (215→240a/240b, 216→220/224+238c, 217→220/224, 218→242), and 224 itself; flip 224's packet.spec.md
  to `status: superseded` with `superseded_by:` (AC-5).
- TASK-163b-orca-ref disposition re-confirmation against the fresh references (AC-6).
- Absorbed 218 scope: a support-marked e2e case in
  `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` proving `;TYPE:Support` /
  `;TYPE:Support interface` markers coexist with layer images (AC-7).
- Register closure audit: this packet ADDS a fifth `Disposition` column to
  `docs/specs/support-parity-gap-register.md` — updating the table's header row to
  `| # | Gap | Evidence | Destination | Disposition |` and its separator row to match — and writes
  exactly one of `[CLOSED <packet> <date>]` / `[WAIVED <date>: <justification>]` /
  `[CARRIED -> <owner>: <reason>]` into that cell for every `| G-NN |` row, mirrored in
  `design.md §Gap Register Disposition Ledger` (AC-8, AC-N3). The register's prose framing still
  names packet **224** as the closing packet; re-point that framing text at 242 as part of the
  same edit. Row totals are re-derived at audit time, never quoted from this document.
- Deviation dispositions DEV-141..146 (AC-9); divergence dispositions for all eight
  squashed-commit sections (AC-10).
- Final human gate: full differential inspection both families vs fresh references + whole-suite
  green run (E5) recorded in `packet.spec.md §Human Validation Gate`.
- docs/07 registration of TASK-429..440 and the TASK-335 closure row (packet-owned closure work;
  the only docs/07 edits in the queue).

## Out of Scope

- Any new support geometry, planner algorithm, config key, scheduler rule, renderer semantic, or
  rasterizer behavior — that work belongs to
  237/238a/238b/238c/239a/239b/239c/239d/240a/240b/241. Discovering a
  defect in those areas routes it back to the owner (or `[BLOCK]`), never a fix in passing.
- Recreating `missing_fixture_is_blocking` or any dedicated missing-fixture test: the
  `support_test_path` resolver panic contract is the gate; the dedicated test was deleted
  (commit `4c67ccd9`) for asserting `std::fs::read` NotFound behavior (AC-N2).
- Re-opening disproved premises: the `c3c1ed5a` mesh-path-gate hypothesis, DEV-145's false
  "PnP-invented key" premise, the void "Orca 205 vs PnP 150 print-Z" figure, and the stale
  1.58x/1.75x pre-AC-1-fix deficit figures (T11) — never requoted.
- Exact Orca toolpath identity; parity claims stay on the five behavioral axes.
- G-14 (`ERR_MALFORMED_LAYER_MARKER` noise) and G-15 (61 inherited `check-literals` violations
  across 34 files) — pre-existing debt; dispositioned as waived/registered-only, never
  re-diagnosed as support defects or credited as fixes (T10).
- G-20 (`erSupportTransition`) — register-only per prior human decision; do not conflate with
  238c's `SupportPlanRole::BaseInterface` / `ExtrusionRole::SupportBaseInterface` (which maps to
  `;TYPE:Support interface` via the same emitter path).
- Re-running or re-executing packets 219–223; their suites remain the regression net.
- Editing `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md`, other packets' files (one
  exception: the 224 superseded flip, which packet-safety rules assign to the superseding
  packet), or the plan file.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` (very long) - direct range reads only:
  §3 Ruling 2, §7 E1-E9, §8 human gate, §9 references, §10 supersession, §12 brief 242, §13
  traps T1-T11, §14 authoring rules. Never full-file loads in implementation.
- `docs/specs/support-parity-gap-register.md` (short) - direct read; row bodies of owner
  packets delegated when auditing.
- `docs/spec_packets/224-support-family-orca-closure/packet.spec.md` + `design.md` - delegated
  SUMMARY of amended ACs, §Orca reference profile, and §Orca Inspection Checklist (both long;
  ranged reads only, never a full read).
- `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` - direct read
  of section headers; section bodies delegated.
- `docs/19_visual_debug.md` - delegated bounded summary for bundle/manifest contract questions.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — canonical tree-family axis definitions (contact generation, collision/avoidance, interface generation) consulted only when a differential verdict must name the canonical function; cite file + function, never line numbers.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — traditional-family orchestration, contacts, propagation, and roof/floor band semantics for interface-count verdicts.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — shared interface-generation semantics referenced by interface verdicts.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1`..`AC-10` (inherited-suite re-proof; matched-height artefact precondition +
  written inspection; differential invariants + written record; final G-code roles; supersession
  records + 224 flip; TASK-163b re-confirmation; absorbed-218 e2e `;TYPE:` case; register-closure
  audit; deviation dispositions; divergence dispositions).
- Negative: `AC-N1` (invalid geometry fails), `AC-N2` (resolver panic contract intact; no
  recreated missing-fixture test), `AC-N3` (no unwritten waiver).
- Cross-packet impact: consumes the finished state of
  237/238a/238b/238c/239a/239b/239c/239d/240a/240b/241; flips 224
  to `superseded`; closes TASK-335 in docs/07; registers TASK-429..440. The only cross-packet
  file edit is the 224 `packet.spec.md` status flip (packet-safety rule: the superseding packet
  owns it).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| AC-1 inherited suite: eight chained single-name guarded commands — `(cargo test -p slicer-runtime --test integration -- fixture_invariants --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed") && (… family_reaches_region_routing …) && (… invalid_geometry_fails …) && (… matched_height_evidence …) && (… differential_evidence …) && (… final_gcode_roles …) && (… supersedes_packet_213_and_task_329 …) && (… task_163b_disposition …) && echo P242_INVARIANT_SUITE_8_OF_8` — every `(…)` expands to exactly `cargo test -p slicer-runtime --test integration -- <that name> --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"`; the fully expanded form is authoritative in packet.spec.md AC-1 | AC-1: inherited suite, per-name asserted count | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p slicer-runtime --test integration -- fixture_invariants --exact` (and each of the other seven names singly, same shape) | Per-name isolation when AC-1's shared run is red | FACT pass/fail per name |
| `cargo test -p slicer-runtime --test integration -- invalid_geometry_fails --exact` | AC-N1 | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_gcode_renderer_tdd -- gcode_support_type_markers_render_alongside_layer_images --exact 2>&1 \| tee target/test-output.log \| grep -E "^test result: ok\. 1 passed"` | AC-7: absorbed-218 e2e `;TYPE:` evidence | FACT pass/fail; failure SNIPPETS ≤20 lines |
| `cargo run -q --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-tree.json --output target/vd-p242-support-family-tree --overwrite && cargo run -q --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-normal.json --output target/vd-p242-support-family-normal --overwrite && cargo test -p slicer-runtime --test integration -- matched_height_evidence --exact` | AC-2 precondition renders | FACT pass/fail |
| `rg -q '^## Matched-Height Inspection Record \(242\)' docs/spec_packets/242-support-family-orca-closure/design.md && rg -q '^## Differential Inspection Record \(242\)' docs/spec_packets/242-support-family-orca-closure/design.md && rg -q '^## TASK-163b and TASK-335 Disposition' docs/spec_packets/242-support-family-orca-closure/design.md` | AC-2/AC-3/AC-6 written halves (E2) | FACT present/absent |
| `test "$(grep -cE '^\| G-[0-9]+ ' docs/specs/support-parity-gap-register.md)" -eq "$(grep -cE '^\| G-[0-9]+ .*\| \[(CLOSED\|WAIVED\|CARRIED)[^]]*\] \|$' docs/specs/support-parity-gap-register.md)" && test "$(grep -cE '^\| G-[0-9]+ ' docs/specs/support-parity-gap-register.md)" -gt 0` | AC-8/AC-N3 register-closure audit (register-local counts, live total re-derived — no frozen literal; asserts the new `Disposition` column) | FACT pass/fail with both counts |
| `for d in 141 142 143 144 145 146; do rg -q "DEV-$d: (CLOSED\|CARRIED)" docs/spec_packets/242-support-family-orca-closure/design.md \|\| exit 1; done && cargo xtask check-deviations >/dev/null` | AC-9 deviation dispositions | FACT pass/fail |
| `test "$(grep -c '^## Squashed commit' docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md)" -eq "$(grep -cE '^- Squashed commit [0-9]+ of 8: DISPOSITIONED' docs/spec_packets/242-support-family-orca-closure/design.md)"` | AC-10 divergence dispositions | FACT pass/fail with both counts |
| `rg -q '^status: superseded' docs/spec_packets/224-support-family-orca-closure/packet.spec.md && rg -q '^superseded_by: 242-support-family-orca-closure' docs/spec_packets/224-support-family-orca-closure/packet.spec.md` | AC-5 224 flip | FACT pass/fail |
| `rg -q 'fn support_test_path' crates/slicer-runtime/tests/integration/support_family_closure.rs && ! rg -q 'fn missing_fixture_is_blocking' crates/slicer-runtime/tests/integration/support_family_closure.rs` | AC-N2 resolver contract intact, no recreated test | FACT pass/fail |
| `cargo check --workspace --all-targets` | Type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask test --summary --workspace -- --no-fail-fast` | Packet-level completion gate (E5), Step 8 ceremony only — never an AC pipe | FACT pass/fail from `target/test-output.log` |

## Step Completion Expectations

- Steps 2-7 (audit ledgers) must not edit production code; a failing audit is either a routing
  defect to `[BLOCK]`/route-back or a ledger-writing defect. Only Step 5 (AC-7) touches
  production code, and only `crates/pnp-cli/src/visual_debug_gcode.rs`, only if the new test
  fails for a real renderer reason.
- The 224 status flip (Step 7) happens only after AC-1..AC-4, AC-6..AC-10 are green — never
  before the evidence it vouches for exists.
- Every cargo invocation tees to `target/test-output.log`; results are read from the file.
- Guest-freshness (E4) precedes any attribution of a guest/parity failure:
  `cargo xtask build-guests --check`, exit codes 0/1/3.

## Context Discipline Notes

- `docs/07_implementation_status.md` is huge and append-only here: edit only via a delegated
  worker dispatch returning the inserted-row confirmation (Step 1), never a full read.
- `docs/spec_packets/224-support-family-orca-closure/design.md` is long; use ranged reads of
  §Orca reference profile and §Orca Inspection Checklist only, or delegate a SUMMARY.
- `tmp/` is gitignored — direct `ls tmp/` to verify reference freshness (T1); globs lie.
- Never re-run a cargo command to see more output; read `target/test-output.log`.
