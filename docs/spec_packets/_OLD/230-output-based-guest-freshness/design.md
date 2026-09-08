# Design: 230-output-based-guest-freshness

## Controlling Code Paths

- Primary code path: `check_command` in `xtask/src/build_guests.rs` — today `pub fn check_command(ws_root: &Path) -> i32`; it calls `compute_shared_freshness`, `discover_guests`, then `is_stale` per guest and prints `STALE: {crate_name}`.
- `is_stale` in the same file — unions `compute_guest_freshness`'s newest-mtime comparison with `metadata_matches` against the fingerprint sidecar at `target/guest-fingerprints/{crate_name}.fingerprint` (path from `fingerprint_metadata_path`).
- `build_one` in the same file — runs `build_one_inner`, resolves the WIT dir via `crate::wit_verify::module_stage_wit_dir`, loads canonical, verifies, and on mismatch calls `force_rebuild_wit_bindings` and retries once before returning `BuildError::StaleEmbeddedWorld`. **The fingerprint sidecar is written at the end of `build_one_inner`, i.e. before any of this verification runs** — the defect this packet fixes.
- `GuestSpec` in the same file — 7 fields: `crate_name`, `lib_name`, `manifest_path`, `guest_dir`, `artifact_path`, `tree: GuestTree`, `stage_id: Option<String>`. `stage_id` is populated by `parse_stage_id_from_module_manifest` during `discover_guests` and is `None` for test guests.
- Call site 1: the `Some("--check")` arm of the `build-guests` match in `xtask/src/main.rs`, currently `std::process::exit(build_guests::check_command(&ws))`.
- Call site 2: `test_command` in `xtask/src/test.rs`, which binds `check_code` and on non-zero calls `build_guests::build_command(ws_root)` — rebuilding all 42 guests.
- Injectable-seam precedent in the same file: `ensure_pnp_cli_fresh` delegates to `ensure_pnp_cli_fresh_with(ws_root, run_rebuild)` where `run_rebuild: impl FnOnce(&Path) -> io::Result<ExitStatus>`; the existing `#[cfg(test)] mod tests` in `xtask/src/test.rs` uses it.
- Stage vocabulary: `slicer_schema::STAGES` — 16 rows, one of which (`PrePass::PaintSegmentation`) carries `wit_package: ""`, `wit_dir: ""`, `wit_export: ""`. `StageSpec` fields are `method`, `stage_id`, `wit_export`, `tier_id`, `trait_name`, `wit_dir`, `wit_package`, `wit_interface`, `wit_world`.
- Consumed from packet 229 (`xtask/src/wit_verify.rs`): `WorldModel`, `PackageModel`, `InterfaceModel`, `StageExpectation`, `stage_expectation`, `Drift`, `DriftKind`, `SHARED_PACKAGES`, `ROOT_COMPONENT_PACKAGE`, `canonical_world_model`, `embedded_world_model`, `compare_worlds`, `verify_embedded_world`, `embedded_wit_text`, and `VerifyError::{Decode, Parse, CanonicalEmpty, CanonicalUnreadable}`.
- Neighboring tests/fixtures: the 4 existing `#[test]`s in `build_guests`' `mod tests` (`fingerprint_is_deterministic_and_content_sensitive`, `missing_fingerprint_metadata_is_stale`, `stage_wit_dir_is_charged_only_to_matching_guest`, `stage_wit_unknown_stage_is_conservative`) and their `TempDir` helper; the existing `mod tests` in `xtask/src/test.rs`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- This packet changes the staleness gate itself, so the usual "trust `--check`" reflex is suspended for its own steps: an unexpected `STALE:` report may be a bug in this packet's logic rather than a genuine stale guest. Confirm by decoding the named artifact before rebuilding.
- **The fingerprint version prefix moves `v1-` → `v2-`.** That is a public-ish version constant for the sidecar format: every guest invalidates exactly once, forcing one full rebuild. The step that lands the prefix owns that rebuild and owns updating any test that asserts on the `v1-` literal — do not defer it to a later `cargo check`.
- **The check must not be self-referential (R5-4).** An artifact declaring its own stage and then being judged only against that declaration compares equal by construction. The core guest's manifest `[stage] id` is the independent expectation that breaks the circle. `module_stage_wit_dir`'s own doc comment records the packet-164 regression that arose the last time manifest-derived resolution was lost.
- **Freshness is asserted by exit code, never by grepping for `STALE:` (R5-3).** Therefore `StaleReason`'s and `Drift`'s `Display` output must never contain the substring `STALE:`, and the infrastructure code must be distinct from both `0` and `1`.
- **No fail-open remains.** Every "cannot tell" outcome is either staleness (artifact-side: undecodable, unresolvable, missing) or an infrastructure error (tooling-side: `wasm-tools` absent, canonical unusable). Nothing maps to fresh.
- `GuestSpec` is a `pub` struct with 7 named fields, so the struct-literal churn gate applies to every new test literal: use `..` rest or an `// exhaustive: <reason>` waiver, per `docs/21_data_defaults_and_fixtures.md`; `cargo xtask check-literals` enforces it.
- `xtask` has no `[lib]` and no `xtask/tests/` directory; all tests are inline `#[cfg(test)] mod tests`, so AC commands are `cargo test -p xtask <module>::tests::<name> -- --exact`.

