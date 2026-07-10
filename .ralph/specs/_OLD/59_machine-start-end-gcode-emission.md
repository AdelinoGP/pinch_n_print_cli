---
status: implemented
packet: 59_machine-start-end-gcode-emission
task_ids:
  - TASK-194
  - TASK-194a
  - TASK-194b
---

# 59_machine-start-end-gcode-emission

## Goal

Emit a configurable printer start sequence before the first move and a configurable finish sequence after the last move, produced by a NEW core module that runs at the existing `PostPass::GCodePostProcess` stage. The module reads four config keys, performs `[key_name]` substitution against the effective `ConfigView` inside its WASM guest, and rebuilds `GCodeIR.commands` as `[Raw(resolved_start), ...existing commands..., Raw(resolved_end)]`.

To make this position-correct, the existing hard-coded `M82`/`M83` strings written by `DefaultGCodeSerializer::serialize_gcode` at `crates/slicer-host/src/gcode_emit.rs:1154-1156` are promoted to a new `GCodeCommand::ExtrusionMode { absolute: bool }` variant that `DefaultGCodeEmitter::emit_gcode` pushes at the head of the commands list. After the promotion, every byte the serializer writes between `HEADER_BLOCK_END` and `CONFIG_BLOCK_START` comes either from `GCodeIR.commands` or from the `ThumbnailAwareSerializer` wrapper — nothing is hard-coded inside the per-command loop. This is what lets the `GCodePostProcess` module prepend `Raw(machine_start_gcode)` *before* `M82`/`M83`, matching OrcaSlicer ordering.

Concretely:

