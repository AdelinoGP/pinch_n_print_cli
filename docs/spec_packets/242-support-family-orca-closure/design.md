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
  `tests/fixtures/support-family/orca-matched-config.json` (tracked authoritative fixtures,
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
- **Invariant 16 is enforced by count, not by trust.** Every suite command asserts its matched
  count (`8 passed`; `1 passed` for the new AC-7 test). Measured 2026-08-23 with `--list`: the
  inherited multi-name shared-filter `--exact` form matches exactly the eight registered bare
  wrappers and exits non-zero on any mismatch via the grep gate. A future wrapper rename turns
  the count red instead of silently filtering to zero.
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
  `cargo xtask test --summary --workspace -- --no-fail-fast` output in
  `target/test-output.log`; fail-fast truncation has twice produced false greens.

## Code Change Surface

- Selected approach: audit-and-record closure. Six ledgers/records live in this packet's own
  documents (gap-register disposition ledger + mirror tokens, deviation dispositions,
  divergence dispositions, supersession records, matched-height inspection record, differential
  inspection record); one new test proves the absorbed-218 e2e evidence; one status flip marks
  224 superseded; docs/07 gets TASK-429..440 rows + the TASK-335 closure edit.
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
  - `docs/07_implementation_status.md`: insert TASK-429..440 rows (via delegated dispatch) and
    close TASK-335 with a pointer to this packet.
- Rejected alternatives and reasons:
  - Re-running the full 224 implementation flow — rejected: the sequence moved; 237..241 changed
    the behavior under those tests; closure must re-prove against the current tree instead.
  - Splitting the eight-name shared filter into eight single-name commands — rejected by
    measurement: the shared-filter form matches all eight (invariant 16 satisfied with the
    asserted count); eight separate invocations lose the single-run count proof without adding
    coverage. Per-name isolation remains available as a debugging command.
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

- Question: "Register TASK-429..TASK-440 as open rows attributed to packet 242 and amend the
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

- The eight wrapper names and the `8 passed` count are locked for this packet's lifetime; a
  rename upstream is a deliberate breakage of this AC and must update both sides consciously.
- No test reads `tmp/*_Orca.gcode`; no Orca-derived constant enters any test (224 locked gate
  shape stands unchanged).
- Parity claims remain limited to termination, coverage, collision freedom, interfaces,
  independent heights; exact path identity is never claimed.
- Rafts stay signed negative global-layer prefix entries (ADR-0009 as amended by 240b); the
  independent-heights axis inspects them accordingly.

## Risks and Tradeoffs

- **E1/E2/E3 violations recurring** (the 224 failure modes): mitigated by the written-record
  anchors being grep-verified (AC-2/AC-3/AC-6) and by forbidding golden reblessing here.
- **Zero-match filters (T2):** mitigated by asserted counts everywhere; the measured baseline
  makes any drift visible as a red count, not a green nothing.
- **Fail-fast truncation (T3):** the whole-suite gate uses `--summary --workspace --
  --no-fail-fast` and results are read from `target/test-output.log`.
- **Stale guests (T4):** freshness check precedes attribution and the Step 8 ceremony.
- **Feature-gated blindness (T5/E6):** the workspace run unifies `host-algos`; if a narrow
  slicer-core run is ever dispatched for diagnosis, it must carry `--features host-algos`.
- **Pre-existing noise misattribution (T10):** G-14/G-15 dispositions are written waivers that
  forbid re-diagnosis; the clippy/literal gates stay at inherited counts.
- **Disproved premises resurrected (T11):** the out-of-scope list names them explicitly.
- Tradeoff: asserting the exact `8 passed` couples the AC to the wrapper inventory; accepted
  because the opposite (unasserted filter) is precisely how 224's false green happened.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 8 ceremony: whole-suite summary + human-gate record)
- Highest-risk dispatch and required return format: the Step 2 cross-packet disposition survey —
  return FACT table, reject anything larger.

## Open Questions

Tag implementer-resolvable questions `[FWD]`; tag activation blockers `[BLOCK]`. Scope/interface/verification questions keep the packet `draft`. Delegate answers requiring out-of-bounds reads. Write `None.` when absent.

- `[BLOCK]` Activation requires every dependency in `packet.spec.md`'s frontmatter to reach
  `implemented`. The named remaining blockers are **240a-support-raft-substrate**,
  **240b-support-raft-module**, and **241-support-agg-rasterizer**; 237, 238a, 238b, 238c and
  239a/239b/239c/239d had already reached `implemented` at authoring. Do NOT trust that split —
  statuses are ledger facts and rot. Re-derive every dependency's status at activation with
  `grep '^status:' docs/spec_packets/<dep>/packet.spec.md` and treat anything not `implemented`
  as a live blocker. This resolves as the queue executes; no authoring action.
- `[FWD]` If 239c's measure-first `height_delta` protocol lands CONSISTENT (emitter unchanged),
  the independent-heights inspection axis still applies (Z schedule from the layer executor);
  Step 6 records which branch actually landed before writing the verdict.
