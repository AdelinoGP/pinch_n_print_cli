# Requirements: 231-guest-closure-fingerprint

## Packet Metadata

- Grouped task IDs: `TASK-342`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`cargo xtask build-guests --check` decides guest staleness from a fingerprint whose "shared" half is a hardcoded list, `let shared_crates = ["slicer-macros","slicer-sdk","slicer-ir","slicer-schema","slicer-core"]` inside `shared_input_paths` (`xtask/src/build_guests.rs`), plus a depth-2 WIT walk yielding `crates/slicer-schema/wit/root.wit` and the flat `crates/slicer-schema/wit/deps/*.wit`. Every discovered guest is charged that whole set regardless of what it actually links: 11 of the 21 discovered test guests declare **no path dependency at all** (measured 2026-08-19) and link nothing from those five crates, yet a one-byte edit anywhere in them marks every guest stale. `stage_wit_snapshot` adds a second charge on the same axis — per-stage WIT directories, or conservatively *all* of them when `GuestSpec.stage_id` is `None`.

Packets 229 and 230 move WIT staleness to artifact verification, which makes both charges wrong rather than merely coarse: the fingerprint should now cover **code inputs only**, and it should cover each guest's real code inputs rather than a union. The list is also under-inclusive in a way the union hides — `has_parent_path_dep`, the existing manifest reader, inspects only `tab["dependencies"]`, so a walk modelled on it would silently drop `crates/slicer-sdk/Cargo.toml`'s `[target.'cfg(not(target_arch = "wasm32"))'.dependencies] slicer-core` and `modules/core-modules/classic-perimeters/Cargo.toml`'s `[target.'cfg(target_arch = "wasm32")'.dependencies] wit-bindgen` (Round 5 finding R5-6).

Two dependents of the deleted model must move with it. `xtask/src/test.rs`'s `ensure_pnp_cli_fresh_with` is the sole consumer of `compute_shared_freshness` outside `build_guests.rs`; it uses it as the cutoff of a hand-rolled mtime gate that decides whether to invoke the rebuild closure at all. That gate missed per-stage WIT, host crates, optional deps and `include_str!` assets (locked decision C7), and it disappears with the model it reads. And `crates/pnp-cli-locator/src/lib.rs`'s `staleness_reason` carries a rustdoc that ADR-0054 Decision rule 5 makes normative: it is a documented mirror of `is_stale`, and it currently describes `is_stale`'s omitted third disjunct as a hash over "shared crates, the guest's own inputs, and its per-stage WIT package". Both halves of that sentence stop being true here (R5-9).

This is one coherent slice because all four edits are consequences of a single model change — what set of files a guest's fingerprint covers — and leaving any of them behind leaves a false statement or dead code in the tree.

## In Scope

