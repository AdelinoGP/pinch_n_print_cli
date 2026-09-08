# Design: 199-literal-gate-enforcement

## Controlling Code Paths

- Primary code path: `test_command` (`xtask/src/test.rs`) — the gated test entry point. Current structure (verified 2026-08-07): flag parsing, `--summary-from` early return (explicitly gate-free), Step 0b arachne feature/quarantine arg munging, Step 1 freshness (`build_guests::check_command` then `ensure_pnp_cli_fresh`), then the cargo-test spawn. The new preflight inserts at the top of Step 1, before `build_guests::check_command`.
- Scan entry point: `xtask/src/check_literals.rs` (authored by packet 194; exports ledger gives the CLI contract — exit 0 clean / 1 violations / 2 usage, violation line `<ws-relative-path>:<line>: exhaustive literal of watched type \`<TypeName>\``, summary `check-literals: <N> violation(s) in <M> file(s) (watchlist: <K> types)`).
- Neighboring tests/fixtures: the `#[cfg(test)]` mod already in `xtask/src/test.rs` (its `ensure_pnp_cli_fresh_with` injected-runner test is the precedent for the fake-workspace-root technique AC-N1 reuses); packet 194's xtask unit tests for the scanner.
- Residue conversion paths: `crates/slicer-model-io/tests/*.rs` + `src/loader.rs` `#[cfg(test)]` `make_object`; `crates/slicer-helpers/tests/{decimate_tdd,repair_tdd}.rs`; `crates/slicer-macros/tests/slicer_module_tdd.rs` mock impls.
- OrcaSlicer comparison: none — no parity surface; the orca-delegation snippet is intentionally absent packet-wide.

## Architecture Constraints

- **Preflight order and scope (locked):** the check-literals preflight runs workspace-wide in enforce mode regardless of `-p`/filter args passed to `cargo xtask test`, and runs BEFORE `build_guests::check_command` — a pure-syntax scan must abort the run before the (potentially slow, possibly guest-rebuilding) freshness gate spends work on a red tree. The `--summary-from` path stays gate-free (no test run = no gate), mirroring the existing guest-freshness exemption.
- **Guest-WASM staleness — snippet intentionally omitted, with both facts stated:** the grounded change surface (xtask sources, CLAUDE.md, docs/21, model-io tests + `src/loader.rs` cfg-test mod, helpers tests, macros `tests/slicer_module_tdd.rs`) touches NO guest-fingerprinted path. The fingerprint code (`shared_input_paths`, `xtask/src/build_guests.rs`) collects, per shared crate {slicer-macros, slicer-sdk, slicer-ir, slicer-schema, slicer-core}: `src/` files, `Cargo.toml`, and `build.rs` — NOT `tests/**`. CLAUDE.md's prose trigger list is broader (`crates/slicer-macros/**`), but the code wins: the macros edit is confined to `tests/`, so no guest goes stale. If implementation drift ever pushes an edit into `crates/slicer-macros/src/**`, `crates/slicer-ir/src/**`, or any shared crate's `src/`/`Cargo.toml`/`build.rs`, the implementer MUST add `cargo xtask build-guests --check` to that step's verification and rebuild on `STALE:` before interpreting failures. AC-N2/AC-N3's probe file lives under `crates/slicer-ir/tests/` — scanned by check-literals, invisible to both cargo and the guest fingerprint.
- **Exit-code narrowing:** xtask's `main` narrows `i32` to `ExitCode`'s `u8`; the preflight abort returns 1 explicitly (never a propagated platform status), the same rule `ensure_pnp_cli_fresh_with`'s comment documents.
- **No committed red fixtures:** all violation fixtures are either in-memory/temp-dir (unit tests, cleaned up by the test) or shell-temp files created and removed inside a single AC command. Nothing red is ever committed.

## Code Change Surface

