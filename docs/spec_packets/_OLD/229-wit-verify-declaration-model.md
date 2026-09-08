---
status: implemented
packet: 229-wit-verify-declaration-model
task_ids:
  - TASK-340
---

# 229-wit-verify-declaration-model

## Goal

Rebuild `xtask/src/wit_verify.rs` on `wit_parser` so that both the canonical `.wit` tree and a decoded artifact world are parsed into one package-qualified declaration model, compared with stage-package full equality on the exported interface plus subset direction everywhere else, and fail closed on unexpected packages and on an empty or unreadable canonical set.

## Problem Statement

`xtask/src/wit_verify.rs` is the only mechanism that answers the *semantic* question "does this built guest's embedded WIT world still agree with canonical?". Packet 230 wants to promote it into the freshness gate itself, but its current form cannot bear that weight, for five independently verified reasons:

1. **It is a hand-rolled scanner.** `extract_type_blocks` keys declarations by bare name in a flat `BTreeMap<String, String>`, so a name declared in two packages collapses; `canonical_type_blocks` works around that by *deleting* every ambiguous name when the stage is unknown. `matching_brace` also carries a latent index bug (R5-11): `open` is a byte index into `stripped` while `matching_brace(&bytes, open)` indexes a `Vec<char>`.
2. **It models only four keywords.** `BRACED_KEYWORDS` covers `variant`, `enum`, `record`, `flags`. Type aliases (`type X = ...;`), resources, interface functions, resource methods and `use` declarations are invisible, so nominal type-identity drift and signature drift pass silently.
3. **It compares only names present in both sides.** `verify_embedded_world`'s loop skips any canonical declaration the artifact does not embed, so a *missing* export is indistinguishable from an unused import.
4. **It fails open on infrastructure problems** (R5-7): `canonical_type_blocks` swallows unreadable files inside `if let Ok(text)`, and an empty canonical set reads as "nothing to verify".
5. **Its canonical file list is wrong by omission.** It reads 4 flat files plus one stage file; the `#[slicer_module]` macro `include_str!`s **20** (`prepass-types.wit` and all 15 stage files included). Separately, `crates/slicer-macros/build.rs` emits 9 `cargo:rerun-if-changed` paths of which **4 do not exist** (`deps/world-{prepass,postpass,finalization,layer}/world-*.wit`), and it watches none of the 15 stage files nor `prepass-types.wit` — so 16 of the 20 embedded files trigger no macro rebuild.

`wit-parser = "0.247"` is already a direct dependency of `crates/slicer-runtime` and `crates/slicer-wasm-host` and already resolved in `Cargo.lock`; `crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs` already resolves the canonical WIT dir with `wit_parser::Resolve`. Parsing both sides with it (user-ruled) removes the whole scanner class of defect in one slice, which is why this is one coherent packet rather than five fixes.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- `crates/slicer-macros/**` is on that guest-WASM input list, so the `build.rs` edit in Step 6 marks guests stale by design. That is expected, not a defect; it must be rebuilt, not explained away.
- **The canonical file list must be derived, never hardcoded.** If the verifier hardcodes the 20 paths and the audit test compares against that same constant, AC-1 becomes tautological. The list is produced by parsing `crates/slicer-macros/src/lib.rs` for `include_str!` targets ending in `.wit`, multiline-aware, and `crates/slicer-schema/wit/root.wit` is filtered out because the macro does not embed it.
- **Declaration bodies are ABI-ordered.** `wit_parser` preserves record field order and variant case order; the rendering used as a comparison key must preserve them too. Sorting is permitted only for the *set* of declarations within an interface and for `use` targets.
- **Fail closed on infrastructure.** No path in the verifier may convert "could not read / could not parse / nothing found" into a clean result. Every such condition is a `VerifyError`.
- **No new dependency version enters the graph.** `wit-parser = "0.247"` must match the string already in `crates/slicer-runtime/Cargo.toml` and `crates/slicer-wasm-host/Cargo.toml`; verify with `rg -n 'wit-parser' crates/*/Cargo.toml` before editing `xtask/Cargo.toml`.
- **No public schema/version constant is bumped by this packet**, so the schema-constant locking rule does not apply here; the fingerprint version prefix (`v1-` → `v2-`) belongs to packet 230.
- `xtask` has no `[lib]` and no `xtask/tests/` directory. All tests are `#[cfg(test)] mod tests` inside `xtask/src/*.rs`, so every AC command is `cargo test -p xtask <path::to::test> -- --exact`. There is no aggregator `mod` registration to add.

## Data and Contract Notes

- IR/manifest contracts: none changed. Module manifests are read only through the retained `module_stage_wit_dir`.
- WIT boundary: no canonical `.wit` file is edited. The packet changes only how those files are read; the WIT/Type Changes Checklist in `CLAUDE.md` is therefore not triggered on the WIT side. It **is** triggered on the `crates/slicer-macros/**` side purely as a guest-staleness consequence.
- Determinism/scheduler constraints: `WorldModel` uses `BTreeMap`/`BTreeSet` throughout so the drift list is deterministic and diffable across runs; packet 230's per-guest reporting depends on that determinism.
- Version handling: package membership is decided on the version-stripped name; the export-name comparison is exact including version. These are deliberately different and must not be unified.

## Locked Assumptions and Invariants

- The allowed embedded-package set is exactly `root:component` ∪ the 5 shared packages ∪ the resolved stage package. This is fail-closed by intent: a genuinely new shared package requires editing `SHARED_PACKAGES` in the same commit that introduces it.
- `crates/slicer-schema/wit/root.wit` is never part of the canonical model set, because the macro does not `include_str!` it.
- Declaration body rendering preserves source order of record fields and variant cases, permanently. A future "normalize for readability" change would silently defeat AC-3/AC-4.
- `Drift`'s `Display` output never contains the substring `STALE:` — packet 230's reporting contract puts the reason on a second line that must not be mistaken for a stale marker.
- `module_stage_wit_dir` survives this packet unchanged, so `build_one`'s behaviour for a never-built or manifest-less guest is unchanged here.

## Risks and Tradeoffs

- **`wit_parser` may reject the decoded text of some artifact that the old scanner tolerated.** Mitigation: AC-11 runs the real prepass and finalization artifacts through the full path before the packet closes; `AC-N3` pins that a parse failure is an error, not a pass. If a real artifact fails to parse, that is a finding to report, not a reason to loosen the model.
- **Fail-closed loading could break `build_one` in an environment with a partial checkout.** Accepted deliberately: R5-7 rules that an unreadable canonical set is an infrastructure error. The new `BuildError` variant makes the cause explicit rather than silently passing.
- **The `crates/slicer-macros/build.rs` fix marks all 42 guests stale on first run after the edit** (21 core-module guests plus 21 test guests; `crates/slicer-wasm-host/test-guests/witness/` is the one test-guest directory `discover_guests` skips, having no cdylib). This is the correct behaviour finally arriving (16 of 20 embedded files previously triggered no rebuild); it costs one full guest rebuild.
- **Extra/missing-declaration full equality on the exported interface may surface pre-existing drift** in an artifact that has silently disagreed with canonical. If so, the artifact is rebuilt — the finding is real, and must not be worked around by weakening the comparison.
