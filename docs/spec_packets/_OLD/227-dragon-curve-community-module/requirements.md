# Requirements: 227-dragon-curve-community-module

## Packet Metadata

- Grouped task IDs: `TASK-338`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`
- Aggregate context cost: `M`

## Problem Statement

The design spec (`docs/specs/community-modules-dragon-curve-infill.md` §1, §4, §5) requires the first `modules/community-modules` entry: a sparse-infill module that tiles the sparse-fill polygon with the dragon curve and tiles the region with four dragons rotated 90 degrees apart and colours each dragon deterministically by its rotation index, wrapped into the host's tool count. The mechanism it depends on — the per-path `tool-index` carrier and the two-sided `claim:authored-coloring` grant — does not exist yet and is authored by draft 226. This packet therefore authors the module as a **consumer** of 226's symbols and branches its authoring language on 225's recorded verdict, exactly as §6 of the spec prescribes.

This is one coherent slice because it is the entire *module artifact* surface: no host code, no canonical WIT edit, no workspace membership. The authoring language is resolved from the recorded 225/225a verdict rather than re-probed here; see the correction note in §In Scope.

## In Scope

**Authoring language corrected 2026-08-14.** This section was written around a Go-vs-Rust-fallback branch. `docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a (2026-08-13)" supersedes the probes that branch rested on: Go is **NOT_LOADABLE_OR_CORRECT (terminal)** (wit-bindgen-go v0.7.0 emits imports `go:wasmimport` rejects), MoonBit is **LOADABLE_AND_CORRECT** (the earlier "strings corrupted" verdict was a fixture packaging error), and under the locked priority order that section records **"Dragon Curve authoring language: MoonBit"**. The user directed MoonBit explicitly. The module is therefore a MoonBit guest: **no `Cargo.toml`, no `src/lib.rs`, no `tests/*.rs`, no `#[slicer_module]`, no `slicer-sdk`, no `go/`, no `wit-guest/`.**