## Code Change Surface

### Selected approach

Move the freshness decision from "inputs changed" to "output disagrees", per guest, with the manifest stage retained as an independent expectation.

Net-new in `xtask/src/wit_verify.rs`:

- `pub enum StageResolutionError { NoStagePackage, Ambiguous(Vec<String>), UnknownPackage(String) }` with a `Display` that never emits `STALE:`.
- `pub fn resolve_stage_from_world(world: &WorldModel) -> Result<StageExpectation, StageResolutionError>` — collects embedded package names, removes `ROOT_COMPONENT_PACKAGE` and `SHARED_PACKAGES`, requires exactly one remainder, strips its version suffix, and matches it against `STAGES` rows **whose `wit_package` is non-empty**, returning `stage_expectation(row.stage_id)`. `stage_expectation` returns `Option<StageExpectation>`; a `None` (which can only mean the matched row has no WIT package, and is therefore unreachable once empty-`wit_package` rows are filtered out) maps to `StageResolutionError::UnknownPackage` rather than being unwrapped.
- `module_stage_wit_dir` and its test `core_modules_resolve_their_stage_wit_dir` are **deleted**; the WIT dir now comes from the resolved `StageExpectation.wit_dir`.

Net-new in `xtask/src/build_guests.rs`:

- `pub const EXIT_FRESH: i32 = 0; pub const EXIT_STALE: i32 = 1; pub const EXIT_INFRA_ERROR: i32 = 3;`
- `pub const FINGERPRINT_VERSION: &str = "v2";` — the emitted string becomes `v2-{:016x}{:016x}`.
- `pub struct CheckOutcome { pub stale: Vec<GuestSpec>, pub code: i32 }`.
- `pub enum StaleReason { ArtifactMissing, FingerprintMismatch, Undecodable(String), StageUnresolved(StageResolutionError), StageMismatch { expected: String, resolved: String }, EmbeddedWorldDrift(Vec<Drift>) }` — `Display` renders one line per reason, never containing `STALE:`.
- `pub fn stale_reason(spec: &GuestSpec, ws_root: &Path, ctx: &CheckContext) -> Option<StaleReason>` — the primary predicate.
- `pub fn is_stale(spec: &GuestSpec, ws_root: &Path, ctx: &CheckContext) -> bool` — **signature change, resolved explicitly.** The live signature takes `shared: &FreshnessSnapshot`, which cannot supply the `canonical: WorldModel` and `wasm_tools_version` that `stale_reason` needs, so a `stale_reason(..).is_some()` wrapper is *not* implementable behind the old signature. The third parameter therefore becomes `&CheckContext` (which owns the `FreshnessSnapshot` as its `shared` field), and the body is exactly `stale_reason(spec, ws_root, ctx).is_some()`. Existing callers do **not** keep compiling; the full caller set is small and closed, and is budgeted in Step 2's edit list: the loop inside `check_command` and the two `assert!(is_stale(&spec, &temp.0, &shared))` / `assert!(!is_stale(...))` call sites in `missing_fingerprint_metadata_is_stale`, all three in `xtask/src/build_guests.rs`. `rg -n 'is_stale' --glob '*.rs'` returns no other call site; `crates/pnp-cli-locator`'s `staleness_reason` only *mentions* `is_stale` in a doc comment (it is a documented mirror, not a caller) and is packet 231's surface, so it neither breaks nor is edited here. AC-18 pins the post-change shape.
- `pub struct CheckContext { pub shared: FreshnessSnapshot, pub canonical: WorldModel }` — built once per `check_command` invocation so canonical WIT is parsed once, not 42 times. The `wasm-tools` version string is checked once up front (missing tool ⇒ `EXIT_INFRA_ERROR`) and folded into the fingerprint via `compute_guest_freshness`; it is not stored on the context.
- `pub fn check_command(ws_root: &Path) -> CheckOutcome` — signature change. Its testable core is the private `check_command_with(ws_root, wasm_tools, canonical, guests, out)`, which takes the wasm-tools result, canonical result, guest list and output writer as injected parameters; production `check_command` gathers them from the real tree and writes to stdout.
- `pub fn build_stale_command(ws_root: &Path, stale: &[GuestSpec]) -> i32`.
- `pub fn wasm_tools_version() -> Result<String, BuildError>` and `pub fn rustc_version_verbose() -> Result<String, BuildError>` — the two tool-version strings folded into the fingerprint.
- `fingerprint_entries`' input gains the workspace-root `Cargo.toml`, the guest's own `Cargo.lock` (`spec.guest_dir.join("Cargo.lock")`), and the two version strings as synthetic entries.

