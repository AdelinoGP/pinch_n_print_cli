---
status: implemented
packet: 253-build-guests-incremental-and-shared-target
task_ids:
  - TASK-560
---

# 253-build-guests-incremental-and-shared-target

## Goal

Make `cargo xtask build-guests` rebuild only stale guests by default, compile every guest into one shared workspace-local target directory backed by converged guest lockfiles, and remove the per-guest fixed overhead from the freshness check, so the warm no-change path costs the freshness check rather than a full rebuild of every discovered guest.

## Problem Statement

`cargo xtask build-guests` rebuilds and componentizes every discovered guest on every invocation. `build_command` in `xtask/src/build_guests.rs` loops the full discovery result through `build_one` with no freshness consultation, even though `check_command` and `build_stale_command` already exist in the same file and are already composed in the freshness-aware order by `handle_guest_freshness_with` in `xtask/src/test.rs`. The default entry point is therefore the only guest-build path in the tree that ignores machinery the tree already trusts. `docs/03_wit_and_manifest.md` already documents the command as building any stale guests, so the code also contradicts its own documentation.

The cost is compounded by three structural issues. First, `build_one_inner` sets a shared `CARGO_TARGET_DIR` only when `spec.tree == GuestTree::TestGuest`; every core guest compiles its own private copy of `slicer-sdk`, `slicer-core`, `slicer-ir`, and `slicer-schema` into its own `target` directory. Second, those private target directories live outside the workspace `target/`, so the `Swatinem/rust-cache@v2` action in `.github/workflows/ci.yml` never caches them. Third, the freshness check itself carries fixed per-guest overhead: `compute_guest_freshness` spawns `rustc -vV` and `wasm-tools --version` once per guest, `canonical_world_model` in `xtask/src/wit_verify.rs` reparses the whole WIT directory per call, and `stale_reason` decodes each artifact twice, once through `embedded_world_model` and again through `verify_embedded_world` inside a block its own comment labels defensive.

Sharing a target directory only pays off when the guests resolve the same dependency versions. They do not: a survey during packet authoring found that only 15 of the 23 core `wit-guest/Cargo.lock` files agree on shared registry crate versions, with divergence on crates including `anyhow`, `autocfg`, and `hashbrown`, and the test-guest locks pinning two different `wit-bindgen` versions. Cargo keys artifacts by package, version, features, and profile, so a divergent lock silently reintroduces a full recompile of the shared dependency stack. Lock convergence is therefore part of the target-sharing work, not an optional tidy-up.

This is one coherent slice because all four phases touch the same orchestration file and the same freshness contract, and because measuring any one of them in isolation gives a misleading number: the shared target changes what a warm rebuild costs, which changes whether the fast local profile is worth shipping at all.

### Measured baseline

Measured on the requester's machine before any change. Re-measure in Step 8; these are the before column of `measurements.md`.

| Scenario | Measured |
| --- | --- |
| Warm `cargo xtask build-guests`, nothing changed | 1m54.6s |
| `cargo xtask build-guests --check` | 4.8s |
| Fresh isolated guest build, cold target | 1m29.7s |
| Second guest reusing the same `CARGO_TARGET_DIR` | 2.3s |

Guest count is a ledger fact; re-derive it with `cargo xtask build-guests --list` rather than quoting a number from this document. At authoring time discovery returned 23 core guests and 24 test-guests.

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
