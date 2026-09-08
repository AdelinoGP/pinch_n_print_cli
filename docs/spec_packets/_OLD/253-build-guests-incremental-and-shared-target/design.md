# Design: 253-build-guests-incremental-and-shared-target

## Controlling Code Paths

- Primary code path: `build_command`, `build_stale_command`, `check_command`, `check_command_with`, `build_one`, `build_one_inner`, `force_rebuild_wit_bindings`, `compute_guest_freshness`, `stale_reason`, `CheckContext`, `ClosureCache`, `FINGERPRINT_VERSION`, `EXIT_FRESH` / `EXIT_STALE` / `EXIT_INFRA_ERROR`, `rustc_version_verbose`, `wasm_tools_version`, and `ensure_wasm_tools_available` — all in `xtask/src/build_guests.rs`.
- Secondary code paths: the `build-guests` match arm in `xtask/src/main.rs`; `DistArgs`, `parse_dist_args`, and the guest-build call in `xtask/src/dist.rs`; `handle_guest_freshness_with` in `xtask/src/test.rs`; `canonical_world_model` in `xtask/src/wit_verify.rs`.
- Neighboring tests/fixtures: the `#[cfg(test)] mod tests` block at the end of `xtask/src/build_guests.rs`, which owns the fingerprint, closure, and staleness unit tests, including `fingerprint_is_written_only_after_final_verification`, `v2_fingerprint_covers_workspace_manifest_lockfile_rustc_and_wasm_tools`, `all_fresh_yields_empty_stale_list_and_zero_code`, `missing_wasm_tools_is_infrastructure_error_not_staleness`, and `unusable_canonical_set_is_infrastructure_error_not_fresh`. The dist-arg tests live in `xtask/src/dist.rs`; the freshness-gate tests live in `xtask/src/test.rs`.
- OrcaSlicer comparison: not applicable. This packet touches build tooling only; no geometry, no parity surface.

## Architecture Constraints

- The artifact-verified freshness property defined in `CONTEXT.md` is what makes Phase A safe. Rebuilding only stale guests is correct because a fresh verdict is established by decoding the artifact and comparing its embedded WIT world against canonical, not by assuming a timestamp. Any change that weakens `stale_reason` to make the default path faster is out of bounds; the fast path is achieved by not rebuilding fresh guests, never by checking less.
- The three exit codes are a public contract: `EXIT_FRESH` is 0, `EXIT_STALE` is 1, `EXIT_INFRA_ERROR` is 3. Lock divergence is a form of staleness and must map to `EXIT_STALE`, never to `EXIT_INFRA_ERROR`, because `EXIT_INFRA_ERROR` means the checker could not form an opinion. `handle_guest_freshness_with` in `xtask/src/test.rs` branches on both codes and must keep working unchanged.
- Guests are separate Cargo workspaces by design and must stay that way. Every `[workspace]` sentinel is retained. Sharing `CARGO_TARGET_DIR` across separate workspaces is supported by Cargo and is already done for test-guests today; the change generalises an existing pattern rather than introducing a new one.
- Cargo keys build artifacts by package, version, feature set, and profile. Target sharing therefore recovers compile time only across guests whose locks agree. The `arachne-perimeters` `default-features = false` dependency on `slicer-core` is a legitimate second feature variant and will correctly produce a second artifact; it is not lock drift and must not be normalised.
- `.gitignore` already carries `**/target/`, and `Swatinem/rust-cache@v2` in `.github/workflows/ci.yml` caches the workspace `./target`. Placing the shared guest target at `<ws_root>/target/guests` therefore inherits both without a new gitignore rule and without a new CI action. Any other location forfeits one or both.
- ADR-0014 (`docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md`) is the only ADR governing this area. Its normative content locks the per-guest `[workspace]` sentinels, forbids pulling `cargo_metadata` or other heavy dependencies into xtask, and requires shape predicates to be added rather than relaxed. This packet conforms rather than amends: discovery is untouched, the sentinels are retained, and the lock analyser parses `[[package]]` name and version pairs with the existing TOML handling rather than adding a dependency. No deviation row is needed, and none is authored.
- `check_command_with` is private (`fn`, not `pub fn`), and so is `handle_guest_freshness_with`. The new testable cores follow that precedent: production wrappers are `pub`, injected cores are private and exercised from the crate's own `#[cfg(test)] mod tests`. Do not widen visibility to make a test reachable.
- The fingerprint input set is a versioned contract. `FINGERPRINT_VERSION` exists precisely so a change to the input set invalidates old sidecars instead of silently comparing incomparable hashes. Adding the build profile to the input set (Phase D) therefore requires the bump; adding it without the bump is a correctness bug, not a shortcut.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