Changed behaviour in `build_one`:

1. Remove the sidecar (`fingerprint_metadata_path`) at build start.
2. `build_one_inner` no longer writes the sidecar; its write moves to the end of `build_one`.
3. Resolve the stage from the freshly built artifact via `embedded_world_model` + `resolve_stage_from_world`; for a core guest, cross-check against `spec.stage_id`.
4. Load canonical via `canonical_world_model(ws_root, Some(&expect))?`, mapping `VerifyError::CanonicalEmpty` / `CanonicalUnreadable` to the new infrastructure `BuildError`. **The `if canonical.is_empty() { return Ok(()) }` guard is deleted.**
5. On drift: `force_rebuild_wit_bindings`, rebuild once, re-verify (unchanged structure).
6. On persistent failure (`EmbeddedWorldUndecodable`, `StaleEmbeddedWorld`): ensure the sidecar is absent, then return the error.
7. Only on success: write the `v2-` fingerprint.

Call-site migrations:

- `xtask/src/main.rs`, `Some("--check")` arm → `std::process::exit(build_guests::check_command(&ws).code)`.
- `xtask/src/test.rs`, `test_command` → bind `CheckOutcome`; on `code == EXIT_INFRA_ERROR` print the detail and return that code **without rebuilding**; on `code == EXIT_STALE` call `build_stale_command(ws_root, &outcome.stale)` and abort the suite if it returns non-zero. The rebuild call is routed through a new `test_command_with`-style seam mirroring `ensure_pnp_cli_fresh_with`, so AC-9, AC-10 and AC-N4 assert without spawning real cargo builds.

### Rejected alternatives

- **Resolve the stage only from the artifact (the unamended plan).** Rejected by R5-4: it makes the check self-referential for the one failure mode — a guest exporting the wrong stage — that manifest-derived resolution used to catch.
- **Keep `check_command -> i32` and re-derive the stale list inside `test_command`.** Rejected: it decodes all 42 artifacts twice and lets the two paths disagree.
- **Signal the `wasm-tools`-missing case as staleness.** Rejected by C9: it would trigger a mass rebuild that cannot possibly succeed, since the build path needs the same tool.
- **Keep writing the fingerprint in `build_one_inner` and delete it afterwards on failure.** Rejected: a process killed between the write and the verification leaves a fingerprint claiming a freshness that was never established. Write-last is the only ordering that cannot lie.
- **Parse each guest's canonical WIT per guest.** Rejected: `CheckContext` parses canonical once per invocation; 42 re-parses would dominate the measured `--check` time this packet must report.

## Files in Scope (read + edit)

Four files. The two beyond the recommended three are the mandatory call sites — leaving either unmigrated does not compile.