- Add `guest_closure_input_paths(spec, ws_root, cache)` to `xtask/src/build_guests.rs`: for each guest, start at `GuestSpec.manifest_path`, read it with `toml::Table` (as `has_cdylib` / `has_parent_path_dep` already do), collect every dependency entry carrying a `path` key from `dependencies`, every `target.<cfg>.dependencies` table, and `build-dependencies`; include entries with `optional = true`; **exclude** `dev-dependencies`.
- Resolve each `path` relative to the directory containing the manifest that declared it, canonicalize it, and recurse into that crate's `Cargo.toml`. Dedupe by canonical manifest path; guard cycles; cache results for the whole invocation in one `ClosureCache` keyed by canonical manifest path.
- For each closure member, charge `<crate>/src/**` (all files, matching `input_files`' extension-unfiltered behaviour), `<crate>/Cargo.toml`, and `<crate>/build.rs` when present.
- Keep the existing per-guest charges from `guest_input_paths`: the guest's own `src/**` and manifest, and for `GuestTree::Core` guests the parent module's `src/**` and the parent crate's own `<module>/Cargo.toml`.
- **Add the module manifest to the input set** (this packet's correction to plan decision C5): charge every `*.toml` directly under a core guest's parent module directory, which today is `<module>/Cargo.toml` plus the module manifest `<module>/<module>.toml` (21 module `.toml` files exist, one per core module, measured 2026-08-19). C5 justified the sibling-manifest charge with "the parent `include_str!`s it"; that justification is **false as stated** and the correction is load-bearing. Measured 2026-08-19: the only `include_str!` of a module `.toml` anywhere under `modules/` is in `modules/core-modules/classic-perimeters/src/lib.rs` — and it sits inside that file's `#[cfg(test)] mod tests`, so it never compiles into the guest `.wasm`; no production code embeds a module `.toml`, and `Cargo.toml` is embedded by nothing at all. The module manifest is nonetheless a build-relevant declaration input for two verified reasons: `parse_stage_id_from_module_manifest` parses its `[stage] id` into `GuestSpec.stage_id`, which packet 230 cross-checks against the artifact's resolved stage (R5-4), and its `[config.schema.*]` sections decide which keys the host's `ConfigView::from_declared` filter forwards to the guest. Neither `guest_input_paths` nor `shared_input_paths` charged it, so a module `.toml` edit left the guest FRESH. The charge is deliberately conservative — a `.toml`-only edit produces a byte-identical `.wasm` and therefore one converging no-op rebuild, which is the same accepted trade as C8's optional path deps.
- Delete `shared_input_paths`, `compute_shared_freshness`, and `stage_wit_snapshot`.
- Delete the two unit tests that exist only to prove `stage_wit_snapshot`'s behaviour: `stage_wit_dir_is_charged_only_to_matching_guest` and `stage_wit_unknown_stage_is_conservative`.
- Update `missing_fingerprint_metadata_is_stale`, which constructs a `GuestSpec` literal and calls `compute_shared_freshness`, to the new signature.
- Re-thread the callers: `compute_guest_freshness` loses its `shared: &FreshnessSnapshot` parameter and its `stage_wit_snapshot` fold; `is_stale` takes the closure cache instead of a shared snapshot; packet 230's `CheckContext.shared` field becomes the closure cache; `build_one`'s post-build fingerprint recomputation uses the same cache.
- Make `ensure_pnp_cli_fresh_with` unconditional: delete the `newest_source_mtime` / `pnp_cli_mtime_src` / `cutoff` computation and the early `code: 0` return, so `run_rebuild` is always invoked; delete `newest_mtime_in`, whose only call site that computation was.
- Update `crates/pnp-cli-locator/src/lib.rs`'s `staleness_reason` rustdoc so the mirror it documents is the dependency-closure model, satisfying ADR-0054 Decision rule 5. Signature, both message strings, and the crate's std-only dependency posture are unchanged.
- Add the `TASK-342` row to `docs/07_implementation_status.md` under "### Workstream 5 — Governance and closure drift", in that section's local terser row format.

## Out of Scope

- Artifact decoding, the declaration model, drift comparison, and the canonical coverage audit — packet 229.
- Stage resolution from artifacts, `check_command`'s return type, `build_stale_command`, `test_command`'s rebuild routing, exit codes, and the `v2-` fingerprint's *content* (workspace-root `Cargo.toml`, per-guest `Cargo.lock`, `rustc -vV`, `wasm-tools --version`) and *lifecycle* — packet 230. This packet changes only which **files** feed the hash.
- Deleting `GuestSpec.stage_id` or `parse_stage_id_from_module_manifest` — R5-4 keeps both.
- `xtask/src/dist.rs`, which is untouched by the whole plan.
- All freshness-contract documentation prose: `CLAUDE.md` "Guest WASM Staleness", `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, ADR-0014, ADR-0045, `CONTEXT.md`, the `wasm-staleness` snippet, `.claude/skills/spec-review/SKILL.md`, and `.github/workflows/ci.yml` — packet 232.
- Amending ADR-0054. Its normative content is conformed to, not contradicted; see `design.md` §Locked Assumptions and Invariants.
- Any other packet directory (user-ruled 2026-08-19), including the six whose ACs use the grep form of the freshness check (`206`, `207`, `209`, `210a`, `210b`, `211`, `212`).

## Authoritative Docs

- `docs/adr/0054-host-side-test-support-crate.md` — moderate; direct ranged read of "## Decision" only.
- `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — short; direct read (read-only here; packet 232 edits it).
- `docs/07_implementation_status.md` — long; delegate. Only the Workstream 5 section is edited.
- `docs/specs/guest-freshness-artifact-verification-plan.md` — moderate; direct read of "Locked decisions" C2/C7/C8 and the Round 5 findings R5-6 and R5-9.
- `CLAUDE.md` — direct read of "Guest WASM Staleness", "Test Discipline", "In-Tree Citation Style (MUST follow)", "No Unverified Metrics".

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-15`.
  - `AC-1`, `AC-5`, `AC-6` pin the walk's mechanics (transitivity, cycle guard, dedupe, one invocation-wide cache, optional deps included).
  - `AC-2` and `AC-N1` are the R5-6 pair: cfg-gated and build-dependency tables in, dev-dependencies out.
  - `AC-3` and `AC-4` are the two real-tree shapes — a core guest with a full chain, and a wit-bindgen-only test guest with an empty closure.
  - `AC-7` pins the code-inputs-only consequence (no `.wit` path in the fingerprint set).
  - `AC-15` is the module-manifest coverage fix: a `<module>/<module>.toml` edit must mark its core guest stale.
  - `AC-8`/`AC-9` are the deletion/retention pair; `AC-10`/`AC-11` the pnp_cli gate; `AC-12`/`AC-13` the ADR-0054 reconciliation and its unchanged consumers.
- Negative: `AC-N1` (dev-deps excluded), `AC-N2` (its real-tree consequence, no guest reaches `slicer-model-io`), `AC-N3` (out-of-closure edit does not mark a guest stale — the packet's reason to exist), `AC-N4` (a truncated closure is an error, never a smaller closure).
- Cross-packet impact: packet 230's `CheckContext` gains a different field type here; packet 232 documents the resulting contract. No packet directory other than this one is edited.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p xtask build_guests 2>&1 \| tee target/test-output.log \| rg '^test result:'; if rg -q 'test result: FAILED' target/test-output.log; then echo FAIL; else echo PASS; fi` | Whole closure-walk and staleness surface, including every `build_guests::tests::*` AC test | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `mkdir -p target && cargo test -p xtask test:: 2>&1 \| tee target/test-output.log \| rg '^test result:'; if rg -q 'test result: FAILED' target/test-output.log; then echo FAIL; else echo PASS; fi` | The pnp_cli freshness gate tests (AC-10, AC-11) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration pnp_cli_freshness_tdd 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. 3 passed'` | AC-13: the three `staleness_reason` consumers still pass | FACT pass/fail |
| `if rg -q 'shared_input_paths\|compute_shared_freshness\|stage_wit_snapshot' xtask/src/build_guests.rs xtask/src/test.rs; then echo FAIL; else echo PASS; fi` | AC-8 deletion sweep | FACT PASS/FAIL |
| `rg -q 'pub stage_id: Option<String>' xtask/src/build_guests.rs && rg -q 'parse_stage_id_from_module_manifest' xtask/src/build_guests.rs && echo PASS \|\| echo FAIL` | AC-9 retention sweep (R5-4) | FACT PASS/FAIL |
| `rg -q 'dependency closure' crates/pnp-cli-locator/src/lib.rs && echo PASS \|\| echo FAIL` | AC-12 / Doc Impact grep | FACT PASS/FAIL |
| `rg -q '^- \[.\] TASK-342 ' docs/07_implementation_status.md && echo PASS \|\| echo FAIL` | AC-14 / Doc Impact grep | FACT PASS/FAIL |
| `cargo check --workspace --all-targets` | Compiles every target, including the `slicer-runtime` and `slicer-scheduler` test targets that consume `pnp-cli-locator` | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Closure gate; also the dead-code detector for the deleted helpers | FACT pass/fail |
| `cargo xtask build-guests --check; echo "exit=$?"` | End-to-end smoke of the new input set on the real tree. Assert on the **exit code**, never on a `STALE:` grep (R5-3) | FACT exit code |

`cargo test --workspace` is **not** part of this matrix. The struct-literal churn gate (`cargo xtask check-literals`) is unaffected: it watches `pub` structs with >=5 named fields under `crates/*/src`, and `GuestSpec` lives in `xtask/src`.

## Step Completion Expectations

- Steps 1-2 must land together in the sense that the tree does not compile between them: Step 1 authors failing tests against symbols Step 2 creates. Do not run `cargo clippy -D warnings` as a gate at the end of Step 1.
- The `ClosureCache` created in Step 2 is threaded, not re-created per guest. `check_command` (packet 230) constructs it once per invocation and passes `&mut` down; `build_one` reuses the same instance when it recomputes the fingerprint after a successful build. A per-guest cache would still be correct but would re-read `crates/slicer-sdk/Cargo.toml` once per guest.
- Step 4's rustdoc edit must be re-read against `is_stale`'s *final* shape from Step 2, not its shape at packet-authoring time. The point of ADR-0054 rule 5 is that the two stay legible as siblings; a rustdoc describing an intermediate state is the same defect in a new place.
- Packet 230 must be `status: implemented` before Step 2 begins: `CheckContext` and `stale_reason` do not exist until it lands.

## Context Discipline Notes

- `xtask/src/build_guests.rs` and `xtask/src/test.rs` are both well over the direct-read threshold. Read only the named function bodies (`shared_input_paths`, `guest_input_paths`, `compute_guest_freshness`, `is_stale`, `stage_wit_snapshot`, `has_parent_path_dep`, `input_files`, `ensure_pnp_cli_fresh_with`, `newest_mtime_in`) plus the `#[cfg(test)] mod tests` block; do not read either file end to end.
- `docs/07_implementation_status.md` is long and its TASK-146b row alone is a single very long line (thousands of characters). Delegate the row append; never read the file to find the section.
- Do not open `docs/spec_packets/229-*` or `docs/spec_packets/230-*` `design.md` / `implementation-plan.md`. The 8 items this packet consumes from packet 230 are listed in `packet.spec.md` §Prerequisites; reconstruct anything further with a bounded SUMMARY dispatch against those packets' `packet.spec.md` only.
- `Cargo.lock`, `target/`, and the 42 `.wasm` artifacts are never loaded.