### Selected approach

Follow the injection pattern the file already uses. `check_command` delegates to `check_command_with`, which takes its wasm-tools result, canonical model, guest list, and output writer as parameters so the decision logic is unit-testable without a real tree. Every new behaviour in this packet is added the same way: a thin production wrapper that gathers real inputs, plus a testable core taking those inputs as parameters. This keeps the ACs verifiable by fast unit tests rather than by multi-minute end-to-end builds, and it matches `handle_guest_freshness_with` in `xtask/src/test.rs`.

### Exact functions, traits, manifests, tests, and fixtures

Phase A:

- New `enum BuildGuestsFlag { Default, Force, Check, List, SyncLocks, Unknown(String) }` plus `parse_build_guests_flag(args: &[String]) -> BuildGuestsFlag` in `xtask/src/build_guests.rs`. The `build-guests` arm in `xtask/src/main.rs` becomes a match on this enum; `Unknown` keeps today's usage print and exit code 2.
- New `build_command_with(check, rebuild_stale, rebuild_all, force) -> i32` in `xtask/src/build_guests.rs`, mirroring `handle_guest_freshness_with`'s closure-injection shape. Force short-circuits to `rebuild_all` without calling `check`. Non-force calls `check`; on `EXIT_INFRA_ERROR` it returns 3 immediately; otherwise it calls `rebuild_stale` with `outcome.stale`.
- `build_command` becomes the production wrapper binding real `check_command` and `build_stale_command`, plus a new `build_all_command` holding today's unconditional loop.
- `DistArgs` in `xtask/src/dist.rs` gains `force_guests: bool`; `parse_dist_args` gains the `--force-guests` arm; the `build_guests::build_command` call site passes the flag through.

Phase B:

- New `guest_target_dir(ws_root: &Path) -> PathBuf` returning `<ws_root>/target/guests`, and `guest_profile_dir(profile) -> &'static str` mapping the cargo profile to its output directory name (`release` maps to `release`, `dev` maps to `debug`).
- `build_one_inner` sets `CARGO_TARGET_DIR` unconditionally and derives `intermediate_base` from `guest_target_dir(ws_root).join("wasm32-unknown-unknown").join(guest_profile_dir(..))` for both `GuestTree` variants. The `GuestTree::TestGuest` special case and the `crates/slicer-wasm-host/test-guests/target` literal are deleted.
- `force_rebuild_wit_bindings` gains a `ws_root` parameter and sets `CARGO_TARGET_DIR` on both `cargo clean -p` commands. This repairs a latent defect: today it cleans `<guest_dir>/target` for test-guests, which the build never reads, so the stale-WIT recovery path is already inert for that tree.
- New `parse_lock_packages(text: &str) -> Vec<(String, String)>` and `lock_divergences(locks: &[(String, Vec<(String, String)>)]) -> Vec<LockDivergence>` in `xtask/src/build_guests.rs`, where `LockDivergence` names the crate and its distinct versions. The analyser takes parsed input so its tests use synthetic fixtures, never real lockfiles.
- New `sync_locks_command(ws_root) -> i32` that removes and regenerates each guest lock via `cargo generate-lockfile` in each guest directory.
- `check_command` gains the divergence check ahead of the per-guest loop, printing one line per diverging crate and naming `--sync-locks`, and folding the result into `EXIT_STALE`.

