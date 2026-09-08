# Design: 242-support-family-orca-closure

## Controlling Code Paths

- Primary code path: `crates/slicer-runtime/tests/integration/support_family_closure.rs` — the
  eight closure tests (`fixture_invariants`, `family_reaches_region_routing`,
  `invalid_geometry_fails`, `matched_height_evidence`, `differential_evidence`,
  `final_gcode_roles`, `supersedes_packet_213_and_task_329`, `task_163b_disposition`) plus the
  four invariant wrappers (`support_never_intersects_model_at_exact_z`,
  `accepted_demands_terminate_on_plate_or_model`, `interface_is_topmost_and_carved_out`,
  `no_overhang_mesh_produces_zero_support`). Registered bare in
  `crates/slicer-runtime/tests/integration/main.rs`.
- Neighboring tests/fixtures:
  `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` +
  `crates/slicer-runtime/tests/fixtures/support-family/orca-matched-config.json` (tracked authoritative fixtures,
  resolved by the panicking `support_test_path` / `matched_config_path` resolvers);
  `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` (G-code-mode renderer TDD, today
  only inline `;TYPE:Outer wall` / `;TYPE:Solid infill` fixtures);
  `orca_type_label` mapping in `crates/slicer-gcode/src/emit.rs`
  (`ExtrusionRole::SupportMaterial → ";TYPE:Support"`,
  `ExtrusionRole::SupportInterface → ";TYPE:Support interface"`). **Non-injective:**
  `ExtrusionRole::SupportBaseInterface` maps to the SAME `;TYPE:Support interface` literal, so
  `;TYPE:` markers cannot discriminate base-interface from top-interface. AC-4 and AC-7 are
  worded to claim only interface-family marker survival; any base-interface claim must assert on
  the IR role, not the label.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **Closure-only surface.** This packet audits, records, and re-proves. Production-code edits
  are forbidden except `crates/pnp-cli/src/visual_debug_gcode.rs` when AC-7's new test fails for
  a real parser/renderer reason; every other failure routes back to its owning packet (237..241)
  or becomes a `[BLOCK]`/written waiver — never an in-passing fix.
- **Invariant 16 is enforced by count, not by trust.** Each inherited wrapper runs in its own
  `--exact` invocation and asserts `1 passed`; AC-7 does the same for its standalone pnp-cli test
  binary. The eight wrappers and the `slicer-runtime --test integration` target were re-resolved
  on 2026-09-07. A future wrapper rename turns its command red instead of silently filtering to
  zero.
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
  (This packet's normal surface does not feed guest WASM; the check is required before
  attributing ANY guest/parity failure during the closure runs — E4/G-24 — and before the Step 8
  whole-suite ceremony.)
- **E1/E2/E3 discipline.** The two evidence tests stay invariant-only halves; judgement lives in
  the written records below. Golden reblessing is out of scope; if a dependency packet's change
  invalidates a golden, that packet owns it. No Orca-derived constant may be hardcoded into any
  test; no test may read `tmp/*_Orca.gcode` (locked 224 gate shape).
- **E5 totals discipline.** The whole-suite green claim comes only from
  `cargo xtask test --workspace --summary` output in
  `target/test-output.log`; fail-fast truncation has twice produced false greens.

## Code Change Surface

- Selected approach: audit-and-record closure. Six ledgers/records live in this packet's own
  documents (gap-register disposition ledger + mirror tokens, deviation dispositions,
  divergence dispositions, supersession records, matched-height inspection record, differential
  inspection record); one new test proves the absorbed-218 e2e evidence; one status flip marks
  224 superseded; docs/07 gets TASK-538..549 rows + the TASK-335 closure edit.
