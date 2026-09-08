# Packet 253 — Measurements

Measurement machine: Windows 11 (MINGW64_NT-10.0-26200, `uname -sr`), `nproc` = 12,
bash shell, repo at `D:\slicerProject\pinch_n_print_cli_2`. All Phase B numbers below
were measured on 2026-09-03 with a foreground `cargo xtask build-guests ...` invocation
on an already-warm Cargo cache (`target/guests` populated). Rows marked "carried" are
the prior worker's figures from an earlier session on the same machine, taken with bash
`SECONDS`; the post-convergence re-measurement in this session used `time` and is the
authoritative column. Phase C/D rows are appended by later steps of the packet.

## Phase B — lock convergence and guest-build timings

| Row | Command | Result | Measured |
| --- | --- | --- | --- |
| B1 | forced full guest build BEFORE lock convergence | not run before convergence | unmeasured gap |
| B2 | `cargo xtask build-guests --force` AFTER convergence | exit 0 | 12.971 s (`time`, this session); 23 s carried from the prior worker's first post-sync run |
| B3 | `cargo xtask build-guests` (warm, no changes) | exit 0 | 4.573 s (`time`, this session); 10 s carried |
| B4 | `cargo xtask build-guests --check` | exit 0, 0 `LOCK-DIVERGENCE` lines | 4.585 s (`time`, this session); 8 s carried |
| B5 | regenerated guest lockfiles (`--sync-locks`, Step 4b) | 42 changed lock files | 42 |
| B6 | `--sync-locks` outcome | exit 0, `synced 42 lockfile(s), skipped 4 root-workspace member(s)` | 42 synced / 4 skipped; wall time `unmeasured gap` |
| B7 | diverging crates BEFORE convergence (established by Step 4) | 42 | 42 |
| B8 | diverging crates AFTER convergence, union-of-versions definition | 7 (all intra-lock semver-major coexistence) | 7 |
| B9 | diverging crates AFTER convergence, compat-line definition (Step 4c) | 0 | 0 |
| B10 | root `Cargo.lock` clobbered by `--sync-locks`? | `git status --porcelain Cargo.lock` empty | no |

4 guests are members of the ROOT workspace (no `[workspace]` sentinel in their own
manifest), so `cargo generate-lockfile` there would rewrite the root `Cargo.lock`.
They are deliberately skipped by `--sync-locks` and excluded from the divergence
analysis (their on-disk lockfile is vestigial and is not what they resolve against).

## Note on the 7 residual divergences under the old definition (B8 → B9)

They were never cross-guest drift: every one of the 7 crates resolves to multiple
versions *inside individual guest lockfiles* (semver-major coexistence pulled in
transitively), which is ordinary Cargo resolution. Counted over the 42 lock-owning
guests:

| Crate | Versions across guest locks | Guest locks holding >1 version internally |
| --- | --- | --- |
| equator | 0.2.2, 0.6.0 | 1 / 42 |
| equator-macro | 0.2.1, 0.6.0 | 1 / 42 |
| getrandom | 0.2.17, 0.3.4 | 31 / 42 |
| syn | 1.0.109, 2.0.119, 3.0.4 | 42 / 42 |
| thiserror | 1.0.69, 2.0.20 | 31 / 42 |
| thiserror-impl | 1.0.69, 2.0.20 | 31 / 42 |
| wit-bindgen | 0.57.1, 0.60.0 | 31 / 42 |

Step 4c refined the analyser: a crate is divergent only when two guest locks resolve
DIFFERENT versions within the SAME semver-compatibility line (`X` for `X.Y.Z` with
`X > 0`, `0.Y` for `0.Y.Z` with `Y > 0`, `0.0.Z` otherwise). Each of the 7 crates above
differs only ACROSS lines, so all 7 clear, `--check` reaches exit 0 (B9), and the
load-bearing case (two guests on e.g. `syn` 2.0.117 vs 2.0.119 — same line, duplicated
compilation) is still reported and still folded into `EXIT_STALE`.

