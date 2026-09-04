---
status: draft
packet: 242-support-family-orca-closure
task_ids:
  - TASK-429
  - TASK-430
  - TASK-431
  - TASK-432
  - TASK-433
  - TASK-434
  - TASK-435
  - TASK-436
  - TASK-437
  - TASK-438
  - TASK-439
  - TASK-440
depends_on:
  - 237-support-analysis-parity
  - 238a-support-pattern-config-keys
  - 238b-tree-planner-canonical-fidelity
  - 238c-support-renderer-flow-interfaces
  - 239a-anchored-host-seams
  - 239b-anchored-wit-contract
  - 239c-support-layer-height-producer
  - 239d-support-coarse-floating-planes
  - 240a-support-raft-substrate
  - 240b-support-raft-module
  - 241-support-agg-rasterizer
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 242-support-family-orca-closure

This packet supersedes 224-support-family-orca-closure and closes the support-families sequence
(`docs/specs/support-families-anchored-entities-plan.md` §12 final brief). It absorbs the scope of
the deleted draft 218-support-gcode-e2e.

## Goal

Close the support-family Orca sequence: prove the inherited 224 invariant suite against the
post-237..241 tree, produce the absorbed 218 e2e `;TYPE:` evidence, dispose every gap-register
row, deviation DEV-141..146, and divergence entry in writing, record all supersessions, close
TASK-335 here and only here, and pass the final human differential gate.

## Scope Boundaries

This is closure, audit, and evidence work: running and extending existing suites, writing
grep-able disposition ledgers, and recording inspections. New support geometry, config keys,
planner algorithms, scheduler rules, and rasterizer behavior belong entirely to dependency
packets 237/238a/238b/238c/239a/239b/239c/239d/240a/240b/241 — discovering such a defect
here routes it back to its
owner (or `[BLOCK]`) rather than fixing it in passing. The only permitted production-code surface
is `crates/pnp-cli/src/visual_debug_gcode.rs` if the new e2e support-marker test fails for a real
parser/renderer reason.

## Prerequisites and Blockers

- Depends on (all eleven frontmatter entries; FORWARD-DEP: this terminal packet consumes the
  finished state of every other packet in the queue and cannot activate until each reaches
  `implemented`): 237-support-analysis-parity, 238a-support-pattern-config-keys,
  238b-tree-planner-canonical-fidelity, 238c-support-renderer-flow-interfaces,
  239a-anchored-host-seams, 239b-anchored-wit-contract,
  239c-support-layer-height-producer, 239d-support-coarse-floating-planes,
  240a-support-raft-substrate, 240b-support-raft-module, 241-support-agg-rasterizer.
  The former `239-support-independent-layer-z` is superseded; its behaviour is owned by
  239a/239b (anchored host seams + WIT transport) and 239c/239d (independent support layer
  height + off-grid support planes). The former `240` was split into 240a + 240b.
  Re-derive every dependency's live status at activation with
  `grep '^status:' docs/spec_packets/<dep>/packet.spec.md` — never quote a status from prose.
- Unblocks: merge of `parity/support-planners-clean` to master after the human gate signs
  (plan §14 rule 8); nothing else — this is the terminal packet.
- Activation blockers: any dependency packet not yet `implemented`; fresh Orca references
  (plan §9) absent under `tmp/` at gate time blocks the gate, not generation.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the post-dependency tree with `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`
  and `tests/fixtures/support-family/orca-matched-config.json` tracked, **when** each of the eight
  registered bare-wrapper closure tests runs under its own single-name `--exact` command (chained
  with `&&`; invariant 16: every command asserts exactly `1 passed`, so zero-match or partial-match
  runs fail), **then** all eight pass: fixture_invariants, family_reaches_region_routing,
  invalid_geometry_fails, matched_height_evidence, differential_evidence, final_gcode_roles,
  supersedes_packet_213_and_task_329, task_163b_disposition. |
  `(cargo test -p slicer-runtime --test integration -- fixture_invariants --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- family_reaches_region_routing --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- invalid_geometry_fails --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- matched_height_evidence --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- differential_evidence --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- final_gcode_roles --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- supersedes_packet_213_and_task_329 --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- task_163b_disposition --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && echo P242_INVARIANT_SUITE_8_OF_8`