- Exact functions, traits, manifests, tests, and fixtures:
  - NEW test `gcode_support_type_markers_render_alongside_layer_images` in
    `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`: inline G-code with
    `;TYPE:Support` and `;TYPE:Support interface` marked extrusion segments alongside
    `;TYPE:Outer wall`, driven through `parse_gcode` → `render_gcode_visual_debug` (or the
    `_styled`/`from_path` variant the neighboring tests use), asserting support/interface moves
    appear as layer images coexisting with wall/infill roles in the manifest/PNG set.
    Red-first per repo discipline.
  - A NEW fifth `Disposition` column added to the register table in
    `docs/specs/support-parity-gap-register.md` (today a four-column
    `| # | Gap | Evidence | Destination |`): the header becomes
    `| # | Gap | Evidence | Destination | Disposition |`, the separator row gains a fifth cell,
    and every `| G-NN |` row carries exactly one token in that final cell, format
    `[CLOSED <packet> <date>]` / `[WAIVED <date>: <justification>]` /
    `[CARRIED -> <owner>: <reason>]`. The same edit re-points the register's prose framing, which
    still names packet 224 as the closing packet, at packet 242.
  - `docs/spec_packets/224-support-family-orca-closure/packet.spec.md` YAML flip to
    `status: superseded` + `superseded_by: 242-support-family-orca-closure` (no other line of
    that file changes).
  - `docs/07_implementation_status.md`: insert TASK-538..549 rows (via delegated dispatch) and
    close TASK-335 with a pointer to this packet.
- Rejected alternatives and reasons:
  - Re-running the full 224 implementation flow — rejected: the sequence moved; 237..241 changed
    the behavior under those tests; closure must re-prove against the current tree instead.
   - One multi-name `--exact` filter — rejected because Cargo/libtest name resolution can produce
     a zero-match false green and the current bucket layout registers bare wrappers. Eight
     single-name invocations, each asserting exactly one pass, are the locked command shape.
  - A dedicated missing-fixture regression test — rejected: deleted for asserting `std::fs`
    behavior; the resolver panic contract is the gate (AC-N2).

## Files in Scope (read + edit)

Target at most 3 primary files; justified extras below are ledgers owned by this packet.

- `docs/spec_packets/242-support-family-orca-closure/design.md` - role: closure ledger host
  (six sections written by Steps 2-7); expected change: append disposition ledgers + inspection
  records.
- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - role: absorbed-218 e2e evidence;
  expected change: add one support-marked test (Step 5).
- `docs/specs/support-parity-gap-register.md` - role: register closure; expected change: add the
  fifth `Disposition` column (header + separator + one token cell per `| G-NN |` row) and
  re-point the 224-framed prose at 242 (Step 3). Justified extra: it is the audited artifact
  itself.
- `docs/spec_packets/224-support-family-orca-closure/packet.spec.md` - role: superseded flip
  (Step 7); two YAML lines. Justified extra: assigned to the superseding packet by rule.
- `docs/07_implementation_status.md` - role: task registration + TASK-335 closure (Step 1);
  delegated worker dispatch only. Justified extra: registration is packet-owned closure work.
- `crates/pnp-cli/src/visual_debug_gcode.rs` - role: CONDITIONAL production fix (Step 5) if the
  new e2e test exposes a real renderer gap; otherwise untouched.

## Read-Only Context

Include ranges for files over 300 lines.

- `docs/spec_packets/224-support-family-orca-closure/design.md` - §Measured Baseline, §Orca
  reference profile, §Orca Inspection Checklist ranges only - purpose: inherited amended-AC
  semantics, reference profile settings, prior verdicts being re-inspected.
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` (**very long** — ranged or
  delegated reads only, never a full read) - purpose: know what each of the twelve closures
  asserts before re-running them. Declaration shape: the file contains exactly one `#[test]` fn
  (`tree_branch_a_merge_keeps_drawable_nodes_on_merge_layer`); every other closure case is a
  `pub fn` returning `Result`, wrapped by a bare `#[test]` shim in
  `crates/slicer-runtime/tests/integration/main.rs` — which is why the eight AC-1 names carry no
  module prefix. Locate each case by symbol name via grep, then read only its body.
- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - helper range (`write_gcode`,
  `gcode_request`, `manifest_at`, `png_dimensions`, ~lines 42-100) plus one existing test body -
  purpose: reuse the fixture/request pattern for the new test.
- `crates/slicer-gcode/src/emit.rs` - `orca_type_label` vicinity only - purpose: confirm the
  marker strings under test.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load (T1: verify existence by direct listing first).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- All owner-packet sources (`crates/slicer-core/src/algos/mesh_analysis.rs`,
  `overhang_annotation.rs`, `modules/core-modules/{tree-support-planner,traditional-support-planner,tree-support}/`,
  `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`, scheduler validation,
  marshal transports) - read-only diagnosis at most; fixes route back to their packets.
