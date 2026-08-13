# Requirements: 227-dragon-curve-community-module

## Packet Metadata

- Grouped task IDs: `TASK-338`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The design spec (`docs/specs/community-modules-dragon-curve-infill.md` §1, §4, §5) requires the first `modules/community-modules` entry: a sparse-infill module that tiles the sparse-fill polygon with the dragon curve and colors each emitted scan segment deterministically from a stable tiling property, wrapped into the host's tool count. The mechanism it depends on — the per-path `tool-index` carrier and the two-sided `claim:authored-coloring` grant — does not exist yet and is authored by draft 226. This packet therefore authors the module as a **consumer** of 226's symbols and branches its authoring language on 225's recorded verdict, exactly as §6 of the spec prescribes.

This is one coherent slice because it is the entire *module artifact* surface: no host code, no WIT edit, no workspace membership. The authoring-language branch (Go vs Rust fallback) is resolved from the single 225 verdict rather than re-probed here.

## In Scope

- The new `modules/community-modules/dragon-curve/` directory with module id `com.example.dragon-curve`.
- A `dragon-curve.toml` manifest mirroring the `rectilinear-infill.toml` conventions: `[module]` id/version, `[stage] id = "Layer::Infill"`, `[compatibility]` (`min-host-version`/`min-ir-schema`/`max-ir-schema`, `incompatible-with`/`requires` empty), `[claims].holds = ["claim:sparse-fill", "claim:authored-coloring"]`, and a complete `[config.schema]`.
- Config keys: `infill_density`, `infill_angle`, `infill_speed`, `line_width` (mirroring rectilinear-infill's exact snake_case spellings), plus `tiling_depth` (int, generation/depth) and `color_map` (a tool-sequence list mapping tiling ordinals to tool indices).
- The dragon-curve tiling: deterministic recursive fold over the sparse polygon at `line_width / infill_density` spacing, producing `ExtrusionPath3D` segments where each segment's `tool_index = Some(f(tiling_index))`, `f` a stable function of fold order / generation / segment ordinal wrapped into `[0, tool_count)` via the 226 `tool-count` query.
- Unconditional `Some(tool)` emission (ADR-0058: "Modules never have to guard"); the host strips ungranted overrides. The module's `ExtrusionPath3D` literals must therefore set `tool_index` when the field exists (226), not branch on the grant.
- Edge cases in scope (§4): distinct colors > tool count (wrap); run-to-run reproducibility (byte-identical); sparse polygon with holes; per-region overrides of the dragon's config keys (`tiling_depth`, and the mirrored keys where the host permits).
- Edge cases out of scope (§4/§5): bridges, top/bottom solid, per-layer angle alternation.
- The build script (`Makefile`) and the committed `dragon-curve.wasm` produced manually by it.
- The banner `README.md` with the labeled-example social-rule text.
- In-module unit tests under `tests/` covering tiling determinism, color mapping/wrap, holes, and per-region override. Marshal/grant behavior belongs to 226's surface, not here.
- A documented (not CI) manual slice test: `pnp_cli slice --module-dir modules/community-modules/dragon-curve --input resources/regression_wedge.stl --output <out>.gcode`.
- Both authoring branches concretely specified: (A) Go loadable-and-correct → Go bindings + Makefile + committed Go-built `.wasm`; (B) fallback (expected) → Rust `#[slicer_module]` guest, Go tiling retained under the module dir as a labeled reference implementation only, committed `.wasm` Rust-built.

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
- `docs/14_submodule_programming_languages.md` - 172 lines; delegate §Community-module context as a SUMMARY (do not load the language table).

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
| `cd modules/community-modules/dragon-curve && cargo test` | module unit suite (tiling/color/holes/override) | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | no accidental workspace breakage | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | workspace gate (unchanged surface) | FACT pass/fail |

Commands must have small, parseable output suitable for delegation. `cargo test` is run from inside the standalone module crate, never via a workspace `-p` member (it is not a member).

## Step Completion Expectations

- Step 1 (branch select) must record the 225 verdict verbatim before Step 2 writes any branch-specific file; the two branches share Steps 2-4 (manifest, tiling+color logic, tests) and diverge only in Step 5 (guest/artifact/build).
- The tiling and color-mapping logic is written **without** importing `tool_count` or `ExtrusionPath3D.tool_index`, so Steps 2-4 remain testable even though 226 is a FORWARD-DEP; only Step 5's emission wiring consumes 226's symbols and is deferred by AC-7.
- Locked assumptions L1 (xtask discovery) and L2 (no WASI in the host) are re-verified by a `LOCATIONS`/`FACT` dispatch before authoring the build script.

## Context Discipline Notes

- `crates/slicer-sdk/src/host.rs` is 1108 lines — do not read it wholesale; the 226 `tool_count` wrapper is a FORWARD-DEP and its exact body is out of scope. Read only `views.rs`'s `should_emit` region and `builders.rs`'s `InfillOutputBuilder` if a shape check is needed.
- `modules/core-modules/rectilinear-infill/src/lib.rs` is 565 lines — treat as a structural model via a delegated `SUMMARY` of its module shape (struct + `#[slicer_module] impl LayerModule` + `run_infill`), not a full read.
- The 225/226 packet directories do not exist yet — never attempt to read them; the Central Symbol Contract in the plan file is the only authoritative producer surface.
