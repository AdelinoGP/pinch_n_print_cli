---
status: implemented
packet: layer-module-preparation_07_lightning-geometry-portability-gate
task_ids:
  - TASK-576
---

# layer-module-preparation_07_lightning-geometry-portability-gate

## Goal

Produce the independently recorded canonical input oracle for the five Classic lightning case
families and decide Gate A: whether a restricted earlier-source reconstruction reproduces canonical
planning domains, parameters, and resolved policy. Record a truthful `pass`/`fail`/`inconclusive`
outcome. A failed or inconclusive reconstruction is a completed investigation, not an incomplete
packet.

## Problem Statement

The recorder now produces independent canonical planning domains, scalars, resolved policy,
planning-boundary provenance, and replay records for the five Classic case families. What is
missing is a portable oracle and an independent reconstruction: the gate must show whether a
restricted earlier-source description is sufficient to rebuild the same planning domains, or
whether the strategy fails/inconclusive for a named reason. The previous bundled packet could not
close because it made Gate B and exhaustive final-fill provenance prerequisites of Gate A. This
packet isolates Gate A and accepts a truthful negative result.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Exact freshness outcomes:0 fresh;1 stale or lock divergence (rebuild or `--sync-locks` respectively);3 infrastructure unable to form an opinion, stop. Ordinary/accelerated artifacts are distinct; this gate uses ordinary artifacts. No guest failure attribution before this check. Guest execution uses `cargo xtask test`.

## Risks and Tradeoffs

- The source-only reconstruction may not match canonical on some family. That is a valid Gate A
  `fail` and the packet's product.
- Fixture conversion for five families is the main implementation cost; it is deterministic and
  driven from recorded artifacts.
- A `fail` blocks migration and returns the strategy for design review; it does not block packet
  closure.