- `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md`, other spec-packet directories
  (except the sanctioned 224 flip), and the plan file - never edited by this packet.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: "Register TASK-538..TASK-549 as open rows attributed to packet 242 and amend the
  TASK-335 row to record its pending closure at 242"; scope: `docs/07_implementation_status.md`;
  return: FACT (inserted row IDs + amended row confirmation); purpose: Step 1.
- Question: "SUMMARY of 224 design.md §Orca reference profile + §Orca Inspection Checklist
  (settings, layer indices, prior verdicts)"; scope:
  `docs/spec_packets/224-support-family-orca-closure/design.md`; return: SUMMARY ≤200 words +
  ≤10 settings verbatim; purpose: Steps 4/6 inspection baselines.
- Question: "FACT per owner packet: current disposition state of G-rows routed to you";
  scope: `docs/spec_packets/23{6,7,8a,8b,8c,9a,9b,9c,9d}*`, `docs/spec_packets/24{0a,0b,1}*`
  packet dirs + gap register; return:
  FACT table (row → closed/open/waived candidate); purpose: Step 2 pre-audit.
- Question: "Does the standalone G-code-mode visual-debug path preserve `;TYPE:` role markers
  through parse and render?"; scope: `crates/pnp-cli/src/visual_debug_gcode.rs`; return: FACT;
  purpose: Step 5 red-test triage (only if the first run is red).

## Data and Contract Notes

- IR/manifest contracts: none added or changed. The packet consumes `SupportPlanIR` /
  `SupportIR` / `ExecutionPlan` shapes as they exist post-241; no schema bump.
- WIT boundary: untouched. If any closure run surfaces a WIT-linked failure, E4 freshness rules
  apply before attribution; a real mismatch routes to the owning packet.
- Determinism/scheduler constraints: `differential_evidence`'s structural invariants depend on
  deterministic family routing and serial/parallel determinism (plan invariants 12/13);
  re-running them after the 239a/239b anchored host-seam + WIT-transport enablement must not
  regress ordering guarantees —
  the suite itself is the tripwire.
- Human-owned reference inputs/artifacts: `tmp/p242-orca-tree.gcode`,
  `tmp/p242-orca-normal.gcode`, `tmp/p242-orca-tree-raft.gcode`,
  `tmp/p242-orca-normal-raft.gcode`, their matching patched `.3mf` and `.gcode.3mf` files, and
  `tmp/p242-vd-{pnp,orca}-{tree,normal}.json`. Render outputs are
  `target/vd-p242-{pnp,orca}-{tree,normal}`. These names are identical to packet.spec.md AC-2 and
  implementation-plan.md Step 6; packet 239/240 artifacts do not satisfy this fresh-set gate.

## Closure Ledger Contracts (authored by implementation)

These sections are created in `design.md` by the steps named; their exact anchors are verified
by ACs:

- `## Gap Register Disposition Ledger (242)` (Step 2): one row per live `| G-NN |` register row
  (count re-derived at audit time, never frozen here) `| G-NN | <token> | <one-line
  justification> |`. Token grammar: `[CLOSED <packet-slug> <YYYY-MM-DD>]`,
  `[WAIVED <YYYY-MM-DD>: <justification>]`, `[CARRIED -> <owner>: <reason>]`. Mirror the final
  token into the register's NEW fifth `Disposition` column (Step 3; this packet adds that column
  and its header/separator rows, and re-points the register's 224-framed prose at 242).
  Expected shape (pre-audit, not pre-decided):
  G-14 waived as pre-existing noise (T10), G-15 carried -> repo-wide literal debt, G-20 waived
  as register-only per human decision, G-19 closed-at-224 or explicitly re-triaged, everything
  else closed at its routing destination.
- `## 240b Upstream-Finding Dispositions` (Step 6): exactly two entries. The
  support-branch/raft-plane interleave on non-band routing is **gate-blocking** until the
  `DefaultLayerPlanner::run_layer_planning` support surface is fixed or a human records an
  explicit waiver; a bare `[CARRIED]` token cannot close TASK-335. Band-layer harvested regions
  carrying model-plane polygons remain uncertified DATA: no observed raft-band suppression bundle
  is available in this run. Step 3's register ledger must reference both dispositions rather than
  silently dropping the 240b findings.
