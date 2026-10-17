# Clipper in-tree vendoring and first value-exact engine candidate

Type: task
Label: wayfinder:task
Status: resolved (2026-10-17; human-approved KEEP — the vendoring + candidate
committed `7127b4d6` after all standing gates passed; see ## Answer)
Assignee: claimed by the 2026-10-16 OpenCode wayfinder session (human
route decision: "Pick 70 (candidate take)"). Graduated by the 2026-10-16
[headroom take](69-clipper-vendoring-optimization.md) (measured Rust/C++
ratios 1.42–2.86x per opclass on matched-workload inputs). A take requires
the human's route decision; ticket creation is not execution approval.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Bring the `clipper2-rust` 1.1.0 codebase in-tree (workspace member or
`[patch.crates-io]` redirect), implement the first value-exact hot-path
optimization candidate against the measured dominant slow opclass, and
decide keep/drop on the standing paired gate.

Context: t69 measured the headroom — on identical matched-workload inputs
(295 cases, two passes) the pristine production engine runs **offset
2.03–2.09x / union 2.61–2.63x / intersection 1.94–2.00x / difference
1.60–1.76x slower** than the bundled canonical Clipper2 C++ 1.5.2, and
the conditional mapped scale (≈ 1.2–1.5 % occupancy-style diagnostic of
the directly-sampled CP ceiling; larger if the region-prep transfer
holds) exceeds the map's prior below-floor context. The idealized
accumulated ceilings on base-off are ≈ 256–258 s (offset scope), ≈ 199–200 s
(closing_ex), ≈ 80–82 s (clip_polygons). No engine-internal cost center
was measured; the candidate direction is the measured slow opclasses.

## Notes

- **Vendoring mechanics.** Stay on the 1.1.0 codebase (t31 verdict: the
  1.2.0 bump is parked; `clean_collinear` makes 1.1.0 the diverging side).
  In-tree means a workspace member or a `[patch.crates-io]` path redirect
  with the upstream provenance (registry checksum `0fd663fe…`) recorded in
  the evidence; the t57 precedent vendored a pristine oracle plus candidate
  twin — reuse that shape. Any engine change recompiles into the wasm
  guests: guest rebuild + **both** freshness gates + byte-identity
  qualification on both legs are mandatory (t19's provenance constraint:
  even text-only changes shift the accelerated guest's artifact hash —
  expect and disclose hash churn, do not chase it).
- **Exactness contract.** Candidate-vs-pristine-Rust **byte-exact** on
  identical inputs (t57's dual-engine oracle pattern; the t69 corpus is
  ready-made: `evidence/t69-clipper-engine/corpus/` — t68-site + t55
  legs, format in the corpus READMEs; the harness drivers
  `driver.rs`/`driver.cpp` and the seal pattern are reusable). The 46/295
  Rust-vs-C++ divergences from t69 are the known t31 convergence delta —
  the C++ is the baseline, not the target; candidate exactness is never
  measured against it.
- **Candidate space (measured direction, not pre-approved shapes).**
  Allocation/aliasing, integer-op bottlenecks, data layout, redundant
  intersection sweeps, the `closing_ex`/`offset` inner loops. The take
  should first profile the vendored engine on the t69 corpus to name the
  internal cost centers (t69 deliberately stopped at opclass level),
  then pick the smallest change with a mapped ceiling worth its paired
  campaign.
- **Both legs.** Native (host) and wasm guest codegen must both be
  qualified — an optimization can help one and hurt the other (wasm-side
  codegen effects were explicitly un-measured in t69).
- **Acceptance:** independent exactness (dual-engine value-identity on the
  t69 corpus + full fresh 16-cell output matrix, both legs), then the
  standing paired ordinary + accelerated A/B on pre-declared cells (6
  retained pairs per mode, zero starvation exclusions), keep/drop to the
  human, no auto-commit. Probe/candidate-built binaries purged from the
  standing dirs after the take (t36 audit precedent); evidence under
  `evidence/t70-*`; reducer + verifier in-repo; raw tree gitignored.
- **Recipe constraints:** instrumented runs never mixed into wall claims;
  `--module-dir modules/core-modules` mandatory; `cargo xtask build-guests
  --check` exit 0 before any capture; accumulated spans are never wall
  (the t24/t30 trap); outputs disclosed per the fairness contract.

