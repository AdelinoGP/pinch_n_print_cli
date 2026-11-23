# `PostPass` (`LayerFinalization` + `GCodeSerialize` + `GCodePostProcess`) attribution and speedup screen

Type: task / attribution + mechanism screen (no production code edit)
Label: wayfinder:task
Status: open
Assignee: unassigned
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t185 full-matrix attribution and 8/8 closure headroom map](186-post-t185-full-matrix-attribution.md)

## Question

**Now that `PostPass` owns `14.4–15.1 %` (`403–449 ms`) of `benchy` supports-on, `10.4–10.8 %` (`2,043–2,162 ms`) of `base` supports-on, and `7.6–10.5 %` of supports-off — with all three top stages (`LayerFinalization` `1.8–6.7 %`, `GCodeSerialize` `2.7–3.7 %`, `GCodePostProcess` `2.1–3.0 %`) clearing the `0.80 %` floor at `2.3–8.3×` in ALL 8 CELLS — where does the serial/host time go inside each stage, and what exact (byte-identical to `t182/reference.json`) bundle clears the literal all-8-cell `0.80 %` standing gate?**

## Scope

- Substage attribution inside:
  - `PostPass::LayerFinalization` (`174–201 ms` `benchy-on`, `671–701 ms` `base-on`; WASM `skirt-brim`, `overhang-classifier-default`, `part-cooling`, `wipe-tower`, and host view/marshal/merge overhead),
  - `PostPass::GCodeSerialize` (`98–113 ms` `benchy-on`, `526–595 ms` `base-on`; `host:gcode_serialize` formatting/buffering/parallelism),
  - `PostPass::GCodePostProcess` (`82–88 ms` `benchy-on`, `414–436 ms` `base-on`; `com.core.machine-gcode-emit` + host marshal),
  - `PostPass::GCodeEmit` (`42–46 ms` `benchy-on`, `425–429 ms` `base-on`).
- Screen exact (zero G-code drift) speedups across all 8 cells; note the dual payoff of cutting `benchy-on` toward `8/8 < 1.0x` while widening `benchy-off`'s `< 1.0x` margin (`0.869x`/`0.880x`) before the closing Orca scoreboard.
- Zero production code edits landed in this ticket; graduate an implementation & standing-gate ticket for the winning exact bundle.

## Pre-declared gate

- Attribution & screen only (no production code edits); any temporary probe patch reverted before resolution, `cargo xtask build-guests --check` exit 0, and `100 %` byte-identical to `t182/reference.json`.
- Any graduated implementation candidate must be priced against the literal all-8-cell `0.80 %` standing gate.

## Acceptance

- Reduced evidence under `docs/specs/perf-vs-orca/evidence/t189-postpass-screen/` with `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`--self-test` exit 0).
- Updated `## Answer`, `map.md`, and graduated implementation ticket (if a priced exact mechanism clears the all-8-cell gate).
