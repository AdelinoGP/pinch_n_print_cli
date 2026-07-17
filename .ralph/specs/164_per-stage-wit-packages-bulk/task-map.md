# Task Map: 164_per-stage-wit-packages-bulk

Emitted because `TASK-146c` is a sub-letter of the **reopened** `TASK-146` (`docs/07_implementation_status.md`; reopening rationale in `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Task mapping": the original "wit_world allowlist validation" was shown by ADR-0044 to validate nothing, and ADR-0045 retires `validate_wit_world` outright — this packet is the retirement half plus the bulk migration).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-146c` | Steps 0-1 | `docs/adr/0045-…` §Decision, §naive-shape | `crates/slicer-schema/wit/deps/` (12 new packages + `prepass-types.wit`; 2 tier dirs deleted) | none — no parity surface | S+M | The contract itself moves; everything else follows it |
| `TASK-146c` | Steps 2-3 | ADR-0006; 163 `design.md` §Exports | `crates/slicer-schema/src/lib.rs`, `crates/slicer-macros/src/lib.rs` | none | S+M | Sole-lookup extension + glue split; fallbacks die |
| `TASK-146c` | Steps 4-5 | ADR-0002 | `crates/slicer-wasm-host/src/{host.rs,dispatch.rs}` | none | M+M | 12 `bindgen!` mods; per-stage instantiate; fatal-on-miss reasons |
| `TASK-146c` | Steps 6-7 | `CLAUDE.md` §Guest WASM Staleness | test guests; full guest rebuild; executor + baseline green | none | S+M | Proves behavior neutrality (AC-N4) |
| `TASK-146c` | Step 8 | ADR-0044 (SUMMARY) | `crates/slicer-scheduler/src/manifest.rs`, `crates/pnp-cli/src/module_new.rs`, 20 manifests | none | M | The literal reopened-TASK-146 surface: `validate_wit_world` retired |
| `TASK-146c` | Steps 9-10 | ADR-0045 §Verified empirically | contract guards; binding-test sweep (`ls`-derived set + new arachne-perimeters test); docs/03, CONTEXT.md, wit README; deviation closure | none | M+S | Isolation evidence (AC-N2) is the ADR's headline claim |

Copy costs match `implementation-plan.md`'s roll-up. No row is L; aggregate `M`.
