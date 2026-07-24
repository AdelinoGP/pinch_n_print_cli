# Design: 182-gcode-header-width-defaults

## Controlling Code Paths

- Primary code path: `DefaultGCodeSerializer` (`crates/slicer-gcode/src/serialize.rs` — long; ranged reads only) — the `with_extrusion_mode` constructor is the **sole** assignment site for `outer_wall_line_width` / `inner_wall_line_width`; `serialize_width_comments` writes them into the header via `writeln!(out, "; outer_wall_line_width = {outer_wall}")` and the matching `inner_wall` line.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/golden_emit_tdd.rs` (148 lines; standalone auto-discovered binary, no mod registration; its one test `serialize_gcode_emits_documented_sentinels` asserts `output.contains("; outer_wall_line_width = ")` — presence only). **`crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`** records all five width lines verbatim, including `; outer_wall_line_width = 0.42` and `; inner_wall_line_width = 0.45`, and `legacy_zero_matches_golden` (`crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs`, registered as `mod slicing_precision_integration_tdd;` in `crates/slicer-runtime/tests/e2e/main.rs`) asserts **byte identity** against it.
- OrcaSlicer comparison: none. The corrected value is PnP's own governing internal fallback, not a ported canonical constant; this packet consults no `OrcaSlicerDocumented/` source.

## Architecture Constraints

- The governing authority for `0.4` is `resolve_line_width_mm` (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`), whose doc comment states the fallback "matches the guest-side default used by `classic-perimeters`/`arachne-perimeters`". The replacement comment must cite that symbol — not a file that does not exist.
- **Do not preserve the "OrcaSlicer parity" attribution when swapping the number.** The two field doc comments currently read "OrcaSlicer 0.4 mm nozzle parity default: 0.42". Merely changing the digits keeps a false citation: open deviation `D-164-WALL-WIDTH-KEYS-NOT-FLOAT-OR-PERCENT` records that upstream registers both keys as `coFloatOrPercent` default `0` (auto-derive from nozzle), so "OrcaSlicer parity default: 0.4" would be a fresh D-165-class lie in the very lines being repaired. Attribute `0.4` to `resolve_line_width_mm`.
- **Blast radius includes recorded outputs, not just struct literals.** `DefaultGCodeSerializer` is constructed only via its four constructors, so there are no struct-literal sites to update — but the emitted header is captured byte-for-byte in a golden fixture. A change with zero struct-literal fallout can still have test fallout through recorded output; the golden must be re-blessed in the same packet.
- `DefaultGCodeSerializer` has no config-driven setter for these fields (`new`, `with_extrusion_mode`, `with_flavor`, `with_filament_config` are the complete set on this impl — the second `new`/`with_flavor` pair in the file belongs to `ThumbnailAwareSerializer`; `impl Default` delegates to `new()`). This packet must not add one — making the header config-aware is out of scope.
- The guest-WASM staleness snippet does not apply to this packet's **edit** surface: `crates/slicer-gcode` is a host crate and is not on the guest-WASM input list in `CLAUDE.md` §"Guest WASM Staleness". It does, however, apply to the **test** surface: AC-3's `legacy_zero_matches_golden` shells out to `pnp_cli --module-dir` and executes the real core-module guests. If that test fails, run `cargo xtask build-guests --check` and rebuild if `STALE:` before attributing the failure to this change.
- The same test also requires a freshly built `pnp_cli`. It locates the executable on disk via `pnp_cli_bin()`, and `crates/slicer-runtime` does not depend on the CLI crate, so `cargo test` alone will not pick up a `serialize.rs` edit. `cargo build --workspace` is a precondition of the re-bless step; `cargo check` is insufficient because it produces no executable.
- No coordinate-system bullet applies: the change prints an existing mm-domain `f32` into a comment string; there is no mm↔internal-unit conversion and no geometry.

## Code Change Surface

- Selected approach: correct the two literals in place, fix the three misleading comments (the constructor comment naming the deleted `config_schema.rs`, and the two per-field doc comments), re-bless the one golden that captured the old values, and add one additive whole-line value-asserting test.
- Exact functions, traits, manifests, tests, and fixtures:
  - `DefaultGCodeSerializer::with_extrusion_mode` (`crates/slicer-gcode/src/serialize.rs`) — `outer_wall_line_width: 0.42` → `0.4`; `inner_wall_line_width: 0.45` → `0.4`; replace the `// OrcaSlicer 0.4 mm nozzle parity defaults (matches config_schema.rs registration).` comment.
  - The two field doc comments on the `DefaultGCodeSerializer` struct — restate the default as `0.4` attributed to `resolve_line_width_mm`.
  - New test `header_reports_governing_wall_width_defaults` in `crates/slicer-gcode/tests/golden_emit_tdd.rs`.
  - Re-blessed `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`.
