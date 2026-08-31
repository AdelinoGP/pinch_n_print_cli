# Preflight Report: 257-brim-type-and-brim-keys

Reviewed: 2026-08-30 · Mode: --preflight · Symbol-inventory dispatched: grounded by direct session greps (controller-authored packet; re-verify protocol followed)

## Preflight Gate

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files)     | PASS | all five present, non-empty (`ls docs/spec_packets/257-brim-type-and-brim-keys/`) |
| S1 Prerequisite-status truth      | PASS | no `docs/spec_packets/` prereq references (grep count 0; wayfinder tickets only, all resolved per map) |
| S2 Deviation-ID conformance       | PASS | zero `DEV-*`/`D-<n>` tokens in packet files (grep empty); requirements states "no deviation rows" |
| S3 Schema-version computed        | PASS | zero `SCHEMA_VERSION` references (grep empty); no version constants touched |
| S4 ADR slot allocation            | PASS | no ADR authored or referenced by path (grep `adr/` empty) |
| S5 Shipped-symbol existence/shape | PASS | see evidence below |
| S6 WIT/IR identifier drift        | PASS | packet names no WIT types/IR enum variants as pre-existing; `ConfigValue::String/Float/Bool` exist (host dispatch reads these variants) |
| S7 Test-target wiring             | PASS | see evidence below |
| S8 ADR conformance                | PASS | no packet clause contradicts ADR normative content; ADR-0045's "padding" is stub-package padding, unrelated topic (grep quote) |
| (existing) AC runnable command    | PASS | AC-1..AC-6, AC-N1, AC-N2 each end with one pipe-suffixed command; binaries are `skirt-brim` file-binaries, `slicer-scheduler --test integration`, `slicer-runtime --test integration`, `xtask` — all verified to exist |
| (existing) Doc Impact Statement   | PASS | specific form used (doc 15 + verification grep appended as AC-6) |

## S5 evidence (symbols the packet consumes/extends, grepped this session)

- `rejects_unknown_support_style_value` and `manifest_declared_bound_rejects_out_of_range_value` in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` ✓ (both grepped, single hits)
- `run_pipeline_with_raw_config` → `crates/slicer-runtime/src/pipeline.rs` (pub fn, grepped) — the AC-5 integration driver exists
- `ConfigBoundsIndex::from_modules` (`crates/slicer-scheduler/src/config_resolution.rs`) ✓, `load_module_from_paths` (`crates/slicer-scheduler/src/manifest.rs`) ✓
- `SkirtBrim::from_config` / `generate_brim_entities` / `run_finalization` — read directly in `modules/core-modules/skirt-brim/src/lib.rs` (brim arm gated on `brim_width > 0.0`, two sites)
- `ORCA_CONFIG_PADDING` entries `("brim_type", "auto_brim")` / `("brim_object_gap", "0")` — read in `crates/slicer-gcode/src/serialize.rs` (lines 499/540)
- `"brim_type"` in the 3MF sidecar key classification arm (`crates/slicer-model-io/src/loader.rs`) — read; String-passthrough already works for the key name
- NET-NEW (not flagged): `BrimType` enum, `brim_config_schema_tdd.rs`, all five manifest keys

## S7 evidence

- `brim_config_schema_tdd.rs` is a NEW file in `modules/core-modules/skirt-brim/tests/` — no `tests/main.rs` aggregator exists in that crate (verified; pattern source part-cooling has none either), so each test file is its own `--test` binary; AC-1/AC-N2 target exactly that binary. No registration needed.
- `config_bounds_enforcement_tdd` and `gcode_header_thumbnail_config_blocks_tdd` are already registered (`mod` lines grepped in both registries) — the packet APPENDS to existing registered files, no new registration.
- Step-3 edit cap stays ≤3 edits/file (registry fallback already budgeted in the step).

## S8 evidence

- The design surface (module manifest + module gate + test arms) touches no ADR normative clause; ADR-0045's "padding" term is stub-package padding, not CONFIG_BLOCK padding — no conformance issue.

## Blockers (S4/S5/S6)

None.

## High (S1/S2/S3/S7/S8)

None.

## Accepted FORWARD-DEPs

None.

## Authoring-time fixes applied during grounding (before the gate ran)

- `toml = "0.8"` dev-dependency added to Step 1's edit list (the schema-guard pattern requires it; skirt-brim lacked it — real grep)
- `parity-evidence` snippet inserted into `requirements.md` (verbatim-or-absent rule)
- Doc-Impact converted from `none` to the specific form with AC-6
- AC list and verification matrix synced with AC-6

**Verdict: PREFLIGHT PASS** (0 blockers, 0 high)