1. Add `GCodeCommand::ExtrusionMode { absolute: bool }` to `crates/slicer-ir/src/slice_ir.rs:1697` (the existing `GCodeCommand` enum, currently `Move, Retract, Unretract, FanSpeed, Temperature, ToolChange, Comment, Raw`).
2. In `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-host/src/gcode_emit.rs:304`), push `ExtrusionMode { absolute }` as the first command (`absolute` derived from whatever the existing preamble decision is — currently the M83/M82 branch at `:1154-1156`).
3. In `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-host/src/gcode_emit.rs:1107`), remove the hard-coded `M82`/`M83` preamble writes at `:1154-1156`; add a `GCodeCommand::ExtrusionMode { absolute }` match arm in the command-loop renderer (which lives around the `Temperature` arm at `:1280-1281`).
4. Create `modules/core-modules/machine-gcode-emit/` with three files (`machine-gcode-emit.toml`, `Cargo.toml`, `src/lib.rs`), mirroring `modules/core-modules/part-cooling/` in file shape but with `[stage] id = "PostPass::GCodePostProcess"`. The module's `run_gcode_postprocess` implementation:
   - Reads `machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer` from `&ConfigView`.
   - Builds a `HashMap<String, ConfigValue>` lookup for substitution.
   - Calls a private `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper (≤ 60 LOC, scoped to this module).
   - On the `&mut gcode-output-builder`: pushes the resolved start as one `Raw` command, re-pushes every command from the snapshot `list<gcode-command>` argument (or calls the SDK's `extend_from_snapshot` equivalent — whatever pattern the existing GCodePostProcess SDK trait exposes), then pushes the resolved end as one `Raw` command.
   - If a resolved template is empty or whitespace-only, the corresponding `Raw` push is SKIPPED (no phantom empty command).
   - Returns `Ok(())`.
5. Declare the four `[config.schema.<key>]` blocks in the new module's manifest with defaults:
   - `machine_start_gcode` (string) = `"M190 S[bed_temperature_initial_layer_single]\nM109 S[nozzle_temperature_initial_layer]\nPRINT_START EXTRUDER=[nozzle_temperature_initial_layer] BED=[bed_temperature_initial_layer_single]"` (multi-line, TOML triple-quoted).
   - `machine_end_gcode` (string) = `"PRINT_END"`.
   - `bed_temperature_initial_layer_single` (int) = `60`, `min = 0`, `max = 120` (declarative only — see Out of Scope).
   - `nozzle_temperature_initial_layer` (int) = `215`, `min = 0`, `max = 300` (declarative only).

What this design intentionally does NOT add:
- No new WIT methods on any builder resource.
- No new `FinalizationBuilderPush` variants.
- No new `Option<String>` fields on `GCodeIR`.
- No new dispatch match arms in `dispatch.rs`.
- No byte-offset-targeting code in the serializer (positioning falls out of the command list naturally).

Substitution is INTENTIONALLY a narrow subset of OrcaSlicer's `PlaceholderParser`. Arithmetic, conditionals, loops, and builtins are out of scope and tracked as future packets.

## Problem Statement

The slicer currently produces a G-code file that begins with `HEADER_BLOCK` + width comments and a hard-coded `M82`/`M83` extrusion-mode preamble, then jumps straight to the first layer's commands. It ends with the last layer's commands followed by `THUMBNAIL_BLOCK` + `CONFIG_BLOCK` (packet 55). **No printer start sequence is emitted** — no homing, no temperature waits, no Klipper `PRINT_START` macro invocation. **No printer finish sequence is emitted** either — no `PRINT_END` macro, no shutdown, no heater-off or motors-off sequence.

The produced `.gcode` therefore cannot be sent directly to a printer: extrusion would begin on a cold hotend and un-homed bed. Users would have to prepend boilerplate every print, which defeats one of the core ergonomics of a host slicer.

This is also the first packet to introduce **placeholder substitution** in serialized G-code. OrcaSlicer's full `PlaceholderParser` is a Boost.Spirit Qi grammar of ~2400 lines. Implementing parity in one packet is out of proportion to the immediate need. This packet implements the **smallest viable subset**: `[snake_case_key]` literal substitution against the effective `ConfigView`, performed INSIDE the new core module.

The earlier draft of this packet routed substituted strings through new `FinalizationBuilderPush` variants, new `Option<String>` fields on `GCodeIR`, new dispatch arms, and explicit byte-offset placement in the host serializer. That design was reconsidered because the slicer already has an architecturally clean home for this work: the existing `PostPass::GCodePostProcess` stage (`crates/slicer-host/src/execution_plan.rs:38`), wired in the postpass executor at `crates/slicer-host/src/postpass.rs:215`, whose modules receive `list<gcode-command>` + `gcode-output-builder` + `config-view` (`wit/world-postpass.wit:26`). A `GCodePostProcess` module can rebuild `GCodeIR.commands` as `[Raw(start), ...existing..., Raw(end)]` with zero new WIT, no new IR fields, no new dispatch arms, and no byte-offset arithmetic in the serializer.

The one architectural friction with that simpler design is that `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-host/src/gcode_emit.rs:1107`) currently writes `M82`/`M83` as hard-coded raw strings at `:1154-1156` *between* `HEADER_BLOCK` and the per-command loop. A `GCodePostProcess` module can prepend at the head of `GCodeIR.commands`, but the hard-coded preamble lies *outside* that list — so `Raw(machine_start_gcode)` at index 0 would appear AFTER `M82`/`M83` in output, which deviates from OrcaSlicer ordering (`machine_start_gcode` THEN extrusion-mode preamble).

This packet therefore performs a small companion refactor: promote `M82`/`M83` from the hard-coded serializer preamble to a new `GCodeCommand::ExtrusionMode { absolute: bool }` variant, pushed by `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-host/src/gcode_emit.rs:304`) as the first command. The serializer renders `ExtrusionMode { absolute: true }` as `M82\n` and `ExtrusionMode { absolute: false }` as `M83\n`, matching the existing per-command rendering pattern (see `Temperature` at `:1280-1281`). After the promotion, the entire stream between `; HEADER_BLOCK_END` and `; CONFIG_BLOCK_START` originates from `GCodeIR.commands`, and the `GCodePostProcess` module's prepend lands in the right byte position by construction.

The four config keys (`machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer`) do not exist anywhere in the workspace today. They are declared in this packet by the new module's manifest TOML. The default `machine_start_gcode` template references the two temperature keys via `[bed_temperature_initial_layer_single]` and `[nozzle_temperature_initial_layer]`; without those declarations the substitution would pass through literal placeholder text.

This packet does NOT reopen or supersede any prior packet. Packets 54 (preamble) and 55 (HEADER + CONFIG_BLOCK + ThumbnailAwareSerializer) remain correct as shipped — packet 54's effect (an M82 or M83 between header and first layer) is preserved bit-identically for default configs; only the *origin* of that line shifts from a hard-coded serializer write to a `GCodeCommand` pushed by the emitter.

## Architecture Constraints

- All config keys use snake_case literals (per `CLAUDE.md` Config Key Naming Convention). The four new keys conform.
- Config keys consumed by the runtime are declared in a core-module `[config.schema.<key>]` block. Precedent: `modules/core-modules/part-cooling/part-cooling.toml` (int keys), `modules/core-modules/seam-placer/seam-placer.toml` (string keys).
- The substitution work runs INSIDE the new module's `run_gcode_postprocess` body (in the WASM guest). The serializer only renders commands. This matches the rule that modules do the work their names imply: a module called `machine-gcode-emit` emits machine gcode.
- The substituted start string appears BEFORE the `ExtrusionMode` command (which serializes as `M82` or `M83`). This matches OrcaSlicer ordering.
- The substituted end string appears AFTER the last command, BEFORE the `ThumbnailAwareSerializer` wrapper's `THUMBNAIL_BLOCK` + `CONFIG_BLOCK` append (which lives outside `GCodeIR.commands`).
- The four new config keys flow into `CONFIG_BLOCK` automatically via packet-55's `serialize_config_block()` (`crates/slicer-host/src/gcode_emit.rs:928`). No explicit CONFIG_BLOCK wiring is needed.
- The substitution helper is a private free function INSIDE the module's `src/lib.rs`, NOT a public API. If a future packet needs it elsewhere, that packet may promote it.
- The helper performs ONE pass. A substituted value containing `[other_key]` is NOT re-scanned. Avoids non-termination and matches least surprise.
- Empty / whitespace-only resolved template ⇒ the module SKIPS the corresponding `Raw` push. The resulting serialized output contains zero bytes for that block.
- Additive IR change: ONE new `GCodeCommand` variant. No existing variant removed or changed; no `GCodeIR` field added or removed.
- Zero change to `wit/world-finalization.wit`, `wit/world-postpass.wit`, `FinalizationBuilderPush`, `dispatch.rs`, or `GCodeIR`.

## Data and Contract Notes

- **IR contract touched:** additive only. `GCodeCommand` gains one new variant `ExtrusionMode { absolute: bool }`. All existing variants and `GCodeIR` fields are unchanged. `docs/02_ir_schemas.md` gets a single-line append for the new variant if it documents the enum. No `schema_version` bump is required — old consumers that match all existing variants will see a new variant they don't recognize ONLY if they read an IR produced by this version of the slicer; in practice the IR is produced and consumed by the same version, so no cross-version compatibility concern arises.
- **WIT boundary considerations:** `wit/world-postpass.wit` and `wit/world-finalization.wit` are unchanged. `wit/deps/ir-types.wit`'s `gcode-command` variant set MAY need a new `extrusion-mode` variant if it mirrors the Rust enum; Step 3 confirms via dispatch. If touched, the CLAUDE.md WIT/Type Changes Checklist applies. The new module declares `wit-world = "slicer:world-postpass@1.0.0"` (or whichever string the existing GCodePostProcess host uses).
- **SDK contract touched:** none. The new module consumes the existing GCodePostProcess trait surface unchanged. No SDK methods added.
- **Dispatch contract touched:** none. The existing `runner.run_gcode_postprocess(...)` loop at `crates/slicer-host/src/postpass.rs:215` already executes against `&mut gcode_ir`; the new module slots in as one more iteration of that loop.
- **Determinism or scheduler constraints:** The substituted block is deterministic given the same effective `ConfigView`. The module's `substitute_placeholders()` helper iterates the template left-to-right; HashMap key lookup is value-equality and does not introduce ordering nondeterminism. Empty / whitespace-only ⇒ zero bytes.
- **CONFIG_BLOCK propagation:** Packet 55's `serialize_config_block()` at `:928` iterates the effective `ConfigView` and emits `; <key> = <value>` per key. The four new keys flow through automatically once declared in the manifest and resolved into `effective_config`. AC `new_keys_appear_in_config_block` verifies this.
- **Multi-line `String` value in CONFIG_BLOCK:** The default `machine_start_gcode` contains `\n` characters. Packet 55's CONFIG_BLOCK formatter handles multi-line values per its existing convention. Whichever it picked applies. AC `new_keys_appear_in_config_block` asserts equality after un-escaping; a Step 4 SNIPPETS dispatch confirms the exact wire format.

## Locked Assumptions and Invariants

- The implementer MUST preserve byte-level ordering established by packets 54 / 55:
  - `; HEADER_BLOCK_START` < width comments < (NEW: resolved start string) < `M82` or `M83` line < first `;LAYER_CHANGE` < first `G1` extrusion move.
  - last `G1` extrusion move < (NEW: resolved end string) < `; THUMBNAIL_BLOCK_START` (if present) < `; CONFIG_BLOCK_START` < `; CONFIG_BLOCK_END` < EOF.
- After the promotion, exactly one `M82` or `M83` line appears in default output, in the same byte position relative to surrounding markers as before — only its origin shifts from a serializer hard-coded write to a typed command pushed by the emitter.
- The module's `substitute_placeholders()` is single-pass. Substituted values are not re-scanned.
- `substitute_placeholders()` does not panic, does not infinite-loop, and does not allocate unboundedly. For an N-byte template with K placeholders, runtime is O(N + K · avg-key-length).
- Empty/whitespace-only resolved string ⇒ the module SKIPS the corresponding `Raw` push. Output contains zero bytes for that block.
- `GCodeCommand` grows by exactly one variant (`ExtrusionMode { absolute: bool }`). All other variants unchanged.
- `GCodeIR` struct is UNCHANGED.
- `FinalizationBuilderPush` is UNCHANGED.
- `dispatch.rs` is UNCHANGED.
- `wit/world-finalization.wit` and `wit/world-postpass.wit` are UNCHANGED structurally.
- The host serializer contains NO substitution logic. All substitution happens inside the WASM guest.
- The four new config keys' defaults are EXACTLY as specified in `packet.spec.md` Goal.

## Risks and Tradeoffs

- **Risk: M82/M83 promotion breaks `gcode_emit_tdd.rs` or other suites.** The packet-54 regression suite asserts presence/position of `M82`/`M83`. After the promotion, the line appears in the same byte position by construction (the emitter pushes `ExtrusionMode` at index 0; the serializer renders it as the first per-command output, right after the header/width comments). **Mitigation:** Step 3 includes `cargo test -p slicer-host --test gcode_emit_tdd` as the immediate falsifying check. If the suite breaks, investigate the exact byte offset assertion and adjust either the test (preferred — assertion should compare position relative to markers, not absolute byte offsets) or surface as a packet-local risk.
- **Risk: cross-component diagnostics for unknown placeholders.** With substitution in the WASM guest, host-side `log::warn!` capture is not trivially available. The negative AC `unknown_placeholder_passes_through_verbatim` asserts verbatim passthrough only. Cross-component diagnostic forwarding is tracked as a separate future packet.
- **Risk: end-block position differs from OrcaSlicer.** OrcaSlicer emits `machine_end_gcode` AFTER its CONFIG_BLOCK. We emit it BEFORE `CONFIG_BLOCK` because our CONFIG_BLOCK is structurally a metadata footer that the printer ignores. **Mitigation:** Documented as an intentional deviation here and in `requirements.md`; the printer-visible ordering is correct (machine_end_gcode is the last printable command).
- **Risk: range enforcement is declarative-only.** The manifest declares `min = 0, max = 120` (bed) and `min = 0, max = 300` (nozzle) but `ResolvedConfig::apply_cli_key` does not consult these. A user passing `nozzle_temperature_initial_layer = 999` gets `M109 S999`. Tracked as a separate future packet.
- **Risk: multi-line `machine_start_gcode` value in CONFIG_BLOCK may be unparseable by some downstream tools.** Whichever convention packet 55 picked applies. **Mitigation:** Step 4 dispatch confirms wire format; AC `new_keys_appear_in_config_block` validates round-trip via un-escaping.
- **Risk: guest-wasm staleness after the new module AND after the `slicer-ir` variant change.** Per CLAUDE.md Guest WASM Staleness, `slicer-ir` is a universal guest dep — adding a variant invalidates every guest's bindgen output. **Mitigation:** Steps 4 and 5 both dispatch `--check`.
- **Risk: future "arithmetic" packet breaks one-pass invariant.** Adding `[bed*2]` evaluation means substituted output could contain identifiers that look like further placeholders. **Mitigation:** scope boundary is clear; future packet decides whether to re-scan or to extend the helper into a tokenizer.
- **Risk: `Raw` command in the middle of the stream may surprise other GCodePostProcess modules that match `GCodeCommand` exhaustively.** **Mitigation:** `Raw` is an existing variant; any module that matches `GCodeCommand` already handles `Raw`. The new module's only contribution is two more `Raw` instances at the boundaries.
- **Tradeoff: minimal substitution vs full OrcaSlicer parity.** Accepted at scope-approval gate. Users who need conditionals can wait for the follow-up packet or expand the literal value.
- **Tradeoff: typed `ExtrusionMode` variant vs `Raw("M82")` shortcut.** Chose typed variant for IR clarity, debug-print quality, and parity with how M104/M109 are modeled (`Temperature`, not `Raw`).

## Implementation Deviations (post-implementation, 2026-05-18)

Three small scope expansions surfaced during implementation. All Low severity, all packet-local, none affect output behavior or external contracts. Recorded here rather than in `docs/DEVIATION_LOG.md` because none rise to architecture-level concern.

### IDEV-1 — `crates/slicer-host/src/dispatch.rs` modified despite "UNCHANGED" assertion

- **Asserted in design.md:** `dispatch.rs` listed under "Locked Assumptions and Invariants" as unchanged.
- **Actual:** A small bridge arm was added at the WIT host→guest boundary converting `GCodeCommand::ExtrusionMode { absolute }` → `GcodeCommand::Raw("M82")` or `Raw("M83")`.
- **Why:** The WASM guest re-emit loop in the new `machine-gcode-emit` module cannot push the typed `ExtrusionMode` variant — `GcodeOutputBuilder` exposes per-variant push methods but no `push_extrusion_mode`. The bridge keeps the typed Rust variant on the host side and only flattens it for the cross-component transport.
- **Output impact:** None. Byte-level M82\n / M83\n still appear at the same position.
- **Follow-up:** A future packet adding `push_extrusion_mode` to `GcodeOutputBuilder` (with a matching WIT method on `gcode-output-builder`) could remove the bridge and let the guest re-emit the typed variant directly.

### IDEV-2 — `crates/slicer-macros/src/lib.rs` patched at two sites; not anticipated by the WIT/Type Changes Checklist

- **Asserted in design.md / CLAUDE.md:** The WIT/Type Changes Checklist enumerates `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules as the sites to grep when a WIT-mirrored type changes. `crates/slicer-macros/src/lib.rs` is not listed.
- **Actual:** Two `match` sites in the proc macro (around `:849` and `:2932`) exhaustively destructure `GcodeOutputCommand::Command(GCodeCommand::*)`. Adding the new `ExtrusionMode` variant in Step 3 broke every core-module and test-guest WASM build until both sites grew an `ExtrusionMode` arm that passes through to `push_raw` (mirroring IDEV-1's bridge semantics).
- **Why:** The macro's generated re-emit code is the guest-side analogue of the host's `dispatch.rs` arm. With no `push_extrusion_mode` builder method, the macro must collapse to `push_raw`.
- **Output impact:** None. Mechanical and minimal — one arm per match site.
- **Follow-up:** Update CLAUDE.md's WIT/Type Changes Checklist to include `crates/slicer-macros/src/lib.rs` for any `GCodeCommand` variant addition. (Not in scope for this packet.)

### IDEV-3 — WIT variant added to `wit/world-postpass.wit`, not `wit/deps/ir-types.wit`

- **Asserted in design.md:** "Files in Scope" and "Code Change Surface" pointed at `wit/deps/ir-types.wit` as the conditional WIT edit target if `gcode-command` was mirrored.
- **Actual:** `gcode-command` is declared in `wit/world-postpass.wit:15-24`, not in `wit/deps/ir-types.wit`. The new `record gcode-extrusion-mode-cmd { absolute: bool }` and `extrusion-mode(gcode-extrusion-mode-cmd)` arm landed at the actual declaration site.
- **Why:** A scope-description error in design.md, not a design defect. The Step 3 dispatch verified the actual location before editing.
- **Output impact:** None. The edit landed at the correct unambiguous site.
- **Follow-up:** Future packets touching `gcode-command` should refer to `wit/world-postpass.wit:15-24` rather than `wit/deps/ir-types.wit`.
