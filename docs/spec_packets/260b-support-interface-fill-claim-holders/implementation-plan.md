# Implementation Plan: support-interface-fill-claim-holders

## Execution Rules

- **Do not start this packet while any `[BLOCK]` in `design.md` §Open Questions is open.** Step 0 is the gate.
- Steps run in order; each ends green on its own verification. A step edits only its listed files; `design.md` §Out-of-Bounds Files binds every step.
- Every cargo run is delegated with a `FACT pass/fail` return, tee'd to `target/test-output.log`; read the log rather than re-running.
- No step may declare `support_interface_pattern` as a config key anywhere (rule 4, holder-only), and no step may touch `ORCA_CONFIG_PADDING` or a CONFIG_BLOCK twin (rule 2).

## Steps

### Step 0: Resolve the four blockers (gate — no code)

- Objective: `[BLOCK-1]` (selector), `[BLOCK-2]` (two writers in one stage), `[BLOCK-3]` (angle metadata), and `[BLOCK-4]` (ADR-0059 conformance) each have a written ruling, and this plan is amended to match before any edit.
- Preconditions: `260a` merged.
- Postconditions: each `[BLOCK]` is replaced in `design.md` by the ruling and its consequences; if `[BLOCK-3]` rules against reaching the base angle, the packet shrinks to `concentric` and `grid` moves to §Returned to Queue in `requirements.md`.
- Files allowed to edit: `docs/spec_packets/260b-support-interface-fill-claim-holders/{design.md,requirements.md,implementation-plan.md}` only.
- Dispatches: the two `FACT` dispatches in `design.md` §Expected Sub-Agent Dispatches (second-writer question; angle/layer-index reachability).
- Cost: S
- Verification: `rg -c '\[BLOCK' docs/spec_packets/260b-support-interface-fill-claim-holders/design.md` returns 0 for unresolved entries.
- Exit condition (falsifying): any step below begins while a `[BLOCK]` is still open.

### Step 1: Register the claim and make the scheduler recognize it

