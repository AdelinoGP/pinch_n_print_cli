# Dragon Curve Community Module — Packet Queue Plan

Approved by the user 2026-08-13. Governing design spec:
`docs/specs/community-modules-dragon-curve-infill.md` (grilling complete).

This plan decomposes that spec into 4 spec packets, authored in dependency order
by subagents and independently preflighted by reviewer subagents. The
orchestrator authors no packet files.

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 225-dragon-curve-feasibility-gate | Bump wit-bindgen 0.60.0 + wasmtime 47.0.3 and re-run the Go/MoonBit feasibility probes, recording verdicts in docs/14. | TASK-336 | - | generated | docs/spec_packets/225-dragon-curve-feasibility-gate/ |
| 2 | 226-authored-coloring-carrier | Land the per-path `tool-index` WIT carrier plus the two-sided authored-coloring grant, tool-count query, linker tool-equality guard, and DEV-135 deviation row. | TASK-337 | #1 | generated | docs/spec_packets/226-authored-coloring-carrier/ |
| 3 | 227-dragon-curve-community-module | Author the dragon-curve community module (dragon tiling + deterministic color mapping) at `modules/community-modules/dragon-curve/` with manifest, config schema, banner README, build script, and manual slice test doc. | TASK-338 | #1, #2 | generated | docs/spec_packets/227-dragon-curve-community-module/ |
| 4 | 228-community-module-docs-banner | Land the social-rule docs: CLAUDE.md community-module instruction, docs/ labeled-example note, and docs/07 backlog rows for all four tasks. | TASK-339 | #3 | generated | docs/spec_packets/228-community-module-docs-banner/ |

Packet status on emission: `draft` (all four). No packet may set `active`.

## Central symbol contract (binding on all authoring subagents)

These names are pre-reconciled against the tree and the design spec. Every
packet must use exactly these spellings; reviewer subagents check consistency.

- WIT record field on `slicer:types/geometry.extrusion-path3d`:
  **`tool-index: option<u32>`** (`None` = host decides). Rust mirror field on
  `slicer_ir::ExtrusionPath3D` (`crates/slicer-ir/src/slice_ir.rs`):
  **`tool_index: Option<u32>`**, `#[serde(default)]`.
- Capability claim string (manifest `[claims].holds`): **`claim:authored-coloring`**.
- Config key (ResolvedConfig + per-region overrides): **`fill_authored_coloring`**,
  a list of fill-role claim strings, e.g. `["claim:sparse-fill"]`.
- Host service exposing tool count (new `slicer:common/host-services` function):
  **`tool-count: func() -> u32`**; SDK wrapper **`slicer_sdk::host::tool_count()`**.
- Deviation row to create in packet #2: **`DEV-135`** (highest existing is
  DEV-134 — re-derive at point of use, never trust this file).
- Module id for packet #3: **`com.example.dragon-curve`**; directory
  **`modules/community-modules/dragon-curve/`**.

## Grounding facts (verified 2026-08-13; subagents must re-verify against the tree)

- Workspace toolchain: `Cargo.toml` pins `wasmtime = "43.0.0"` (features
  `call-hook`) and `wit-bindgen = "0.57.1"`. Gate targets: wasmtime 47.0.3,
  wit-bindgen 0.60.0.
- This machine: Go 1.26.5 present; MoonBit binary absent; wasm-tools 1.250.0
  present. The MoonBit probe therefore cannot be re-run unless MoonBit is
  installed — the packet must record "not re-run (toolchain absent)" and treat
  the Go verdict as the gate-deciding evidence.
- Probes: `docs/feasibility-probes/go-wasm.md` (Go: WASI preview2 import
  blocker), `docs/feasibility-probes/moonbit-wasm.md` (MoonBit: UTF-16/UTF-8
  string corruption). Living verdict table: `docs/14_submodule_programming_languages.md`
  §Community-module context (Go row, MoonBit row, and the section paragraph at
  "the two probes are complete and neither is loadable-and-correct").
- `crates/slicer-schema/wit/deps/types.wit` declares `package slicer:types;`
  (unversioned) with `record extrusion-path3d { points, role, speed-factor }`.
  ADR-0044: WIT world versions are advisory and erased from guest binaries.
  The spec's "bump the slicer:types/geometry package version" therefore means a
  doc-visible annotation only, never a manifest `wit-world` change.
- `resolve_region_tool_index` (`crates/slicer-wasm-host/src/dispatch.rs`) is the
  sole host per-region coloring resolver; its infill-output call site stamps
  `entity.tool_index`.
- Infill linker guard: `paths_compatible` (`modules/core-modules/infill-linker/src/orchestrate.rs`)
  currently compares role + speed_factor bits + endpoint widths — it must add
  tool equality. `chain_or_connect_infill` (`modules/core-modules/infill-linker/src/connect.rs`)
  is the linker entry that clones/re-emits paths.
- ADR-0058 (`docs/adr/0058-authored-coloring-per-path-tool-carrier.md`) is
  Accepted and governs packet #2 — its Consequences section mandates the linker
  tool-equality guard and the wipe-tower cost note. No new ADR is needed for
  the mechanism.
- CONTEXT.md already carries the **Community module** and **Authored coloring**
  glossary entries (grill-era landing). Packet #4 must NOT re-add them.
- Backlog `docs/07_implementation_status.md`: highest TASK id 335; the four
  task rows (TASK-336..339) do not exist yet — packet #4 creates them.
- No packet currently has `status: active`.
- WIT host-services interface lives at `crates/slicer-schema/wit/deps/common.wit`
  (`slicer:common/host-services`). It has no tool-count function today.
- `ResolvedConfig` is declared via `declare_resolved_config!` in
  `crates/slicer-ir/src/resolved_config.rs`; fill-role holders
  (`sparse_fill_holder` etc.) are `cli "..." String = ... => extract_string`.
  A list-valued field needs a list extractor; `bed_shape` uses
  `extract_float_list` — no `extract_string_list` exists yet (net-new if used).
- Claim vocabulary: `claim:sparse-fill` etc. are `claim:*` strings; `should_emit`
  (`crates/slicer-sdk/src/views.rs`) gates emission on the held set.

## Execution mode

4 packets → orchestrated per `.claude/skills/spec-packet-generator/references/batch-protocol.md`:
authoring subagents write packet files; independent reviewer subagents run
`spec-review --preflight <packet-dir>` (S0–S8 gate per
`.claude/skills/spec-review/references/preflight-gate.md`) and return only the
gate table + verdict. The orchestrator reads only each `packet.spec.md` to
check plan conformance, and never opens `design.md` / `implementation-plan.md`.

Nothing in this plan may be committed by the orchestrator; the plan file and
the four packet directories should be committed together by the user.

## Resume instruction

If interrupted: read this plan file in full, select the first `pending` row
whose dependencies are all `generated`, rebuild the exports ledger with one
SUMMARY dispatch per generated dependency, and continue with author+reviewer
dispatches in dependency order.
