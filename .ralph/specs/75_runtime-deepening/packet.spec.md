---
status: active
packet: 75
task_ids: [TASK-216, TASK-217, TASK-218, TASK-219]
backlog_source: docs/07_implementation_status.md
---

# Packet 75 — `slicer-runtime` deepening: four shallow→deep refactors

## Goal

Turn four shallow, copy-pasted areas of `slicer-runtime` into deep modules — concentrating wiring bugs in one
place (locality) and paying one implementation back across many call sites (leverage) — with **zero behaviour
change**, validated by the existing suite plus targeted regression tests. Delivered as four phases under one
packet, gated independently, one commit per phase.

## Scope Boundaries

Host-runtime structural refactors only. The canonical WIT (`crates/slicer-schema/wit/`), the component ABI, and
guest crates are **untouched** — no phase changes a guest input, so `cargo xtask build-guests --check` stays clean
throughout. The four phases and their files:

- **Phase 1 (TASK-216)** — PrePass stage runner: `crates/slicer-runtime/src/prepass.rs`.
- **Phase 2 (TASK-217)** — Pure IR harvest extraction: `crates/slicer-runtime/src/dispatch.rs`,
  `wit_host.rs` (visibility change only).
- **Phase 3 (TASK-218)** — WIT marshalling `with:` unification: `crates/slicer-runtime/src/wit_host.rs`
  (host-only bindgen remap; ABI stable).
- **Phase 4 (TASK-219)** — Model intake assembly seam: `crates/slicer-runtime/src/model_loader.rs`,
  `helpers_cmd.rs`; CONTEXT.md glossary sharpen.

Out of scope (noted as future deepenings in `design.md`): the all-prepass-ordering declarative graph (Phase 1
deferral); the layer-world-only region-view accessors / builder `push_*` repetition (Phase 3 deferral); the 3MF
XML-parser decomposition (Phase 4 deferral).

Full in/out lists and rationale live in `requirements.md`; per-phase decisions in `design.md`; stepwise execution
in `implementation-plan.md`.

## Acceptance Criteria

> Verification assumes repo root `F:\slicerProject\pinch_n_print` and a POSIX shell (Git Bash). Integration tests
> bucket into 5 binaries (`unit|contract|executor|integration|e2e`); run `--test <bucket> <module_filter>`. Inline
> `src/` tests run via `--lib`.

**AC-1.1 — PrePass built-in brackets are unified (Phase 1).**
Given the six host-built-in stages, When `prepass.rs` is inspected, Then a single `run_builtin_stage` helper owns
the guard/size/instrument/finish bracket and the six stages are driven through it (no six inline
`StageInstrumentationGuard::start` brackets for built-ins). | `grep -c "fn run_builtin_stage" crates/slicer-runtime/src/prepass.rs` → `1`

**AC-1.2 — PrePass ordering + commit behaviour unchanged (Phase 1).**
Given the runner preserves interleaving and the phase-split, When the ordering tests run, Then they pass. |
`cargo test -p slicer-runtime --test executor prepass_execution_order_tdd prepass_executor_tdd` → all pass (exit 0)

**AC-1.3 — Bracket-per-built-in regression locked (Phase 1).**
Given an instrumentation spy, When the prepass runs the built-ins, Then exactly one `on_stage_end` is emitted per
built-in in declared order. | `cargo test -p slicer-runtime --test integration run_pipeline_with_instrumentation_tdd::prepass_builtins_emit_one_stage_end_each_in_declared_order` → pass.

**AC-2.1 — Harvest cores are pure and testable without WASM (Phase 2).**
Given the dispatch harvest logic, When inspected, Then each `harvest_*` delegates to a `harvest_*_from(<vec>)`
pure core. | `grep -c "fn harvest_layer_plan_ir_from\|fn harvest_seam_plan_ir_from\|fn harvest_support_plan_ir_from" crates/slicer-runtime/src/dispatch.rs` → `3`

**AC-2.2 — `parse_canonical_region_id` has one definition (Phase 2).**
Given the de-dup, When both files are searched, Then exactly one `fn parse_canonical_region_id` definition
remains (in `wit_host.rs`). | `grep -rc "fn parse_canonical_region_id" crates/slicer-runtime/src/dispatch.rs crates/slicer-runtime/src/wit_host.rs | grep -v ':0' | wc -l` → `1`

**AC-2.3 — Pure harvest tests pass (Phase 2).** | `cargo test -p slicer-runtime --lib` → all pass (exit 0)

**AC-3.1 — Cross-world type identity via `with:` remap (Phase 3).**
Given the unification, When the prepass/finalization/postpass `bindgen!` blocks are inspected, Then each remaps
`slicer:types/geometry` to the layer world. | `grep -c "\"slicer:types/geometry\": super::layer" crates/slicer-runtime/src/wit_host.rs` → `3`

**AC-3.2 — Redundant per-world converters deleted (Phase 3).**
Given type identity, When searched, Then the duplicate prepass/finalization/postpass ExPolygon converters are
gone. | `grep -c "fn p_wit_to_ir\|fn f_wit_to_ir\|fn pp_wit_to_ir" crates/slicer-runtime/src/wit_host.rs` → `0`

**AC-3.3 — Builds with type identity; ABI unchanged (Phase 3).** |
`cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings` → exit 0; and
`cargo xtask build-guests --check` → no `STALE:`.

**AC-4.1 — One assembly seam; z-extent duplicate gone (Phase 4).**
Given the seam, When inspected, Then `assemble_object` exists, `compute_z_extent_for_component` is deleted, and a
single identity-transform helper remains. | `grep -c "fn assemble_object" crates/slicer-runtime/src/model_loader.rs` → `1` (≥1); and `grep -rc "fn compute_z_extent_for_component" crates/slicer-runtime/src/helpers_cmd.rs` → `0`

**AC-4.2 — Model + convert behaviour unchanged (Phase 4).** |
`cargo test -p slicer-runtime --test integration model_loader_tdd model_writer_roundtrip_tdd threemf_transform_tdd threemf_sidecar_classification_tdd` and
`cargo test -p slicer-runtime --test unit world_z_below_floor_tdd non_uniform_scale_tdd` and
`cargo test -p pnp-cli --test helpers_cli` → all pass (exit 0).

**AC-4.3 — Single-component z-extent equivalence locked (Phase 4).**
Given convert's single-component split, When routed through `assemble_object`, Then the recomputed extent equals
the reused parent extent. | new regression test passes.

**AC-4.4 — Glossary sharpened (Phase 4).**
Given **Split to objects** is a CLI user choice too, When CONTEXT.md is inspected, Then the entry no longer pins
the operation to the GUI. | `grep -A4 "### Split to objects" CONTEXT.md | grep -c "in the GUI"` → `0`

**AC-CLOSE — Full suite green (packet close).**
Full `cargo test --workspace` via sub-agent → `FACT pass`; e2e CLI slice on STL + 3MF fixture succeeds; reference
`.gcode` byte-identical to pre-refactor.

## ADRs

- **ADR-0001** (`docs/adr/0001-prepass-builtins-commit-in-stage.md`) — Phase 1.
- **ADR-0002** (`docs/adr/0002-wit-marshalling-type-unification.md`) — Phase 3.
