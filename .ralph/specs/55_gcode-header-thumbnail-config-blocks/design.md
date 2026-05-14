# Design: 55_gcode-header-thumbnail-config-blocks

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/gcode_emit.rs::DefaultGCodeSerializer::serialize_gcode()` (`:374-:490`). This function is the single point where `GCodeIR` becomes the final text string. All four new envelope blocks (HEADER, width comments, THUMBNAIL, CONFIG) are emitted from inside it — HEADER + width + THUMBNAIL at the head, CONFIG at the tail. No new module, no new finalization stage, no new TextPostProcess hook.
- Secondary code path: `crates/slicer-host/src/cli.rs` + `crates/slicer-host/src/main.rs` for the `--thumbnail <path>` flag. The flag's value is injected into `config_source: HashMap<ConfigKey, ConfigValue>` (the same map produced by `parse_cli_config_source()` at `execution_plan.rs:193`) before the call to `run_pipeline_with_raw_config()` at `main.rs:280`. No changes to `run_pipeline_with_raw_config`'s signature — the file path travels as a config value.
- Neighboring tests or fixtures: `crates/slicer-host/tests/orca_comment_contract_tdd.rs` is the existing comment-contract regression and must stay green. The new TDD `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` is the only new test file.
- OrcaSlicer comparison surface: only the wire format (sentinel literals, line prefix, Base64 column width, key spellings). Functional logic is NOT copied — PinchAndPrint's serializer remains independent.

## Architecture Constraints

- `GCodeIR` MUST NOT change. The envelope is computed from `PrintMetadata` + `ConfigView` + `LayerCollectionIR.z` already available to the serializer.
- The serializer remains pure: file I/O for the thumbnail PNG happens once at the top of `serialize_gcode()` (or in a thin wrapper called before it) and the bytes are passed in. Failure modes (file not found, bad PNG magic) become `Result::Err` propagated up to `main.rs` for a clean non-zero exit. The serializer never touches the filesystem for any other reason.
- Determinism: `serialize_config_block` iterates `ConfigView` in deterministic, key-sorted (lexical) order. No `HashMap` iteration leaks non-determinism into the file.
- Coordinate system: `max_z_height` is emitted in millimeters. Internal units are 100 nm (see `docs/08_coordinate_system.md`). Conversion is local to the helper.
- The four config keys (`filament_diameter`, `filament_density`, `max_z_height`, `thumbnail_path`) are registered in `config_schema.rs` with defaults; the registration is the schema source of truth (no parallel constants).

## Code Change Surface

- **Selected approach.** Inline emission inside `DefaultGCodeSerializer::serialize_gcode()` plus four small free-standing helper functions in the same file. CLI flag adds one field to the `HostCli` struct and one line in `main.rs` that writes it into `config_source`. Filesystem read for the thumbnail happens once, in `main.rs` (or a small `load_thumbnail_bytes()` helper colocated with `serialize_gcode`'s caller), so the serializer signature only takes `&[u8]`.
- Rejected alternatives:
  - **New finalization-stage WASM core-module emitting the envelope blocks.** Rejected: heavier than the value; the WIT boundary cannot carry the full `ConfigView` cheaply, and the blocks are pure text decoration over data the serializer already has. Packet 53 (cooling) chose the module path for fan because cooling decisions need per-layer geometry context; envelope blocks need no geometry.
  - **New TextPostProcess module appended to `postpass.rs`.** Rejected: would require materializing `ConfigView` and `PrintMetadata` into the TextPostProcess input contract; that's a larger architectural change than this packet justifies.
  - **Extending `run_pipeline_with_raw_config` signature with `thumbnail_png: Option<Vec<u8>>`.** Rejected: would force every existing caller (tests, integration paths) to add `None`. Threading via `config_source` keeps the API stable.
  - **Emitting the full OrcaSlicer ~300-key PrintConfig in CONFIG_BLOCK.** Rejected by user decision: emit only what `ConfigView` carries (user-passed merged with defaults that the pipeline actually consumed). Avoids misleading downstream tools with dead keys.
  - **Software mesh-silhouette rasterizer for the thumbnail.** Rejected by user decision: external PNG via `--thumbnail` flag is the chosen pattern.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/gcode_emit.rs`: add four free functions (`serialize_header_block`, `serialize_width_comments`, `serialize_thumbnail_block`, `serialize_config_block`); modify `DefaultGCodeSerializer::serialize_gcode()` (`:374-:490`) at its start (insert HEADER + width + optional THUMBNAIL) and end (append CONFIG). Total expected delta: ~150 LoC additions, ~10 LoC modifications.
  - `crates/slicer-host/src/config_schema.rs`: add four `ConfigFieldSchema` entries (`filament_diameter`, `filament_density`, `max_z_height`, `thumbnail_path`) plus any width keys missing today. Total delta: ~40 LoC additions following the existing pattern at `:121` and `:171`.
  - `crates/slicer-host/src/cli.rs`: add `thumbnail: Option<PathBuf>` field to the `Slice` variant (or equivalent) of `HostCommands`. Total delta: ~5 LoC.
  - `crates/slicer-host/src/main.rs`: read the PNG once at `:280` ± 20 lines, validate magic, insert `("thumbnail_path", ConfigValue::String(path_string))` into `config_source`. Pass the bytes through to the serializer via a new field on whatever struct currently flows into `serialize_gcode()` (most likely a small additive field on the serializer's invocation site, NOT on the serializer struct itself — bytes are passed at call time). Total delta: ~20 LoC.
  - `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs`: new file, ~250 LoC, all ACs. The valid-PNG fixture is the already-committed `resources/fake_thumb.png` (940×940, ≈132 KB, PNG-magic verified). The non-PNG negative case writes 64 bytes without PNG magic into `std::env::temp_dir()` at test runtime. No new committed binary fixtures.

## Files in Scope (read + edit)

Primary edit surface (≤ 3 main files, plus thin auxiliary edits):

- `crates/slicer-host/src/gcode_emit.rs` — role: G-code text serializer (the single emission point); expected change: four new helpers + serialize_gcode() head/tail insertions.
- `crates/slicer-host/src/config_schema.rs` — role: config-key registration; expected change: 4 new key entries (+ any missing width keys).
- `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` — role: new TDD test; expected change: file creation with all positive and negative ACs.

Thin auxiliary edits (≤ 5 LoC each, listed for completeness):

- `crates/slicer-host/src/cli.rs` — add `thumbnail: Option<PathBuf>` to the slice subcommand.
- `crates/slicer-host/src/main.rs` — read PNG, validate magic, inject into `config_source`, pass bytes to serializer call site.

Fixture handling (NO new committed binary fixtures):

- Valid-PNG fixture: reuse the already-committed `resources/fake_thumb.png` (940×940, ≈132 KB). Resolve from the test as `Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/fake_thumb.png"))` — `CARGO_MANIFEST_DIR` points at `crates/slicer-host/`, so two `..` segments reach the workspace root. Use this absolute path everywhere a thumbnail is needed; never copy the bytes into a `tests/fixtures/` directory.
- Non-PNG negative-case fixture: materialized in-test using `tempfile::NamedTempFile` (if `tempfile` is already a workspace dev-dep) or a hand-rolled `std::env::temp_dir().join("packet55_not_a_png_<nanos>.bin")` with 64 bytes of non-magic data (e.g., `b"this is plainly not a png file, no magic at all\n\0..."`). Lifetime is the test function's scope; clean up best-effort.
- The 132 KB PNG inflates to ≈176 KB Base64 (≈2300 lines at 76 chars/line). Assertions MUST avoid `assert_eq!(actual_gcode, expected_full_gcode)` on the whole file; instead assert on (a) regex/grep counts of sentinels, (b) byte-roundtrip equality between decoded payload and `std::fs::read("resources/fake_thumb.png")`, and (c) ≤ 120-char snippets when reporting failure. This keeps failure SNIPPETS under the 20-line dispatch budget.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` lines `467-:520` (`ConfigView`), `1218-:1230` (`Point3WithWidth`), `1524-:1540` (`LayerCollectionIR.z`), `1634-:1660` (`PrintMetadata`). File is > 1600 lines — never load in full.
- `crates/slicer-host/src/gcode_emit.rs` lines `:370-:490` for the serializer body.
- `crates/slicer-host/src/postpass.rs` lines `:163-:266` only if the implementer must trace where `gcode_text` is finalized — preferred path is just inserting inside `serialize_gcode()`, in which case this read is unnecessary.
- `crates/slicer-host/src/pipeline.rs` lines `:217-:265` (entrypoint `run_pipeline_with_raw_config`) — read only if Step 5's CLI-config-source wiring is unclear.
- `crates/slicer-host/src/execution_plan.rs` lines `:60-:200` for `parse_cli_config_source` shape — for Step 5 only.
- `docs/02_ir_schemas.md` — range-read or delegate; only the `PrintMetadata` / `ConfigView` / `LayerCollectionIR` sections.
- `docs/03_wit_and_manifest.md` — config-schema-validation section only.
- `docs/08_coordinate_system.md` — unit-conversion paragraph only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate all five FACT dispatches; never load.
- `target/`, `Cargo.lock`, generated WIT bindings — never load.
- Any other `.ralph/specs/*/` packet — confirmed disjoint by predecessor SUMMARY; do not re-read.
- `crates/slicer-helpers/` — not touched by this packet.
- `modules/core-modules/*` — not touched (no new module, no manifest changes).
- The full `crates/slicer-ir/src/slice_ir.rs` outside the four enumerated ranges.
- The full `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (> 6000 lines); only five small ranges matter and they go via FACT.

## Expected Sub-Agent Dispatches

- "Read `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2640-2710`. Return FACT ≤ 12 lines: exact sentinel literals (HEADER_BLOCK_START/END, CONFIG_BLOCK_START/END), comment line prefix (`;` vs `; `), and how `total layer number` value is formatted." — purpose: ground Step 3's header emission.
- "Read `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2670-2705`. Return FACT ≤ 10 lines: exact spelling and value format of `filament_diameter`, `filament_density`, `max_z_height`, `filament` (filament order)." — purpose: ground Step 3's four required header fields.
- "Read `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2750-2765`. Return FACT ≤ 8 lines: extrusion-width comment format (`; key = value` vs `; key=value`) and the canonical list of width keys." — purpose: ground Step 4's width-comment helper.
- "Read `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp` lines `100-135`. Return FACT ≤ 12 lines: THUMBNAIL_BLOCK_START/END literals, line prefix, Base64 column width (76 or 78), and the per-thumbnail metadata line (`; thumbnail begin WxH bytes`)." — purpose: ground Step 5's thumbnail wire format.
- "Read `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `5590-5620` (`append_full_config()`). Return FACT ≤ 8 lines: iteration order, separator (` = ` vs `=`), and whether keys are sorted or insertion-order." — purpose: ground Step 6's CONFIG_BLOCK helper.
- "Inspect `crates/slicer-host/src/config_schema.rs`; return LOCATIONS (≤ 40 entries) listing every registered key with type and default." — purpose: Step 6 needs to know the full effective-config key set so the test's `config_block_covers_effective_config` assertion is grounded.
- "Run `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on first failing assertion." — purpose: per-step verification.
- "Run `cargo test -p slicer-host --test orca_comment_contract_tdd`; return FACT pass/fail." — purpose: regression after envelope insertion.

## Data and Contract Notes

- IR or manifest contracts touched: NONE. `PrintMetadata`, `ConfigView`, `LayerCollectionIR` are read but not modified. No new IR fields.
- WIT boundary considerations: NONE. No WIT files touched, no module manifests modified.
- Determinism or scheduler constraints: `serialize_config_block` MUST iterate keys in deterministic order. Use a sorted iterator (`BTreeMap` clone or `keys.sort()`); a `HashMap` iteration would make the output non-reproducible across runs.
- Config-schema validation: `thumbnail_path` is `ConfigValue::String`; an empty string is the explicit "no thumbnail" sentinel and is the registered default. The serializer treats `path.is_empty()` as "skip thumbnail block".

## Locked Assumptions and Invariants

- The sentinel literals `HEADER_BLOCK_START`, `HEADER_BLOCK_END`, `THUMBNAIL_BLOCK_START`, `THUMBNAIL_BLOCK_END`, `CONFIG_BLOCK_START`, `CONFIG_BLOCK_END` are byte-for-byte OrcaSlicer's — no PinchAndPrint-flavored variants. Confirmed by FACT dispatch 1.
- Comment line prefix is OrcaSlicer's (most likely `; ` — confirmed by FACT dispatch 1). The same prefix is used inside all four blocks.
- HEADER block is emitted BEFORE any `;TYPE:` or motion command. CONFIG block is emitted AFTER the final motion command. Width comments and the optional THUMBNAIL block sit between HEADER_END and the first motion command.
- The block ordering is HEADER → width comments → THUMBNAIL (optional) → motion body → CONFIG.
- `--thumbnail` is the sole way to inject a thumbnail; absence is OrcaSlicer-CLI-parity (no block).
- PNG validation is magic-only (`89 50 4E 47 0D 0A 1A 0A`); no IHDR parsing, no size validation, no re-encoding.
- Effective config = `ConfigView` constructed from user-passed `config_source` merged with `config_schema.rs` defaults. CONFIG_BLOCK enumerates exactly that view.
- `filament_used_mm` (already in `PrintMetadata`) is the source of truth for the filament-order line. Tools with `filament_used_mm[i] > 0` are emitted in ascending index order.

## Risks and Tradeoffs

- **Risk: ConfigValue → string formatting drift from OrcaSlicer.** Floats may be `0.16` vs `0.160000` vs `0.16f`. Mitigation: keep the formatter simple (`{:.4}` for floats, strip trailing zeros, `to_string()` for ints/bools); the AC `config_block_includes_user_passed` accepts `22` OR `22.0` to leave room.
- **Risk: width-key list drift.** OrcaSlicer's emitted set may not match PinchAndPrint's registered set. Mitigation: AC4 names five specific keys (`outer_wall_line_width`, `inner_wall_line_width`, `sparse_infill_line_width`, `top_surface_line_width`, `support_line_width`). Step 2 registers any missing from this list with OrcaSlicer-parity defaults; Step 4 emits exactly these five.
- **Risk: large PNG inflating the file unboundedly.** Mitigation: out of scope for this packet; document in `requirements.md` Out-of-Scope. Acceptance is byte-roundtrip, not byte-budget. The chosen test fixture (`resources/fake_thumb.png`, 940×940, ≈132 KB) deliberately exercises the >100 KB case so the chunking, prefix, and roundtrip assertions stress the Base64 path; tests still finish in well under a second.
- **Risk: CONFIG_BLOCK at file tail causes naive parsers to choke on `; ` lines after the last `G1`.** Mitigation: this is OrcaSlicer's behavior; any parser that mishandles it is non-conformant. AC `block_ordering_header_before_body_config_after` codifies the placement.
- **Tradeoff: by routing `--thumbnail` through `config_source` rather than a new pipeline parameter, the API stays stable but a misleading "thumbnail_path is a config key" registration enters `config_schema.rs`.** Accepted; the field's semantic is documented in its `ConfigFieldSchema::description`.
- **Tradeoff: no real per-print thumbnail.** Accepted at the user's explicit decision; the `--thumbnail` ingestion path makes follow-up rendering work additive (the wire-format helper is reused).

## Context Cost Estimate

- Aggregate (sum across all steps): `M`.
- Largest single step: Step 5 (`--thumbnail` CLI flag + PNG validation + Base64 chunking + serializer wiring). Cost `M` — moderate because it touches both `cli.rs`/`main.rs` and `gcode_emit.rs` in one logical change, and the Base64 chunking must match OrcaSlicer column width exactly.
- Highest-risk dispatch: the OrcaSlicer FACT dispatches. Any dispatch returning more than its 8-12 line FACT budget MUST be rejected and re-dispatched with tighter scope. Specify "≤ N lines, no code blocks > 4 lines" verbatim in each dispatch.

## Open Questions

These are pre-implementation answerable; they do not block activation. The implementer resolves each in-step via a small FACT dispatch.

- **Q1.** What is OrcaSlicer's exact canonical list of extrusion-width comments emitted at `GCode.cpp:2752-2760`? AC4 names five keys based on common OrcaSlicer practice; FACT dispatch 3 confirms or amends. If OrcaSlicer emits a different five, Step 4 emits OrcaSlicer's list and the AC is amended in-step (not a scope change — it's a clarification of "OrcaSlicer-parity").
- **Q2.** Does OrcaSlicer's `append_full_config` sort keys lexically, or preserve PrintConfig's declaration order? FACT dispatch 5 confirms. Step 6 chooses sort order accordingly. Either choice satisfies the AC `config_block_covers_effective_config`, which is order-insensitive.
- **Q3.** Does the OrcaSlicer thumbnail wire format prepend a `; thumbnail begin <W>x<H> <bytes>` metadata line? FACT dispatch 4 confirms. If yes, Step 5 emits it; if no, omit. The roundtrip AC tolerates either.

If FACT dispatches return contradictory or ambiguous results, surface as a packet-local risk and emit the simplest OrcaSlicer-matching variant; do NOT block on a perfect match.