- The new `modules/community-modules/dragon-curve/` directory with module id `com.example.dragon-curve`.
- A `dragon-curve.toml` manifest mirroring the `rectilinear-infill.toml` conventions: `[module]` id/version, `[stage] id = "Layer::Infill"`, `[compatibility]` (`min-host-version`/`min-ir-schema`/`max-ir-schema`, `incompatible-with`/`requires` empty), `[claims].holds = ["claim:sparse-fill", "claim:authored-coloring"]`, and a complete `[config.schema]`.
- Config keys: `infill_density`, `infill_angle`, `infill_speed`, `line_width` (mirroring rectilinear-infill's exact snake_case spellings), plus `tiling_depth` (int, generation/depth) and `color_map` (a tool-sequence list mapping dragon-instance indices to tool indices).
- The dragon-curve tiling: a deterministic recursive fold, laid down as **many dragon instances on a rep-tile lattice** covering the sparse polygon at `line_width / infill_density` spacing, producing segments whose `tool_index = Some(map_tiling_index_to_tool(seg.tile, color_map, tool_count))` — keyed on the **dragon-instance index**, so one dragon prints in one tool and the tiling is legible in the part — wrapped into `[0, tool_count)` via the 226 `tool-count` query. (An earlier per-segment-ordinal version coloured *within* each dragon, made the tiling invisible, and was removed. `ordinal` and `generation` remain on `Seg` and are asserted by the tests as the curve's defining structure; they do not drive colour.)
- Unconditional `Some(tool)` emission (ADR-0058: "Modules never have to guard"); the host strips ungranted overrides. The module's `ExtrusionPath3D` literals must therefore set `tool_index` when the field exists (226), not branch on the grant.
- Edge cases in scope (§4): distinct colors > tool count (wrap); run-to-run reproducibility (byte-identical); sparse polygon with holes; per-region overrides of the dragon's config keys (`tiling_depth`, and the mirrored keys where the host permits).
- Edge cases out of scope (§4/§5): bridges, top/bottom solid, per-layer angle alternation.
- A frozen snapshot of the `Layer::Infill` WIT closure under `modules/community-modules/dragon-curve/wit/` (`layer-infill.wit` plus `deps/common/common.wit`, `deps/config/config.wit`, `deps/ir-types/ir-types.wit`, `deps/types/types.wit`), taken from the canonical `crates/slicer-schema/wit/` and refreshed by hand. A foreign-language guest has no `slicer-sdk`, so it generates its bindings from this snapshot.
- The MoonBit sources: `moon.mod.json` (module root, `name = "slicer/layer-infill"`), the import-free pure package `src/dragon/{dragon.mbt,dragon_test.mbt,moon.pkg.json}`, and the WIT glue templates `src/glue/{main.mbt.in,moon.pkg.json.in}` that the `Makefile` copies into the generated exported-interface package.
- Hand reimplementation of two `slicer-sdk` conveniences the guest cannot call, each unit-tested directly: the `should_emit` gate as `holds_sparse_fill` (fail-closed on an empty held-claims list) and `resolve_float`'s per-region override precedence as `pick_tiling_depth`.
- The build script (`Makefile`) and the committed `dragon-curve.wasm` produced manually by it: `wit-bindgen moonbit` → glue-template copy → `moon build --target wasm --release` → `wasm-tools component embed --encoding utf16` → `wasm-tools component new`.
- The banner `README.md` with the labeled-example social-rule text, and a `.gitignore` for the generated `gen/`, `interface/`, `world/`, `_build/`, `embedded.wasm` trees.
- In-module unit tests in `src/dragon/dragon_test.mbt` covering tiling determinism, colour mapping/wrap, holes, per-region override, and one-tool-per-dragon. Marshal/grant behaviour belongs to 226's surface, not here.
- A documented (not CI) manual slice test in the `README.md`, using `--model` (not `--input`) with `--module-dir modules/community-modules/dragon-curve` over `resources/20mm_cube.obj`, plus the config keys required to see colour at all.

## Out of Scope

- Any edit to `crates/slicer-schema/wit/**`, `crates/slicer-ir/**`, `crates/slicer-sdk/**`, `crates/slicer-wasm-host/**`, or `crates/slicer-macros/**` — those are 226's symbols.
- Adding the new module directory to the workspace `Cargo.toml` `members` list (explicitly forbidden; the module sits outside the Cargo graph per spec §1).
- `cargo xtask build-guests` integration for this module (its discovery walk is `modules/core-modules` and `crates/slicer-wasm-host/test-guests` only; see locked assumption L1).
- Any `docs/*.md` edit (packet 228 owns the docs/social-rule deliverables).
- `CONTEXT.md` glossary entries (already landed, verified present).
- Bridges, solid infill, top/bottom roles, `claim:top-fill`/`bottom-fill`/`bridge-fill`, per-layer 90° alternation.
- Any CI integration for the module.
- Re-running the Go/MoonBit probes (225 owns the gate).

## Authoritative Docs

- `docs/specs/community-modules-dragon-curve-infill.md` - 279 lines; direct read of §§1, 4, 5 (within 2000-line limit; the whole file is readable but only §§1/4/5 drive this packet).
- `docs/specs/community-modules-dragon-curve-plan.md` - 102 lines; direct read (Central Symbol Contract + Grounding Facts are binding).
- `docs/adr/0058-authored-coloring-per-path-tool-carrier.md` - 38 lines; direct read.
- `docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a (2026-08-13)" - long; delegate that section as a SUMMARY (do not load the language table). **This section is the binding one** — it supersedes the older §Community-module context probe verdicts, which record a Go WASI blocker and a MoonBit string-corruption failure that no longer hold.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (manifest id/stage/claims), `AC-2` (six config keys snake_case), `AC-3` (tiling determinism), `AC-4` (color-map wrap), `AC-5` (holes excluded), `AC-6` (per-region override), `AC-7` (tool_index emission — FORWARD-DEP on 226).
- Negative: `AC-N1` (zero tool_count/color_map → `None`, no panic).
- Cross-packet impact: consumes `tool-index` / `slicer_sdk::host::tool_count()` / `claim:authored-coloring` from draft 226 (names match the Central Symbol Contract); consumes the 225 verdict. Produces the module artifact for packet 228's docs banner and the manual slice example.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'id = "Layer::Infill"' modules/community-modules/dragon-curve/dragon-curve.toml` | manifest stage id | FACT pass/fail |
| `rg -q '"com\.example\.dragon-curve"' modules/community-modules/dragon-curve/dragon-curve.toml` | manifest module id | FACT pass/fail |
| `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon` | module unit suite (tiling/colour/holes/override/one-tool-per-dragon) | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f '<test name>'` | a single AC's test; `-f` is a name glob | FACT pass/fail |
| `cd modules/community-modules/dragon-curve && make` | bindgen + `moon build` + componentize; the only check that the `src/glue/*.in` templates compile against the generated bindings | FACT pass/fail |
| `wasm-tools component wit modules/community-modules/dragon-curve/dragon-curve.wasm \| rg -q 'tool-index: option<u32>'` | 226's carrier is present in the committed component's world | FACT pass/fail |
| `cargo check --workspace --all-targets` | no accidental workspace breakage | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | workspace gate (unchanged surface) | FACT pass/fail |

**The `cargo test` row that stood here is gone: it is unrunnable.** It read `cd modules/community-modules/dragon-curve && cargo test`, from the packet's original Rust-fallback branch. The module has no `Cargo.toml` (it is a MoonBit guest, per the correction in §In Scope), so that command fails outright rather than reporting anything. `moon test` above replaces it; `make test` is the equivalent shorthand. The two `cargo` rows that remain only prove this packet did not break the workspace — the module is outside the Cargo graph, so no `cargo` command builds or tests it.

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step 1 (language select) must record the 225a verdict verbatim before any language-specific file is written. The verdict resolved to MoonBit, so the original "Steps 2-4 shared, Step 5 diverges" branch structure does not apply; Step 3 is the pure MoonBit package, Step 4 the WIT glue, Step 5 the WIT snapshot and build.
- The tiling and colour-mapping logic in `src/dragon/` imports **nothing** — its `moon.pkg.json` has an empty `import` list. That is load-bearing: it keeps the pure logic testable with `moon test` alone, with no host, no WIT closure, and no component, which is what made Step 3 independent of 226 while 226 was still a FORWARD-DEP.
- Locked assumption L1 (xtask discovery never reaches `modules/community-modules`) is re-verified by a `FACT` dispatch before authoring the build script. L2 (originally "no WASI in the host") is superseded — see `design.md` §Locked Assumptions.

## Context Discipline Notes

- `crates/slicer-sdk/**` is not a dependency of this module and must not be read as one. A MoonBit guest cannot link the SDK; it reaches `tool-count` through its generated `host-services` bindings. Read `should_emit` (`crates/slicer-sdk/src/views.rs`) and `resolve_float` (`crates/slicer-sdk/src/config_resolution.rs`) only as **specifications** for the hand-rolled `holds_sparse_fill` and `pick_tiling_depth`. `crates/slicer-sdk/src/host.rs` is long — never read it wholesale.
- `run_infill` (`modules/core-modules/rectilinear-infill/src/lib.rs`) is long — treat as a structural model for role/region discipline via a delegated `SUMMARY`, not a full read, and **not as a template**: it is a Rust `#[slicer_module]` guest and this module is not.
- `modules/community-modules/dragon-curve/{gen,interface,world,_build}/` and `embedded.wasm` are generated by `make`, gitignored, and destroyed by the `bindings` target. Never read them as a source of truth and never hand-edit them; the hand-written glue lives only in `src/glue/*.in`.
- The module's `wit/` is a frozen snapshot, not the contract. The canonical WIT is `crates/slicer-schema/wit/`; read that when checking whether the snapshot has drifted, and never edit it from this packet.