## Answer (take complete 2026-10-16/17; awaiting human keep/drop)

The take vendored `clipper2-rust` 1.1.0 in-tree, profiled the engine
internally on the t69 corpus, and implemented one value-exact hot-path
candidate. **All standing gates passed; the take's recommendation is KEEP**,
returning to the human uncommitted, per this ticket's contract.

**Vendoring** (sealed provenance `evidence/t70-clipper-candidate/vendor-provenance.json`):

- `vendor/clipper2-rust` is a byte-identical registry extraction of
  `clipper2-rust` 1.1.0 (crate archive sha256
  `0fd663fe209e7030c956e3be4c051dcc20cdb73da794f31466762cff12ca11bf`, pinned
  in the provenance file; `vendor-setup.py` re-verifies idempotently).
  It is a plain root workspace member; `slicer-core` depends on it via an
  explicit `path` dependency so `xtask`'s guest freshness fingerprint walk
  covers engine sources (workspace-indirect deps are invisible to the walk
  — verified: after vendoring, all 48 guests went STALE and rebuilt in both
  modes).

**Where the headroom actually lives** (first internal cost-center profile,
t69's missing layer; `evidence/t70-clipper-candidate/profiling/`):

- Tier 1: the pristine Rust engine allocates 2.6–3.4x more gross bytes than
  the C++ on identical inputs while making far fewer allocation calls —
  alloc volume does not by itself explain the opclass ratios.
- Tier 2 phase pairing on the full 295-case corpus: every phase is at parity
  or faster than C++ except `do_top_of_scanbeam` (2.19–4.26x) and
  `build_intersect_list` (1.25–2.26x). Root cause: the port's `nearbyint_f64`
  emulates round-half-to-even in software where the canonical C++ uses one
  hardware instruction — on the engine's hottest loop (`top_x` once per
  non-top edge per scanline, plus twice per intersection).
- Sub-span attribution inside `do_top_of_scanbeam` was attempted and rejected
  as measurement distortion (disclosed in FINDINGS); per-edge spans cost more
  than the work they measure.

**The candidate (v2, `candidate.patch`, 44 lines):**

- `nearbyint_f64(x)`: finite inputs take `x.round_ties_even()` (the hardware
  round-half-to-even), non-finite inputs keep the original software path
  verbatim. Value-exact on all 295 t69-corpus cases (hash + shape counts,
  5-repeat medians, engine-time ratios union 0.559 / offset 0.587 /
  intersection 0.650 / difference 0.964) — `results/v2-nearbyint-hardware.json`.
- Includes the vendored MSRV bump 1.70 → 1.77 the change requires (clippy
  `incompatible_msrv`; `Cargo.toml`-only, no engine semantics).
- A v1 candidate (scanbeam walk restructuring) was measured value-exact but a
  wash (inside noise) and dropped; the pristine engine was byte-restored.

**Standing gates (2026-10-16, both `cargo xtask build-guests --check` and
`--accelerated --check` exit 0 before every capture):**

1. Corpus dual-engine value-identity — **PASS** (candidate vs pristine Rust,
   295/295, above).
2. Fresh 16-cell output matrix, both legs — **PASS**: all 16 proof cells
   produced baseline-arm-identical outputs (detached campaign per
   `evidence/t70-clipper-candidate/campaign/`, qualification.json pass=True),
   and all 16 qualified shas byte-match the t58 preserved standing references
   (`evidence/t58-finalization-copies/qualification.json`, 16/16) — the
   candidate arm reproduces the exact standing output identity.
