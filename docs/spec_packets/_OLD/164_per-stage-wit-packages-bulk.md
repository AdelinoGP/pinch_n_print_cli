---
status: implemented
packet: 164_per-stage-wit-packages-bulk
task_ids:
  - TASK-146c
---

# 164_per-stage-wit-packages-bulk

## Goal

Migrate every remaining WASM-dispatched stage — 8 layer stages and 4 prepass stages (12 packages; `PrePass::PaintSegmentation` is host-built-in since packet 97 and gets none) — onto the per-stage versioned WIT-package machinery packet 163 built, and retire the now-unfalsifiable tier-world surface: the manifest `wit-world` key, `SUPPORTED_WIT_WORLDS`, and `validate_wit_world`.

## Problem Statement

Packet 163 proved the per-stage versioned-package mechanism on the three cheapest stages, deliberately leaving two contract mechanisms live in-tree: postpass/finalization on per-stage packages, layer/prepass on monolithic tier worlds. That intermediate is an accepted deviation **owned by this packet**, and the plan's dependency note is blunt about why it cannot linger: leaving it undone reproduces the exact failure ADR-0045 exists to end (cf. `run.rs`'s 2026-05 "pragmatic fix", still load-bearing 14 months later). The layer tier is where the original pain landed — `arachne-perimeters` was invalidated by packet 130's infill change — and it is still exposed: every layer module ships benign-`Ok` padding for 7 sibling stages it never implements, violating ADR-0015 by construction. Meanwhile `wit-world` / `SUPPORTED_WIT_WORLDS` / `validate_wit_world` continue comparing one hand-written string to another (ADR-0044's "unfalsifiable ceremony"), and after 163 they additionally name two packages that no longer exist.

