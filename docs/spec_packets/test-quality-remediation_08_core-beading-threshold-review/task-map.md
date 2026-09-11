# Task Map: core-beading-threshold-review

The user-approved program IDs are used directly; no `TASK-###` mapping is authorized or required. This packet owns one test-only remediation slice plus its implementation-time §7 ledger bookkeeping and exports no symbols, APIs, or test files.

| Approved program task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/DUP-CORE (beading factory)` | Step 1 | `docs/specs/test-quality-remediation-plan.md` §5.1/§6; `docs/22_test_quality.md`; `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-core/tests/beading/factory.rs` only: preserve all three tests and add the contrasting `.20/.75` propagation case | See delegated full paths/functions in `packet.spec.md` and `requirements.md`; never load | `S` | KEEP default seed, full-stack default propagation, and clamp cases; contrasting values are independent literals and additive, not replacements. |
| `core/DUP-CORE (beading factory)` | Step 2 | `docs/specs/test-quality-remediation-plan.md` §7 | `docs/specs/test-quality-remediation-plan.md` §7 `core` row only, after Step 1; preserve accumulated `core` content and do not edit non-`core` rows or the Packet Queue | none; doc-only | `S` | Set state to `partial`; record actual validation and remaining core markers; no queue-only or wrong-column evidence; non-`core` contents are not frozen. |

The `beading_factory` target is an existing explicit Cargo `[[test]]` registration, not a new aggregator or module. The dependency `core-geometry-dup-review` is generation-only and exports nothing consumed here; downstream row #9 likewise receives no API or file export.