- `xtask/src/build_guests.rs` — role: freshness gate, fingerprint and build orchestration; expected change: `CheckOutcome`, `StaleReason`, `stale_reason`, `CheckContext`, `build_stale_command`, exit-code consts, `v2-` fingerprint content and lifecycle, `build_one` restructure.
- `xtask/src/wit_verify.rs` — role: verifier; expected change: add `resolve_stage_from_world` and `StageResolutionError`, delete `module_stage_wit_dir` and its test.
- `xtask/src/test.rs` — role: gated test entry point; expected change: `test_command` consumes `CheckOutcome`, rebuilds only stale specs through a testable seam, aborts on the infrastructure code.
- `xtask/src/main.rs` — role: CLI dispatch; expected change: one line, `check_command(&ws).code`.

## Read-Only Context

- `crates/slicer-schema/src/lib.rs` — over 600 lines; read only the `STAGES` rows' `stage_id` / `wit_package` / `wit_dir` fields and `stage_by_id`. Purpose: the resolution table and the one empty-`wit_package` row.
- `xtask/src/dist.rs` — read only its `build_guests::build_command(ws_root)` call. Purpose: confirm `build_command`'s signature must not change. Do not edit.
- `docs/03_wit_and_manifest.md` — section "Build & Freshness Contract (Normative)" and its staleness-guard table row only.
- `docs/21_data_defaults_and_fixtures.md` — the struct-literal waiver format only.
- `docs/spec_packets/229-wit-verify-declaration-model/packet.spec.md` §Prerequisites (the 16-item consumed-symbol list) and `docs/spec_packets/229-wit-verify-declaration-model/design.md` §Code Change Surface (their signatures) — those two sections only. Do not open 229's `requirements.md` or `implementation-plan.md`. Once 229 is implemented, prefer a `FACT` dispatch against the real `xtask/src/wit_verify.rs`, which is then the ground truth.

## Out-of-Bounds Files

