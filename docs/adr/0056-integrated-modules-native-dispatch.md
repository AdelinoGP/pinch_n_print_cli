# ADR-0056 — Integrated modules: native dispatch inside one module model

<!-- filename: 0056-integrated-modules-native-dispatch -->

## Status

Accepted (2026-08-07). Decided in a design-grilling session for the
multi-edition distribution plan. Partially supersedes ADR-0033's rejected
alternative "host-native-only module"; enabling decision for ADR-0057
(editions). Companion glossary terms **integrated module** / **external
module** in `CONTEXT.md`.

## Context

Three goals drove this:

1. **Deployment** — end users who never develop core modules should receive a
   self-contained binary, not a binary plus a directory of 21 loose
   `.wasm`/`.toml` pairs (today's only layout, staged by `cargo xtask dist`).
2. **Platforms** — future targets where wasmtime cannot ship need the pipeline
   to run without a WASM runtime: iOS forbids JIT, and a browser/library build
   cannot host wasmtime at all. Desktop (Windows/macOS/Linux), ARM SBCs,
   mobile, and browser/library embedding are all in scope long-term.
3. **Performance** for users who never modify modules.

The measured record reshaped goal 3. Per ADR-0055, the WIT boundary itself is
cheap (67,300 `medial-axis` host calls per benchy slice with per-call overhead
at or below the noise floor), and `arachne-perimeters`' heavy half already runs
natively through the ADR-0033 host-service bridge. The dominant cost is
single-threaded clipper2 executing *inside* guests — `classic-perimeters`
measured at 90% of per-layer module CPU, roughly 39% of it polygon ops
(ADR-0055), and `support-planner`'s prepass time is 98% geometry (ADR-0049) —
and a guest cannot spawn threads (ADR-0049). Slice-time performance is
therefore chiefly the batched host-service workstream (DEV-094), which
benefits every edition and community modules without any edition split. Native
compilation of modules is chiefly a **deployment and portability** decision,
not the perf lever.

ADR-0033 rejected "make `arachne-perimeters` a host-native-only module"
because it "would require a second module-loading code path." This decision
removes that premise instead of overruling the concern: there remains exactly
one module-loading model.

## Decision

**Core module crates become single-source, dual-target: the same crate
compiles both to a WASM component (as today) and natively into the host
binary. A natively compiled module is an *integrated module* and remains a
full citizen of the one existing module model.**

1. **One model.** An integrated module embeds its manifest TOML
   (`include_str!`) and passes through the identical ingestion, claims, DAG
   validation, and config-schema machinery as any external module (the
   `load_modules_from_roots` pipeline in
   `crates/slicer-scheduler/src/manifest.rs`, generalized over artifact
   source). Scheduling, claims, and config resolution never learn what
   "native" means.
2. **Lowest search priority.** Integrated modules form a new tier 5 beneath
   the four existing search-path tiers (`assemble_search_roots` in
   `crates/slicer-scheduler/src/module_search_path.rs`); first-root-wins dedup
   by `module.id` is unchanged. An external module with the same id at any
   higher tier overrides the integrated one and dispatches over the WASM path
   — user override falls out of the existing mechanism.
3. **Dispatch-level split only.** Provenance decides dispatch — direct native
   call vs WASM instantiation — behind the ADR-0005 runner-trait seam
   (`LayerStageRunner` and siblings in
   `crates/slicer-wasm-host/src/traits.rs`). The `#[slicer_module]` macro
   grows a native adapter emitting the same stage contract natively that the
   wit-guest shim emits for wasm32.
4. **Parity gate.** A module ships integrated only with a contract test
   running the same inputs through both dispatch paths and asserting
   structural invariants (ADR-0042) plus tolerance-based IR comparison.
   Byte-equality is explicitly not the gate: DEV-093 run-to-run
   nondeterminism and expected ULP-level libm/codegen drift between wasm32 and
   native would make it flake, inviting test-weakening. Residual divergence is
   recorded in `docs/DEVIATION_LOG.md`.
5. **Single-threaded module logic on both paths (default).** An integrated
   module does not use internal parallelism by default — that would diverge
   from its wasm twin. Parallelism comes from batched host services
   (ADR-0049 / DEV-094) and host-side layer fan-out. Per-module internal
   parallelism is a later, per-module decision requiring a deterministic
   merge.
6. **No-wasm builds.** On targets where wasmtime cannot ship, the wasm host is
   compiled out; external modules found on the search path are skipped with a
   loud per-module diagnostic naming the reason. Desktop/SBC builds always
   keep wasmtime, so extension and override work identically there in every
   edition.

## Rejected alternatives

- **Embed WASM artifacts in the binary (`include_bytes!`, optionally
  precompiled `cwasm`).** Single-file deployment, but zero slice-time gain
  (the boundary is not the cost), wasmtime still required everywhere, and no
  path to wasm-less platforms. Startup-time precompilation may still happen
  independently; it is not this decision.
- **Register integrated modules as host builtins (ADR-0024 `Producer`
  shape).** Less macro work, but forks the claims/config/override model into a
  special case — precisely the "second path" ADR-0033 warned about.
- **Hand-written native ports of module logic.** Permanent parity liability
  between two implementations of every hot algorithm.
- **Bridges only, no integration.** Completing DEV-094 delivers the
  performance but neither single-file deployment nor wasm-less platforms.

## Consequences

- ADR-0033's status notes the partial supersession; its four-layer bridge
  remains the pattern for guest access to host-only algorithms, and its
  `cfg`-split SDK wrappers are exactly what lets one crate run its geometry
  natively when compiled in.
- Integrated modules are version-locked to the host by construction — the
  `[compatibility]` matrix cannot fail for them; it still applies to external
  overrides.
- An edition must never stage an external copy of a module it integrates: a
  higher search tier would silently shadow the native path (first-root-wins),
  negating the edition. The duplicate-id diagnostic becomes provenance-aware
  ("external module X shadows integrated module X").
- Guest-WASM staleness discipline (`CLAUDE.md`) still applies to the wasm twin
  of every integrated module; parity contract tests require both artifacts
  fresh.
- A browser/library target additionally requires `host-algos` (rayon,
  boostvoronoi) to build for wasm32 hosts — unresolved research, explicitly
  out of scope here.