**Grounded counts (falsifies the plan's arithmetic — see `design.md` for evidence):** the plan and ADR-0045 say "17 packages: 10 layer + 4 prepass + 2 postpass + 1 finalization". The tree says otherwise: `slicer_schema::STAGES` has 16 rows — **8** Layer, **5** PrePass, 2 PostPass, 1 Finalization — and `world-layer.wit` declares 8 stage exports (10 was stage exports **plus** the two lifecycle exports packet 162 deletes). Of the 5 PrePass rows, `PrePass::PaintSegmentation` is **host-built-in** (packet 97; executes in `crates/slicer-runtime/src/prepass.rs`, no WIT export in `world-prepass.wit`, no core module). The delivered end state is therefore **15 per-stage packages** (8+4+2+1), of which 163 shipped 3 and this packet ships 12.

## Architecture Constraints

- **Consume 163's decisions; do not re-derive them.** The naming rule (`slicer:<tier>-<stage-local-kebab>@1.0.0`, tier from `StageSpec.tier_id` (named `world_id` when 163 wrote this), never from splitting `stage_id`, never from `wit_export`), `wit_export == "run"`, the imported-`-types`/exported-`run` shape, fatal-on-miss with the engine's expected-only diagnostic, `@1.0.0` as mechanically load-bearing (`alternate_lookup_key` major-track requires major ≥ 1), and conservative `stage_wit_mtime` all come from `.ralph/specs/163_per-stage-wit-packages-pilot/design.md` §"Exports handed to packet #3".
- **A resource in an exported interface is guest-owned** (ADR-0045 §"The naive shape inverts resource ownership"). The four prepass resources are host-implemented today (`crates/slicer-wasm-host/src/host.rs` prepass resource impls), so each moves to that stage's **imported** `<iface>-types` interface. The 8 layer packages need **no** `-types` companion — `world-layer.wit` takes every type from the imported `slicer:ir-handles/ir-handles` / `slicer:config/config-types` / `slicer:common/module-errors`, exactly the shape 163 predicted ("most layer packages will need no `-types` companion at all" — confirmed: all 8).
- **One Rust type set across worlds (ADR-0002).** Every new `bindgen!` mod repeats the five-key `with:` block verbatim from 163's pilot mods. The new shared `slicer:prepass-types/prepass-types` interface follows the same discipline: one mod defines its bindings; every other mod importing it aliases via `with:`. Records need this as much as resources do — without the alias, `MeshObjectView` becomes two distinct Rust types and every host converter forks.
- **ADR-0006:** all new columns/lookups read `STAGES`; no parallel table anywhere (including `xtask`, which already goes through `wit_dir_for_stage_id`).
- **ADR-0015 / packet 181:** fatal-on-miss replaces the padding arms; no `Ok(())` stub survives for any stage. missing-component dispatch defect is closed: all five `MissingComponent` arms are fatal and `dispatch_missing_component_tdd::missing_component_is_fatal_for_all_five_stages` is the existing regression gate. Packet 164 may move those arms as part of dispatch restructuring, but must preserve the fatal behavior and must not reintroduce success laundering.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- **IR/manifest contracts:** `[stage] id` unchanged and still singular; `wit-world` deleted (legacy key tolerated-ignored — AC-N3); no IR schema or claim moves; config keys stay snake_case.
- **WIT boundary:** the central risk 163 retired mostly survives at larger n — resource identity across **15** `bindgen!` calls instead of 5. 163 proved the mechanism with real imports and `with:`-mapped resources; this packet adds two new wrinkles: (a) the canonical `with:` definer moves from the dying `layer` mod to `layer_perimeters`, so all 15 mods' alias value paths change in one sweep — a mismatch is a compile/link error of the `CLAUDE.md` §"WIT/Type Changes Checklist" kind, loud not subtle; (b) `slicer:prepass-types` is a *new* shared interface crossing two mods — the executor prepass round-trips (seam-planning and support-geometry both marshal `mesh-object-view`) are its falsifier. If either breaks in a way that cannot be fixed by path correction, stop and report — it would falsify 163's generalization, not just this packet.
- **Prepass resources stay `with:`-unmapped**, exactly like 163's pilot resources: bindgen markers + `ResourceTable` push. Do not add `with:` entries for them.
- **Determinism/scheduler:** untouched. `STAGE_ORDER`, DAG planning, `[stage] id` ingestion all unchanged; no WASM instantiated during planning (ADR-0006's rejected alternative stays rejected).
- **PaintSegmentation is the one deliberate non-package row.** Any future guard asserting "every STAGES row has a package" is wrong by design; the guard shape is "every row except the documented host-built-in set".

## Locked Assumptions and Invariants

- Package names/versions per the name table are a public contract at `@1.0.0` the moment this lands; breaking them later costs what ADR-0045 §Consequences says it costs. `every_stage_package_major_is_at_least_one` (163) already enforces `major >= 1` over the 12 new packages with zero new code.
- `slicer:prepass-types` is **unversioned** and shared — the same status as `slicer:common`/`slicer:ir-handles`; a breaking change to it is a cross-stage event by design.
- After this packet, `qualified_export_for_stage_id` is total over WASM stages and `None` exactly for unknown ids and `PrePass::PaintSegmentation`.
- Two-mechanism intermediate ends here; the deviation row closes (AC-8). No successor packet inherits contract-migration work.
- `stage_wit_mtime` stays conservative for stage-less guests. Never invert to make AC-N2 pass.
- Reversibility: breaking contract change, no flag; revert = revert the packet.

## Risks and Tradeoffs

- **The `with:` canonical-definer move is the highest-risk edit** (15 mods × 5-6 alias strings, all must agree). Mitigation: do it in the same step that creates the mods, gate with `cargo check -p slicer-wasm-host`, and treat any `imported interface ... has the wrong type` linker error as a path mismatch before suspecting the design.
- **Tree is red from the WIT step until dispatch lands** (six layers move together, as in 163). The step order pins this; no compile exit before the gate step.
- **`dispatch_layer_call` restructuring moves ~8 marshalling arms.** The arms' bodies are copy-moves, but the surrounding pool-lease/store/config-handle scaffolding is per-call and must be replicated per arm or hoisted; the step contract requires the executor suite (unfiltered), AC-N4, and packet 181's existing `missing_component_is_fatal_for_all_five_stages` gate as falsifiers. The five fatal `MissingComponent` arms may move with the routers, but their behavior must not change.
- **`prepass-guest` may pad multiple prepass stages today.** Its consumer survey decides narrow-vs-split; wrong narrowing surfaces as a `0 passed` guard trip in the affected suite, which is why every name-filtered gate matches `^test result: ok\. [1-9][0-9]* passed` (a pattern that also fails on a FAILED run).
- **Manifest-fixture fallout is wide but shallow**: every test helper that writes `wit-world` or asserts `.wit_world()` breaks at compile time — enumerable via `rg`, mechanical to fix, but easy to under-count; the step allocates a dispatch for the inventory.
- **20 + 15 mechanical file edits** strain the ≤3-edits-per-step rule; the plan groups them into explicit sweep steps with per-file one-line contracts rather than pretending they fit.
