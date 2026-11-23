# t70: Clipper in-tree vendoring and first value-exact engine candidate

Status: complete and **KEPT by the human** (2026-10-17) — the vendoring +
candidate committed as `7127b4d6`; all standing gates passed; standing dirs
purged pre-commit and refreshed at the kept HEAD (see "Standing-state
purge"). This file records the take's method, evidence inventory and
results; nothing was auto-committed — the human approved the keep and the
commit explicitly.

## Vendoring (done)

- `vendor/clipper2-rust` is a byte-identical copy of the verified registry
  extraction of `clipper2-rust` 1.1.0 (`vendor-setup.py`; crate archive sha256
  `0fd663fe209e7030c956e3be4c051dcc20cdb73da794f31466762cff12ca11bf`, equal to
  the pre-vendoring root `Cargo.lock` checksum; registry-vs-archive file
  equality asserted the same way as t69's `setup-engines.py`).
- It is a plain member of the root workspace (root `Cargo.toml` members list),
  the same role `crates/slicer-core` already plays for the independent
  wit-guest workspaces. `slicer-core` depends on it via an explicit `path`
  dependency (NOT `{ workspace = true }`): `xtask`'s guest freshness
  fingerprint walks `path` deps and their `src/**`, so any engine source
  change marks every guest carrying slicer-core stale. A registry requirement
  would be invisible to that walk.
- 42 guest lockfiles re-synced (`cargo xtask build-guests --sync-locks`); the
  registry entry leaves the root lock; all guests resolved the path package.
- Both guest freshness gates verified exit 0 on the vendored tree (ordinary
  and accelerated; the fingerprint staleness of the vendoring itself was
  verified first — all 48 guests STALE on the switch, as required).
- Code gates: `cargo clippy --workspace --all-targets -- -D warnings` green,
  `cargo xtask check-literals` green, `cargo xtask check-test-quality
  --report` 0 findings.

## Internal cost-center profile (the gap t69 left at opclass level)

Method: two scratch twins, instrumented identically at engine-internal phase
boundaries, run on the full t69 corpus (295 cases):

- Rust twin: `profiling/prof-twin.patch` applied to a copy of the vendored
  tree (`profiling/profile2-setup.py`); spans + phase-tagged allocation
  buckets (`profiling/profile-driver2.rs`).
- C++ twin: the bundled canonical Clipper2 1.5.2 copied to scratch and
  instrumented at the same phase boundaries (`profiling/cpp-twin-patch.py`,
  `profiling/profile-driver-phases.cpp`). The documented OrcaSlicerDocumented
  tree is never modified.
- Instrumented runs are corroboration-only; never mixed into wall claims.

Reduced results (committed): `profiling/results/tier1.json` (Rust timing +
allocation volume), `profiling/results/alloc-compare.json` (Rust vs C++ gross
allocation volume), `profiling/results/tier2.json` (Rust phase map),
`profiling/results/tier2-compare.json` (phase-paired Rust/C++ ratios).

Findings:

- Tier 1: the pristine Rust engine allocates **2.6–3.4x more gross bytes**
  than the C++ on identical inputs (median per case) while making far fewer
  allocation calls (0.02–0.27x) — arena Vecs vs per-node `new`. Allocation
  volume correlates with, but does not by itself explain, the opclass ratios.
- Tier 2 phase pairing: **every phase except `do_top_of_scanbeam` and
  `build_intersect_list` is at parity or faster than C++** (insert-local-minima
  0.92–1.23x, do-horizontal 1.0–1.2x, clean-up 0.2x, build-result 0.5–0.9x,
  scanline admin 0.84–1.0x). The two hot phases run Rust/C++ **4.26x / 2.19x /
  3.34x (union / intersection / offset)** for `do_top_of_scanbeam` and
  **2.26x / 1.6x / 1.99x / 1.25x (union / intersection / offset /
  difference)** for `build_intersect_list`.
- Sub-span attribution inside `do_top_of_scanbeam` was attempted and
  **rejected as measurement distortion**: per-edge/per-scanline spans cost
  more than the work they measure (union case totals inflated 115M→302M ns
  Rust, 50M→235M ns C++). The rejected run is disclosed, not used.
- Root cause (code reading, confirmed by the candidate measurement): the
  port's `nearbyint_f64` (`core.rs`) emulates round-half-to-even in software
  (trunc/sub/abs/branch/signum/modulo), while the canonical C++ rounds with a
  single hardware instruction (`clipper.engine.cpp` defines `nearbyint(a)` as
  `_mm_cvtsd_si64(_mm_set_sd(a))` under MSVC, and documents — in its own
  comment — that nearbyint "substantially *improves* performance here").
  `top_x` calls it once per non-top active edge per scanline — the engine's
  hottest loop — and `get_intersect_point` calls it twice per intersection.
- A first candidate (v1, `do_top_of_scanbeam` walk restructuring to minimize
  arena re-indexing) was implemented, measured **value-exact on all 295
  cases** and a wash (union −6%, offset +1%: inside noise), and **dropped**
  (`results/v1-do-top-scanbeam.json`). The pristine `engine.rs` was restored.

## The candidate (v2)

`candidate.patch` — one file (`src/core.rs`), plus the vendored `Cargo.toml`
MSRV bump 1.70 → 1.77 that the change honestly requires (clippy
`incompatible_msrv` enforced it):

- `nearbyint_f64(x)`: finite inputs return `x.round_ties_even()` — the
  hardware round-half-to-even, bit-identical to the software emulation for
  every finite value (tie-to-even, sign of zero preserved, subnormals exact).
  Non-finite inputs keep the original software path verbatim, preserving the
  emulation's NaN-at-infinity behavior.
- Measured on the t69 corpus (dual harness, pristine vs candidate, 5-repeat
  medians, `candidate-setup.py` + `candidate-reduce.py`):
  **value-exact on all 295 cases (hash + shape counts), native leg**, with
  engine-time ratios union **0.559**, offset **0.587**, intersection **0.650**,
  difference 0.964 (`results/v2-nearbyint-hardware.json`).
- Patch reconstruction verified byte-exact against a fresh registry
  extraction + `git apply` (`patch-check/` scratch; `diff -r` clean).
- Timing caveat, disclosed: the v2 engine-time ratios above were measured
  before the vendored `Cargo.toml` MSRV bump was added to the patch. The MSRV
  bump touches `Cargo.toml` only (no engine code), so semantics are identical;
  the 295-case value-exactness result and both freshness gates were re-run on
  the MSRV-final tree (exit 0). The wall-time truth for the standing gate is
  the qualification campaign's snapshots below, which were built from the
  MSRV-final tree.

## Acceptance pipeline (standing gates)

1. Corpus dual-engine value-identity — PASS (above).
2. Fresh 16-cell output matrix, both legs (ordinary + accelerated) — **PASS**
   (2026-10-16, detached driver pid 252, ~86 min wall): all 16 cells exit 0;
   per-cell baseline/candidate sha256 byte-identical (clean `unclean`/
   `probe`/drift guards throughout); `cargo xtask build-guests --check` and
   `--accelerated --check` exit 0 on the candidate tree immediately before
   the capture; `build-qualification.py` → qualification.json pass=True,
   16/16 cells. The base-fixture shas also reproduce the t58 preserved
   references (e.g. `20e7204c…` ordinary-base-classic-off).
   Cross-check added post-qualification: all 16 qualified shas are
   byte-identical to the t58 preserved references
   (`t58-finalization-copies/qualification.json`) — 16/16 match, so the
   standing output identity has been stable across the tree since t58 and
   the candidate arm reproduces it exactly.
3. Standing paired ordinary + accelerated A/B on the pre-declared four
   classic-supports-on cells (6 retained pairs per mode, zero starvation
   exclusions), keep/drop to the human, no auto-commit — **PASS**
   (2026-10-16, detached driver pid 21008, ~4 h 02 min): all 4 pre-declared
   cells, 6/6 retained pairs, zero starvation exclusions, wall clause pass
   and CPU clause pass in every cell, both modes
   (`campaign/ab-reduction.json`, reduced by `campaign/reduce_ab.py`):

   | cell                              | wall Δ (paired median) | cpu Δ (paired median) | pairs |
   |-----------------------------------|------------------------|-----------------------|-------|
   | ordinary-benchy-classic-on        | −15.59 %               | −12.25 %              | 6/6   |
   | ordinary-base-classic-on          | −13.70 %               | −13.45 %              | 6/6   |
   | accelerated-benchy-classic-on     | −16.30 %               | −13.71 %              | 6/6   |
   | accelerated-base-classic-on       | −18.30 %               | −16.82 %              | 6/6   |

   All 28 measured runs plus warmups emitted the per-cell sha-identical
   output (`f60e5e9b…` for both base cells, constant through every pair),
   matching the proof-phase identity — the engine change is value-exact in
   production, not only in the corpus harness.
4. Candidate-built binaries purged from the standing dirs after the take
   (t36 audit precedent) — see "Standing-state purge" below.

## Standing-state purge (t36 audit precedent)

Purged and rebuilt from HEAD (`ac17c054`, pristine registry engine) after
the campaign completed: `target/dist/developer/`,
`target/dist-accelerated/developer/`, `target/guests/`,
`target/guests-accelerated/`, `target/release/pnp_cli.exe`,
`target/debug/pnp_cli.exe` (rebuilt via `-p pnp-cli`, the HEAD package
name), and the in-tree staged guest artifacts (`modules/core-modules/*/
*.wasm` + `crates/slicer-wasm-host/test-guests/*.wasm`, 48 files, mirrored
to the HEAD-built set; stray nested `target/` build caches under the guest
workspaces removed). `target/debug/` top-level exes had take-earlier mtimes
(Oct 13/Oct 9), i.e. no take-built binary was standing there.

Method (measured): `xtask` resolves its own artifact/staging paths from the
calling repo root while `cargo` honors `CARGO_TARGET_DIR`, so the HEAD
rebuild ran inside a detached worktree (`git worktree add` of `ac17c054`,
which has no `vendor/`). All rebuilt artifacts were copied byte-exact back
into the main tree's standing locations.

Verification (all measured 2026-10-17):

- The standing dist binaries are byte-identical to the worktree's
  HEAD-built outputs: ordinary `1af71de6…`, accelerated `492cff54…`;
  the release exe `752b58d8…` was rebuilt at HEAD directly (mtime 01:58).
- The take's candidate sha set is absent from every standing location:
  ordinary dist exe `ac1ad497…`, accelerated dist exe `9377584c…`, and the
  candidate engine wasm `a04a77f7…` are not the bytes on disk now.
- Engine-content proof, independent of the exe: the candidate and the
  baseline-arm wasm of `classic-perimeters` were built at the SAME
  build-root (main tree) and differ byte-wise (`53d2a4f6…` vs `a04a77f7…`)
  — the content difference of the purged wasm `907a5513…` is the engine
  change, not pathing.
- The purged artifacts are NOT byte-equal to the campaign's baseline-arm
  binaries (`ad832ab8…`/`5f5e938a…`) — explained, measured, and disclosed:
  builds at different roots embed their build path (the purged exe contains
  the worktree-root string 11x, wasm 4x; the baseline-arm files contain it
  0x). Both are HEAD-provenance builds; byte-identity across build roots
  was never achievable.
- Worktree-side freshness gates: `cargo xtask build-guests --check` and
  `--accelerated --check` both exit 0 (the HEAD staging chain is fresh by
  the canonical check as well).
- The main tree (candidate take state) was untouched by the purge; the
  worktree was then removed (`git worktree remove`, clean worktree list at
  close). Raw logs: `.local-artifacts/.../t70-clipper-candidate/purge-rebuild*.log`.

## Post-keep standing refresh (addendum, 2026-10-17)

After the keep commit (`7127b4d6`) the standing dirs were refreshed at the
kept HEAD; the refresh surfaced a standing-state hazard worth its own record:

- The first refresh pass printed "built 0 guest(s)" in both modes and both
  freshness gates exited 0 — disproven by content: the dist modules still
  carried the pristine engine (WAT `f64.nearest` count 0 vs 16 in the
  candidate artifact; `wasm-tools print` on
  `classic-perimeters.wasm`).
- Cause (structural, measured): the purge's wholesale artifact mirrors
  brought pristine-content in-tree artifacts and shared-target intermediates
  into the standing tree, while the fingerprint sidecars
  (`target/guest-fingerprints/*.fingerprint` + accelerated namespace) stayed
  take-era candidate fingerprints (Oct 16 19:26–37) matching the kept HEAD's
  sources. The sidecar-vs-source comparison therefore said fresh over
  stale-content bytes. Any wholesale artifact copy can split the
  sidecar/artifact correspondence; gate exits cannot see it — inspect artifact
  content (or rebuild with `--force`) before trusting a clean gate.
- Remedy (the documented force path): `cargo xtask build-guests --force` +
  `--accelerated --force` + both dists — `built 48 guest(s)` both modes,
  sidecars rewritten (03:28), in-tree wasm restaged, dists rebuilt (03:49).
- Result (measured): both `--check` gates exit 0; the fresh dist module is
  `a04a77f7…` — **byte-identical to the take-era candidate-arm module**
  (same-root, same-sources determinism) — the standing layer carries the
  measured candidate engine byte-for-byte; dist exes ordinary `2bd06600…`,
  accelerated `402cd624…`.

## Notes and traps honored

- Instrumented runs never mixed into wall claims; accumulated spans are never
  wall; `--module-dir modules/core-modules` mandatory on every cell run;
  `cargo xtask build-guests --check` exit 0 required before captures.
- The take's tree diff was never committed during capture; baseline
  snapshots were built from a HEAD-state tree swap, and the take's state
  returned to the working tree afterwards. A checkpoint record claiming the
  diff was held in stash@{0} was re-derived and found stale (stash@{0} holds
  the older t69 recording-probe stash; the take's diff lived only in the
  working tree). `.local-artifacts/` is locally excluded
  (`.git/info/exclude`) so scratch payloads are never stashed, and the
  campaign scripts are copied to `.local-artifacts/.../campaign-run/`
  because a stash would otherwise remove the evidence tree itself.
- Known 46-case t31 `clean_collinear` Rust-vs-C++ divergences are disclosed,
  not investigated (t69 posture unchanged); candidate exactness is measured
  against pristine Rust, never against C++.
- **xtask's `workspace_root()` is compile-time-baked** (`env!
  ("CARGO_MANIFEST_DIR")` — `xtask/src/build_guests.rs`): a `cargo xtask`
  invocation from a non-main checkout poisons the shared target's cached
  xtask with the foreign root, after which the main tree's pre-commit gates
  silently scan zero files and still print green (here: "watchlist: 0
  types" while the real watchlist is 159). Discovered and contained in the
  closeout; upstream fix filed as a named follow-up for the human. Any
  future `git worktree`-based rebuild must purge
  `target/{debug,release}/xtask.exe`, `deps/xtask-*`, `.fingerprint/xtask-*`
  afterwards (or the xtask root is fixed first).
- **Path-drift trap (found and fixed pre-launch, disclosed):** the t67
  campaign machinery was cloned into a new `campaign/` subdir, one level
  deeper than t67's originals. t67's scripts resolve the repo root with a
  fixed five-`..` walk — correct at t67's depth, wrong from BOTH t70 copy
  sets (in-repo copies land at `docs/`, local `campaign-run/` copies land
  above the repo), and `run-cell.ps1`'s config path `..\matched-pair`
  similarly lost its anchor. Every affected resolver was re-pointed at a
  depth-agnostic walk for the `.local-artifacts` repo marker (the pattern
  `build-qualification.py` already used), configs now resolve against the
  canonical shared `evidence/matched-pair/configs/`, `reduce_ab.py`'s fixed
  `parents[4]` got the same walk, both copy sets were re-synced byte-identical
  (`diff -rq` clean), and a path probe asserted every derived path existed
  (both copies + `Test-MeasurementEvidence` dot-source) before the drivers
  were launched. First proof cell ran clean end-to-end on the first launch.