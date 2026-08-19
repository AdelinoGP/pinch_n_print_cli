# Design: 231-guest-closure-fingerprint

## Controlling Code Paths

- Primary code path: `shared_input_paths` -> `compute_shared_freshness` -> `compute_guest_freshness` -> `is_stale` in `xtask/src/build_guests.rs`, plus `stage_wit_snapshot` folded into `compute_guest_freshness`. After packet 230 the same chain is reached through `stale_reason` and `CheckContext`.
- Secondary code path: `ensure_pnp_cli_fresh_with` in `xtask/src/test.rs`, the sole consumer of `compute_shared_freshness` outside `build_guests.rs`.
- Neighboring tests/fixtures: the `#[cfg(test)] mod tests` block at the end of `xtask/src/build_guests.rs` (`fingerprint_is_deterministic_and_content_sensitive`, `missing_fingerprint_metadata_is_stale`, `stage_wit_dir_is_charged_only_to_matching_guest`, `stage_wit_unknown_stage_is_conservative`, plus the `TempDir` helper); `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail` in `xtask/src/test.rs`; `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs`.
- OrcaSlicer comparison: not applicable. This packet touches no geometry and no ported algorithm; there is no canonical counterpart to compare against, so no OrcaSlicer obligation section appears in `packet.spec.md` or `requirements.md`.

## Architecture Constraints

- The `wasm-staleness` snippet is **deliberately omitted**. Its applies-to list is `crates/slicer-schema/wit/**`, `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`, `modules/core-modules/*/src|Cargo.toml|wit-guest`, and `crates/slicer-wasm-host/test-guests/*/src|Cargo.toml`. This packet's entire change surface is `xtask/src/build_guests.rs`, `xtask/src/test.rs`, `crates/pnp-cli-locator/src/lib.rs` and `docs/07_implementation_status.md`. None of those feeds a guest `.wasm`: `xtask` is host-only tooling, and `crates/pnp-cli-locator` is a std-only dev-dependency that ADR-0054 Decision rule 3 forbids from ever compiling into guest WASM. No guest needs rebuilding for this packet's edits to take effect, so quoting the snippet would assert an obligation that does not exist here. (The implementer will still *run* `cargo xtask build-guests --check` as an end-to-end smoke of the new input set — see `requirements.md` §Verification Commands — but that is verification of the tool, not a rebuild obligation on the change surface.)
- The `coord-system` snippet does not apply: no geometry, no mm/unit conversion.
- Closure over-approximation is intentional and must not be "optimized": optional path deps are included (locked decision C8), and a crate reached only under a `cfg` that never matches this build is still charged. The convergence argument is that the fingerprint is a content hash rewritten after each successful build, so a spurious rebuild converges to fresh; a *missing* input never converges and is a silently-stale guest.
- No public schema/version constant is bumped here. `FINGERPRINT_VERSION` is set to `"v2"` by packet 230; this packet changes the input set behind that version, not the version itself, and must not bump it again.
- Struct-literal churn gate: `cargo xtask check-literals` watches `pub` structs with >=5 named fields defined under `crates/*/src`. `GuestSpec` is defined in `xtask/src/build_guests.rs`, so the one test-code `GuestSpec` literal (in `missing_fingerprint_metadata_is_stale`) is out of the watchlist and needs no `..` rest or waiver. Do not add one.

## Code Change Surface

Selected approach: replace the shared-set half of the fingerprint with a per-guest closure walk, keeping the existing `FreshnessSnapshot` / `FingerprintEntry` / `fingerprint_entries` machinery untouched so packet 230's `v2-` hash content and lifecycle work is unaffected.

New in `xtask/src/build_guests.rs`:

- `pub struct ClosureCache` — one field, a `std::collections::HashMap<PathBuf, Vec<PathBuf>>` keyed by **canonicalized** manifest path, valued by that manifest's direct path-dep manifest paths. Created once per xtask invocation.
- `fn path_dep_manifests(manifest: &Path) -> Result<Vec<PathBuf>, ClosureError>` — reads the manifest with `toml::Table`; iterates `dependencies`, every sub-table of `target` under key `dependencies`, and `build-dependencies`; for each value that is a table with a `path` string, joins it to the manifest's parent directory and canonicalizes it plus `/Cargo.toml`. `dev-dependencies` is never read. `optional = true` is not filtered.
- `fn guest_closure_input_paths(spec: &GuestSpec, cache: &mut ClosureCache) -> Result<Vec<PathBuf>, ClosureError>` — breadth-first from `spec.manifest_path` with a `HashSet<PathBuf>` visited set (the cycle guard); for each visited crate root emits `input_files(root.join("src"), None)`, `root/Cargo.toml`, and `root/build.rs` when it is a file; sorts and dedupes, matching `shared_input_paths`' existing output contract.
- `enum ClosureError { Unreadable { manifest: PathBuf, reason: String }, MissingPathDep { manifest: PathBuf, dep: String, resolved: PathBuf } }` — AC-N4's observable. A truncated closure must never be silently returned.

Changed in `xtask/src/build_guests.rs`:

- `compute_guest_freshness(spec, ws_root, cache: &mut ClosureCache)` — drops the `shared: &FreshnessSnapshot` parameter and the `stage_wit_snapshot` fold; its entry set becomes `snapshot_from_paths(ws_root, &guest_input_paths(spec))` unioned with `snapshot_from_paths(ws_root, &guest_closure_input_paths(spec, cache)?)`.
- `is_stale(spec, ws_root, cache: &mut ClosureCache)` — same substitution; the two disjuncts (artifact-mtime and `metadata_matches`) are unchanged.
- Packet 230's `CheckContext` — its `shared: FreshnessSnapshot` field becomes `closure: ClosureCache`; `stale_reason` and `check_command` thread `&mut ctx.closure`. **[FWD]** below records what to do if packet 230 lands that field under a different name.
- `build_one`'s post-build fingerprint recomputation (the `compute_shared_freshness` call guarded by the comment "Record the inputs only after both cargo and componentization succeeded") uses the threaded cache.
- `guest_input_paths` — its `GuestTree::Core` branch already charges the parent module's `src/**` and `<module>/Cargo.toml`; **extend it** to charge every `*.toml` directly under the parent module directory, which adds the module manifest `<module>/<module>.toml`. Implement as `input_files(parent_dir, Some("toml"))` restricted to depth 1, or an explicit `read_dir` filter — do not recurse, or a module's `tests/` fixtures would enter the fingerprint. Do not duplicate the parent-crate logic in the closure walk; the closure walk covers path-dep crates, `guest_input_paths` covers the guest's own module.
- **Correction to plan decision C5 (packet-231 finding).** C5 justifies the sibling-manifest charge as "the parent `include_str!`s it". Measured 2026-08-19: the single `include_str!` of a module `.toml` under `modules/` is in `modules/core-modules/classic-perimeters/src/lib.rs`, inside its `#[cfg(test)] mod tests` block, so it never reaches the guest `.wasm`; and nothing embeds `Cargo.toml`. The real reasons to charge `<module>/<module>.toml` are that `parse_stage_id_from_module_manifest` derives `GuestSpec.stage_id` from its `[stage] id` (packet 230's independent stage expectation, R5-4) and that its `[config.schema.*]` sections drive the host's `ConfigView::from_declared` key filter. Write the accurate justification into any comment or doc text this step produces; do not repeat C5's `include_str!` claim.

Deleted from `xtask/src/build_guests.rs`: `shared_input_paths`, `compute_shared_freshness`, `stage_wit_snapshot`, and the two tests `stage_wit_dir_is_charged_only_to_matching_guest` and `stage_wit_unknown_stage_is_conservative`. `has_parent_path_dep` is **retained** — it is a discovery-shape predicate used by `discover_guests` (ADR-0014), not a freshness input, and the closure walk does not replace it.

Changed in `xtask/src/test.rs`: `ensure_pnp_cli_fresh_with` loses its `pnp_cli_path` probe, the `compute_shared_freshness` call, `pnp_cli_mtime_src`, `cutoff`, and the early `PnpCliFreshness { code: 0, .. }` return, so the body reduces to the `eprintln!` notice plus the `match run_rebuild(ws_root)` arms, all three of which are preserved verbatim (they are what `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail` asserts). `fn newest_mtime_in` is deleted; its only call site was the deleted `pnp_cli_mtime_src`. The `use` list must be pruned so `cargo clippy -D warnings` stays green.

Changed in `crates/pnp-cli-locator/src/lib.rs`: only the `staleness_reason` rustdoc paragraph describing `is_stale`'s third disjunct. It must keep naming `is_stale` and `xtask/src/build_guests.rs` (ADR-0054 rule 5, and CLAUDE.md's In-Tree Citation Style: symbol name plus crate-qualified path, no line numbers), keep the sentence that the omission is intentional, and replace "shared crates, the guest's own inputs, and its per-stage WIT package" with the dependency-closure description. Signature, both message strings, `newest_source_mtime`, `pnp_cli_bin`, `workspace_root` and the crate's empty `[dependencies]` are untouched.