## Step 5 gate results (2026-09-03)

| Gate | Result |
| --- | --- |
| `cargo xtask build-guests --check` exit code | 0 |
| `cargo xtask build-guests --check \| rg -c 'LOCK-DIVERGENCE'` | 0 matches (`rg` exit 1) |
| `git status --porcelain Cargo.lock` | empty |
| `cargo test -p xtask` | ok, 128 passed, 0 failed |
| `cargo clippy -p xtask --all-targets -- -D warnings` | exit 0 |
| `cargo xtask check-literals` | exit 0, 0 violations (watchlist: 140 types) |

## Phase C — post-shared-target re-verification (Step 8 session, 2026-09-03)

All rows below were measured in this session with a foreground `time` on the same
machine described at the top of this file. "warm" means the shared guest target dir
(`target/guests`) already held artifacts for that profile; "cold" means it did not.

| Row | Command | Result | Measured wall time |
| --- | --- | --- | --- |
| C1 | `cargo xtask build-guests --force` (release, warm) | exit 0 | 9.974 s |
| C2 | `cargo xtask build-guests` (release, warm, no changes) | exit 0 | 2.298 s |
| C3 | `cargo xtask build-guests --check` (release, after full revert) | exit 0 | unmeasured gap (run without `time` as part of the revert proof) |
| C4 | `cargo xtask build-guests --force` (release, after revert, refreshing artifact mtimes) | exit 0 | 40.706 s |
| C5 | `cargo test -p xtask` (after revert) | ok, 133 passed, 0 failed | 5.23 s (cargo-reported) |
| C6 | `cargo test -p slicer-runtime`, release guests, COLD host test-binary compile | exit 0, 19 binaries, 1295 passed, 0 failed | 35 m 42.569 s |
| C7 | `cargo test -p slicer-runtime`, release guests, WARM (host binaries already built) | exit 0, 19 binaries, 1295 passed, 0 failed | 32 m 47.704 s |
| C8 | `cargo xtask build-guests` (release, warm, no changes) — baseline for the C9 pair | exit 0, `built 0 guest(s)` | 3.896 s |
| C9 | `cargo xtask build-guests` (release) AFTER a real content edit under `crates/slicer-core/src` | exit 0, `built 35 guest(s)` of 46 | 3 m 2.448 s |
| C10 | `cargo xtask build-guests` (release) after reverting the edit by hand | exit 0 | 2 m 46.239 s |
| C11 | `cargo xtask build-guests --check` (release, after the revert rebuild) | exit 0 | unmeasured gap (run without `time` as part of the revert proof) |

C6 vs C7 shows the host-compile share of a cold `-p slicer-runtime` invocation is
small relative to the suite's own runtime; the suite itself dominates.

### C9 — the incremental-rebuild row (AC-16)

Method: a single semantically inert trailing comment line was appended to
`crates/slicer-core/src/lib.rs` (a *content* change, because the freshness
fingerprint hashes file content, not mtime), the freshness-aware DEFAULT
`cargo xtask build-guests` was timed, then the line was deleted by hand. No
`git checkout` / `restore` / `stash` / `reset` was used. After the revert,
`crates/slicer-core/src/lib.rs` hashed byte-identical to its `acb2808a` content
and `git diff --stat HEAD -- crates/` was empty.

Result: **35 of 46 guests rebuilt, 3 m 2.448 s**, against a 3.896 s warm
no-change baseline (C8). The 11 guests that did not rebuild are the ones whose
Cargo path-dependency closure does not reach `slicer-core`, so the fingerprint
correctly scoped the rebuild rather than forcing all 46. The saving is real but
modest in this particular case: `slicer-core` sits near the root of most guests'
dependency closures, so a change there is close to the worst case for incremental
scoping. A change confined to a leaf crate would touch far fewer guests; that
narrower case was not measured in this session.