- `## Deviation Dispositions` (Step 4): six lines of the form `DEV-NNN: CLOSED — …` or
  `DEV-NNN: CARRIED — …`, one each for DEV-141..DEV-146. **No verb is pre-written here**: the
  packet's own rule is that dispositions are established by the closure work (Step 2's audit of
  238b/238c outcomes against `docs/DEVIATION_LOG.md`), never asserted at authoring time.
  Re-derive each row's live state at Step 2 with
  `grep -n 'DEV-14[1-6]' docs/DEVIATION_LOG.md` rather than trusting any status quoted in a spec
  packet. Note as of authoring the log records DEV-141..DEV-144 as **Open** and DEV-145/DEV-146
  as already **Closed-implemented** — so for DEV-145/DEV-146 the disposition work is a
  re-verification of an already-closed row, not a closure, and writing `CARRIED` for either would
  contradict the log.
- `## Divergence Dispositions` (Step 4): one line per squashed-commit section:
  `- Squashed commit N of 8: DISPOSITIONED — <verdict citing consuming packet or void premise>`.
- `## Supersession Records (242)` lives in `requirements.md` (AC-5 anchor there).

## Locked Assumptions and Invariants

- The eight wrapper names and eight separate `1 passed` assertions are locked for this packet's
  lifetime; a rename upstream is a deliberate breakage of this AC and must update both sides
  consciously.
- No test reads `tmp/*_Orca.gcode`; no Orca-derived constant enters any test (224 locked gate
  shape stands unchanged).
- Parity claims remain limited to termination, coverage, collision freedom, interfaces,
  independent heights; exact path identity is never claimed.
- Rafts are a front-contiguous positive-index prefix (`GlobalLayer.is_raft`) below the shifted
  model Z stack; band output is raft-only, bottom-shell classification starts at the first
  non-raft layer, and raft paths use canonical `;TYPE:Support` labels. The independent-heights
  axis identifies raft bands by marker/Z membership, never by a custom label.

## Risks and Tradeoffs

- **E1/E2/E3 violations recurring** (the 224 failure modes): mitigated by the written-record
  anchors being grep-verified (AC-2/AC-3/AC-6) and by forbidding golden reblessing here.
- **Zero-match filters (T2):** mitigated by asserted counts everywhere; the measured baseline
  makes any drift visible as a red count, not a green nothing.
- **Fail-fast truncation (T3):** the whole-suite gate uses the mandated
  `cargo xtask test --workspace --summary` entry point and results are read from
  `target/test-output.log`.
- **Stale guests (T4):** freshness check precedes attribution and the Step 8 ceremony.
- **Feature-gated blindness (T5/E6):** the workspace run unifies `host-algos`; if a narrow
  slicer-core run is ever dispatched for diagnosis, it must carry `--features host-algos`.
- **Pre-existing noise misattribution (T10):** G-14/G-25 and G-15 are audited from their live
  premises; stale warning/literal counts are not frozen or credited as fixes.
- **Known whole-suite flake:** `instrument_stderr_is_superset_of_core` can fail when JSONL and
  progress output interleave at a newline. It is not waived: a red occurrence keeps the closure
  gate red and routes the atomic-newline-write repair to the emitter owner.
- **Disproved premises resurrected (T11):** the out-of-scope list names them explicitly.
- Tradeoff: asserting eight exact `1 passed` results couples the AC to the wrapper inventory; accepted
  because the opposite (unasserted filter) is precisely how 224's false green happened.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 8 ceremony: whole-suite summary + human-gate record)
- Highest-risk dispatch and required return format: the Step 2 cross-packet disposition survey —
  return FACT table, reject anything larger.

## Open Questions

None. All eleven direct dependencies and the 241b remediation are implemented; 239c's
measure-first `height_delta` verdict is CONSISTENT. Their statuses remain mutable ledger facts and
must be re-derived at activation and Step 2 rather than copied from this paragraph.

## Gap Register Disposition Ledger (242)

The 29 live register rows were audited on 2026-09-07. These dispositions are mirrored in the
register's fifth `Disposition` column. The 240b findings are not omitted: see
`## 240b Upstream-Finding Dispositions` for the blocking interleave and non-emitting harvested-data
records.