Rejected alternatives:

- **Use `cargo metadata` for the closure.** Rejected: ADR-0014 records that `cargo_metadata` returns zero guests because of the `[workspace]` sentinel in each guest manifest, which is exactly the population being walked. Its "Future reviewers" section forbids migrating discovery back to it.
- **Keep a shared set but shrink it.** Rejected: any hand-maintained list reproduces the 2026-07-25 failure where `slicer-core` was missing from `shared_crates` and `--check` reported clean while every guest ran old geometry code. A derived closure has no list to forget to update.
- **Include `dev-dependencies` "to be safe".** Rejected on measurement: `crates/slicer-core/Cargo.toml` dev-depends on `slicer-model-io` by path and `crates/slicer-sdk` dev-depends on itself, so every guest closure would gain `slicer-model-io` and the SDK's own test tree. Dev-deps do not compile into a guest artifact.
- **Keep the pnp_cli mtime gate but fix its holes.** Rejected per locked decision C7: the gate had four distinct holes (per-stage WIT, host crates, optional deps, `include_str!` assets) and re-deriving Cargo's fingerprint by hand is the same class of bug as the hardcoded `shared_crates` list.

## Files in Scope (read + edit)

- `xtask/src/build_guests.rs` — role: owns the fingerprint input set and the closure walk; expected change: add `ClosureCache` / `path_dep_manifests` / `guest_closure_input_paths` / `ClosureError`, delete three functions and two tests, re-thread three call sites.
- `xtask/src/test.rs` — role: owns the pnp_cli freshness gate; expected change: unconditional rebuild, delete `newest_mtime_in`, add one test.
- `crates/pnp-cli-locator/src/lib.rs` — role: ADR-0054's documented mirror of `is_stale`; expected change: one rustdoc paragraph.
- `docs/07_implementation_status.md` — role: backlog ledger; expected change: one appended `TASK-342` row. Justification for the fourth file: it is a single-line append performed in its own step through a delegated dispatch, never a read of the file.

## Read-Only Context

- `xtask/src/build_guests.rs` — long (well over the direct-read threshold; re-derive with `wc -l` if a number is needed); read only `shared_input_paths`, `guest_input_paths`, `compute_guest_freshness`, `is_stale`, `stage_wit_snapshot`, `input_files`, `has_parent_path_dep`, `has_cdylib`, `snapshot_from_paths`, `fingerprint_entries`, `fingerprint_metadata_path`, `metadata_matches`, `discover_guests`, and the `#[cfg(test)] mod tests` block.
- `xtask/src/test.rs` — long (over the direct-read threshold); read only `newest_mtime_in`, `ensure_pnp_cli_fresh`, `ensure_pnp_cli_fresh_with`, `PnpCliFreshness`, and `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail`.
- `crates/pnp-cli-locator/src/lib.rs` — short (well under the direct-read threshold); direct read in full is acceptable and is the only whole-file read this packet allows.
- `docs/adr/0054-host-side-test-support-crate.md` — moderate; read "## Decision" only.
- `crates/slicer-sdk/Cargo.toml`, `modules/core-modules/classic-perimeters/Cargo.toml`, `modules/core-modules/classic-perimeters/wit-guest/Cargo.toml`, `crates/slicer-core/Cargo.toml` — small; read as closure-walk fixtures for AC-2, AC-3 and AC-N1.

## Out-of-Bounds Files

