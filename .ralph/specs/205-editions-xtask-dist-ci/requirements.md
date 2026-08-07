# Requirements: 205-editions-xtask-dist-ci

## Packet Metadata

- Grouped task IDs: `ADR-0057`, `ADR-0056` (no `docs/07_implementation_status.md` TASK row exists for this program — see the plan's §"Backlog anchoring [FWD]"; do not invent one, and do not edit `docs/07` while the parallel 194–199 session is active)
- Backlog source: `docs/specs/multi-edition-distribution-plan.md` (queue row 6, the final row)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

ADR-0057 decides that three editions ship — Developer (no integrated modules, every core module staged loose), Hybrid (an evidence-driven integrated set, the rest staged loose), Integrated (every core module integrated, nothing staged) — but the only distributable that exists is `dist_command`'s single fixed layout in `xtask/src/dist.rs`: build every guest, build `pnp-cli` with default features, wipe `target/dist/`, copy the binary, copy every `GuestTree::Core` guest's `<stem>.wasm` + `<stem>.toml` into `target/dist/modules/<stem>/`. There is no edition selector, no path from `dist/editions.toml` (packet 204's committed config) into the `pnp-cli` cargo feature set, and no complement logic — so no edition other than Developer is expressible.

Worse, the layout is unverified: `.github/workflows/ci.yml` never invokes `cargo xtask dist` in any of its four jobs (`fmt`, `docs-guard`, `clippy`, `test`). A broken dist bundle ships silently.

The invariant that makes editions meaningful is ADR-0056's consequence: *an edition must never stage an external copy of a module it integrates*, because integrated modules sit at search tier 5 and any higher-tier disk copy wins first-root-wins dedup — the artifact would look like a Hybrid build and behave like a Developer build, with only a provenance warning on stderr to distinguish them. That invariant currently has no enforcement point anywhere in the tree.

## In Scope

- An edition selector on the `dist` subcommand: `cargo xtask dist [--edition <NAME>] [--debug] [--plan]`, `--edition` defaulting to `developer`, flags accepted in any order, unknown flags exiting `2` per the existing xtask convention.
- A pure `parse_dist_args` helper for the above, unit-tested in-file.
- Edition resolution: read `dist/editions.toml` through packet 204's `xtask::editions::load_editions` (**FORWARD-DEP**), producing a `DistPlan { edition, out_dir, cargo_features, integrated, external_stage }` from the edition spec plus `build_guests::discover_guests`' `GuestTree::Core` stems. `integrate_all = true` expands to every core stem; otherwise the integrated set is `integrated_modules` verbatim; `external_stage` is always the exact complement.
- `pnp-cli` feature derivation: `cargo_features` = `integrated-<name>` per integrated module, passed as `--features` to the existing `cargo build -p pnp-cli` invocation inside `dist_command`.
- Passthrough cargo features on `crates/pnp-cli/Cargo.toml` (`integrated-<name> = ["slicer-integrated-modules/<name>"]`) for every module in the resolved Hybrid integrated set, extending the single `integrated-classic-perimeters` feature packet 203 introduces.
- Per-edition output root `target/dist/<edition>/`, replacing the single `target/dist/`. The wipe-then-stage behaviour is retained but scoped to the edition subdirectory, so building two editions in one CI job does not destroy the first artifact.
- `preflight_edition`: the single named pre-build gate composing resolution, coverage verification, and the plan-time disjointness check. `dist_command` calls it exactly once, before any build phase. Naming the composition is what makes the ordering assertable positionally (AC-N2) and behaviorally (AC-9's "no build was spawned" clause) rather than merely described in prose.
- `verify_integrated_feature_coverage`: a fail-fast check, run **before** any build, that every module the edition wants integrated has a `integrated-<name>` feature declared in `crates/pnp-cli/Cargo.toml`. This is what keeps `--edition integrated` honest while the registry covers only the pilot set: it errors with a named list instead of silently producing an "Integrated" artifact that stages the un-integrated remainder externally.
- `assert_staging_disjoint`: the ADR-0056 invariant as a pure function, called twice — once against the planned `external_stage` before copying, once against the directory names actually present under `target/dist/<edition>/modules/` after copying (which catches a leftover directory, a stale artifact, or a future refactor that stages from a different list).
- `--plan`: print the resolved plan as tab-separated lines (`edition\t<name>`, `out_dir\t<path>`, `features\t<comma-list>`, one `integrated\t<name>` per integrated module, one `external\t<name>` per staged module) and exit `0` without building. This is the machine-readable surface CI and the ACs verify artifacts against.
- A new `.github/workflows/ci.yml` job `Dist editions` building and verifying the Developer and Hybrid artifacts.
- Doc edits listed in `packet.spec.md` §Doc Impact Statement. The output path is named by exactly four surfaces, established by measurement (`rg -n "target/dist" CLAUDE.md README.md xtask/src/main.rs docs/*.md .claude/*.md`): `docs/01_system_architecture.md`, `README.md`, `xtask/src/main.rs`'s `USAGE`, and `CLAUDE.md` §"Build & Test Commands". All four are updated; AC-8's negative half asserts none of them still names the pre-edition root. Only the single `cargo xtask dist` line in `CLAUDE.md` may be touched.

## Out of Scope

- Authoring or editing `dist/editions.toml`, its `schema_version`, its `# evidence:` block, or its Hybrid membership — packet 204 owns the file and the profiling that finalizes it. This packet only reads it.
- Editing `xtask/src/editions.rs` (204's file). If `load_editions`' shape differs from the Exports ledger, this packet adapts its callers; it does not change 204's API.
- Integrating additional modules into `crates/slicer-integrated-modules/` or authoring parity gates for them. Until every core module is registry-available, `--edition integrated` is expected to fail `verify_integrated_feature_coverage` with a named list — that is the designed behaviour, not a defect.
- Dispatch routing, macro emission, marshalling (202); `--no-integrated-modules`, `module diagnose` provenance JSON, the CLI verb tree (203); parity comparators (204).
- ADR-0057 phase 4: aarch64 matrix, iOS AOT builds, browser research. Explicitly deferred by the plan; no packet.
- Release packaging (archives, checksums, upload steps, release pages). This packet produces and verifies the artifact tree only.
- `cargo test --workspace` in CI. The existing `test` job's narrow-crate strategy is untouched.

## Authoritative Docs

- `docs/adr/0057-three-editions-and-integrated-tier.md` — short; direct read.
- `docs/adr/0056-integrated-modules-native-dispatch.md` — short; direct read; Decision item 2 and §Consequences.
- `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — short; direct read.
- `docs/01_system_architecture.md` — large; ranged read of §"Producing the tier-4 layout: `cargo xtask dist`" only, located by heading text. Delegate anything else.
- `CONTEXT.md` — delegate a `FACT` lookup of the terms **Edition**, **Integrated module**, **External module**; never load the file.
- `.ralph/specs/204-hybrid-pilot-parity/packet.spec.md` and `.ralph/specs/203-integrated-cli-provenance/packet.spec.md` (both short) — read-only, whole file each, to reconcile FORWARD-DEP shapes. Never modified.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9`. Measurable refinements not restated in their Given/When/Then text:
  - AC-1/AC-2/AC-3 must derive the core module set from `discover_guests` at test time, never from a literal count. A test that hardcodes a module count rots the moment a module is added or removed and is a defect even while green.
  - AC-5 is the only AC that performs a real build; it is the packet's slowest verification and is the one CI reproduces. It intentionally asserts against `--plan` output rather than a fixed module list so it stays correct when `hybrid.integrated_modules` changes. Its binary check must be a disjunctive per-file test (`[ -f a ] || [ -f b ]`), never a multi-operand `ls`: `ls <present> <missing>` exits `2`, and exactly one of `pnp_cli` / `pnp_cli.exe` can exist on any platform, so a two-operand `ls` guard is unconditionally red.
  - AC-7 verifies the feature **body**, not the feature name. A grep that a name appears somewhere in the manifest is satisfied by a comment and proves nothing.
  - AC-8 has a negative half: no updated surface may still name the pre-edition `target/dist/` root. The positive `--edition` greps alone would pass on a half-landed edit.
  - AC-9 is the end-to-end proof of the packet's headline rejection behaviour and is self-guarding — it reports `SKIP` once the Integrated edition becomes fully covered, so it never inverts into a false failure.
- Negative: `AC-N1` through `AC-N4`. `AC-N1` is the ADR-0056 disjointness invariant; `AC-N2` is the fail-fast registry-coverage gate **plus** the positional assertion that `preflight_edition` precedes `build_guests::build_command` in `xtask/src/dist.rs` (a bare grep for the function name cannot distinguish its own definition from a call site, so it does not appear in this packet); `AC-N3`/`AC-N4` are the CLI rejection paths and their distinct exit codes (`1` for a resolvable-but-invalid input, `2` for a malformed command line — the existing xtask convention).
- Cross-packet impact: `crates/pnp-cli/Cargo.toml` gains features whose targets are packet 204's `slicer-integrated-modules` features; if 204 finalizes a different Hybrid set, this packet's feature list follows it and nothing else changes. `xtask/src/editions.rs` is read but never edited. Nothing in `crates/slicer-runtime`, `crates/slicer-scheduler`, or `crates/slicer-wasm-host` is touched.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p xtask dist_plan_developer_stages_every_core_module` | AC-1 | FACT pass/fail |
| `cargo test -p xtask dist_plan_hybrid_derives_features_and_complement` | AC-2 | FACT pass/fail |
| `cargo test -p xtask dist_plan_integrated_stages_nothing_externally` | AC-3 | FACT pass/fail |
| `cargo test -p xtask dist_arg_parsing_accepts_edition_and_debug_in_any_order` | AC-4 | FACT pass/fail |
| `cargo test -p xtask dist_disjointness_rejects_integrated_module_in_staged_set` | AC-N1 | FACT pass/fail |
| `cargo test -p xtask dist_registry_coverage_rejects_missing_pnp_cli_feature` | AC-N2, unit half only | FACT pass/fail |
| `cargo test -p xtask dist_` | all in-file dist units at once (closure gate) | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `sh -c 'cargo xtask dist --nope >/dev/null 2>&1; a=$?; cargo xtask dist --edition >/dev/null 2>&1; b=$?; [ "$a" = "2" ] && [ "$b" = "2" ] && echo PASS \|\| { echo "FAIL a=$a b=$b"; exit 1; }'` | AC-N4 exit codes | FACT `PASS` / `FAIL` |
| AC-N3's `sh -c` command (see `packet.spec.md`) | unknown edition fails fast, creates nothing | FACT `PASS` / `FAIL` |
| AC-5's `sh -c` command (see `packet.spec.md`) | real artifact + disjointness on disk | FACT `PASS` / `FAIL` |
| AC-6's `sh -c` command (see `packet.spec.md`) | CI job exists and names the invariant | FACT `PASS` / `FAIL` |
| AC-7's `sh -c` command (see `packet.spec.md`) | pnp-cli passthrough feature **bodies** delegate correctly | FACT `PASS` / `FAIL` |
| AC-8's `sh -c` command (see `packet.spec.md`) | doc greps, positive and negative (no stale `target/dist/` root) | FACT `PASS` / `FAIL` |
| AC-9's `sh -c` command (see `packet.spec.md`) | `--edition integrated` rejects end to end without building | FACT `PASS` / `SKIP` / `FAIL` |
| AC-N2's `sh -c` command (see `packet.spec.md`) | coverage unit test + `preflight_edition` precedes the guest build | FACT `PASS` / `FAIL` |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness before any AC-5 run | FACT clean / `STALE:` list |

## Step Completion Expectations

- **Output-path change is a single atomic move.** `target/dist/` becoming `target/dist/<edition>/` must land together with the doc and README edits in the same step. A tree where the code stages to the new path and the docs describe the old one is a worse state than either endpoint.
- **The fail-fast ordering is load-bearing across steps.** Edition resolution, `verify_integrated_feature_coverage`, and the plan-time `assert_staging_disjoint` all run *before* `build_guests::build_command` and before `cargo build -p pnp-cli`. A later step must not reorder them for convenience: AC-N3's "no build spawned" clause and CI's cost model both depend on it.
- **`--plan` is the contract between the ACs and the artifact.** Its line format (`<kind>\t<value>`, one record per line, no header) is fixed once Step 2 lands; Steps 5–7's verification commands parse it with `rg` + `cut -f2`. Changing the separator later silently breaks four ACs.
- **Re-derive, never quote, the Hybrid membership.** No step may write the finalized Hybrid module names into code, CI, or docs. Every consumer reads them from `dist/editions.toml` via `load_editions`, or from `--plan` output. Packet 204's profiling can change that list after this packet lands, and nothing here may need editing when it does.

## Context Discipline Notes

- `xtask/src/build_guests.rs` is over 900 lines. Read only `discover_guests`, `GuestSpec`'s `artifact_path` / `tree` fields, `GuestTree::Core`, `workspace_root`, `build_command`, and `tail_lines` — located by symbol, not by browsing.
- `docs/01_system_architecture.md` is large. Locate §"Producing the tier-4 layout: `cargo xtask dist`" by heading text and read a bounded window around it. Do not read the file.
- `xtask/src/editions.rs` is a FORWARD-DEP file that will not exist until 204 lands. If it is absent, read `.ralph/specs/204-hybrid-pilot-parity/packet.spec.md` (AC-7, AC-N1) for the shape and stop — do not read 204's `design.md` or `implementation-plan.md` directly; delegate a `SUMMARY` if more is needed.
- AC-5 performs a full guest build plus a `pnp-cli` build. Dispatch it; never run it inline and never absorb its output — the command already reduces to `PASS` / one `FAIL` line.
- `target/`, `Cargo.lock`, and `modules/core-modules/*/wit-guest/Cargo.lock` are never loaded.
