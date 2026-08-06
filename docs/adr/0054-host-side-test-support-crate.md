# ADR-0054 — Host-side test support lives in a std-only `pnp-cli-locator` crate

<!-- filename: 0054-host-side-test-support-crate -->

## Status

Accepted (2026-07-31). Resolves the `[FWD]` open question packet 162 left open
when it fixed the `pnp_cli` staleness trap in place rather than extracting it.

> **Rename note (2026-08-05):** the crate was created as `slicer-test-support`
> and renamed to **`pnp-cli-locator`** in the same session it shipped. The
> old name read as a sibling of `slicer_sdk::test_support` (ADR-0004), which
> it is not: it does one job, freshness-gated location of the `pnp_cli`
> binary. References below use the current name. The rename is cosmetic —
> the crate, its four functions, and this ADR's constraints are unchanged.

## Context

**Seven** separate host-side test/bench sites locate the `pnp_cli` binary before
spawning it. They fall into two groups that differ in an important way.

Group A — the three sites packet 162 knew about. Each locates the binary **and
asserts it is not stale**:

- `pnp_cli_bin` in `crates/slicer-runtime/tests/common/slicer_cache.rs`
- `pnp_cli_bin` in `crates/slicer-runtime/benches/gate_evidence.rs` (a
  `harness = false` bench, so it cannot import `tests/common/`)
- `bin` in `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`

Each carries its own copy of the same three concerns: workspace-root derivation
from `CARGO_MANIFEST_DIR`, a newest-source-mtime scan, and a panic whose message
must name the binary, the word `stale`, the resolved path, and a remedy.

Group B — four further sites, found during packet 165's implementation sweep,
each with a private `pnp_cli_bin` of its own:

- `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`
- `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`
- `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`
- `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`

These were **strictly worse than group A**, in two independent ways:

1. **No freshness gate at all.** They resolved a path and spawned it. Nothing
   compared the artifact's mtime against sources, so a stale `pnp_cli` produced
   a plausible-but-wrong e2e result rather than a failure — the exact hazard
   packet 162 existed to close, still open in four places it never looked.
2. **A dead profile branch.** Each selected `release` vs `debug` by testing
   `std::env::var("PROFILE") == Ok("release")`. `PROFILE` is a **build-script**
   environment variable: Cargo sets it for `build.rs`, not for test or bench
   executables. The branch was therefore never taken, and all four resolved
   `target/debug/pnp_cli` even under `cargo test --release` — silently spawning
   a debug binary from a release run.

Both groups also shared a third latent bug that the extraction fixes: the
per-site form hardcoded `<root>/target/`, so any build honouring
`CARGO_TARGET_DIR` put the binary somewhere these helpers would never look.
`pnp_cli_bin` in `crates/pnp-cli-locator` derives the profile directory from
`std::env::current_exe()` instead, which is correct under any target directory
and any profile.

**On the "three" figure.** Packet 162
(`.ralph/specs/162_wit-lifecycle-export-removal`) deliberately fixed the three
group-A sites **in place** and filed a `[FWD]` in its §Open Questions: "A
reviewer who DRYs these three copies inside this packet is making that decision
silently — the exact failure mode this packet exists to correct." That packet's
scope — and therefore its count — covered only the sites that already had a
freshness gate to fix. This ADR was authored from that inherited figure and
originally said three; the real count of seven surfaced only when packet 165
swept the tree for callers. Recorded here rather than quietly corrected, because
the provenance of the estimate is the useful signal: a duplication count taken
from a prior packet's scope is a lower bound, not a census.

The reason extraction was deferred is that the helper has no obvious home:

- **ADR-0004** places test support in `slicer-sdk` — but explicitly as
  *guest-side* support, behind a `test` feature, in a crate that compiles into
  guest WASM. Nothing there governs host-side binary location.
- `slicer-test`, the one crate that could have hosted a host-side helper, was
  **deleted by packet 78** (commit `c68f8973`) precisely to end a two-surface
  test-support split.

The freshness algorithm itself is a deliberate mirror, not a shared import:
`is_stale` lives in `xtask/src/build_guests.rs`, and `xtask` is a **bin-only**
crate (no `[lib]`, `build_guests` is a private `mod` of `xtask/src/main.rs`), so
there is nothing for a test target to `use`.

## Decision

Host-side test support lives in a new workspace member,
**`crates/pnp-cli-locator`**, with these constraints:

1. **std-only.** The crate declares no `[dependencies]` at all. Everything it
   needs (`std::fs`, `std::path`, `std::time`, `std::env`) is in the standard
   library.
2. **Dev-dependency only.** No production crate may depend on it; it appears
   solely in `[dev-dependencies]` of the crates whose tests and benches spawn
   `pnp_cli`.
3. **Host-side only.** It never compiles into guest WASM. ADR-0004's
   `slicer_sdk::test_support` remains the guest-side surface; the two are
   disjoint by design and neither re-exports the other.
4. It owns exactly four functions: `workspace_root`, `newest_source_mtime`,
   `staleness_reason`, and `pnp_cli_bin`.