- `docs/spec_packets/229-wit-verify-declaration-model/**` and `docs/spec_packets/230-output-based-guest-freshness/**` — never edited; `design.md` and `implementation-plan.md` there are never opened. Consume only the item list in this packet's `packet.spec.md` §Prerequisites, or a bounded SUMMARY dispatch against those packets' `packet.spec.md`.
- Every other `docs/spec_packets/` directory, explicitly including `206`, `207`, `209`, `210a`, `210b`, `211`, `212` (user-ruled 2026-08-19).
- `CLAUDE.md`, `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, `docs/adr/0014-*.md`, `docs/adr/0045-*.md`, `CONTEXT.md`, `.claude/skills/**`, `.github/workflows/ci.yml` — read-only here; packet 232 owns every edit to them.
- `xtask/src/dist.rs` — untouched by the whole plan.
- `target/`, `Cargo.lock`, the 42 `.wasm` artifacts, generated code, vendored dependencies — never loaded.
- `OrcaSlicerDocumented/...` — not applicable to this packet; never loaded.

## Expected Sub-Agent Dispatches

- Question: does packet 230 as implemented name `CheckContext`'s shared-snapshot field `shared`, and what is its type? scope: `xtask/src/build_guests.rs`; return: `FACT` (<=5 lines); purpose: Step 2's re-threading.
- Question: after Step 2's deletions, does `cargo clippy --workspace --all-targets -- -D warnings` report any unused import, unused function or dead-code warning in `xtask/`? scope: clippy run; return: `FACT pass/fail` plus <=20 lines of the first failure; purpose: Steps 2 and 3.
- Question: list every call site of `staleness_reason`, `newest_source_mtime` and `pnp_cli_bin` outside `crates/pnp-cli-locator/`. scope: `crates/**`; return: `LOCATIONS` (<=20 entries); purpose: Step 4's blast-radius confirmation. At authoring time (measured 2026-08-19) this was: `staleness_reason` re-exported by `crates/slicer-runtime/tests/common/slicer_cache.rs` and called by 3 tests in `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs`; `newest_source_mtime` with no external caller; `pnp_cli_bin` with roughly 13 call sites across `slicer-runtime` e2e/integration tests, `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`, and `crates/slicer-runtime/benches/gate_evidence.rs`. Re-derive rather than trusting these counts.
- Question: append the `TASK-342` row under "### Workstream 5 — Governance and closure drift" in the section's local terser format and confirm with a grep. scope: `docs/07_implementation_status.md`; return: `FACT pass/fail`; purpose: Step 5.

## Data and Contract Notes

- IR/manifest contracts: none changed. The walk *reads* Cargo manifests; it writes none, and it does not touch module manifests' `[stage]`, `[config]` or `[claims]` sections. Config-key naming is not in play.
- WIT boundary: none crossed. This packet removes `.wit` files from the fingerprint input set; it does not read, parse or compare WIT. That responsibility is entirely packets 229 and 230.
- Determinism/scheduler constraints: `guest_closure_input_paths` must be order-deterministic (sort then dedupe) because its output feeds `fingerprint_entries`, which sorts by `(path, bytes)` but whose input path strings are recorded relative to `ws_root`. Canonicalization must not leak absolute machine paths into the hash: keep `snapshot_from_paths`' existing `ws_root`-relative rendering, and canonicalize only for cache keying and cycle detection.

## Locked Assumptions and Invariants

- **ADR-0054 is conformed to, not amended, and packet 231 owns that decision.** Rule 5 requires `staleness_reason`'s rustdoc to pin `is_stale` by crate-qualified path and symbol name "so the two stay legible as siblings when either changes". `is_stale` changes here, so the rustdoc must be updated — that update *is* the conformance. None of the five Decision rules is contradicted: the crate stays std-only (no dependency added), dev-dependency only, host-side only, and owns exactly the same four functions with unchanged signatures. Therefore no `D-231-ADR-0054-AMENDED` deviation is filed and no superseding ADR is authored. Packet 232 must not amend ADR-0054 either; it owns ADR-0014 and ADR-0045 only. Exactly one packet touches ADR-0054's subject matter, and it is this one.
- **ADR-0014 is amended, but by packet 232, not silently.** Its `## Amendments` section records packet 185's rule that `slicer-core` is tracked "in `xtask/src/build_guests.rs::shared_crates`", and its Consequences claim "Touching `slicer-core` does not trigger a guest rebuild storm". This packet deletes `shared_crates` and changes when that claim is true. ADR-0014's *normative decision* — guest discovery by validated filesystem walk rather than `cargo_metadata`, and no heavy `xtask` dependency — is conformed to exactly: `discover_guests` and `has_parent_path_dep` are untouched and no dependency is added. The stale amendment and consequence text is repaired by packet 232's AC-12, which names this packet. That cross-reference is what makes the change an explicit amendment rather than a silent ADR rewrite; do not also edit ADR-0014 here.
- Dev-dependencies are permanently excluded from the closure. This is a correctness rule, not a performance choice: a dev-dep does not compile into the guest artifact, and including one makes unrelated test-tree edits mark guests stale — the behaviour this packet exists to end.
- Optional path deps are permanently included (C8).
- The fingerprint covers code inputs only. Re-adding any `.wit` path to the fingerprint would restore double-counting on top of packet 230's artifact verification.
- `GuestSpec.stage_id` and `parse_stage_id_from_module_manifest` survive (R5-4). They are packet 230's independent stage expectation; deleting them re-opens the self-referential-check regression that `module_stage_wit_dir`'s own doc comment records from packet 164.

## Risks and Tradeoffs

- **Closure under-approximation is silent.** If the walk misses a table form, the affected guest reports fresh while running old code — the same failure class as the 2026-07-25 missing-`slicer-core` incident, which `--check` reported clean. Mitigated by AC-2 (three table forms), AC-3 (a real chain), AC-N4 (errors instead of truncation), and by the `cargo xtask build-guests --check` exit-code smoke.
- **Over-approximation costs rebuilds.** A guest whose closure includes an optional dep it never compiles rebuilds when that dep changes. Accepted per C8; it converges.
- **Unconditional `cargo build --bin pnp_cli` adds a fixed cost to every `cargo xtask test`.** The cost is a no-op Cargo fingerprint check when nothing changed. **Unmeasured on this machine at authoring time**; the implementer should time `cargo xtask test -- --help` before and after if a figure is wanted, and must not quote one otherwise.
- **`ClosureCache` threading touches packet 230's freshly-landed `CheckContext`.** Sequencing risk, not correctness risk: if 230 has not landed, Step 2 cannot compile. Guarded by the activation blocker in `packet.spec.md`.
- **The rustdoc can drift again.** ADR-0054 rule 5 is a documentation obligation with no compiler enforcement. AC-12's grep is the only automated guard, and it is a text grep; it will not catch a rustdoc that is merely stale in some other clause.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — the closure walk plus three deletions plus three re-threaded call sites in a long file, read by named function only)
- Highest-risk dispatch and required return format: the post-deletion `cargo clippy --workspace --all-targets -- -D warnings` run — `FACT pass/fail` plus at most 20 lines of the first failure. Never absorb full clippy output.

## Open Questions

- **[FWD]** Packet 230 introduces `CheckContext { shared, canonical, wasm_tools_version }`. This packet replaces the `shared` field with a `ClosureCache`. If 230 lands that field under a different name or type than `shared: FreshnessSnapshot`, adapt to whatever it actually shipped and record the difference in the Step 2 exit condition; do not "restore" the expected name. Resolve with the `FACT` dispatch listed in §Expected Sub-Agent Dispatches before writing Step 2's code.
- **[FWD]** `crates/pnp-cli-locator::newest_source_mtime` globs `crates/*/src/**`, `crates/*/Cargo.toml`, `crates/slicer-schema/wit/**/*.wit` and the workspace `Cargo.toml`, and has no caller outside `pnp_cli_bin` within its own crate. It is a *pnp_cli* input set, not a guest input set, and this packet deliberately leaves its behaviour alone — the ADR-0054 obligation is about `staleness_reason`, not about re-deriving `newest_source_mtime` from a Cargo closure. If the implementer finds a concrete case where its WIT glob produces a false stale-pnp_cli panic after packets 229-231, report it rather than fixing it here; it is a separate slice.
