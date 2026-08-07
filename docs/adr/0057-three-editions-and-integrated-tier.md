# ADR-0057 — Three editions and the integrated-module search tier

<!-- filename: 0057-three-editions-and-integrated-tier -->

## Status

Accepted (2026-08-07). Companion to ADR-0056, which decides *how* a module
integrates; this ADR decides *what ships*.

## Context

The only distributable today is `cargo xtask dist`'s loose layout — `pnp_cli`
plus `modules/<name>/{<name>.wasm,<name>.toml}` (21 core modules), never
exercised in CI. Users who will never develop modules still receive, and can
accidentally break, the loose module set; there is no artifact for wasm-less
platforms; and module developers need today's layout preserved.

## Decision

Ship three **editions** (canonical term — "distribution profile" is avoided
because "profile" is already overloaded by cargo build profiles and the fuel
`--profile` flag of ADR-0055):

| Edition | Integrated set | External core modules staged |
|---|---|---|
| **Developer** | none — today's layout, unchanged | all |
| **Hybrid** | evidence-driven hot set | the rest, loose beside the binary |
| **Integrated** | every core module | none |

- The Hybrid set is seeded from the measured record — `classic-perimeters`,
  `arachne-perimeters`, `support-planner` — and finalized by fuel/wall-clock
  profiling at packet time. It is a dist-configuration list, not a hardcoded
  constant.
- Every edition keeps external-module loading (and per-id override of
  integrated modules) wherever a WASM runtime ships; wasm-less platform builds
  are per-target variants of the Integrated edition per ADR-0056.
- New `--no-integrated-modules` flag disables the integrated tier entirely
  (for module developers testing pure-external setups on Hybrid/Integrated
  binaries). `--no-default-module-paths` keeps its current meaning (drops the
  config-dir and exe-dir tiers only); the flags compose.
- Disjointness invariant: an edition's staged external set and its integrated
  set never intersect (ADR-0056 consequence).
- Delivery phases: (1) DEV-094 batched host bridges — perf for all editions,
  no edition machinery; (2) integration infrastructure + Hybrid pilot with
  parity gates; (3) Integrated edition, xtask edition support, CI artifacts;
  (4) platform builds (aarch64 matrix, iOS AOT, browser research) — deferred.

## Consequences

- `cargo xtask dist` grows an edition dimension, and CI gains
  build/verification of the edition artifacts (today CI never runs `dist` at
  all — the shipped layout is unverified).
- `pnp_cli` module listing/diagnostics surface provenance: integrated vs
  external, and any shadowing of an integrated module by an external one.
- Release/download pages use the edition names as user-facing vocabulary.