- `xtask/src/dist.rs` — user-ruled untouched.
- `compute_shared_freshness`, `stage_wit_snapshot`, `shared_input_paths`, `guest_input_paths` and `ensure_pnp_cli_fresh_with`'s mtime model — packet 231's surface; read them, do not change their behaviour here beyond what `CheckContext` requires.
- `crates/pnp-cli-locator/**` — packet 231's surface.
- `CLAUDE.md`, `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, `docs/adr/**`, `.github/workflows/ci.yml`, `.claude/skills/**` — packet 232's surface.
- Every other packet directory under `docs/spec_packets/**` — never edit.
- `target/`, `Cargo.lock`, all `.wasm` artifacts, generated code, vendored dependencies — never load.
- `OrcaSlicerDocumented/` — not applicable; no parity surface.

## Expected Sub-Agent Dispatches

- Question: "In `slicer_schema::STAGES`, list every row's `stage_id` and `wit_package`, and confirm exactly one row has an empty `wit_package`."; scope: `crates/slicer-schema/src/lib.rs`; return: `FACT` (count + the empty row's `stage_id`); purpose: Step 1 resolution table.
- Question: "For `modules/core-modules/wipe-tower/wipe-tower.wasm` and one test-guest artifact, decode with `wasm-tools component wit` and return only the `package` declaration lines and the export line."; scope: those two artifacts; return: `FACT` (<=10 lines); purpose: Step 1 resolution fixtures, without loading the decodes.
- Question: "Every call site of `check_command`, `build_command`, `is_stale` and `fingerprint_metadata_path` in the workspace."; scope: `xtask/src/*.rs`; return: `LOCATIONS` (<=20 entries); purpose: Step 3 migration completeness.
- Question: "Does `cargo clippy --workspace --all-targets -- -D warnings` pass? If not, the first 20 lines of the first error."; scope: workspace; return: `FACT pass/fail` + bounded `SNIPPETS`; purpose: every step's gate.
- Question: "Timed `cargo xtask build-guests --check` with all guests fresh: report the real/user/sys line only."; scope: workspace; return: `FACT` (<=4 lines); purpose: Step 6's AC-16 measurement, before and after.

## Data and Contract Notes

- IR/manifest contracts: module manifests are read only for `[stage] id`, through the surviving `parse_stage_id_from_module_manifest`. No manifest key is added, renamed or removed. Manifest section headers and runtime key strings remain snake_case.
- WIT boundary: no canonical `.wit` file is edited. The stage↔package mapping is read from `slicer_schema::STAGES`, which stays the single source of truth (ADR-0006's stage table, ADR-0045's per-stage versioned packages). Resolution *adds* a consumer of that table; it does not create a parallel one.
- Determinism/scheduler constraints: `check_command` iterates `discover_guests`' order and prints one marker line per stale guest, so output is stable and diffable run to run. `CheckContext` is built once so all 42 comparisons see identical canonical input.
- Sidecar format: `target/guest-fingerprints/{crate_name}.fingerprint`, content `v2-{:016x}{:016x}`. The `v1-` → `v2-` change is deliberately not backward compatible; a `v1-` sidecar reads as a mismatch and triggers exactly one rebuild per guest.

## Locked Assumptions and Invariants

- For a core guest, `GuestSpec.stage_id` is authoritative as the *expectation*; the artifact's resolved stage must equal it. This asymmetry is the whole anti-self-reference argument and must not be "simplified" later.
- Exit codes are locked: `0` fresh, `1` stale, `EXIT_INFRA_ERROR` infrastructure. Downstream automation (packet 232's snippet rewrite, CI) depends on those three being distinct.
- No `Display` impl on `StaleReason`, `Drift` or `StageResolutionError` may contain the substring `STALE:`.
- The fingerprint is written only after final verification, and its absence is always safe (it means "rebuild"), while its presence is a positive claim that verification passed.
- `build_command` keeps its current signature and full-rebuild behaviour for `xtask/src/dist.rs` and CI.

## Risks and Tradeoffs

- **Decode cost per `--check` is unmeasured on this machine.** AC-16 forces a measurement rather than an assumption. If the measured "after" figure is materially worse than "before", report it — do not bury it, and do not quote the plan's earlier unmeasured `~38ms`/`~2s` figures.
- **First run after the `v2-` prefix lands marks all 42 guests stale.** Expected and one-time; must not be misread as a regression in the new comparison.
- **A pre-existing wrong-stage guest would now surface as `StageMismatch`.** That is the intended catch (R5-4), but it may appear as a surprising failure on an artifact that has been "working". Investigate the artifact; do not relax the cross-check.
- **`check_command` now depends on `wasm-tools` for a *check*, not just a build.** `.github/workflows/ci.yml` installs it via `taiki-e/install-action` in both the `test` and `dist-editions` jobs; the `test` job is the one that runs `cargo xtask build-guests --check`, and `dist-editions` runs `cargo xtask dist` (which reaches `build_command`). Both therefore have the tool available, so CI is safe; a developer without it gets `EXIT_INFRA_ERROR` with an actionable message rather than a silent pass.
- **`test_command`'s new seam adds an indirection** to a hot path in developer workflow. Accepted: without it, AC-9, AC-10 and AC-N4 can only be verified by spawning real builds, which is not delegation-friendly.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3, `check_command`/`CheckContext`/`stale_reason` and both call sites)
- Highest-risk dispatch and required return format: the artifact-decode dispatches — `FACT`, at most 10 lines, package declarations and export line only. A full `wasm-tools component wit` dump would blow the step budget on its own.

## Open Questions

- `[FWD]` Packet 229 is `status: draft` at authoring time. Every symbol this packet consumes from `xtask/src/wit_verify.rs` is a FORWARD-DEP on 229's planned surface with matching names and shapes. Before starting, confirm 229 is `implemented` and re-verify each consumed symbol's actual signature against the tree; reconcile any rename in this packet's `design.md` before Step 1.
- `[FWD]` `xtask` tests still do not run in CI after this packet (`.github/workflows/ci.yml`'s `test` job runs `-p slicer-runtime`, `-p pnp-cli`, `-p slicer-helpers` only). Packet 232 adds `cargo test -p xtask`. Until then every AC here must be run locally; do not treat a green CI as evidence for any AC in this packet.
- `[FWD]` `ensure_pnp_cli_fresh_with` in `xtask/src/test.rs` still calls `compute_shared_freshness`, which packet 231 deletes. This packet leaves that call untouched; the implementer must not opportunistically clean it up, or packet 231's step list desynchronizes.