- Selected approach: reuse packet 194's scan machinery through a small testable helper, keep the CLI as the single source of scan semantics, and convert residue with the same base-plus-waiver patterns packets 195–198 established (`common::pipeline_config_base` precedent).
- Exact functions, traits, manifests, tests, and fixtures:
  - `xtask/src/test.rs`: new `fn check_literals_preflight(ws_root: &Path) -> i32` — invokes the check-literals enforce scan (no path filter) via the module's internal entry point and returns its exit code. `test_command` calls it at the top of "Step 1: freshness check"; on nonzero it prints `xtask test: check-literals preflight failed; fix violations or add reasoned waivers (docs/21_data_defaults_and_fixtures.md), then re-run.` to stderr (the scan itself has already printed the violation lines) and returns 1. Two new unit tests in the existing `#[cfg(test)]` mod: `preflight_blocks_on_violating_fixture_tree` (temp fake ws root: `crates/probe/src/lib.rs` with a plain `pub struct` of 5 named fields + `crates/probe/tests/probe.rs` with an exhaustive literal of it → nonzero) and `preflight_passes_on_clean_fixture_tree` (same tree, literal carries `..Default::default()` → 0). Both build under `std::env::temp_dir()` and remove their tree before asserting, mirroring `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail`.
  - `xtask/src/check_literals.rs`: ONLY if 194's scan entry is CLI-argv-shaped, add a thin `pub(crate) fn run_enforce(ws_root: &Path, filters: &[String]) -> i32` wrapper delegating to the existing internals; no scan-semantics change of any kind ([FWD] below).
  - `xtask/src/main.rs`: USAGE lines for `test [ARGS...]` updated to name the preflight ("Run the check-literals preflight, then `cargo xtask build-guests --check` (rebuild if stale), then `cargo test ARGS...` …"). No dispatch-arm change (194 owns the `check-literals` arm).
  - Residue conversions (rules: FRU over base, omit default-equal fields, reasoned waivers only where exhaustiveness is the intent, never change assertions):
    - `crates/slicer-model-io/tests/common/mod.rs` (new): `pub fn object_mesh_base() -> ObjectMesh` — id `"base-object"`, empty `IndexedTriangleSet`, identity `Transform3d` (identity, never the zero matrix), empty `ObjectConfig`, empty `modifier_volumes`, `paint_data: None`, `world_z_extent: None`; its single exhaustive literal carries `// exhaustive: file-shared FRU base — a new ObjectMesh field must be routed here deliberately`. The four residue test files add `mod common;` and rewrite each `ObjectMesh` literal as overrides + `..common::object_mesh_base()`.
    - `crates/slicer-model-io/src/loader.rs` `#[cfg(test)]` `make_object`: reasoned waiver on its `ObjectMesh` literal (src cfg-test cannot reach `tests/common`; the helper deliberately computes `world_z_extent`, so exhaustive routing is the intent).
    - `crates/slicer-helpers/tests/decimate_tdd.rs`, `tests/repair_tdd.rs`: each file's single `ObjectMesh` literal sits inside the file's lone constructor helper — waiver each with the file-local-FRU-base reason (two sites do not justify a new `tests/common` tree).
    - `crates/slicer-macros/tests/slicer_module_tdd.rs`: waivers on `Self { paths: Vec::new() }` (`impl InfillOutputBuilder`) and `Self { loops: Vec::new() }` (`impl PerimeterOutputBuilder`) — reason: 1-field local mocks that must keep the watched sdk builder names because the `#[slicer_module]` expansion resolves those identifiers in scope; renaming is impossible, FRU is meaningless on a 1-field mock.
  - `.github/workflows/ci.yml`: one new step appended to the `docs-guard` job — `- name: Struct-literal gate` / `run: cargo run -q -p xtask -- check-literals` (enforce mode: no `--report`, no path filter). Grounded 2026-08-07: the workflow declares four jobs (`fmt`, `docs-guard`, `clippy`, `test`); `docs-guard` already runs `cargo run -q -p xtask -- check-deviations --check`, so the invocation form and the "xtask guard lives here" precedent both exist, and the job needs no extra toolchain component (the check is parse-only — no guest WASMs, no release build). The `test` job deliberately keeps its direct `cargo test -p ...` calls; rerouting it through `cargo xtask test` (which would pick the preflight up transitively) is a larger change and is out of scope.
  - Docs edits: CLAUDE.md ×4 sections + `docs/21_data_defaults_and_fixtures.md`, per the Doc Impact Statement in `packet.spec.md`. The §Feature-gated repair is END-STATE: the working tree already carries the uncommitted gcode-sentence fix and the `crates/slicer-gcode/Cargo.toml` dep removal (observed 2026-08-07); the step verifies/finalizes wording rather than assuming the raw edit is still needed, and re-verifies the three remaining host-algos dependents against their `Cargo.toml`s before restating them.
