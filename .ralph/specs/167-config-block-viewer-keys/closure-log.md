## Step 1 — RED integration tests appended
- Tests added: config_block_meets_orca_minimum_key_gate, config_block_synthesizes_non_bbl_printer_model, config_block_fork_keys_never_shadowed
- RED/GREEN status as observed: `cargo test -p slicer-runtime --test integration -- config_block` reported 21 passed and 1 failed; minimum-key gate and fork-key no-shadowing passed, while printer-model synthesis failed as expected.
- Failing assertion: `config_block_synthesizes_non_bbl_printer_model` found no `printer_model` line instead of `; printer_model = Generic PNP Printer`.
## Step 4 — Golden inventory + invariants
- Golden/e2e tests asserting CONFIG_BLOCK: `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs:225` `legacy_zero_matches_golden` compares complete G-code bytes with `precision_legacy_20mmbox.gcode`; `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs:214` sentinel/structure checks and `:424`-`:532` region content, count, and printer_model checks; `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs:381`-`:415` placement checks and `:606`-`:663` CONFIG_BLOCK key/count checks; `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs:327`-`:337` trailing CONFIG_BLOCK presence check.
- precision_legacy_20mmbox.gcode re-bless: YES (motion-line diff: none; regenerated golden is byte-identical)
- `gcode_header` filter result: PASS with 20 passed/0 failed
## Step 5 — Doc + crosswalk
- docs/02_ir_schemas.md: subsection "CONFIG_BLOCK viewer-key contract" appended under "G-code envelope blocks"
- docs/07_implementation_status.md: TASK-273 row appended in section "Workstream 5 — Governance and closure drift"