Phase C:

- New `VersionProbes` struct holding the memoized `rustc -vV` and `wasm-tools --version` strings, constructed once per invocation and threaded through `CheckContext` and the build path. `compute_guest_freshness` takes it as a parameter instead of calling `rustc_version_verbose` and `wasm_tools_version` itself.
- Canonical model memoization: `build_stale_command` loads the canonical model once and passes it into `build_one`, which stops calling `canonical_world_model` per guest. `build_one`'s two existing canonical loads (the initial verification and the post-forced-rebuild re-verification) both read the memoized value.
- `stale_reason` loses the `#[cfg(not(test))]` `verify_embedded_world` block. Its error mapping moves onto the single `compare_worlds` path: `VerifyError::Decode` and `VerifyError::Parse` map to `StaleReason::Undecodable`; `VerifyError::CanonicalEmpty` and `VerifyError::CanonicalUnreadable` map to the synthetic `Drift` with `DriftKind::MissingStagePackage`. Because the canonical model is now loaded once by the caller, those canonical error variants are surfaced by the loader rather than re-derived per guest.

Phase D, conditional:

- New `resolve_guest_profile(env_value: Option<&str>) -> Result<GuestProfile, String>` where `GuestProfile` is `Release` or `Dev`, defaulting to `Release` on `None` and erroring on any other string.
- `compute_guest_freshness` appends a synthetic entry with path `synthetic:guest-profile` and the resolved profile name as bytes.
- `FINGERPRINT_VERSION` changes from `"v2"` to `"v3"`.
- `xtask/src/dist.rs` resolves the guest profile to `Release` unconditionally.
- `xtask/src/test.rs` honours the env var when it drives the freshness gate.

### Rejected alternatives and reasons

- **Keep `dist` on an unconditional full rebuild.** Rejected: freshness is artifact-verified, so a fresh verdict is proof, not optimism. A distribution build gains nothing from redoing proven-correct work, and `--force-guests` covers the paranoid case.
- **Fall back to a full rebuild when the pre-build check hits an infrastructure error.** Rejected: it hides a missing `wasm-tools` or an unreadable canonical WIT behind a two-minute build that then fails anyway, and it contradicts the exit-code contract that `EXIT_INFRA_ERROR` means no opinion was formed.
- **Share the target directory without converging the locks.** Rejected: measured survey found only 15 of 23 core locks agree, so the shared directory would still recompile the shared dependency stack several times over. The convergence check is what keeps the win from silently eroding.
- **Put the shared target outside the workspace, for example at a user-level cache path.** Rejected: forfeits the existing `**/target/` gitignore rule and the existing CI cache, and adds machine-specific configuration.
- **Adopt sccache now.** Rejected for this packet and recorded as deferred. Its marginal value is unmeasured until Phase B brings guest dependencies under the CI cache, and it conflicts with an incremental dev profile locally.
- **Ship the dev profile unconditionally.** Rejected: unoptimized wasm for `slicer-core` geometry may cost more in host test runtime than it saves in build time, and nobody has measured it. Step 8 measures before Step 9 ships.
- **Normalise the `arachne-perimeters` `default-features = false` slicer-core dependency.** Rejected: it is a deliberate feature variant, not drift, and changing it would alter a guest's compiled behaviour, which is outside this packet's scope.

## Files in Scope (read + edit)

- `xtask/src/build_guests.rs` — role: owns every function this packet changes plus the unit suite that is its oracle; expected change: flag enum and parser, testable build core, shared target helper, lock analyser and sync command, version-probe and canonical memoization, duplicate-decode removal, and conditionally the profile resolver and fingerprint bump.
- `xtask/src/main.rs` — role: the `build-guests` dispatch arm; expected change: match on the new flag enum, adding `--force` and `--sync-locks` and preserving the exit-2 unknown-flag path.
- `xtask/src/dist.rs` — role: the second caller of the guest build; expected change: `force_guests` field, `--force-guests` parsing, freshness-aware call, and conditionally the forced release profile.