- Rejected alternatives and reasons:
  - Shelling out to `cargo xtask check-literals` from `test_command` — a nested cargo invocation is slower, racy on the target dir, and untestable in-process; the in-process helper reuses the exact scan path.
  - Running the preflight after `build-guests --check` — wastes a possible multi-minute guest rebuild on a tree the syntax gate would have rejected in seconds.
  - Adding `impl Default for ObjectMesh` for the residue — touches `crates/slicer-ir/src` (guest-fingerprinted, all-guests rebuild), creates a production API whose derived `Transform3d::default()` zero matrix is a footgun, and violates the sweep-packet precedent that conversion never adds `Default`.
  - Renaming the slicer-macros mocks to `Mock*` — the macro-generated code requires the sdk builder names in scope; would break the macro tests outright.
  - A committed red fixture for the negative AC — forbidden; temp-file + fake-root fixtures cover it cleanly.

## Files in Scope (read + edit)

- `xtask/src/test.rs` - role: preflight wiring + unit tests; expected change: one helper, one call site in `test_command`, two tests.
- `xtask/src/check_literals.rs` - role: scan entry reuse; expected change: none, or one thin `run_enforce` wrapper.
- `xtask/src/main.rs` - role: USAGE text; expected change: `test` description lines.
- `.github/workflows/ci.yml` - role: CI enforcement; expected change: one step appended to the `docs-guard` job, no job added/renamed, no other job touched.
- `CLAUDE.md` - role: enforcement flip + stale-fact repair + sdk hazard; expected change: four section edits.
- `docs/21_data_defaults_and_fixtures.md` - role: rule page; expected change: gate-off phrasing → enforced-state wording.
- Residue files (bounded per-crate globs, justified by the residue sweep): `crates/slicer-model-io/tests/{common/mod.rs,model_writer_roundtrip_tdd.rs,threemf_writer_roundtrip_tdd.rs,world_z_below_floor_tdd.rs,world_z_canonical_surface_tdd.rs}`, `crates/slicer-model-io/src/loader.rs` (cfg-test mod only), `crates/slicer-helpers/tests/{decimate_tdd.rs,repair_tdd.rs}`, `crates/slicer-macros/tests/slicer_module_tdd.rs`.

## Read-Only Context

- `xtask/src/build_guests.rs` - `shared_input_paths` and `check_command` regions only - purpose: preflight insertion sits before `check_command`; fingerprint facts above.
- `crates/slicer-model-io/src/loader.rs` - `#[cfg(test)]` mod region only (locate via `rg -n 'fn make_object'`) - purpose: waiver placement; file is >3000 lines.
- `crates/slicer-ir/src/slice_ir.rs` - `ObjectMesh` definition region only - purpose: field list for base/override decisions (7 fields, verified 2026-08-07).
- `.cargo/config.toml` - `[alias]` block only - purpose: confirms `xtask = "run --quiet -p xtask --"`, so the CI step's `cargo run -q -p xtask --` form and the local `cargo xtask` form are equivalent.
- `docs/specs/struct-literal-churn-gate-plan.md` - whole file (short) - purpose: locked decisions 2 and 4.
- `docs/spec_packets/194-check-literals-gate/packet.spec.md` and `198-literal-sweep-sdk-modules/packet.spec.md` - purpose: consumed export contracts (CLI shape, sdk `--features test` facts) if the exports ledger needs re-confirmation.

## Out-of-Bounds Files

- Predecessor packets' `design.md` / `implementation-plan.md` (packets 194–198) - consume exports via their `packet.spec.md` or a SUMMARY dispatch only.
- `docs/specs/_OLD/default-builder-migration.md` - not needed; never load.
- `OrcaSlicerDocumented/...` - no parity surface; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- Packet 196/197/198 area test files and `crates/slicer-wasm-host/test-guests/**` - already swept / rule-exempt; do not edit or browse.
- `docs/07_implementation_status.md` - worker dispatch only.

## Expected Sub-Agent Dispatches