- Rejected alternatives and reasons:
  - *Wire the header to the resolved config widths.* Correct end state, but needs a new setter plus edits in `crates/slicer-runtime/src/run.rs` and `pipeline.rs`; a different, larger packet.
  - *Delete the width header comments entirely.* They are documented sentinels asserted by `serialize_gcode_emits_documented_sentinels`; removing them breaks a green test and loses information consumers may parse.
  - *Also correct the three sibling width fields.* Each needs its own governing-fallback proof; D-165 charters only the two wall keys.

## Files in Scope (read + edit)

- `crates/slicer-gcode/src/serialize.rs` — role: owns the wrong defaults and their comments; expected change: two literals → `0.4`, three comments corrected.
- `crates/slicer-gcode/tests/golden_emit_tdd.rs` — role: existing header-sentinel binary; expected change: one additive whole-line test.
- `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` — role: byte-identity golden capturing the emitted header; expected change: re-blessed so its two wall-width lines read `0.4`.

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-gcode/src/serialize.rs` (long; never read in full) — three windows only, located by symbol rather than trusting the pin: the `DefaultGCodeSerializer` struct declaration plus `with_extrusion_mode` (~`66-116`), `serialize_width_comments` (~`274-301`), and its call site (~`687-695` — the `output.push_str(&serialize_width_comments(` opening with its five field arguments; a window ending at 689 truncates before `self.outer_wall_line_width` is visible).
- `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` — `resolve_line_width_mm` only, via dispatch; do not open directly.
- `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs` — the `legacy_zero_matches_golden` body only, to learn how the golden is compared and re-blessed. **Caution:** the `BLESS_GOLDEN=1 … --test slicing_precision_integration_tdd` hint written inside that file is stale and names a non-existent cargo target; the real invocation is `--test e2e`. Do not follow the in-file hint verbatim.
- `docs/DEVIATION_LOG.md` — the D-165 row only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — not consulted; never load.
- `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/pipeline.rs` — serializer construction sites; config wiring is out of scope.
- `modules/core-modules/classic-perimeters/**`, `modules/core-modules/arachne-perimeters/**` — the width-resolution authority; owned by the T2 flow packets.
- The other golden fixtures under `crates/slicer-runtime/tests/fixtures/golden/` — only `precision_legacy_20mmbox.gcode` records these two lines; do not bulk re-bless.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.

## Expected Sub-Agent Dispatches

- Question: what is `resolve_line_width_mm`'s exact fallback expression and doc-comment wording, to cite in the replacement comment?; scope: `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`; return: `SNIPPETS` (<=15 lines); purpose: Step 2.
- Question: which golden fixtures under `crates/slicer-runtime/tests/fixtures/golden/` contain a `; outer_wall_line_width =` or `; inner_wall_line_width =` line, and what value?; scope: `crates/slicer-runtime/tests/fixtures/golden/**`; return: `LOCATIONS` (<=20 entries); purpose: Step 3 — confirm only the one golden needs re-blessing.
- Question: report the D-165 row's current Status cell verbatim; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (<=5 lines); purpose: completion-gate status flip.

## Data and Contract Notes

- IR/manifest contracts: none touched. The two fields are private serializer state.
- WIT boundary: none. `crates/slicer-gcode` is host-side and exports no WIT.
- Determinism/scheduler constraints: none. The emitted header is already deterministic; only the printed constant changes — which is exactly why a byte-identity golden notices.

## Locked Assumptions and Invariants

- Locks the emitted default header values to `0.4`/`0.4` for as long as the serializer remains config-blind. Reversible by a one-line edit, and superseded the moment the header is wired to resolved config (out of scope). No behavior lock beyond the printed constant.

## Risks and Tradeoffs

- Any external consumer parsing `; outer_wall_line_width = 0.42` as a stable sentinel value would see `0.4`. Low risk: the value was never config-accurate, and the in-tree sentinel test asserts only the key's presence.
- The header remains a *default*, not the slice's real resolved width. This packet makes it honest, not correct-per-slice; the residual gap is recorded in Out of Scope so it is not mistaken for closure of the broader question.
- Re-blessing a byte-identity golden is a blunt instrument: the diff must be inspected to confirm **only** the two wall-width lines changed. A blind re-bless would silently absorb any unrelated drift.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: the golden-fixture sweep — `LOCATIONS` capped at 20, to confirm the re-bless surface is exactly one file without loading fixture bodies.

## Open Questions

None.
