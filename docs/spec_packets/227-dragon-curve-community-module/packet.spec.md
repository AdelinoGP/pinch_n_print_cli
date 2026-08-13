---
status: draft
packet: 227-dragon-curve-community-module
task_ids:
  - TASK-338
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 227-dragon-curve-community-module

## Goal

Author the first community module — the dragon-curve sparse-infill tiling with deterministic per-segment tool coloring — at `modules/community-modules/dragon-curve/`, branching on the recorded 225 feasibility verdict so the module ships either as Go bindings + committed `.wasm` (Go loadable-and-correct) or as the standard Rust `#[slicer_module]` guest (fallback, expected).

## Scope Boundaries

This packet owns only the new `modules/community-modules/dragon-curve/` directory and its artifacts: `dragon-curve.toml` manifest, `Cargo.toml`/`src/lib.rs`/`tests/` (Rust fallback) or `go/` + committed `.wasm` (Go branch), the build `Makefile`, the committed `dragon-curve.wasm`, and the banner `README.md`. It consumes the `tool-index` carrier, `claim:authored-coloring` grant, and `tool-count` query that draft 226 produces but does not re-land them. It touches no workspace Cargo members, no `docs/*.md`, and no host/WIT code.

## Prerequisites and Blockers

- Depends on: draft `225-dragon-curve-feasibility-gate` (the recorded Go/MoonBit verdict that selects the authoring branch), draft `226-authored-coloring-carrier` (the `tool-index: option<u32>` WIT field, `slicer_ir::ExtrusionPath3D.tool_index: Option<u32>`, `claim:authored-coloring`, `fill_authored_coloring`, host `tool-count: func() -> u32`, and SDK `slicer_sdk::host::tool_count()`).
- Unblocks: `228-community-module-docs-banner`.
- Activation blockers:
  - **FORWARD-DEP on draft 225-dragon-curve-feasibility-gate** — the authoring branch (Go vs Rust fallback) is unreadable until 225 records its verdict in `docs/14_submodule_programming_languages.md`.
  - **FORWARD-DEP on draft 226-authored-coloring-carrier** — `ExtrusionPath3D.tool_index`, `slicer_sdk::host::tool_count()`, and the `claim:authored-coloring` grant surface do not exist until 226 lands. AC-7 and its emission test are placed behind this blocker; the pure tiling/color-mapping logic is deliberately 226-free and testable now.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the module manifest at `modules/community-modules/dragon-curve/dragon-curve.toml`, **when** it is inspected, **then** `[module].id` is `com.example.dragon-curve`, `[stage].id` is `Layer::Infill`, and `[claims].holds` contains exactly `claim:sparse-fill` and `claim:authored-coloring`. | `rg -q '"com\.example\.dragon-curve"' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q 'id = "Layer::Infill"' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '"claim:sparse-fill"' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '"claim:authored-coloring"' modules/community-modules/dragon-curve/dragon-curve.toml`
- **AC-2. Given** the module manifest, **when** its `[config.schema]` is inspected, **then** all six keys are declared with the rectilinear-mirrored spellings `infill_density`, `infill_angle`, `infill_speed`, `line_width` plus the dragon-specific `tiling_depth` and `color_map`, each in snake_case. | `rg -q '\[config\.schema\.infill_density\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.infill_angle\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.infill_speed\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.line_width\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.tiling_depth\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.color_map\]' modules/community-modules/dragon-curve/dragon-curve.toml`
- **AC-3. Given** the dragon tiling helper driven by `(ExPolygon, line_spacing, tiling_depth)` with no RNG, clock, or map-iteration-order dependence, **when** it is run twice over the same sparse polygon, **then** the two emitted segment lists are byte-identical (same segment order, ordinals, and coordinates). | `cd modules/community-modules/dragon-curve && cargo test --test dragon_tiling_tdd tiling_is_deterministic_across_runs -- --exact`
- **AC-4. Given** the pure color-mapping helper `map_tiling_index_to_tool(segment_ordinal, generation, color_map, tool_count)`, **when** `color_map` exceeds `tool_count`, **then** the returned tool is wrapped into `[0, tool_count)` and is deterministic for a fixed `(ordinal, generation, color_map, tool_count)`. | `cd modules/community-modules/dragon-curve && cargo test --test dragon_color_map_tdd color_map_wraps_into_tool_count -- --exact`
- **AC-5. Given** an `ExPolygon` whose contour encloses one or more holes, **when** the tiling helper runs, **then** no emitted segment endpoint lies inside a hole and every segment lies within the contour minus holes. | `cd modules/community-modules/dragon-curve && cargo test --test dragon_tiling_tdd holes_are_excluded_from_tiling -- --exact`
- **AC-6. Given** a region whose per-region `ConfigView` carries a `tiling_depth` override, **when** the dragon resolves the region's tiling depth through the `slicer_sdk::config_resolution::resolve_float` path, **then** the override value wins over the module-global default. | `cd modules/community-modules/dragon-curve && cargo test --test dragon_config_override_tdd per_region_tiling_depth_override -- --exact`
- **AC-7. Given** draft 226 has landed the `ExtrusionPath3D.tool_index: Option<u32>` field and `slicer_sdk::host::tool_count()`, **when** `run_infill` emits a sparse path, **then** the path carries `tool_index = Some(map_tiling_index_to_tool(...))` wrapped into the host's `tool_count()` range (unconditional `Some`; the host strips ungranted per ADR-0058). **FORWARD-DEP on draft 226-authored-coloring-carrier — deferred until 226 lands.** | `cd modules/community-modules/dragon-curve && cargo test --test dragon_emission_tdd emitted_paths_carry_tool_index_some -- --exact`

## Negative Test Cases

- **AC-N1. Given** `tool_count == 0` or `color_map == 0` as inputs to `map_tiling_index_to_tool`, **when** the helper is called, **then** it returns `None` (no division-by-zero, no out-of-range tool) and `run_infill` maps `None` to `ExtrusionPath3D.tool_index = None`. | `cd modules/community-modules/dragon-curve && cargo test --test dragon_color_map_tdd tool_count_zero_returns_none -- --exact`

## Verification

- `cargo check --workspace --all-targets` (proves the packet added no accidental workspace breakage; the new module dir is outside the workspace graph).
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cd modules/community-modules/dragon-curve && cargo test` (the module's own suite; run from inside the standalone crate, not via `-p`).

## Authoritative Docs

- `docs/specs/community-modules-dragon-curve-infill.md` - direct read, §§1, 4, 5 (the governing design spec; 279 lines).
- `docs/specs/community-modules-dragon-curve-plan.md` - direct read of the Central Symbol Contract and Grounding Facts (binding; 102 lines).
- `docs/adr/0058-authored-coloring-per-path-tool-carrier.md` - direct read (Accepted; the strip-ungranted rule and linker consequences).
- `docs/14_submodule_programming_languages.md` §Community-module context - delegated SUMMARY for the recorded 225 verdict (the implementer reads only this section, not the whole 172-line file).

## Doc Impact Statement (Required)

- **`none`** - this packet changes no `docs/*.md`, IR schema, WIT contract, scheduler, claim, manifest, host-service, or SDK contract. Those surfaces belong to draft 225/226. The module `README.md` is a module artifact under `modules/community-modules/dragon-curve/`, not a `docs/` edit, and the manual slice-test command documented there is a module-level example, not a contract doc change.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