| Gap | Disposition | Justification |
| --- | --- | --- |
| G-01 | [CLOSED 224-support-family-orca-closure 2026-09-07] | Tree contact derivation was implemented by packet 224. |
| G-02 | [CLOSED 239c-support-layer-height-producer 2026-09-07] | The successor producer/height work owns the independent support-layer Z seam. |
| G-03 | [CLOSED 238a-support-pattern-config-keys 2026-09-07] | Base and interface pattern configuration was consumed by 238a. |
| G-04 | [CLOSED 238a-support-pattern-config-keys 2026-09-07] | Support expansion configuration was consumed by 238a. |
| G-05 | [CLOSED 238a-support-pattern-config-keys 2026-09-07] | Bottom-Z configuration was consumed by 238a. |
| G-06 | [CLOSED 240b-support-raft-module 2026-09-07] | Raft substrate and module routing were closed by 240a/240b. |
| G-07 | [CLOSED 241-support-agg-rasterizer 2026-09-07] | The AGG rasterizer gap was consumed by packet 241. |
| G-08 | [CLOSED 238a-support-pattern-config-keys 2026-09-07] | Support-specific line-width configuration was consumed by 238a. |
| G-09 | [CLOSED 238a-support-pattern-config-keys 2026-09-07] | Effective layer-height transport handling was consumed by 238a. |
| G-10 | [CLOSED 238c-support-renderer-flow-interfaces 2026-09-07] | Tree branch rendering and density scaling were consumed by 238c. |
| G-11 | [CLOSED 238c-support-renderer-flow-interfaces 2026-09-07] | Support flow accounting was consumed by 238c. |
| G-12 | [CLOSED 238c-support-renderer-flow-interfaces 2026-09-07] | Branch radius cap parity was consumed by 238c. |
| G-13 | [CLOSED 238c-support-renderer-flow-interfaces 2026-09-07] | Interface-driven branch-radius behavior was consumed by 238c. |
| G-14 | [WAIVED 2026-09-07: pre-existing machine-gcode-emit warning noise, unrelated to support] | The malformed-marker warning is pre-existing noise with support disabled. |
| G-15 | [WAIVED 2026-09-07: inherited repo-wide check-literals debt predates packet 242] | Literal violations are inherited repository debt, not support closure work. |
| G-16 | [CLOSED 238a-support-pattern-config-keys 2026-09-07] | Tree planner manifest key declarations were consumed by 238a. |
| G-17 | [CLOSED 237-support-analysis-parity 2026-09-07] | Support eligibility classification was consumed by 237. |
| G-18 | [CLOSED 238c-support-renderer-flow-interfaces 2026-09-07] | Roof/floor interface layer-count handling was consumed by 238c. |
| G-19 | [CLOSED 224-support-family-orca-closure 2026-09-07] | The pre-existing failure set was re-triaged and its support debt assigned at 224. |
| G-20 | [WAIVED 2026-09-07: register-only by human decision; revisit if a producer appears] | No in-tree producer requires a distinct transition role. |
| G-21 | [CARRIED -> 236-support-stabilization: owner state not verified in this audit] | Startup DAG advisory cleanup remains routed to 236 pending owner-state evidence. |
| G-22 | [CARRIED -> 236-support-stabilization: owner state not verified in this audit] | Support-angle bounds enforcement remains routed to 236 pending owner-state evidence. |
| G-23 | [CARRIED -> 236-support-stabilization: owner state not verified in this audit] | Tree tripwire strengthening remains routed to 236 pending owner-state evidence. |
| G-24 | [CARRIED -> 236-support-stabilization: owner state not verified in this audit] | Native/WASM harness staleness handling remains routed to 236 pending owner-state evidence. |
| G-25 | [CARRIED -> unassigned: warning root cause remains undiagnosed] | Repeated emitter warnings have no verified root-cause owner yet. |
| G-26 | [CARRIED -> TASK-441: residual organic tree geometry deltas remain] | Residual tip, collapse, and interface-placement deltas remain with the organic-engine work. |
| G-27 | [CARRIED -> 239c-support-layer-height-producer: production anchored-entity producer remains absent] | The host seam is closed, but production still has no anchored-entity producer. |
| G-28 | [CLOSED 239b-anchored-wit-contract 2026-09-07] | Anchored-event WIT transport was wired by 239b. |
| G-29 | [CLOSED 239d-support-coarse-floating-planes 2026-09-07] | Coarse floating support planes were closed by 239d. |