Two extras are justified and each is a small, localized edit:

- `xtask/src/wit_verify.rs` — role: holds `canonical_world_model`; expected change: expose a form callers can memoize. The function already ignores its `stage` argument, so no parsing logic changes.
- `xtask/src/test.rs` — role: holds `handle_guest_freshness_with`; expected change: none in Phases A to C beyond confirming the exit-code branches still hold; conditionally honours `PNP_GUEST_PROFILE` in Phase D.

Consider splitting only if Phase D ships and Step 9 cannot stay within its budget; the plan already isolates Phase D behind a measurement gate.

## Read-Only Context

- `CLAUDE.md` — §"Guest WASM Staleness" only; purpose: the exact sentence asserting the retired test-guest target path, and the `--check` exit-code wording that must stay accurate.
- `CONTEXT.md` — the "Artifact-verified freshness" term block only; purpose: the definition Phase A relies on.
- `.github/workflows/ci.yml` — the `test` job only; purpose: confirm the `build-guests` then `--check` ordering and the `Swatinem/rust-cache@v2` step.
- `Cargo.toml` at the workspace root — the `[profile.release]` block only; purpose: confirm it does not reach guests, which are separate workspaces. Do not edit it.
- `modules/core-modules/arachne-perimeters/Cargo.toml` and its `wit-guest/Cargo.toml` — purpose: the canonical shape of a core guest and its `[workspace]` sentinel. Read one guest, not all of them.

## Out-of-Bounds Files

- `target/` in any form, including `target/guests` and `target/guest-fingerprints` — never load.
- Any `Cargo.lock`. The lock analyser is tested against synthetic fixtures; the real regeneration is performed by a command, not by hand-editing. Never open one.
- `OrcaSlicerDocumented/...` — not applicable to this packet; never load.
- Guest sources under `modules/core-modules/*/src` and `crates/slicer-wasm-host/test-guests/*/src` — this packet changes how they are built, never what they contain.
- `crates/slicer-schema/wit/**` — the WIT contract is untouched.
- `crates/pnp-cli/src/module_new.rs` — its scaffolding text describes an external module's own target directory, not the in-tree guest set.
- Every other packet directory under `docs/spec_packets/`.

## Expected Sub-Agent Dispatches

- Question: what does `docs/03_wit_and_manifest.md` currently state about `cargo xtask build-guests` and its `--check` variant, and which rows describe the default rebuild behaviour?; scope: `docs/03_wit_and_manifest.md`; return: `SUMMARY` under 200 words; purpose: Step 10 doc edit.
- Question: does `docs/05_module_sdk.md` quote the per-tree guest target path anywhere in its workspace-contributor guidance?; scope: `docs/05_module_sdk.md`; return: `LOCATIONS` at most 10 entries; purpose: Step 10 doc edit scoping.
- Question: run the named cargo command and report only pass/fail plus the failing assertion; scope: the single command given; return: `FACT` at most 5 lines; purpose: every AC verification in every step.
- Question: run the named build twice and report only the wall-clock line; scope: the single command given; return: `FACT` at most 3 lines; purpose: Step 5 and Step 8 measurements. Never return the build log.
- Question: which unit tests in `xtask/src/build_guests.rs` assert on the literal `v2-` fingerprint prefix or carry `v2` in their name?; scope: `xtask/src/build_guests.rs`; return: `LOCATIONS` at most 10 entries; purpose: Step 9 blast radius, re-derived at the moment of the bump.
- Question: what is the next free `TASK-` number in `docs/07_implementation_status.md`?; scope: that file; return: `FACT` one line; purpose: Step 10 registration. Must be re-derived at that moment, never quoted from this packet.

## Data and Contract Notes

