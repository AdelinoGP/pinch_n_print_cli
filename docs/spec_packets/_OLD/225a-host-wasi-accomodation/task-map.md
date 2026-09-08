# Task Map: 225a-host-wasi-accomodation

`TASK-336` is reused from packet 225: it is the same feasibility gate continued under ADR-0060's accommodating host. Packet 228 creates the `docs/07_implementation_status.md` row covering `TASK-336`; this packet extends that row's scope and makes no docs/07 edit. No OrcaSlicer references apply.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-336` | Step 1 | `docs/adr/0060-host-wasi-accommodation-for-foreign-language-guests.md` | `Cargo.toml`, `crates/slicer-wasm-host/{Cargo.toml,src/host.rs,src/instance.rs,src/dispatch.rs}` | none | `M` | Default-deny preview2 host accommodation; proves AC-1 through AC-5. |
| `TASK-336` | Step 2 | probe fixture README | `crates/slicer-wasm-host/tests/integration/foreign_language_feasibility_tdd.rs` | none | `S` | Independent oracle gains matching WASI wiring; proves AC-6 and AC-N1. |
| `TASK-336` | Step 3 | MoonBit record and fixture README | `docs/feasibility-probes/moonbit-text-postprocess.md` | none | `M` | Released generator re-measurement with full trap diagnostics. |
| `TASK-336` | Step 4 | C++ record and fixture README | `docs/feasibility-probes/cpp-text-postprocess.md` | none | `M` | Released generator/WASI SDK re-measurement. |
| `TASK-336` | Step 5 | Go record and fixture README | `docs/feasibility-probes/go-text-postprocess.md` | none | `M` | Released generator/wit-bindgen-go re-measurement. |
| `TASK-336` | Step 6 | AssemblyScript record and fork gate | `docs/feasibility-probes/assemblyscript-text-postprocess.md` | none | `M` | Confirmed clean fork, immediate HEAD provenance, UTF-16 embed; proves AC-8 and AC-N2 protocol. |
| `TASK-336` | Step 7 | `docs/14_submodule_programming_languages.md` | `docs/14_submodule_programming_languages.md` | none | `S` | Four-result summary and exact fixed-priority verdict; proves AC-9. |
