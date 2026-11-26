# Requirements: 298-organic-tree-support-keys

## Packet Metadata

- Grouped task IDs: `TASK-000` (queue packet, `task_ids: []`)
- Backlog source: `docs/specs/orca-feature-gap/issues/79-author-packet-p72-support-tree-supports-tree-support.md` (wayfinder map: Close the OrcaSlicer FFF feature gap)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P72 is the organic half of the tree-support parameter surface: eight keys that tune the organic tree engine in canonical, all zero-occurrence as behaviour in this tree. The port implements only the classic `TreeSupport.cpp` engine and substitutes its Strong style wherever canonical would run the organic `TreeSupport3D.cpp` engine (DEV-156, with a code-1005 Warn on explicit `organic`). The six organic branch parameters therefore have no reader, and the two classic brim parameters have a reader upstream (`TreeSupport::draw_circles`) whose local counterpart was never built — the renderer emits no brim loops at all. Users asking for organic trees get classic geometry driven by classic defaults, with no way to influence it through the canonical organic surface. This packet makes that surface live on the explicit-organic path — one coherent slice across the two tree modules — while the full engine port stays queued where it belongs (remediation-plan row 7) and classic styles stay byte-identical.

## In Scope

- Declare the six organic parameter keys on `tree-support-planner` at canonical defaults and types: `tree_support_angle_slow` (float, 25.0), `tree_support_branch_angle_organic` (float, 40.0), `tree_support_branch_diameter_organic` (float, 2.0), `tree_support_branch_distance_organic` (float, 1.0), `tree_support_tip_diameter` (float, 0.8), `tree_support_top_rate` (percent, `"30%"`).
- Declare the two brim keys on `tree-support` at canonical defaults and types — `tree_support_auto_brim` (bool, true), `tree_support_brim_width` (float, 3.0) — plus the `support_style` gate row the renderer needs to see the style (enum, default `"default"`, all 7 canonical values; supporting declaration, not a queue key).
- Style-gated parameter selection in `SupportPlanner::from_config`: explicit `support_style = organic` resolves branch angle/diameter/distance, tip floor, top-rate scaling, and the slow-angle cap from the organic set with canonical value semantics (degree→radian conversion, constructor clamps, tip-clamped-to-diameter); every other style keeps the classic set untouched.
- First-layer brim stage in the renderer on the same explicit-organic gate: auto mode derives the brim width from node radius (canonical `draw_circles` selection), fixed mode uses `tree_support_brim_width`; zero width emits no loops.
- No range rejection anywhere (canonical minima are GUI hints, ticket-113 rule); sub-canonical inputs saturate at structural floors. The `Print.cpp` diameter/tip cross-validations are named non-borrows riding ticket 124.
- `DEV-189` row recording the three divergences; `docs/15` regeneration surfacing the new keys.
- Disposition table below: 8 keys in, 0 shed, 0 returned, declaration-only keys: 0.

## Out of Scope