- **AC-2. Given** fresh regenerated Orca references for both families under `tmp/` (plan §9;
  239c/239d and 240a/240b gate outputs confirmed current), **when** the matched-height artefact-presence
  precondition passes and the dual-family visual-debug bundles are rendered, **then** the
  inspection itself is satisfied ONLY by a written record naming source, layer, tap, and verdict
  per family and per axis (E2) in `design.md §Matched-Height Inspection Record (242)` — the test
  proves artefacts exist and are indexed, never judgement (T6). | `cargo run -q --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-tree.json --output target/vd-p242-support-family-tree --overwrite && cargo run -q --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-normal.json --output target/vd-p242-support-family-normal --overwrite && cargo test -p slicer-runtime --test integration -- matched_height_evidence --exact && rg -q '^## Matched-Height Inspection Record \(242\)' docs/spec_packets/242-support-family-orca-closure/design.md && echo P242_MATCHED_HEIGHT_EVIDENCE_PRESENT`
- **AC-3. Given** PnP and standalone-Orca renders at matched physical heights for both families,
  **when** differential review runs, **then** `differential_evidence` asserts the PnP-side
  structural invariants (per-entry attribution, decline reasons on unmet demands, role presence
  for both families) and the differential verdicts are recorded by inspection in
  `design.md §Differential Inspection Record (242)` with source, layer, tap, and disposition per
  family; parity claims stay limited to termination, coverage, collision freedom, interfaces,
  independent heights; exact path identity is never claimed. | `cargo test -p slicer-runtime --test integration -- differential_evidence --exact && rg -q '^## Differential Inspection Record \(242\)' docs/spec_packets/242-support-family-orca-closure/design.md && echo P242_DIFFERENTIAL_RECORD_PRESENT`
- **AC-4. Given** final PNP G-code for both family selections, **when** role inspection runs,
  **then** support and interface output contains the exact markers `;TYPE:Support` and
  `;TYPE:Support interface` and family attribution remains present in the closure manifest.
  **Marker-resolution limit:** `orca_type_label` (`crates/slicer-gcode/src/emit.rs`) maps BOTH
  `ExtrusionRole::SupportInterface` AND `ExtrusionRole::SupportBaseInterface` to the same literal
  `;TYPE:Support interface`; only `ExtrusionRole::SupportMaterial → ";TYPE:Support"` is distinct.
  This AC therefore claims only that *an* interface role reached the G-code — base-interface vs
  top-interface CANNOT be discriminated from `;TYPE:` markers alone. Any claim of base-interface
  role retention must assert on the IR role (`ExtrusionRole::SupportBaseInterface`) or on a
  distinct marker, never on the `;TYPE:` label. | `cargo test -p slicer-runtime --test integration -- final_gcode_roles --exact`
- **AC-5. Given** the closure ledger, **when** supersession records are reviewed, **then**
  `requirements.md §Supersession Records (242)` names 213/TASK-329 (superseded 2026-08-12;
  degenerate-disk result is not closure evidence), deleted drafts 215/216/217/218 with their
  absorption mapping (215→240a/240b, 216→220/224+238c, 217→220/224, 218→this packet), and 224 itself
  (amended ACs inherited here), AND `docs/spec_packets/224-support-family-orca-closure/packet.spec.md`
  carries `status: superseded` with `superseded_by: 242-support-family-orca-closure`. | `rg -q '^## Supersession Records \(242\)' docs/spec_packets/242-support-family-orca-closure/requirements.md && rg -q 'TASK-329' docs/spec_packets/242-support-family-orca-closure/requirements.md && rg -q '218-support-gcode-e2e' docs/spec_packets/242-support-family-orca-closure/requirements.md && rg -q '^status: superseded' docs/spec_packets/224-support-family-orca-closure/packet.spec.md && rg -q '^superseded_by: 242-support-family-orca-closure' docs/spec_packets/224-support-family-orca-closure/packet.spec.md && echo P242_SUPERSESSIONS_RECORDED`
- **AC-6. Given** `TASK-163b-orca-ref` (closed 2026-08-20 by 224) and the regenerated
  references, **when** the disposition is reviewed, **then** `task_163b_disposition` asserts its
  PnP-side invariants (fixture resolves via `support_test_path`; no Orca-derived constant and no
  Orca-G-code read in any test) and the written half-b disposition is re-confirmed against the
  fresh references in `design.md §TASK-163b and TASK-335 Disposition`; exact path parity is
  never claimed. | `cargo test -p slicer-runtime --test integration -- task_163b_disposition --exact && rg -q '^## TASK-163b and TASK-335 Disposition' docs/spec_packets/242-support-family-orca-closure/design.md && echo P242_TASK163B_RECONFIRMED`
