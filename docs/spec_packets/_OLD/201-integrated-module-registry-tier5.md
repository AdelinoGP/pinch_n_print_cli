---
status: implemented
packet: 201-integrated-module-registry-tier5
task_ids:
  - ADR-0056
---

# 201-integrated-module-registry-tier5

## Goal

Register integrated modules (embedded manifest TOML, no on-disk `.wasm`) as search tier 5 beneath the four existing search-path tiers, flowing through the one existing ingestion/claims/DAG pipeline with a `ModuleProvenance` marker and a provenance-aware shadow diagnostic, per ADR-0056 Decision items 1–2.

## Problem Statement

ADR-0056 decides that a module compiled into the host binary (an *integrated module*, CONTEXT.md glossary) stays a full citizen of the one existing module model: its manifest flows through the same ingestion, claims, DAG validation, and config-schema machinery as any disk module. Today that pipeline is disk-only: `load_modules_from_roots` (`crates/slicer-scheduler/src/manifest.rs`) discovers `*.toml` files, and `ingest_manifest` hard-fails without a same-stem `.wasm` (`ensure_same_stem_wasm_exists`, `LoadErrorKind::MissingWasm`). There is no provenance notion on `LoadedModule`, no tier beneath the four search-path tiers assembled by `assemble_search_roots` (`crates/slicer-scheduler/src/module_search_path.rs`), and no home for embedded manifests. This packet builds exactly that registration layer — nothing about how an integrated module *executes* (packet 202).

## Architecture Constraints

- ADR-0056 Decision item 1 "One model" requires only that ingestion be **generalized over artifact source**, and that scheduling, claims, and config resolution never learn what "native" means. This packet satisfies that: claims, DAG, and config machinery never branch on provenance.
- **Packet decision (not an ADR clause):** the integrated tier enters at the *loader*, not the root assembler — `assemble_search_roots` keeps returning `Vec<PathBuf>` untouched; tier 5 is a post-roots ingestion phase inside `load_modules_from_roots_with_integrated`. ADR-0056 makes no loader-vs-assembler statement; this choice is argued on its merits in §Rejected alternatives ("Tier 5 as a synthetic search root"). Downstream packets 202 and 203 treat *this file*, not the ADR, as the authority for it.
- ADR-0056 Decision item 2 "Lowest search priority": integrated entries are processed **after** every disk root through the same `seen_ids` set, so first-root-wins dedup by `module.id` is literally the same code path.
- `IntegratedModuleRegistration` carries only manifest text and an origin label. Dispatch information (native entries) is a packet-202 concern that lives at the wasm-host layer, never in `slicer-scheduler`.
- Downstream invariants untouched (verified present at authoring): `dedup_same_claim_modules_with_wall_generator` (`crates/slicer-scheduler/src/execution_plan.rs`), `ExecutionPlanError::DuplicateModuleBinding` (same file), `validate_startup_dag` (`crates/slicer-scheduler/src/validation.rs`). No edit to any of them.
- No wasm-staleness snippet: this packet's change surface (`slicer-scheduler`, `slicer-wasm-host`, `slicer-runtime`, new `slicer-integrated-modules`, docs) is host-only — none of it feeds guest WASM builds per the applies-to list in `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md`.
- No coord-system snippet: manifest/loader wiring, no geometry or mm/unit conversion.

## Data and Contract Notes

- IR/manifest contracts: manifest TOML schema unchanged (`docs/03_wit_and_manifest.md` §Module Manifest Schema); identity stays `[module].id` (reverse-domain string); directory/file stem matters only for disk discovery pairing, which integrated entries bypass. All five `[compatibility]` keys remain required in embedded manifests — they are the same TOMLs staged by `cargo xtask dist` today. Per ADR-0056, integrated modules are version-locked by construction; the compatibility matrix still parses but cannot fail for them (no behavior change needed here — validation runs identically).
- WIT boundary: untouched.
- Determinism/scheduler constraints: integrated entries are appended in registration order after a sorted disk walk (`discover_manifest_paths` sorts); `integrated_registrations()` must return a deterministic order (feature-gated blocks in fixed source order) so module ordering stays reproducible.
- Config keys: none added; any future key must be snake_case per `CLAUDE.md`.

## Locked Assumptions and Invariants

- Integrated `LoadedModule.wasm_path` carries `integrated://<module-dir-name>` and is meaningful as a file path only for `External` provenance (enforced by the `execution_plan_live.rs` guard; documented on the accessor).
- Pre-202, an integrated module that survives dedup gets `wasm_component: None` and fails dispatch loudly via the existing `DispatchPhase::MissingComponent` path (`crates/slicer-wasm-host/src/dispatch.rs`); production cannot reach this in 201 because the default registry is empty. 202 replaces this seam with native routing.
- The shadow-diagnostic wording `external module X shadows integrated module X` is this packet's canonical contract string, matching ADR-0056's consequence bullet verbatim (the id is substituted for `X`; no inner quoting), and becomes a contract consumed by 203's diagnostics surfacing; changing it later requires touching 203's tests.
- Empty-registration behavior is a strict identity (AC-N2) — the reversibility lock.

## Risks and Tradeoffs

- Feature unification: no workspace member may enable a `slicer-integrated-modules` feature in normal/dev/test profiles, or every `--workspace` build silently grows an integrated tier. Mitigation: loader tests construct `IntegratedModuleRegistration` values inline; only the registry crate's own AC command passes `--features classic-perimeters` explicitly.
- `ingest_manifest` refactor risk: the disk wrapper must preserve error ordering (MissingWasm before TOML parse) — `manifest_ingestion_tdd.rs` already pins this; run the whole `--test scheduler_integration` binary in Step 2's exit.
- The 202 signature extension of `load_live_modules_for_plan_with_integrated` (native-entry table parameter) is a known planned change; 201 must not accrete other callers of the new entry point beyond `run.rs` to keep that churn bounded.