## Deferral — sccache

sccache adoption is **deferred, not rejected**. Its marginal value here is
unmeasured: until the shared guest target directory (`target/guests`, Phase C)
brings guest dependency artifacts under the CI cache, we cannot tell how much of
a guest build sccache would actually deduplicate versus what the shared target
dir already recovers on its own. Locally it also conflicts with an incremental
dev profile, so enabling it now would confound exactly the measurement that would
justify it.

**Trigger for revisiting:** the post-Phase-B CI timing measurement. Once that
number exists for the shared-target layout, re-evaluate sccache against it.

## Phase D — dev vs release guest profile

Method: `GUEST_PROFILE` in `xtask/src/build_guests.rs` was temporarily set to `"dev"`
and the `--release` flag removed from `guest_build_cargo_command`'s cargo args, guests
were rebuilt so the on-disk artifacts genuinely matched the profile being timed, then
the edit was reverted by hand (`git status --porcelain` empty, `git diff --stat HEAD`
empty) and release artifacts rebuilt.

### D-build — guest build side

| Row | Command | Profile | Result | Measured wall time |
| --- | --- | --- | --- | --- |
| D1 | `cargo xtask build-guests --force` | release, warm | exit 0 | 9.974 s |
| D2 | `cargo xtask build-guests --force` | dev, COLD (first dev build in the shared target dir) | exit 0 | 4 m 51.418 s |
| D3 | `cargo xtask build-guests --force` | dev, warm | exit 0 | 13.792 s |
| D4 | `cargo xtask build-guests --force` | dev, warm (second, mtime refresh) | exit 0 | 18.377 s |
| D5 | `cargo xtask build-guests` (no changes) | release, warm | exit 0 | 2.298 s |
| D6 | `cargo xtask build-guests` (no changes) | dev, warm | exit 0 | 2.708 s |
| D7 | `cargo xtask build-guests --force` | release, COLD | not run — the release artifacts were already warm at session start and clearing them would have meant touching `target/`, which Step 8 forbids | unmeasured gap |
| D8 | incremental rebuild after a real source edit | release | MEASURED — see C9: an inert content edit under `crates/slicer-core/src` rebuilt 35 of 46 guests. (`touch` alone remains a no-op, 2.899 s and no rebuild, because the fingerprint hashes file CONTENT; the dev-profile variant of this row was not measured.) | 3 m 2.448 s (release) |

**Build-side finding: the dev profile is not faster in the warm case.** Every
like-for-like warm pair (D1 vs D3/D4, D5 vs D6) has dev *slower*, not faster. The warm
forced build is dominated by componentization and WIT work rather than by codegen, so
dropping optimization does not buy back time there. The only large build figure on the
dev side (D2) is a cold-cache artifact of introducing a second profile into the shared
target dir — it measures the cost of *adding* the dev profile, not a steady-state win.

### D-test — host test-runtime side

The dev-profile `cargo test -p slicer-runtime` run was stopped by the harness after 11
of 19 test binaries had completed (0 failures at that point, 1152 passed). Rather than
compare a truncated wall time against a complete one, the comparison below uses cargo's
own per-binary `finished in` figures for the **same 11 binaries** in both runs. Both
runs had host test binaries already compiled and the matching guest artifacts on disk.

| Test binary | Release guests | Dev guests |
| --- | --- | --- |
| `unittests` (lib) | 0.39 s | 0.02 s |
| `tests/arachne_parity.rs` | 0.02 s | 0.02 s |
| `tests/arachne_parity_gaps.rs` | 0.01 s | 0.01 s |
| `tests/arachne_parity_round2.rs` | 0.85 s | 1.25 s |
| `tests/arachne_structural_invariants.rs` | 380.49 s | 631.40 s |
| `tests/arachne_wall_sequence_e2e_tdd.rs` | 3.46 s | 8.07 s |
| `tests/contract/main.rs` | 24.18 s | 42.06 s |
| `tests/e2e/main.rs` | 475.65 s | 983.45 s |
| `tests/executor/main.rs` | 292.98 s | 595.43 s |
| `tests/integration/main.rs` | 691.53 s | 1729.60 s |
| `tests/integration/support_family_routing.rs` | 0.00 s | 0.00 s |
| **Total, these 11 binaries** | **1869.56 s** | **3991.31 s** |