- Objective: `claim:support-interface-fill` exists as a first-class claim — documented, recognized, resolvable — and an unmatched holder fails startup validation.
- Preconditions: Step 0 closed.
- Postconditions: AC-1 and AC-N1 green; no filler module exists yet, so the default path is untouched.
- Files allowed to edit: `docs/03_wit_and_manifest.md`, `crates/slicer-scheduler/src/**`, `crates/slicer-scheduler/tests/integration/support_interface_fill_claim_resolution_tdd.rs` (net-new) **and its `mod` registration in `crates/slicer-scheduler/tests/integration/main.rs`** (an unregistered file under an aggregated binary silently compiles to zero tests and reports a false green), plus the selector surface named by `[BLOCK-1]`'s ruling. If that busts the 3-edit cap, split the registration into its own sub-step.
- Out of bounds: `modules/**`, `crates/slicer-runtime/**`, `crates/slicer-gcode/**`, `crates/slicer-schema/wit/**`.
- Dispatches: `FACT` pass/fail per verification command.
- Cost: M
- Authorities: `docs/01_system_architecture.md` § Claim System and § Claim Conflict Resolution; `docs/04_host_scheduler.md` § Claim Resolution.
- Verification: `cargo test -p slicer-scheduler --test scheduler_integration support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition (falsifying): a holder naming no loaded module resolves to empty instead of erroring — the exact silent failure Authoring rule 4 names.

### Step 2: Ship the concentric interface filler

- Objective: a module holding only `claim:support-interface-fill` that reads `SupportPlanIR` interface geometry and emits nested closed `SupportInterface` loops.
- Preconditions: Step 1 green.
- Postconditions: the concentric arms of AC-2 green.
- Files allowed to edit: `modules/core-modules/support-interface-concentric/**` (net-new), `crates/slicer-runtime/tests/contract/support_interface_fill_claim_resolution_tdd.rs` (net-new) **and its `mod` registration in `crates/slicer-runtime/tests/contract/main.rs`**, `crates/slicer-runtime/Cargo.toml` (dev-dependency only if the arm drives the module natively — verify first).
- Out of bounds: the support renderers (Step 4), `crates/slicer-scheduler/src/**` (Step 1 froze it), `modules/core-modules/support-interface-grid/**`.
- Dispatches: `SUMMARY` on `FillConcentric`.
- Cost: M
- Authorities: `requirements.md` §Per-Key Canonical Evidence; `docs/08_coordinate_system.md`.
- Verification: `cargo test -p slicer-runtime --test contract support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition (falsifying): the module holds any claim besides `claim:support-interface-fill`, or emits open polylines where canonical emits closed loops.

### Step 3: Ship the grid interface filler

- Objective: a second single-claim module emitting two crossing line families at canonical's grid angle.
- Preconditions: Step 2 green; `[BLOCK-3]`'s ruling confirms the base angle is reachable (if it is not, this step is deleted and `grid` moves to §Returned to Queue).
- Postconditions: the grid arms of AC-2 and the grid arm of AC-4 green.
- Files allowed to edit: `modules/core-modules/support-interface-grid/**` (net-new), `crates/slicer-runtime/tests/contract/support_interface_fill_claim_resolution_tdd.rs`.
- Out of bounds: as Step 2, plus `modules/core-modules/support-interface-concentric/**`.
- Dispatches: `SUMMARY` on the grid variant of `FillRectilinear`.
- Cost: M
- Authorities: canonical `support_interface_angle()` (grid = base angle).
- Verification: `cargo test -p slicer-runtime --test contract support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition (falsifying): the emitted line families' angle does not equal the support base angle.

### Step 4: Wire the seam — `auto` resolution, renderer suppression, region override

- Objective: canonical's branch order decides the holder, the renderer skips its own interface fill exactly when a filler holds the claim, and a region override changes only its own region.
- Preconditions: Steps 1–3 green.
- Postconditions: AC-3, AC-4, AC-5, AC-N2 green.
- Files allowed to edit: `modules/core-modules/{traditional-support,tree-support}/src/lib.rs`, `crates/slicer-runtime/tests/contract/support_interface_fill_claim_resolution_tdd.rs`, and the `auto`-resolution site named by `[BLOCK-1]`'s ruling.
- Out of bounds: both filler modules (frozen), `docs/03_wit_and_manifest.md` (Step 1 owns it).
- Dispatches: `SUMMARY` on the `contact_fill_pattern` branch order.
- Cost: M
- Authorities: `requirements.md` §Per-Key Canonical Evidence (branch order); `design.md` §Divergences Recorded.
- Exit condition (falsifying): the no-holder default path is not byte-identical to `260a`'s output (AC-N2), or interface geometry is emitted twice for any region.
- Verification: `cargo test -p slicer-runtime --test contract support_interface_fill_claim_resolution_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p traditional-support --test support_contact_loops_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

### Step 5: Lints, docs, and gates

- Objective: the holder-only rule is enforced by a lint, the claim is documented, and every gate is green.
- Preconditions: Steps 1–4 green.
- Postconditions: AC-N3 green; `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and `cargo xtask build-guests --check` (exit 0) all pass.
- Files allowed to edit: `docs/15_config_keys_reference.md` (generated output only).
- Out of bounds: everything else.
- Cost: S
- Verification: `rg -l 'support_interface_pattern' modules/core-modules/*/[a-z-]*.toml; test $? -ne 0; echo "exit=$?"`, then `rg -q 'claim:support-interface-fill' docs/03_wit_and_manifest.md; echo "exit=$?"`, then `cargo xtask build-guests --check; echo "exit=$?"`
- Exit condition (falsifying): any manifest declares `support_interface_pattern`, or `build-guests --check` returns non-zero (a distinct exit code means `wasm-tools` is missing — infrastructure, not freshness).

## Per-Step Budget Roll-Up

| Step | Cost | Notes |
| --- | --- | --- |
| 0 | S | Rulings only, no code |
| 1 | M | Claim registration + scheduler recognition + validation variant |
| 2 | M | Net-new concentric module |
| 3 | M | Net-new grid module (may be deleted by `[BLOCK-3]`) |
| 4 | M | Seam wiring across both renderers |
| 5 | S | Lint, docs, gates |

Aggregate: **L** at packet level, no single step L. If `[BLOCK-1]`'s ruling makes Step 1 L, split the selector into its own packet before activation.

## Packet Completion Gate

- AC-1 … AC-5, AC-N1, AC-N2, AC-N3 green by their own commands.
- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals` clean.
- `cargo xtask build-guests --check` exit 0.
- Map gate (a): zero declaration-only keys — the packet's one key is built, and AC-N3 lints against it ever becoming a declaration. Map gate (b): AC-2 asserts the behaviour change at a non-default holder selection.

## Acceptance Ceremony

Run `cargo xtask test --summary --workspace` once, at closure only, after every narrower command has passed, dispatched to a sub-agent with a `FACT pass/fail` return.