- **AC-7. Given** the absorbed 218-support-gcode-e2e scope, **when** the G-code-mode
  visual-debug renderer is driven over an inline G-code containing `;TYPE:Support` and
  `;TYPE:Support interface` extrusion-role markers, **then** the new test in
  `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` (which today uses only inline
  `;TYPE:Outer wall` / `;TYPE:Solid infill` fixtures) proves support- and interface-marked moves
  render as layer images coexisting with the other roles in the produced bundle — e2e evidence
  that the emitter mapping `ExtrusionRole::SupportMaterial → ";TYPE:Support"` /
  `ExtrusionRole::SupportInterface → ";TYPE:Support interface"` (`orca_type_label`,
  `crates/slicer-gcode/src/emit.rs`) survives the standalone G-code parse-render round-trip.
  Same marker-resolution limit as AC-4: `ExtrusionRole::SupportBaseInterface` shares the
  `;TYPE:Support interface` literal, so this AC proves marker survival for the interface family as
  a whole and never which interface role produced it. | `cargo test -p pnp-cli --test visual_debug_gcode_renderer_tdd -- gcode_support_type_markers_render_alongside_layer_images --exact 2>&1 | tee target/test-output.log | grep -E "^test result: ok\. 1 passed" && echo P242_E2E_TYPE_MARKERS_PROVEN`
- **AC-8. Given** `docs/specs/support-parity-gap-register.md` (its `| G-NN |` row inventory
  re-derived at audit time — the total is a ledger fact and is never frozen into this AC),
  **when** the closure audit runs, **then** this packet has added a fifth `Disposition` column to
  the register table — header `| # | Gap | Evidence | Destination | Disposition |` with the
  matching separator row — and every `| G-NN |` row carries exactly one grep-able token as its
  final cell: `[CLOSED <packet> <date>]`, `[WAIVED <date>: <justification>]`, or
  `[CARRIED -> <owner>: <reason>]`, so the count of token-bearing rows IN THE REGISTER equals the
  live count of all register rows, both computed in the same command (zero un-dispositioned rows;
  G-14/G-15/G-20 get explicit register-only/waiver tokens, never silence; `design.md`'s mirror
  ledger is documentation, not the asserted artifact). | `test "$(grep -cE '^\| G-[0-9]+ ' docs/specs/support-parity-gap-register.md)" -eq "$(grep -cE '^\| G-[0-9]+ .*\| \[(CLOSED|WAIVED|CARRIED)[^]]*\] \|$' docs/specs/support-parity-gap-register.md)" && test "$(grep -cE '^\| G-[0-9]+ ' docs/specs/support-parity-gap-register.md)" -gt 0 && echo P242_REGISTER_CLOSURE_AUDIT_PASS`
