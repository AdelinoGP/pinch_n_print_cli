# Implementation Plan: 205-editions-xtask-dist-ci

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- `xtask` is bin-only (`xtask/Cargo.toml` declares no `[lib]`). Every test this packet writes is an in-file `#[cfg(test)] mod tests` unit inside `xtask/src/dist.rs`. Do not create `xtask/tests/`; an integration test there cannot reach `pub(crate)` items and would compile to a binary that asserts nothing.
- No step may write a module name read from `dist/editions.toml` into this plan, into an AC, into CI YAML, or into a doc. Re-derive it at the point of use.

## Steps

### Step 1: Reconcile the FORWARD-DEP surface from packets 203 and 204

- Task IDs: `ADR-0057`, `ADR-0056`
- Objective: establish, against the tree, the exact shapes this packet consumes — `load_editions`' signature and return type, `EditionSpec`'s field names, `EDITIONS_CONFIG_PATH`'s value, `dist/editions.toml`'s edition keys, the `slicer-integrated-modules` feature names, and which `integrated-*` features `crates/pnp-cli/Cargo.toml` already declares — and record any divergence from this packet's assumptions before writing code.
- Precondition: packets 203 and 204 are `implemented`; `xtask/src/editions.rs` and `dist/editions.toml` exist on disk.
- Postcondition: a written note (in the swarm working log, not a new file) listing the six reconciled facts, plus an explicit statement of any divergence from `design.md` §Code Change Surface.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/editions.rs` — whole file
  - `dist/editions.toml` — whole file
  - `crates/pnp-cli/Cargo.toml` — whole file
  - `xtask/src/build_guests.rs` — over 900 lines; **only** the declarations of `GuestTree`, `GuestSpec`, `discover_guests`, `workspace_root`, `build_command`, `tail_lines`, located by `rg -n 'pub (fn|struct|enum)' xtask/src/build_guests.rs`
  - `xtask/src/dist.rs` — whole file (short)
- Files allowed to edit (at most 3): none — read-only discovery step.
- Files explicitly out of bounds:
  - `.ralph/specs/203-*/design.md`, `.ralph/specs/203-*/implementation-plan.md`, `.ralph/specs/204-*/design.md`, `.ralph/specs/204-*/implementation-plan.md` — `SUMMARY` dispatch only
  - `crates/slicer-integrated-modules/src/**`, `crates/pnp-cli/src/**`
  - `target/`, `Cargo.lock`
- Blast-radius discipline: not applicable — no struct field and no schema/version constant is added or changed in this step.
- Expected sub-agent dispatches:
  - Question: exact signature of `load_editions`, field names/types of `EditionSpec`, value of `EDITIONS_CONFIG_PATH`? scope: `xtask/src/editions.rs`; return: `SNIPPETS` (1, ≤30 lines)
  - Question: which `integrated-*` features does `crates/pnp-cli/Cargo.toml` declare and what does each delegate to? scope: `crates/pnp-cli/Cargo.toml`; return: `FACT` (≤5 lines)
  - Question: exact declarations of `discover_guests`, `GuestSpec`, `GuestTree`, `workspace_root`, `build_command`, `tail_lines`? scope: `xtask/src/build_guests.rs`; return: `LOCATIONS` (≤10 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0057-three-editions-and-integrated-tier.md` — short; direct read of the edition table
  - `.ralph/specs/204-hybrid-pilot-parity/packet.spec.md` — AC-7 and AC-N1 only
- OrcaSlicer refs: none — distribution packaging has no canonical equivalent.
- Verification:
  - `sh -c 'test -f xtask/src/editions.rs && test -f dist/editions.toml && rg -q "load_editions" xtask/src/editions.rs && rg -q "EDITIONS_CONFIG_PATH" xtask/src/editions.rs && echo PASS'` — FACT pass/fail
- Exit condition: the six facts are recorded and either match `design.md` or the divergence is written down with the adaptation it forces. If `xtask/src/editions.rs` or `dist/editions.toml` is absent, STOP — packet 204 has not landed and this packet cannot proceed.

### Step 2: Planning layer — `DistArgs`, `parse_dist_args`, `DistPlan`, `plan_edition`

- Task IDs: `ADR-0057`
- Objective: land the pure resolution layer that turns (`dist/editions.toml`, `discover_guests`) into a `DistPlan`, together with its four unit tests, without touching `dist_command`'s existing behaviour.
- Precondition: Step 1's facts are recorded; `dist_command(ws_root, debug)` still has its original signature and `main.rs` still calls it.
- Postcondition: `cargo test -p xtask dist_plan_` and `cargo test -p xtask dist_arg_parsing_` pass; `cargo xtask dist` still behaves exactly as before (nothing calls the new code yet).
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/dist.rs` — whole file
  - `xtask/src/editions.rs` — whole file
  - `xtask/src/build_guests.rs` — **only** the six symbols from Step 1, by `rg`
  - `xtask/src/check_deviations.rs` — its `#[cfg(test)] mod tests` block only, for the in-file test idiom
- Files allowed to edit (at most 3):
  - `xtask/src/dist.rs`
- Files explicitly out of bounds:
  - `xtask/src/editions.rs` (packet 204's file — read-only), `xtask/src/main.rs` (Step 5), `.github/workflows/ci.yml`, `crates/**`
- Blast-radius discipline: `DistArgs` and `DistPlan` are net-new types with zero pre-existing struct-literal sites, and no schema/version constant is bumped. No sweep required.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p xtask dist_` pass, and if not what is the assertion? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0057-three-editions-and-integrated-tier.md` — the edition table; `integrate_all = true` must expand to every core stem
  - `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — short; `discover_guests` is the only permitted source of the core module set
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p xtask dist_plan_developer_stages_every_core_module` — FACT pass/fail
  - `cargo test -p xtask dist_plan_hybrid_derives_features_and_complement` — FACT pass/fail
  - `cargo test -p xtask dist_plan_integrated_stages_nothing_externally` — FACT pass/fail
  - `cargo test -p xtask dist_arg_parsing_accepts_edition_and_debug_in_any_order` — FACT pass/fail
- Exit condition: all four tests pass, and none of them contains a literal module name or a literal module count — every expectation is derived from `discover_guests` or from `load_editions` at test time. A test that hardcodes `21` fails this exit even if it is green.

### Step 3: Enforcement helpers — disjointness and feature coverage

- Task IDs: `ADR-0056`
- Objective: land `assert_staging_disjoint`, `pnp_cli_integrated_features`, `verify_integrated_feature_coverage`, and the `preflight_edition` composition (resolve → coverage → plan-time disjointness) with their two negative unit tests, so the ADR-0056 invariant has a real, independently falsifiable enforcement point — and a single named gate whose position relative to the build can be asserted — before anything calls it.
- Precondition: Step 2's `DistPlan` exists and its tests pass.
- Postcondition: `cargo test -p xtask dist_disjointness_` and `cargo test -p xtask dist_registry_coverage_` pass; `dist_command` still unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/dist.rs` — whole file
  - `crates/pnp-cli/Cargo.toml` — whole file, for the `[features]` table shape `pnp_cli_integrated_features` parses
  - `docs/adr/0056-integrated-modules-native-dispatch.md` — Decision item 2 and §Consequences only
- Files allowed to edit (at most 3):
  - `xtask/src/dist.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/Cargo.toml` (edited in Step 4, read-only here), `xtask/src/editions.rs`, `xtask/src/main.rs`, `.github/workflows/ci.yml`
- Blast-radius discipline: not applicable — three free functions, no struct field, no constant.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p xtask dist_` pass, and if not what is the assertion? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` — §Consequences' "an edition must never stage an external copy of a module it integrates"; Decision item 2 for why (tier 5 + first-root-wins)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p xtask dist_disjointness_rejects_integrated_module_in_staged_set` — FACT pass/fail
  - `cargo test -p xtask dist_registry_coverage_rejects_missing_pnp_cli_feature` — FACT pass/fail
- Exit condition: both negative tests fail when the corresponding `Err` branch is deleted (verify by temporarily returning `Ok(())` and re-running — the tests must go red). A negative test that passes against a stubbed-out check is vacuous and does not satisfy this exit. `preflight_edition` exists and composes all three checks in the order resolve → coverage → disjointness; it is not yet called by `dist_command` (Step 5 does that).

### Step 4: `pnp-cli` passthrough features for the Hybrid set

- Task IDs: `ADR-0057`
- Objective: declare `integrated-<name> = ["slicer-integrated-modules/<name>"]` in `crates/pnp-cli/Cargo.toml` for every module in the resolved Hybrid integrated set, extending packet 203's single `integrated-classic-perimeters` entry so that `cargo build -p pnp-cli --features <derived list>` compiles.
- Precondition: Steps 2–3 landed; `dist/editions.toml`'s Hybrid membership has been re-derived by dispatch in this step (never carried in from a document). **AND** `crates/pnp-cli/Cargo.toml`'s `[dependencies]` table already declares `slicer-integrated-modules`. Verify this first: `rg -q 'slicer-integrated-modules' crates/pnp-cli/Cargo.toml`. If it is absent, **STOP** — no packet-203 AC asserts that dependency edge (it is described only in 203's `requirements.md`/`design.md`), so an implementer working from 203's ACs alone can land 203 green without it. The edge is 203's surface; report the gap rather than adding the dependency here, because this packet owns only the `[features]` table of that manifest.
- Postcondition: for every name in the Hybrid set, `crates/pnp-cli/Cargo.toml` declares `integrated-<name>`; `cargo check -p pnp-cli --features <the full derived list>` succeeds; `cargo check -p pnp-cli` with no features is unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `dist/editions.toml` — whole file
  - `crates/pnp-cli/Cargo.toml` — whole file
  - `crates/slicer-integrated-modules/Cargo.toml` — whole file, to confirm each target feature exists
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/Cargo.toml`
- Files explicitly out of bounds:
  - `crates/slicer-integrated-modules/**` (packet 204's surface), `crates/pnp-cli/src/**` (packet 203's surface), `dist/editions.toml`, `xtask/**`
- Blast-radius discipline: not applicable — additive, off-by-default cargo features; no struct field, no constant. `default = ["report"]` is not modified, so no existing build changes.
- Expected sub-agent dispatches:
  - Question: what are the edition keys in `dist/editions.toml` and the exact contents of `hybrid.integrated_modules`? scope: `dist/editions.toml`; return: `FACT` (≤5 lines)
  - Question: which features does `crates/slicer-integrated-modules/Cargo.toml` declare? scope: that file; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `.ralph/specs/203-integrated-cli-provenance/packet.spec.md` — the `integrated-classic-perimeters` precedent this step extends
  - `docs/adr/0057-three-editions-and-integrated-tier.md` — the Hybrid row of the edition table
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'set -e; for m in $(rg -o "\"[a-z0-9-]+\"" dist/editions.toml | tr -d "\"" | sort -u); do rg -q "^integrated-$m *=" crates/pnp-cli/Cargo.toml || echo "absent: $m"; done; echo DONE'` — FACT: the only `absent:` lines may be names that are not in `hybrid.integrated_modules` (the grep is deliberately over-broad; reconcile against the Step-4 dispatch `FACT`)
  - `cargo check -p pnp-cli --all-targets` — FACT pass/fail
- Exit condition: every Hybrid module has a passthrough feature whose body names the `slicer-integrated-modules` feature of the identical name, and `cargo check -p pnp-cli --all-targets` passes with and without those features. If any Hybrid name has no matching `slicer-integrated-modules` feature, STOP. Attribute correctly: the registry crate's per-module cargo features are **packet 201's** surface (its `design.md` §Code Change Surface item 4, locked to the bare module-directory name); `dist/editions.toml` and its `hybrid.integrated_modules` list are **packet 204's**. A mismatch is a defect in whichever of the two disagrees with the other — never something to paper over here by renaming this packet's passthrough.

### Step 5: Rewrite `dist_command`, wire the subcommand, add `--plan`

- Task IDs: `ADR-0057`, `ADR-0056`
- Objective: make `dist_command` execute a `DistPlan` — resolve, verify coverage, check disjointness, optionally print the plan and stop, then build guests, build `pnp-cli` with the derived `--features`, stage into `target/dist/<edition>/`, and re-check disjointness against the directory names actually on disk — and wire `--edition` / `--debug` / `--plan` through `main.rs` with the existing exit-code convention.
- Precondition: Steps 2–4 landed; all six unit tests green.
- Postcondition: `cargo xtask dist` (no flags) stages the Developer edition into `target/dist/developer/`; `cargo xtask dist --edition bogus` exits `1` before any build; `cargo xtask dist --nope` exits `2`; `cargo xtask dist --edition hybrid --plan` prints the plan and exits `0` without building.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/dist.rs` — whole file
  - `xtask/src/main.rs` — whole file (short)
  - `xtask/src/build_guests.rs` — **only** `build_command` and `tail_lines`, by `rg`
- Files allowed to edit (at most 3):
  - `xtask/src/dist.rs`
  - `xtask/src/main.rs`
- Files explicitly out of bounds:
  - `xtask/src/editions.rs`, `xtask/src/{build_guests,check_deviations,compact_specs,gen_config_docs,test,wit_verify}.rs`, `crates/**`, `.github/workflows/ci.yml`, `docs/**`, `README.md`
- Blast-radius discipline: `dist_command`'s signature changes from `(ws_root: &Path, debug: bool)` to `(ws_root: &Path, args: &DistArgs)`. Its only call sites are the two `Some("dist")` match arms in `xtask/src/main.rs`, both of which this step edits — verified at authoring by `rg -n 'dist_command' xtask/src/`. No other crate can call it (`xtask` is bin-only). **`USAGE` is this step's responsibility, not Step 6's**: all three `dist` lines are updated here — the two new flags (`--edition <NAME>`, `--plan`) and the existing `dist` line, which today reads "stage them under target/dist/." and would otherwise promise the pre-edition root. A signature change that leaves the help text describing the old flags and the old path is a half-landed change; moving this into Step 6 would also push that step to four edits.
- Ordering discipline (AC-N2): `dist_command`'s **first** action is `preflight_edition`; the `build_guests::build_command` call must appear strictly after it in the file. AC-N2 asserts this positionally and AC-9 asserts it behaviorally (no `Compiling ` line before the rejection). Do not "optimize" by starting the guest build early.
- Expected sub-agent dispatches:
  - Question: after the edit, do `cargo xtask dist --nope` and `cargo xtask dist --edition` both exit `2`, and does `cargo xtask dist --edition bogus` exit `1` without creating `target/dist/bogus`? scope: repo root; return: `FACT` (≤5 lines, three exit codes)
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` — §Consequences; the post-stage check is what makes the invariant hold against the artifact rather than against the intent
  - `docs/01_system_architecture.md` §"Producing the tier-4 layout: `cargo xtask dist`" — ranged read; the wipe-then-stage rationale that must survive the move to a per-edition root
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'cargo xtask dist --nope >/dev/null 2>&1; a=$?; cargo xtask dist --edition >/dev/null 2>&1; b=$?; [ "$a" = "2" ] && [ "$b" = "2" ] && echo PASS || { echo "FAIL a=$a b=$b"; exit 1; }'` — FACT `PASS` / `FAIL`
  - `sh -c 'rm -rf target/dist/bogus; out=$(cargo xtask dist --edition bogus 2>&1); rc=$?; [ "$rc" = "1" ] && printf "%s" "$out" | rg -q "bogus" && printf "%s" "$out" | rg -q "dist/editions.toml" && [ ! -d target/dist/bogus ] && echo PASS || { echo "FAIL rc=$rc"; exit 1; }'` — FACT `PASS` / `FAIL`
  - `sh -c 'cargo xtask dist --edition hybrid --plan | rg -q "^edition\thybrid" && cargo xtask dist --edition hybrid --plan | rg -q "^external\t" && echo PASS'` — FACT `PASS` / `FAIL`
  - AC-N2's `sh -c` command from `packet.spec.md` (coverage unit test + `preflight_edition` precedes `build_guests::build_command`) — FACT `PASS` / `FAIL`
  - AC-9's `sh -c` command from `packet.spec.md` (`--edition integrated` rejects, names `crates/pnp-cli/Cargo.toml`, spawns no build, creates no directory) — FACT `PASS` / `SKIP` / `FAIL`
  - `cargo clippy -p xtask --all-targets -- -D warnings` — FACT pass/fail
- Exit condition: the three exit-code checks pass, AC-N2 and AC-9 pass, `--plan` emits the locked TSV kinds (`edition`, `out_dir`, `features`, `integrated`, `external`), and `cargo xtask dist --edition bogus` produces no `target/dist/` subdirectory and spawns no cargo build (confirm by the absence of any `Compiling ` line in its output).

### Step 6: Doc surfaces — `docs/01`, `README.md`, `CLAUDE.md`

- Task IDs: `ADR-0057`
- Objective: bring the remaining prose surfaces in line with the shipped behaviour — the edition flag, the per-edition output root, and the disjointness rule — and prove by sweep that no surface still promises the pre-edition root. (`xtask/src/main.rs`'s `USAGE` is the fourth surface and was updated in Step 5 alongside the signature change; this step only verifies it.)
- Precondition: Step 5 landed, including its `USAGE` edit; `cargo xtask dist --edition developer` produces `target/dist/developer/`.
- Postcondition: both halves of AC-8 pass — the four positive greps, and the negative sweep showing every `target/dist` occurrence across the four surfaces is a per-edition form.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/01_system_architecture.md` — §"Producing the tier-4 layout: `cargo xtask dist`" only, located by heading text (the file is large; do not read it)
  - `README.md` — §"Building and Running" fenced block only
  - `CLAUDE.md` — §"Build & Test Commands" fenced block only
  - `xtask/src/main.rs` — the `USAGE` const only (read-only here; edited in Step 5)
- Files allowed to edit (at most 3):
  - `docs/01_system_architecture.md`
  - `README.md`
  - `CLAUDE.md` — **only** the `cargo xtask dist` line inside §"Build & Test Commands", which today reads "stage into target/dist/ (add --debug for debug binary)". Every other section of the project instruction file is out of bounds.
- Files explicitly out of bounds:
  - `docs/adr/**`, `CONTEXT.md`, `docs/07_implementation_status.md`, `docs/specs/multi-edition-distribution-plan.md`, any other `docs/*.md`, `.claude/**`, every section of `CLAUDE.md` other than the one line named above
- Blast-radius discipline: not applicable — prose only. The surface count is established by measurement, not by preference: `rg -n "target/dist" CLAUDE.md README.md xtask/src/main.rs docs/*.md .claude/*.md` returned exactly these four files at authoring, and there is no release script, `.sh`, `.ps1`, `Makefile`, or `justfile` in the tree. Re-run the sweep before concluding the step; a fifth surface appearing since authoring is in scope for this step.
- Expected sub-agent dispatches:
  - Question: which files still name `target/dist` in a non-per-edition form? scope: `docs/`, `README.md`, `CLAUDE.md`, `xtask/src/main.rs`, `.claude/`; return: `LOCATIONS` (≤10 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0057-three-editions-and-integrated-tier.md` — the edition names are user-facing vocabulary per its §Consequences; use `Developer` / `Hybrid` / `Integrated` verbatim
- OrcaSlicer refs: none.
- Verification:
  - AC-8's `sh -c` command from `packet.spec.md` (four positive greps plus the negative stale-root sweep) — FACT `PASS` / `FAIL` with the offending lines
  - `sh -c 'hits=$(rg -n "target/dist" docs/ README.md CLAUDE.md .claude/ xtask/src/ 2>/dev/null | rg -v "target/dist/<edition>|target/dist/(developer|hybrid|integrated)|target/dist\").join|dist_dir"); [ -z "$hits" ] && echo "PASS: no stale output-root references" || { echo "$hits"; exit 1; }'` — FACT `PASS` / list. Broader than AC-8 on purpose: it also sweeps `.claude/` and the rest of `xtask/src/`, so a fifth surface added since authoring is caught rather than silently missed.
- Exit condition: both halves of AC-8 pass, the broader sweep is clean, and the new `docs/01` paragraph does not restate packet 204's `dist/editions.toml` paragraph — the two must be complementary, not overlapping. If 204's paragraph is absent, that is a 204 gap; note it and do not write 204's content here.

### Step 7: CI job `Dist editions` and the end-to-end artifact proof

- Task IDs: `ADR-0057`, `ADR-0056`
- Objective: add the first CI invocation of `cargo xtask dist` — a job that builds the Developer and Hybrid artifacts and verifies each against its own `--plan` output, failing with a message that names the disjointness invariant — and run the full AC-5 artifact check locally once.
- Precondition: Steps 5–6 landed; `cargo xtask build-guests --check` reports clean.
- Postcondition: `.github/workflows/ci.yml` declares the job; AC-5 and AC-6 pass locally.
- Files allowed to read, with ranges when over 300 lines:
  - `.github/workflows/ci.yml` — whole file (short)
- Files allowed to edit (at most 3):
  - `.github/workflows/ci.yml`
- Files explicitly out of bounds:
  - `xtask/**`, `crates/**`, `docs/**`, `README.md`, any other file under `.github/`
- Blast-radius discipline: not applicable — one additive YAML job; no existing job is modified.
- Expected sub-agent dispatches:
  - Question: does `cargo xtask build-guests --check` report clean? scope: repo root; return: `FACT` clean / `STALE:` list
  - Question: does AC-5's command print `PASS`? scope: repo root; return: `FACT` `PASS` / the single `FAIL` line
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0057-three-editions-and-integrated-tier.md` — §Consequences: "CI gains build/verification of the edition artifacts (today CI never runs `dist` at all)"
  - `CLAUDE.md` §"Guest WASM Staleness" — why the freshness check precedes any artifact assertion
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'rg -q "name: Dist editions" .github/workflows/ci.yml && rg -q -- "dist --edition developer" .github/workflows/ci.yml && rg -q -- "dist --edition hybrid" .github/workflows/ci.yml && rg -q "wasm-tools" .github/workflows/ci.yml && rg -qi "disjoint" .github/workflows/ci.yml && echo PASS'` — FACT `PASS` / `FAIL`
  - AC-5's `sh -c` command from `packet.spec.md` — FACT `PASS` / one `FAIL` line. **Dispatch this; never run it inline.** Its binary check is `[ -f .../pnp_cli ] || [ -f .../pnp_cli.exe ]`; if a future edit reintroduces a two-operand `ls` guard the AC becomes unconditionally red, because `ls <present> <missing>` exits `2` and only one of the two names can exist on any platform.
- Exit condition: the CI job exists with the wasm32 target and `wasm-tools` installed (both required because `dist` builds guests), its verification step fails loudly rather than warning, and AC-5 prints `PASS` against a freshly built tree. If AC-5 fails on the staged-count comparison, do not adjust the AC — the plan and the artifact genuinely disagree and `dist_command`'s staging loop is wrong.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Read-only FORWARD-DEP reconciliation; three bounded dispatches |
| Step 2 | M | Largest step: planning layer plus four unit tests |
| Step 3 | S | Three pure helpers plus two negative tests |
| Step 4 | S | One manifest edit driven by a re-derived module list |
| Step 5 | M | `dist_command` rewrite plus subcommand wiring; two files |
| Step 6 | S | Three prose surfaces |
| Step 7 | M | CI job plus the one full-build artifact proof (dispatched) |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate: `M`; no step is L.

## Packet Completion Gate

- All seven steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` carries no TASK row for this program (see the plan's §"Backlog anchoring [FWD]"); do **not** invent one and do **not** edit that file while the parallel 194–199 session is active. If the user has since ratified a "Distribution & Editions" workstream, update it through a worker dispatch with the TASK number re-derived at write time — never a number quoted from this packet or from the plan.
- Reconcile reopened/superseded status transitions: none — this packet supersedes nothing.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC (AC-1 … AC-9, AC-N1 … AC-N4) and the three packet-level gate commands. AC-9 returning `SKIP` is acceptable only if AC-3 passes and `preflight_edition` reports full coverage; a `SKIP` alongside an incomplete registry means the gate is not firing.
- Run `cargo xtask build-guests --check` immediately before the AC-5 re-run; a `STALE:` report invalidates the artifact proof.
- This packet closes the plan's final queue row. Report the exports listed in `packet.spec.md` §Prerequisites so a future phase-4 platform-build packet can consume the plan-resolution surface without re-deriving it.
- Record remaining packet-local risk: `--edition integrated` remains unbuildable until every core module is registry-available with an ADR-0056 Decision item 4 parity gate; that is designed behaviour asserted by AC-N2, not an open defect.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
