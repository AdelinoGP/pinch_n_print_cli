# Struct-Literal Churn Gate — Batch Plan

Status: approved (user, 2026-08-07, grilling session)
Generator: spec-packet-generator Batch Protocol
Commit rule: this plan file and the `docs/spec_packets/` packet directories it queues
must be committed together.

## Problem (measured)

Adding one field to a widely-constructed struct forces a workspace-wide sweep of
exhaustive struct literals. Three commits demonstrate the churn:

- `a579fc18` (packet 193): 165 files — ~90% one-line `overhang_distance_mm: None`
  filler in test files after `Point3WithWidth` gained a field.
- `383b633b` (packet 189): 26-site `LayerCollectionIR` sweep across 19 test files.
- `defb4b19` (ADR-0050): `SliceRunOptions` gained `profile`/`profile_verbose`;
  every test constructing it exhaustively was edited.

The prior fix (`docs/specs/_OLD/default-builder-migration.md`, TASK-200a–e) landed
completely — `Default` exists on every type above — but failed durably for two
reasons: its call-site sweep never covered per-test-file helper fns (measured
2026-08-06: 3 of 103 test files with `Point3WithWidth` literals use FRU; 72 files
construct `GlobalLayer`, 41 `LayerCollectionIR`), and it produced no ongoing rule
— nothing in CLAUDE.md, no lint, no gate — so later packets freshly wrote
exhaustive literals.

Counter-evidence that scopes the rule: in `a579fc18` the production sites
(`slicer-wasm-host/src/marshal/*`, `interpolate_point`, perimeter producers)
received *real logic* for the new field, not filler. Exhaustive literals there are
compiler-enforced propagation checkpoints; FRU there would have silently dropped
`overhang_distance_mm` at the WIT boundary. Production exhaustiveness is a
feature, not churn.

## Locked decisions (user-ruled; do not re-litigate)

1. **Rule.** In test code, a struct literal of a watched type must contain a `..`
   rest (any base: `Default::default()`, fixture call) OR an inline waiver
   comment with a mandatory reason. Production `src/` literals stay exhaustive on
   purpose.
2. **Enforcement.** New syn-based `cargo xtask check-literals` (exit 1 on
   violation; also a report mode listing violations and a path filter so sweep
   packets can verify per-area). Watchlist auto-derived at run time: every pub
   struct with ≥5 fields defined in `crates/*/src` (regardless of `Default`; no
   manual ledger). Enforced scope: `crates/*/tests/**`,
   `modules/core-modules/*/tests/**`, `#[cfg(test)]` mods in src, `benches/`.
   Exempt: production src, `crates/slicer-wasm-host/test-guests/*/src` (WIT
   adapter shims must break loudly). Must handle `Self { }` via impl-target
   tracking, literals inside macro token trees (`vec!`, `assert_eq!`),
   last-path-segment matching; enum struct-variants cannot fire (watchlist
   derives from struct definitions only). Site conversions omit default-equal
   fields rather than spell-all+FRU, which also sidesteps
   `clippy::needless_update`.
3. **Sweep policy.** (a) Safely-Default-able no-Default types (e.g.
   `SliceRunOptions`, `crates/slicer-runtime/src/run.rs`) gain `Default` impls
   per the old spec's Bucket A/B criteria. (b) Unsafe-default IR types
   (`PrintEntity`, `WallLoop` in `crates/slicer-ir/src/slice_ir.rs` — their
   `ExtrusionRole`/`LoopType` enums have no safe default variant) get shared
   fixture bases in `slicer_sdk::test_support`; user chose this over a new crate,
   accepting the guest-staleness rebuild tax; requires a short addendum to
   ADR-0054 (and the ADR-0004 "disjoint surfaces" wording) naming
   `sdk::test_support` the single IR-fixture home; host crates consuming it take
   a `slicer-sdk` dev-dep with `feature = "test"`. (c) Trait-object holders
   (`PipelineConfig`) get per-crate `tests/common` helper fns.