| Row | Measure | Value |
| --- | --- | --- |
| D9 | dev − release, 11 comparable binaries | +2121.75 s |
| D10 | dev / release ratio, same 11 binaries | 2.135x |
| D11 | full release run, all 19 binaries, warm | 32 m 47.704 s, 1295 passed, 0 failed |
| D12 | full dev run, all 19 binaries | unmeasured gap — the run was stopped by the harness at 11/19 binaries (0 failures, 1152 passed) after exceeding the release run's total wall time |
| D13 | correctness under dev guests | no dev-attributable failure observed in the 11 binaries that completed; the one failure seen in an earlier dev attempt was a `touch`-induced mtime staleness trip in `integrated_parity_harness`, not a profile effect, and did not recur after a forced rebuild |

## Decision

**Phase D: measured, rejected**

Deciding numbers:

- Guest build side, warm forced full build: release **9.974 s** vs dev **13.792 s** — the
  dev profile is *slower*, so there is no build-time saving to trade away.
- Guest build side, warm no-change build: release **2.298 s** vs dev **2.708 s** — again
  no dev win.
- Host test side, same 11 `slicer-runtime` binaries: release **1869.56 s** vs dev
  **3991.31 s** — **+2121.75 s (2.135x)** on the dev profile.
- Full warm release suite: **32 m 47.704 s**, 1295 passed, 0 failed (D11 / C7).

The edit-test loop therefore loses on both sides of the tradeoff simultaneously. The
hypothesised build-time win does not exist here (the warm guest build is dominated by
componentization, not codegen), while unoptimized wasm more than doubles the runtime of
the `slicer-core` geometry-heavy host tests. Phase D (Step 9) is not shipped.

Caveats stated honestly: the cold-cache release full build (D7) and the complete dev
suite wall time (D12) are unmeasured gaps, and the dev-profile variant of the
incremental-edit row was not timed. D8 itself is no longer a gap: the release-profile
incremental edit is measured at 3 m 2.448 s for 35 of 46 guests (C9). None of these can
reverse the verdict — the dev profile was measured *slower* on every like-for-like warm
guest build, so an incremental dev rebuild has no demonstrated saving to offer against a
measured test-runtime cost of 2121.75 s over 11 binaries alone.


## Known traps / follow-ups

- **Vestigial guest lockfiles (4 paths).** These four test-guests are root-workspace
  members (no `[workspace]` sentinel in their `Cargo.toml`), so cargo resolves them
  against the ROOT `Cargo.lock`. The committed `Cargo.lock` next to each manifest is
  unused at build time and is now ignored by both `--sync-locks` and the
  lock-divergence analyser (`guest_owns_lockfile` / `partition_lock_owners` in
  `xtask/src/build_guests.rs`):
  - `crates/slicer-wasm-host/test-guests/sdk-finalization-guest/Cargo.lock`
  - `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/Cargo.lock`
  - `crates/slicer-wasm-host/test-guests/sdk-postpass-text-guest/Cargo.lock`
  - `crates/slicer-wasm-host/test-guests/sdk-prepass-guest/Cargo.lock`

  **Hazard:** a developer editing one of these files will see NO effect — not on the
  build, not on `--check`, not on divergence output. They are retained deliberately
  (deleting them risks confusing unrelated tooling that assumes a lockfile beside every
  manifest); do not treat their contents as evidence of what those guests resolve.
