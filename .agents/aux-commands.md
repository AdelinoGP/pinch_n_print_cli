# Auxiliary Commands — Benchmarks & HTML Slicer Report

**When to read this:** when the user asks to run benchmarks, or when you need the `--report` HTML slicer report for debugging a slice. These commands are slow and not in CI — load them only when needed.

Keywords: benchmarks, bench, polygon_ops, mesh_ops, pipeline, per_stage, wasm_modules, HTML report, slicer report, debugging

---

## Benchmarks (slow; not in CI)

```bash
# Native — fast, no WASM needed:
cargo bench -p slicer-core    --bench polygon_ops
cargo bench -p slicer-helpers --bench mesh_ops
# Host:
cargo bench -p slicer-runtime --bench pipeline       # instrumentation overhead
cargo bench -p slicer-runtime --bench per_stage      # plan-freeze serial-edge helpers
cargo bench -p slicer-runtime --bench wasm_modules   # v1 stub; needs cargo xtask build-guests
```

## HTML slicer report (debugging)

```bash
cargo run --bin pnp_cli --release -- slice \
    --model resources/benchy.stl \
    --module-dir modules/core-modules \
    --output /tmp/out.gcode \
    --report /tmp/slicer-report.html         # opt-in; zero overhead when absent
```

See `docs/16_slicer_report.md` for format, allocator contract, and known v1 limitations.