## Deviation Dispositions

DEV-141: CARRIED — the intentional `smooth_outward` far-vertex correction remains a deliberate divergence from canonical; no 238b/238c change closes it.
DEV-142: CARRIED — unconditional role-region simplification remains a deliberate tree-renderer divergence; no 238b/238c change closes it.
DEV-143: CARRIED — f64 smoothing with one final rounding remains a deliberate arithmetic divergence; no 238b/238c change closes it.
DEV-144: CARRIED — the missing per-node extra-wall transport remains an IR/consumer gap; no 238b/238c change adds that channel.
DEV-145: CLOSED — packet 238c corrected both family defaults to canonical `0.5` and regenerated the config reference.
DEV-146: CLOSED — packet 238c verified the interface-flow-over-line-width pitch derivation in both support-family suites.

## Divergence Dispositions

- Squashed commit 1 of 8: DISPOSITIONED — retained as carried tree-planner parity debt; no 238b/238c correction closes the top-Z mechanism or density-model differences.
- Squashed commit 2 of 8: DISPOSITIONED — retained as carried tree-planner geometry parity debt; no 238b/238c correction restores smoothing, mixed body/interface emission, full contours, or per-circle collision clipping.
- Squashed commit 3 of 8: DISPOSITIONED — retained as carried contact-generation parity debt; no 238b/238c correction adds the undeclared override, host overhang source, or canonical miter limit.
- Squashed commit 4 of 8: DISPOSITIONED — retained as carried tree-volume parity debt; no 238b/238c correction changes the radius, collision, simplification, or to-buildplate mechanisms recorded here.
- Squashed commit 5 of 8: DISPOSITIONED — retained as carried support-analysis/tree-planner parity debt; no 238b/238c correction closes the move-out, enforcer, overhang-step, width, merge, or strong/hybrid-style differences.
- Squashed commit 6 of 8: DISPOSITIONED — DEV-145's premise was voided and its real default discrepancy was consumed by packet 238c.
- Squashed commit 7 of 8: DISPOSITIONED — retained as carried emit/move-pass parity debt; no 238b/238c correction closes largest-part retention or the retry dilation-argument difference.
- Squashed commit 8 of 8: DISPOSITIONED — DEV-142 remains carried as deliberate simplification divergence; packet 238c did not make canonical's square-support-only simplification model the general path.

## Matched-Height Inspection Record (242)

Inspection status: **RECORDED**. Four fresh G-code inputs and four visual-debug bundles were
present and re-rendered with `--overwrite` on 2026-09-07. Each bundle contains four
`final_gcode/filament_lines` images at layers 0..3 with zero manifest warnings. Tree PnP/Orca
layers are Z=0.2, 0.4, 0.6, 0.8; traditional PnP/Orca layers are Z=0.2, 0.4, 0.6, 0.647273.
The `matched_height_evidence` invariant passed with one exact test match; the height comparison is
CONSISTENT with 239c's baseline.

| Family | Source / routing | Physical layers, Z, tap | termination | coverage | collision freedom | interfaces | independent heights |
| --- | --- | --- | --- | --- | --- | --- | --- |
| tree | PnP bundle / plain | 0..3 / 0.2,0.4,0.6,0.8 / `final_gcode/filament_lines` | CONSISTENT | CONSISTENT | CONSISTENT | CONSISTENT | PASS |
| tree | fresh Orca ref / plain | 0..3 / 0.2,0.4,0.6,0.8 / `final_gcode/filament_lines` | CONSISTENT | CONSISTENT | CONSISTENT | CONSISTENT | PASS |
| tree | PnP bundle / raft | not observed (layer/Z/tap unavailable) | BLOCKED | BLOCKED | BLOCKED | BLOCKED | BLOCKED |
| tree | fresh Orca ref / raft | not observed (layer/Z/tap unavailable) | BLOCKED | BLOCKED | BLOCKED | BLOCKED | BLOCKED |
| normal | PnP bundle / plain | 0..3 / 0.2,0.4,0.6,0.647273 / `final_gcode/filament_lines` | CONSISTENT | CONSISTENT | CONSISTENT | CONSISTENT | PASS |
| normal | fresh Orca ref / plain | 0..3 / 0.2,0.4,0.6,0.647273 / `final_gcode/filament_lines` | CONSISTENT | CONSISTENT | CONSISTENT | CONSISTENT | PASS |
| normal | PnP bundle / raft | not observed (layer/Z/tap unavailable) | BLOCKED | BLOCKED | BLOCKED | BLOCKED | BLOCKED |
| normal | fresh Orca ref / raft | not observed (layer/Z/tap unavailable) | BLOCKED | BLOCKED | BLOCKED | BLOCKED | BLOCKED |