- Question: what is the callable scan entry point in `xtask/src/check_literals.rs` (name, signature, whether it takes ws_root + filters and returns an exit code)?; scope: `xtask/src/check_literals.rs`; return: `LOCATIONS`; purpose: Step 4 wiring.
- Question: run `cargo xtask check-literals --report` and list every reported file outside `crates/slicer-{ir,core,gcode,runtime,scheduler,wasm-host,sdk}`, `crates/pnp-cli`, and `modules/core-modules`; scope: workspace; return: `LOCATIONS` (<=20 entries); purpose: Step 1 residue re-derivation.
- Question: per-crate `cargo test -p <crate>` summary-multiset baselines and assert/testattr counts for slicer-model-io, slicer-helpers, slicer-macros; scope: the three crates; return: `FACT` (paths of written baseline files + green/red); purpose: Step 1 capture.
- Question: docs/07 crosswalk update for TASK-321 at close; scope: `docs/07_implementation_status.md`; return: `FACT`; purpose: Step 6.

## Data and Contract Notes

- IR/manifest contracts: untouched. `ObjectMesh` shape unchanged; only construction syntax at test sites.
- WIT boundary: untouched.
- Determinism/scheduler constraints: none; the preflight is a read-only scan.
- CLI contract consumed, not defined: exit 0/1/2, violation-line and summary formats are packet-194 exports; this packet's AC-N2 greps rely on the violation-line fragment `exhaustive literal of watched type`.

## Locked Assumptions and Invariants

- Preflight runs workspace-wide enforce, before the guest-freshness gate, exempt only in `--summary-from` mode; abort exit code is 1; failure line contains `check-literals preflight failed`.
- Residue conversion never adds `Default`, never changes an assertion, never renames the macros mocks.
- The probe path `crates/slicer-ir/tests/data/gate_probe_199_tmp.rs` is transient AC-command state only; it must never be committed (a leftover fails AC-1 loudly, which is the desired failure mode).
- `enforced since packet 199` is the canonical replacement anchor in both CLAUDE.md and docs/21 (greps depend on the exact phrase).

## Risks and Tradeoffs

- 194's scan API shape is unknown until implemented; mitigated by the [FWD] dispatch and the sanctioned thin-wrapper fallback.
- The grounded residue inventory disagrees with an earlier review note (which said model-io 1 file / helpers 0); this packet's own scan (2026-08-07) found more files, and the tool did not exist to arbitrate. Mitigated: Step 1's `--report` re-derivation is authoritative; the inventory here is a navigation hint, not a count contract, and no AC freezes a count.
- The CLAUDE.md §Feature-gated repair may be partially pre-applied (uncommitted working-tree fix observed 2026-08-07); steps verify end-state, so a prior commit of that fix converts the step into a no-op-plus-verify, not a conflict.
- Waivers on the helpers/loader constructor helpers trade strict FRU purity for zero structural churn; the reason strings make the intent auditable, and the waiver audit (Step 6) counts them.
- `cargo xtask test`'s auto-`--features slicer-core/host-algos` makes the AC-N3 false-negative path error out fast (xtask has no slicer-core dep) — bounded, but the error text must not contain the preflight failure line; the grep is specific enough (`check-literals preflight failed`).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4, wiring + unit tests)
- Highest-risk dispatch and required return format: the `--report` residue re-derivation; `LOCATIONS` capped at 20 entries (reject oversized replies and redispatch per-crate).

## Open Questions

- `[FWD]` Exact name/signature of 194's scan entry point for `check_literals_preflight` to call — resolve with the Step-4 LOCATIONS dispatch; add the `run_enforce` wrapper only if the entry is CLI-argv-shaped.
- `[FWD]` If Step 1's `--report` re-derivation surfaces residue beyond the grounded inventory (new files, or the slicer-macros impl-target semantics firing differently than modeled), convert it under the same rules within Steps 2–3's crate boundaries and record the delta in the close notes; if it surfaces residue in a packet-196/197/198 area (meaning a sweep regressed), stop and escalate rather than editing another packet's surface.
- `[FWD]` Whether the uncommitted working-tree fixes (CLAUDE.md gcode sentence, `crates/slicer-gcode/Cargo.toml` dep removal) land in the batch commit before this packet activates — Step 5 verifies end-state either way.