5. `staleness_reason` is a documented **mirror** of `is_stale`
   (`xtask/src/build_guests.rs`), not an import — because `xtask` is bin-only.
   Its rustdoc must pin the source function by crate-qualified path and symbol
   name so the two stay legible as siblings when either changes.

## Consequences

Positive:

- **One drift surface instead of seven.** The panic-message contract from packet
  162 (must contain `pnp_cli`, `stale`, the resolved path, and
  `cargo build -p pnp-cli`) is now asserted in one place, and the pure
  `staleness_reason` seam is unit-testable without spawning anything.
- **Four previously ungated sites gain a freshness gate.** The group-B tests
  above had none; routing them through `pnp_cli_bin` extends the packet-162
  contract to them for the first time.
- **The dead `PROFILE` branch is gone.** Profile selection now derives from
  `std::env::current_exe()`, so a `--release` test run resolves the release
  binary instead of silently spawning the debug one.
- **`CARGO_TARGET_DIR` is honoured.** No site hardcodes `<root>/target/` any
  more.
- The release-over-debug fallback packet 162 deleted cannot be re-introduced
  copy-by-copy; there is only one locator left to review.

Negative / accepted costs:

- **The crate must stay dependency-free.** It is a dev-dependency of at least
  `slicer-runtime` and `slicer-scheduler`, so any dependency added here is taxed
  onto every narrow `cargo test -p slicer-runtime` and
  `cargo test -p slicer-scheduler` build — exactly the narrow invocations
  `CLAUDE.md` tells agents to prefer. A `walkdir` or `toml` dep would be a
  measurable regression for roughly eighty lines of std-only code.
- **Scan-scope over-approximation.** `newest_source_mtime` scans `crates/*/src/**`,
  so the new crate's own `src/` now counts toward `pnp_cli` staleness even though
  it never links into that binary. Accepted: the over-approximation fails
  **loud** (a spurious "rebuild pnp_cli" panic) rather than silent (spawning a
  stale binary against a fresh tree), and narrowing the scan by exclusion list is
  a second drift surface for no correctness gain.

## Alternatives Considered

1. **A `test-support` feature on `pnp-cli`'s existing lib target** — rejected on
   three independent grounds, each verified against `crates/pnp-cli/Cargo.toml`:
   - `pnp-cli`'s `[dependencies]` are **all non-optional** (`slicer-runtime`,
     `slicer-scheduler`, `slicer-schema`, `slicer-model-io`, `slicer-helpers`,
     `slicer-ir`, `clap`, `env_logger`, `serde`, `serde_json`, `toml`, `stl_io`,
     `png`, `ctrlc`). A Cargo feature cannot gate them off, so every narrow
     `cargo test -p slicer-runtime` would compile the entire CLI library to reach
     ~80 lines of std-only code.
   - **Feature unification churn.** `pnp-cli` declares `default = ["report"]` and
     `report = ["slicer-runtime/report"]`. A dev-dependency written
     `default-features = false` gets its feature set re-unified whenever anything
     else in the same build enables `report`, so the resolved feature set for
     `slicer-runtime` differs between a narrow `-p` run and a workspace-wide run
     — producing rebuild churn precisely on the narrow-vs-broad boundary
     `CLAUDE.md` requires agents to cross daily.
   - It **inverts the dependency direction**: library tests would depend on the
     CLI crate that is built on top of them.
2. **`slicer-sdk`** — rejected by **ADR-0004**, which scopes
   `slicer_sdk::test_support` to guest-side module authoring. `slicer-sdk`
   compiles into guest WASM; a host-only binary locator has no business there,
   and adding it would re-create the two-surface confusion ADR-0004 and packet 78
   jointly eliminated.
3. **Give `xtask` a lib target and import `is_stale`** — rejected. `xtask` is
   bin-only today (no `[lib]`; `build_guests` is a private `mod`). Adding a lib
   would drag `walkdir` + `toml` into every downstream test build, and the lib
   still would not carry the `pnp_cli`-specific locator (profile inference,
   resolved-path panic), so the duplication would only shrink, not close.
4. **Revive `slicer-test`** — rejected. It was deleted by **packet 78** (commit
   `c68f8973`) to end a two-surface test-support split; resurrecting the name
   would re-open exactly the ambiguity ("which crate's helper did I call?") that
   deletion resolved. The new crate takes a distinct, narrower name and a
   host-side-only charter so the boundary against ADR-0004 stays legible.

## Amendment — 2026-08-05 (archival cross-reference)

Packet 162's three-site census (its AC-8 and `design.md`'s "spawn site N of 3",
deliberately scoped to the sites that already had a freshness gate) is
**historical scope**, not the current count. It was superseded by this ADR's
seven-site record (above) and by packet 165's shared-crate extraction into
`pnp-cli-locator`. When packet 162 is archived to `_OLD`, readers should
treat ADR-0054 as authoritative for lookup-site counts; packet 162's
`[FWD]` in `design.md` §Open Questions resolves here.