## Differential Inspection Record (242)

Inspection status: **RECORDED for plain sources with a gate-blocking finding; BLOCKED for raft
sources**. The four observed manifests provide `final_gcode/filament_lines` at the layer/Z tuples
listed above. The support-branch/raft-plane interleave remains **gate-blocking pending an owner fix
or explicit human waiver**. No raft-band suppression record was observed; harvested band DATA is
therefore uncertified and is not described as retained evidence.
Band-layer harvested model-plane regions are recorded as intentional non-emitting DATA used to seed the raft footprint; band-suppression proof is unavailable in the current bundles, so this item remains uncertified and non-closing.

| Family | Source / routing | Physical layers, Z, tap | termination | coverage | collision freedom | interfaces | independent heights |
| --- | --- | --- | --- | --- | --- | --- | --- |
| tree | PnP bundle vs fresh Orca ref / plain | 0..3 / 0.2,0.4,0.6,0.8 / `final_gcode/filament_lines` | CONSISTENT | CONSISTENT | CONSISTENT | BLOCKED — interleave | PASS |
| tree | fresh Orca ref vs PnP bundle / plain | 0..3 / 0.2,0.4,0.6,0.8 / `final_gcode/filament_lines` | CONSISTENT | CONSISTENT | CONSISTENT | BLOCKED — interleave | PASS |
| tree | PnP bundle / raft | not observed (layer/Z/tap unavailable) | BLOCKED | BLOCKED | BLOCKED | BLOCKED — interleave | BLOCKED |
| tree | fresh Orca ref / raft | not observed (layer/Z/tap unavailable) | BLOCKED | BLOCKED | BLOCKED | BLOCKED — interleave | BLOCKED |
| normal | PnP bundle vs fresh Orca ref / plain | 0..3 / 0.2,0.4,0.6,0.647273 / `final_gcode/filament_lines` | CONSISTENT | CONSISTENT | CONSISTENT | BLOCKED — interleave | PASS |
| normal | fresh Orca ref vs PnP bundle / plain | 0..3 / 0.2,0.4,0.6,0.647273 / `final_gcode/filament_lines` | CONSISTENT | CONSISTENT | CONSISTENT | BLOCKED — interleave | PASS |
| normal | PnP bundle / raft | not observed (layer/Z/tap unavailable) | BLOCKED | BLOCKED | BLOCKED | BLOCKED — interleave | BLOCKED |
| normal | fresh Orca ref / raft | not observed (layer/Z/tap unavailable) | BLOCKED | BLOCKED | BLOCKED | BLOCKED — interleave | BLOCKED |

## TASK-163b and TASK-335 Disposition

- **TASK-163b:** PnP disposition invariant re-confirmed: `task_163b_disposition` passed with one
  exact test match, including the static prohibition on tests reading Orca-derived G-code. Fresh
  reference manifests are recorded; exact path parity is not claimed.
- **TASK-335:** not closed by this run. The support-branch/raft-plane interleave remains
  gate-blocking until fixed in its owner path or explicitly waived by a human; a bare carried token
  is insufficient.

## 240b Upstream-Finding Dispositions

1. **support-branch/raft-plane interleave:** **GATE-BLOCKING** absent a production fix or explicit
   human waiver. No closure claim is made from the PnP invariant suite.
2. **harvested band DATA:** **UNCERTIFIED**. No observed raft-band bundle proves whether model-plane
   polygons emit on raft-band output layers; no non-emitting disposition is certified by this run.