- IR/manifest contracts: unchanged. No IR schema, no module manifest, no config key is touched.
- WIT boundary: unchanged. No file under `crates/slicer-schema/wit` is edited, and the canonical WIT parsing logic in `canonical_world_model` is memoized, not modified.
- Determinism/scheduler constraints: not applicable to the slicing pipeline. Within the build tool, the lock-divergence report must be deterministically ordered (sort by crate name, then by version string) so its output is stable across runs and diffable in CI.
- Exit-code contract: `EXIT_FRESH` 0, `EXIT_STALE` 1, `EXIT_INFRA_ERROR` 3, and the unknown-flag exit 2 from `xtask/src/main.rs`. All four survive this packet unchanged in meaning.
- Fingerprint contract: the sidecar path scheme under `target/guest-fingerprints` and the `<version>-<hash>` content format are unchanged; only the version literal and the input set change, and only if Phase D ships.

## Locked Assumptions and Invariants

- The shared guest target directory is locked to `<ws_root>/target/guests`. Moving it later forfeits the gitignore and CI-cache inheritance that justified the choice.
- Lock divergence is locked to `EXIT_STALE`, never `EXIT_INFRA_ERROR`.
- The pre-build freshness check never falls back to a full rebuild on infrastructure error.
- Per-guest `[workspace]` sentinels are locked in place; guests remain separate workspaces.
- The `arachne-perimeters` `default-features = false` slicer-core variant is locked as-is.
- If Phase D ships, the profile is locked into the fingerprint input set and `FINGERPRINT_VERSION` is locked to `"v3"`. If Phase D is rejected, `FINGERPRINT_VERSION` stays `"v2"` and no profile entry is added.
- Everything else is reversible: `--force` restores today's build behaviour exactly, and `--sync-locks` can be re-run at any time.

## Risks and Tradeoffs

- **A fresh-but-wrong artifact would now survive.** Mitigated by the freshness verdict being artifact-verified rather than timestamp-based, and by `--force` remaining available. This is the same trust `cargo xtask test` already places in the check today.
- **Lock convergence may pull in a newer transitive dependency that breaks a guest build.** Step 5 regenerates all locks in one pass and its exit condition is a successful forced full build, so a breakage surfaces immediately and inside the step that caused it.
- **The shared target directory serialises concurrent guest builds** through Cargo's target-directory lock. Today's per-guest directories could in principle build in parallel, though the current implementation is a sequential loop, so nothing is lost now. Recorded as a constraint on any future parallelisation.
- **Removing the duplicate decode could change an error path that no test covers.** Mitigated by treating the pre-existing unit suite as the oracle and forbidding any assertion weakening; the removal is only accepted if the suite passes untouched.
- **The Phase D dev profile could make host tests slower than the build time it saves.** This is why Phase D is gated on Step 8's measurement rather than shipped on intuition.
- **The fingerprint bump forces one full rebuild for everyone after merge.** Accepted and documented; it is the correct consequence of changing the input set.
- **Timings are machine-specific.** No numeric threshold is a gate. `measurements.md` records evidence, and every number in it must be measured or labelled `unmeasured gap`.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4, the lock analyser plus its `--check` integration, and Step 9 if Phase D ships)
- Highest-risk dispatch and required return format: the Step 8 timing runs. They invoke multi-minute builds whose logs would swamp any context. Required return is `FACT` of at most 3 lines containing only the wall-clock figure and the command that produced it.

## Open Questions

- `[FWD]` Whether `cargo generate-lockfile` alone converges the guest locks, or whether each guest lock must be deleted first so Cargo resolves from scratch. Resolve empirically in Step 5: try regeneration first, and fall back to delete-then-generate if the divergence analyser still reports rows. Both are within Step 5's declared surface.
- `[FWD]` Whether the shared target directory needs `wasm32-unknown-unknown` to appear once or per profile in the intermediate path. Answer by reading Cargo's actual output layout after the first Step 3 build rather than by assuming; the AC asserts the helper and the build agree, not a specific literal.
- `[FWD]` Whether `xtask/src/test.rs` needs any change at all in Phases A to C. Expected: none, because it already composes the freshness-aware order itself. Confirm by running its existing tests after Step 1 rather than by editing it speculatively.