4. **Wiring (last, only after sweeps are green).** `cargo xtask check-literals`
   added to `cargo xtask test`'s preflight next to `build-guests --check`, AND to
   CLAUDE.md's required-before-commit commands.
5. **Docs.** New page `docs/21_data_defaults_and_fixtures.md` (number re-derived
   at authoring) covering: the rule, the production-exemption rationale,
   watchlist derivation, waiver format, fixture policy, `needless_update`
   guidance. CLAUDE.md gets a short MUST section pointing at it. `CONTEXT.md`
   "Carrier" term already added (2026-08-07).

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 194-check-literals-gate | Implement syn-based `cargo xtask check-literals` (report mode + path filter, exit 1 on violations), author `docs/21_data_defaults_and_fixtures.md`, add CLAUDE.md rule text marked gate-off. | TASK-316 | - | implemented | docs/spec_packets/194-check-literals-gate |
| 2 | 195-defaults-and-fixture-bases | Add safe `Default` impls to Default-able no-Default watched types; add `PrintEntity`/`WallLoop` fixture bases to `slicer_sdk::test_support` + ADR-0054/0004 addendum; per-crate helpers for trait-object holders; rebuild guests. | TASK-317 | #1 | implemented | docs/spec_packets/195-defaults-and-fixture-bases |
| 3 | 196-literal-sweep-core-ir-gcode | Convert exhaustive watched-type literals to FRU in slicer-ir, slicer-core, slicer-gcode test code until `check-literals` reports 0 violations for the area. | TASK-318 | #1, #2 | implemented | docs/spec_packets/196-literal-sweep-core-ir-gcode |
| 4 | 197-literal-sweep-host-runtime | Same sweep for slicer-runtime, slicer-scheduler, slicer-wasm-host, pnp-cli test code. | TASK-319 | #1, #2 | implemented | docs/spec_packets/197-literal-sweep-host-runtime |
| 5 | 198-literal-sweep-sdk-modules | Same sweep for slicer-sdk and modules/core-modules test code. | TASK-320 | #1, #2 | implemented | docs/spec_packets/198-literal-sweep-sdk-modules |
| 6 | 199-literal-gate-enforcement | Flip enforcement on: wire `check-literals` into `cargo xtask test` preflight, CI (`docs-guard` job), and CLAUDE.md required-before-commit; sweep residue crates; repair CLAUDE.md stale facts; workspace-wide green. | TASK-321 | #1–#5 | implemented | docs/spec_packets/199-literal-gate-enforcement |

## Decisions added after the plan was approved

- **CI enforcement is in scope (user ruling, 2026-08-07).** Preflight review of packet
  199 found that `.github/workflows/ci.yml` exists (jobs `fmt`, `docs-guard`,
  `clippy`, `test`) and that its `test` job calls `cargo test -p …` directly,
  never `cargo xtask test` — so locked decision 4's preflight wiring alone would
  leave CI blind to violations. Packet 199 therefore also adds one gate step to
  the `docs-guard` job, which already runs an xtask guard
  (`check-deviations --check`). The `test` job is deliberately NOT rerouted
  through `cargo xtask test`.
- **CLAUDE.md stale-fact repair folded into packet 199.** The same review found
  CLAUDE.md's feature-gated-tests section claiming `slicer-gcode` depends on
  `slicer-core` with `host-algos` — false, and the direct cause of an authoring
  error in packet 196. Packet 199 owns the end-state correction plus a new
  `slicer-sdk --features test` hazard note (whose exact wording must be measured,
  not assumed).

Numbering note (ledger facts): packet numbers 194–199, TASK-316–321, and docs
page number 21 were derived 2026-08-07. Authoring agents MUST re-derive all
three at write time (highest existing `docs/spec_packets/` number in git history,
`rg -o 'TASK-[0-9]{3}' docs/07_implementation_status.md | sort -u | tail -1`,
highest `docs/NN_*.md`) and renumber on collision.