- The organic volumetric engine itself (`TreeSupport3D.cpp` growth, collision volumes, `generate_support_areas`): DEV-156 stands, the Strong substitution stays, the code-1005 Warn stays. Owned by remediation-plan row 7, not this packet.
- Widening the organic gate to `default`/`grid`/`snug`-on-tree (canonical would use organic params there too): deliberate scoping divergence, recorded in DEV-189; rides the engine-port packet so default tree output stays stable until the engine exists.
- Classic-tree brims: canonical draws them, but this port's renderer has no brim seam at all and switching on default brims for every classic tree print is adhesion-affecting default churn beyond this packet's charter; named remaining gap for the engine-port packet.
- `Print.cpp` `PrintObject::validate` diameter/tip cross-checks (no validator seam exists; ticket 124 owns that stage).
- Per-tool vector models for any key (all eight are canonical scalars — scalar-global plus the existing per-object overlay is parity, 290/291 precedent; explicitly not ticket 125's axis).
- CONFIG_BLOCK padding twins (rule 2: padding is never a deliverable); explicit values ride the existing emit path as a side effect; the word-form bool spelling stays with ticket 132.
- No IR field, WIT change, schema-version bump, ADR, or new module/claim.

## Per-Key Canonical Grounding

All reads delegated at authoring (file + function, never line numbers); the implementer re-verifies defaults before mirroring.

| Key | Canonical type / default / bounds | Canonical consumer | Behaviour |
| --- | --- | --- | --- |
| `tree_support_angle_slow` | coFloat, 25, [10, 85] | `TreeSupportCommon.hpp` organic settings constructor (`support_tree_angle_slow`) | Slow-angle cap on the organic branch angle, clamped against the tree angle |
| `tree_support_auto_brim` | coBool, true | `TreeSupport.cpp` `TreeSupport::draw_circles` | When true, brim width derives from node radius + distance-to-top instead of the fixed width |
| `tree_support_branch_angle_organic` | coFloat, 40, [0, 60] | `TreeSupportCommon.hpp` organic settings constructor (`support_tree_angle`) | Max organic branch angle, degrees→radians, clamped |
| `tree_support_branch_diameter_organic` | coFloat, 2, [1, 10] | `TreeSupportCommon.hpp` organic settings constructor (`support_tree_branch_diameter`); `Print.cpp` `PrintObject::validate` | Organic branch diameter; validated against 2× extrusion width and tip diameter (validation rides ticket 124) |
| `tree_support_branch_distance_organic` | coFloat, 1, [1, 10] | `TreeSupportCommon.hpp` organic settings constructor (`support_tree_branch_distance`) | Organic branch spacing |
| `tree_support_brim_width` | coFloat, 3, [0, no max] | `TreeSupport.cpp` `TreeSupport::draw_circles` | Fixed brim width used when auto-brim is off |
| `tree_support_tip_diameter` | coFloat, 0.8, [0.1, 100] | `TreeSupportCommon.hpp` organic settings constructor (`support_tree_tip_diameter`); `TreeSupport3D.cpp` `generate_support_areas` (enforcer offset, non-borrow); `Print.cpp` `PrintObject::validate` (non-borrow) | Organic tip diameter, clamped to branch diameter |
| `tree_support_top_rate` | coPercent, 30, [5, 35] | `TreeSupportCommon.hpp` organic settings constructor (`support_tree_top_rate`) | Percent scaling of the top-contact radius |

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (support-family claim seam; rule-4 trigger test).
- `docs/08_coordinate_system.md` - direct range read (mm↔unit helpers for brim/radius geometry).
- `docs/DEVIATION_LOG.md` - direct read (DEV-156 row; ID convention for DEV-189).
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest schema types incl. percent spelling; declared-view whitelist — the renderer must declare every key it reads, ticket-34 lesson).
- `docs/specs/support-generation-remediation-plan.md` - delegated SUMMARY (row 7 engine scope boundary).
- `docs/ORCASLICER_ATTRIBUTION.md` - direct short read (only if porting `tests/fff_print/` assertions; standard header required).

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-8`; no measurable refinements beyond their Given/When/Then text. `AC-2` is the gate invariant (organic keys inert off-path); `AC-3`–`AC-5` pin the planner wiring; `AC-6`–`AC-7` pin the renderer stage; `AC-1` + `AC-8` pin declarations, dispositions, and docs.
- Negative: `AC-N1` (saturate, never reject — ticket-113 rule), `AC-N2` (DEV-156 substitution + Warn condition survive).
- Cross-packet impact: none. 238b is `implemented` and owns none of these keys; no draft packet names any of the eight as behaviour; remediation row 7 keeps the engine.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p tree-support-planner --test organic_params_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Planner wiring (AC-2–AC-5, AC-N1–AC-N2) | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p tree-support --test tree_brim_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Renderer brim stage (AC-6–AC-7) | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p tree-support-planner --test tree_style_styles_tdd 2>&1 \| tee target/test-output.log \| tail -5` | No regression in existing style routing | FACT pass/fail |
| `cargo test -p tree-support --test tree_support_tdd 2>&1 \| tee target/test-output.log \| tail -5` | No regression in existing renderer behaviour | FACT pass/fail |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | Workspace type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 \| tee target/test-output.log \| tail -3` | Struct-literal churn gate (new struct fields) | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -3` | Generated doc-15 tables current (AC-8) | FACT pass/fail |
| `cargo xtask build-guests --check 2>&1 \| tee target/test-output.log \| tail -3` | Guest freshness after manifest edits (exit 0 = fresh) | FACT pass/fail; never `rg STALE:` |

## Step Completion Expectations

- Step 2 (planner) lands before Step 3 (renderer); both land before Step 4 (regen + gates). No shared scratch state between steps beyond the manifests from Step 1.
- The `DEV-189` log row lands in Step 2 with the planner behaviour it justifies; Step 4 only greps it.

## Context Discipline Notes

- Packet-specific hazards: `tree-support-planner/src/lib.rs` is ~7000 lines — never read whole; Step ranges cite exact windows. `OrcaSlicerDocumented/` reads are delegated, never direct. Planner/renderer `cargo test` binaries are per-file (`--test organic_params_tdd`, `--test tree_brim_tdd`); no aggregator registration exists or is needed.