- **AC-9. Given** deviations DEV-141..DEV-146 (owned by 238b/238c), **when** the deviation
  ledger is audited, **then** each of the six has an explicit `DEV-NNN: CLOSED — …` or
  `DEV-NNN: CARRIED — <corrected premise>` line in `design.md §Deviation Dispositions`, and
  `cargo xtask check-deviations` still passes (any premise correction lands in
  `docs/DEVIATION_LOG.md` through the owning packet's format rules, not silently here). | `for d in 141 142 143 144 145 146; do rg -q "DEV-$d: (CLOSED|CARRIED)" docs/spec_packets/242-support-family-orca-closure/design.md || exit 1; done && cargo xtask check-deviations >/dev/null && echo P242_DEV_DISPOSITIONS_PASS`
- **AC-10. Given** `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md`
  with its eight squashed-commit sections, **when** the divergence ledger is audited, **then**
  `design.md §Divergence Dispositions` carries one `- Squashed commit N of 8: DISPOSITIONED —
  <verdict>` line per section (count equality; verdicts cite the consuming packet or the void
  premise). | `test "$(grep -c '^## Squashed commit' docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md)" -eq "$(grep -cE '^- Squashed commit [0-9]+ of 8: DISPOSITIONED' docs/spec_packets/242-support-family-orca-closure/design.md)" && echo P242_DIVERGENCES_DISPOSITIONED`

Every AC names exact paths, counts, tokens, or output fragments and ends with its own runnable
command. Commands that dump more than 200 successful lines are filtered through
`target/test-output.log` per repo discipline; read results from the file, never re-run.

AC verification command rule: AC-1..AC-4, AC-6 drive real pipeline/module-dispatch behavior and
use the binaries that own that setup today (`slicer-runtime` integration harness with its
`support_family_closure` module and bare wrappers in `crates/slicer-runtime/tests/integration/main.rs`;
measured live 2026-08-23). AC-7 uses the pnp-cli G-code-mode renderer test binary that owns
`parse_gcode`/`render_gcode_visual_debug` today.

## Negative Test Cases

- **AC-N1. Given** a fixture body entering exact-Z model occupancy, lacking valid termination, or
  cross-family overlap, **when** closure validation runs, **then** the body is dropped, its
  demand is marked unmet with a structured diagnostic, and the test fails rather than accepting a
  golden or fallback path. | `cargo test -p slicer-runtime --test integration -- invalid_geometry_fails --exact`
- **AC-N2. Given** the decisive fixture absent from its tracked path, **when** any closure test
  runs, **then** the `support_test_path` resolver panics naming the exact tracked path
  (`crates/slicer-runtime/tests/integration/support_family_closure.rs` — the panic contract IS the
  fixture-absence gate). The dedicated `missing_fixture_is_blocking` test stays DELETED (it
  asserted `std::fs::read` NotFound behavior and tested nothing about this closure): this packet
  forbids recreating it or any dedicated missing-fixture test. | `rg -q 'fn support_test_path' crates/slicer-runtime/tests/integration/support_family_closure.rs && rg -q 'required support-family fixture is missing' crates/slicer-runtime/tests/integration/support_family_closure.rs && ! rg -q 'fn missing_fixture_is_blocking' crates/slicer-runtime/tests/integration/support_family_closure.rs && echo P242_RESOLVER_CONTRACT_INTACT_NO_RECREATED_TEST`
- **AC-N3. Given** any closure claim (register row, deviation, divergence, superseded packet),
  **when** it lacks a written disposition or waiver token, **then** the corresponding audit in
  AC-8/AC-9/AC-10 returns non-zero and the packet may not close — an unwritten waiver is no
  waiver (asserted against the same `Disposition`-column format AC-8 specifies; the total is
  re-derived, never a literal). | `test "$(grep -cE '^\| G-[0-9]+ ' docs/specs/support-parity-gap-register.md)" -eq "$(grep -cE '^\| G-[0-9]+ .*\| \[(CLOSED|WAIVED|CARRIED)[^]]*\] \|$' docs/specs/support-parity-gap-register.md)" && test "$(grep -cE '^\| G-[0-9]+ ' docs/specs/support-parity-gap-register.md)" -gt 0 && echo P242_NO_UNWRITTEN_WAIVERS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Inherited closure suite (AC-1 form, eight chained single-name guarded commands):
  `(cargo test -p slicer-runtime --test integration -- fixture_invariants --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- family_reaches_region_routing --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- invalid_geometry_fails --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- matched_height_evidence --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- differential_evidence --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- final_gcode_roles --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- supersedes_packet_213_and_task_329 --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && (cargo test -p slicer-runtime --test integration -- task_163b_disposition --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed") && echo P242_INVARIANT_SUITE_8_OF_8`

**Invariant-16 note (the 224 lesson).** The eight names are all bare wrapper registrations in
`crates/slicer-runtime/tests/integration/main.rs` delegating to `support_family_closure::*`; no
test name carries the module prefix. Each command asserts exactly `1 passed`, so a future wrapper
rename turns that command red instead of silently filtering to fewer tests.

**Packet-level completion gate (NOT an AC pipe):** the whole-suite green run per E5/invariant-16
is a closure ceremony step — `cargo xtask test --summary --workspace -- --no-fail-fast`, results
read from `target/test-output.log`, every binary green. It is executed once in the final step
(`implementation-plan.md` Step 8) and recorded in the Human Validation Gate, never piped onto an
individual AC.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - direct range reads: §3 Ruling 2
  (parity bar), §7 E1-E9, §8 human gate, §9 reference regeneration, §10 supersession, §12 brief
  242, §13 traps T1-T11, §14 rules.
- `docs/specs/support-parity-gap-register.md` - direct read of the row inventory (short file;
  re-derive the `| G-NN |` row count at read time rather than quoting one) and reading rules; row
  bodies delegated when auditing owner packets.
- `docs/19_visual_debug.md` alongside `docs/17_agent_debugging.md` - delegated bounded summary
  for bundle/manifest contract questions only.

## Doc Impact Statement (Required)

- `docs/specs/support-parity-gap-register.md` gains a NEW fifth `Disposition` column added by
  this packet (header `| # | Gap | Evidence | Destination | Disposition |` plus the matching
  separator row), carrying exactly one `[CLOSED]` / `[WAIVED]` / `[CARRIED]` token per `| G-NN |`
  row; the register's prose framing (which still names packet **224** as the closing packet) is
  re-pointed at 242 in the same edit. Count equality per AC-8: token-bearing rows must equal the
  live total, both re-derived in the command, never against a frozen literal - `test "$(grep -cE '^\| G-[0-9]+ ' docs/specs/support-parity-gap-register.md)" -eq "$(grep -cE '^\| G-[0-9]+ .*\| \[(CLOSED|WAIVED|CARRIED)[^]]*\] \|$' docs/specs/support-parity-gap-register.md)" && test "$(grep -cE '^\| G-[0-9]+ ' docs/specs/support-parity-gap-register.md)" -gt 0 && echo P242_REGISTER_CLOSURE_AUDIT_PASS`
- `docs/spec_packets/242-support-family-orca-closure/design.md` closure ledgers (register
  mirror, deviations, divergences) - `rg -q '^## Deviation Dispositions' docs/spec_packets/242-support-family-orca-closure/design.md`
- `docs/spec_packets/224-support-family-orca-closure/packet.spec.md` superseded flip - `rg -q '^status: superseded' docs/spec_packets/224-support-family-orca-closure/packet.spec.md`
- `docs/07_implementation_status.md` TASK-335 closure + TASK-429..440 registration - `rg -q 'TASK-335' docs/07_implementation_status.md && rg -q 'TASK-440' docs/07_implementation_status.md`
- `docs/spec_packets/242-support-family-orca-closure/design.md` inspection records (E2) -
  `rg -q '^## Matched-Height Inspection Record \(242\)' docs/spec_packets/242-support-family-orca-closure/design.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — delegate only if a five-axis verdict needs the canonical function named (contact generation, collision/avoidance, interface generation); cite file + function, never line numbers.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — traditional-family axis definitions (contacts, propagation, roof/floor bands) for the differential record.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — shared interface-generation semantics referenced by interface verdicts.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

## Human Validation Gate

This section is THE final gate of the support-families sequence (plan §8 + §12 brief 242 +
§9). The packet may not flip to `status: implemented` without the sign-off line below.

Reference-freshness precondition (blocks the gate until met): freshly regenerated Orca
references for BOTH families exist under `tmp/` (plan §9: 242 re-confirms all references fresh,
including 239c/239d's enabled-feature and 240a/240b's raft-enabled sets where their axes are
inspected).
Verify by direct listing — `tmp/` is gitignored; globs lie (T1).

Artifact-producing commands (artifacts under `tmp/p242-*` / `target/vd-p242-*`):

- `cargo run -q --bin pnp_cli -- slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-tree-matched.json --output tmp/p242-tree.gcode --module-dir modules/core-modules`
- `cargo run -q --bin pnp_cli -- slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-normal-matched.json --output tmp/p242-normal.gcode --module-dir modules/core-modules`
- The four visual-debug renders of AC-2 (`target/vd-p242-support-family-{tree,normal}`) plus the standalone-Orca comparison bundles rendered from the fresh references.
- The whole-suite green run: `cargo xtask test --summary --workspace -- --no-fail-fast` (E5; results read from `target/test-output.log`).

Checklist (both families, each verdict naming layer + tap, recorded in
`design.md §Matched-Height Inspection Record (242)` and
`design.md §Differential Inspection Record (242)`):

- Termination: accepted demands reach plate or model; no floating islands.
- Coverage: support spans the overhang footprint seen at the matched reference height.
- Collision freedom: support footprint never enters model occupancy.
- Interfaces: placement topmost and carved out; block counts compared against the references.
- Independent heights: support Z schedule vs object Z (239c/239d outcome) against enabled-feature references.

Sign-off:

_Sign-off: pending (date + verdict required before `status: implemented`)._