3. Standing paired ordinary + accelerated A/B on the four pre-declared
   classic-supports-on cells (the human's 2026-10-15 set) — **PASS** in every
   cell, both modes, 6/6 retained pairs, zero starvation exclusions

   | cell                            | wall Δ        | cpu Δ        | pairs |
   |---------------------------------|---------------|--------------|-------|
   | ordinary-benchy-classic-on      | −15.59 %      | −12.25 %     | 6/6   |
   | ordinary-base-classic-on        | −13.70 %      | −13.45 %     | 6/6   |
   | accelerated-benchy-classic-on   | −16.30 %      | −13.71 %     | 6/6   |
   | accelerated-base-classic-on     | −18.30 %      | −16.82 %     | 6/6   |

   (paired median wall/win + CPU corroboration clause in all 4 cells × 2
   modes; full reduction in `campaign/ab-reduction.json`.)
4. Code gates: `cargo clippy --workspace --all-targets -- -D warnings` exit 0;
   `cargo xtask check-literals` green; `cargo xtask check-test-quality
   --report` 0 findings.

**Disclosures:**

- **xtask root resolution is build-location-sensitive** (found in closeout,
  operational impact contained): `workspace_root()` returns the compile-time
  `env!("CARGO_MANIFEST_DIR")` (xtask/src/build_guests.rs). Invoking
  `cargo xtask` from a worktree/alternate checkout rebuilds xtask with THAT
  directory baked into the shared target's cached binary; afterwards the
  main tree's xtask resolves `crates/` (and every artifact path, including
  guest wasm staging and dist targets) at the dead path. The failure mode is
  nasty: the required pre-commit gates (`check-literals`,
  `check-test-quality`) silently scan **zero files** and still print green
  ("watchlist: 0 types"). Found here when the post-purge gate rerun printed
  "cannot read <purge-path>\crates" then "0 violations / watchlist: 0
  types"; fixed operationally by purging the cached xtask build units
  (`target/debug/xtask.exe`, `deps/xtask-*`, `.fingerprint/xtask-*`) and
  re-running — the real watchlist is 159 types. The fix belongs upstream of
  this ticket (runtime resolution, e.g. git-based, instead of the compile
  macro); filed as a named follow-up for the human.
- The checkpoint record claimed the take's diff was stashed as stash@{0};
  re-derivation before closeout showed stash@{0} is the older t69 recording
  stash and the take's diff lived only in the working tree. The stash was
  therefore NEVER used for the purge; instead a detached worktree carried
  the HEAD rebuild (see FINDINGS "Standing-state purge"). Recorded as the
  ledger-fact discipline in action.
- The 46/295 t31 `clean_collinear` Rust-vs-C++ divergences are the known
  convergence delta, disclosed not investigated (posture unchanged); the C++
  engine is headroom reference only.
- The campaign machinery (t67-clone) initially carried a path-resolution bug
  found and fixed pre-launch: the fixed five-level repo-root walk and config
  path in `run-cell.ps1` (and `reduce_ab.py`'s fixed `parents[4]`) resolve the
  t67 layout, not the t70 clone's `campaign/` depth; all resolvers were
  re-pointed at depth-agnostic repo-marker walks, probe-verified from both
  copy locations before the first launch (details in FINDINGS).
- Engine-time ratios were measured pre-MSRV-bump; the bump is Cargo.toml-only
  and the qualification snapshots were built from the MSRV-final tree.

**Purge record (t36 audit precedent, measured):** after the campaign, every
standing artifact location today's builds touched — `target/dist/developer/`,
`target/dist-accelerated/developer/`, `target/release/pnp_cli.exe`,
`target/guests/`, `target/guests-accelerated/`, `target/debug/pnp_cli.exe`,
and the in-tree staged guest wasm (48 files) — was rebuilt from HEAD
(`ac17c054`, pristine registry engine) inside a detached worktree and copied
byte-exact back. Verified measured: standing dist binaries are byte-identical
to that HEAD build (ordinary `1af71de6…`, accelerated `492cff54…`), the
candidate sha set (`ac1ad497…`, `9377584c…`, engine wasm `a04a77f7…`) is
absent from all standing locations, the candidate-vs-baseline-arm wasm
difference (`a04a77f7…` vs `53d2a4f6…`) is exact-engine-content (same
build-root), and the purged-vs-baseline-arm exe/wasm byte difference is
build-root path embedding (purge-root string 11x/4x vs 0x). Both worktree
freshness gates exit 0; `target/debug/` held no take-era binary (mtimes
Oct 13/Oct 9), its exe was rebuilt at HEAD anyway. FINDINGS carries the full
record.

**State on close:** the human approved KEEP on 2026-10-17; the vendoring +
candidate patch + lockfile sync were committed as `7127b4d6` (production
commit; this evidence/ticket/map set follows as the close record). The
standing artifact dirs, purged pre-commit to the pre-take HEAD state, were
then refreshed at the committed HEAD via the force path; the refresh's
content-level verification is in FINDINGS ("Post-keep standing refresh"
addendum — the standing layer now carries the measured candidate module
byte-for-byte, fresh by both canonical checks). The FINDINGS.md, campaign
machinery, and this ticket stay in-repo as the standing record.