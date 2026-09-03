# 26 — Close P19 — Printer / Machine / Print volume — emitter

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-03)
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P19 — Printer / Machine / Print volume — emitter** — 3 keys, Tier A plumbing, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P19 — Printer / Machine / Print volume — emitter):

`extruder_printable_area`, `extruder_printable_height`, `printable_height`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Resolved by direct implementation, not a packet** (user ruling 2026-09-03; the
map's "Packets are for complex implementation only" rule was added in the same
session and this ticket is its first application). No packet number was consumed;
the empty reserved directory `docs/spec_packets/268-printer-machine-print-volume-emitter/`
was left untouched.

### The ticket's own sizing was wrong

It called this "3 keys, Tier A plumbing". Grounding found **all three keys have
zero read sites in this tree**: `extruder_printable_area` and
`extruder_printable_height` appear only in a parser comment in
`crates/pnp-cli/src/visual_debug_gcode.rs`, and `printable_height` existed only as
the frozen `("printable_height", "250")` literal in `ORCA_CONFIG_PADDING`
(`crates/slicer-gcode/src/serialize.rs`) — which Authoring rule 2 bars as
evidence. There was no decision point to plumb into: **this tree had no
build-volume validation of any kind.** The only model-geometry validator,
`validate_world_z_floor` (`crates/slicer-model-io/src/loader.rs`), is tested but
has zero production call sites, so it was a shape precedent, not a wiring one —
and wiring it was deliberately left alone, since that would newly reject models
that slice today.

### `printable_height` — implemented

Canonical (verified against `OrcaSlicerDocumented`) declares it `coFloat` default
`100.0` in `PrintConfigDef::init_fff_params` (`PrintConfig.cpp`) and hard-rejects
in `Print::validate` (`Print.cpp`) with "The object %1% exceeds the maximum build
volume height." Its CONFIG_BLOCK emission is generic — `GCode::append_full_config`
loops every non-banned key — so canonical emits it as a side effect of the key
existing, not via a padding twin.

Built here:

- `ResolvedConfig::printable_height` (the `declare_resolved_config!` invocation in
  `crates/slicer-ir/src/resolved_config.rs`), `f32`, default `250.0`, min 0.
- `validate_printable_height` (`crates/slicer-model-io/src/loader.rs`), reading
  the same canonical `object_world_z_extent` surface `validate_world_z_floor`
  reads, returning the new `ModelLoadError::ExceedsPrintableHeight { object_id,
  z_max, printable_height }`.
- Called from `run_slice` (`crates/slicer-runtime/src/run.rs`) beside
  `validate_support_layer_heights`, per object, honouring a per-object override
  and falling back to the global value. This required promoting `slicer-model-io`
  from a dev-dependency to a real dependency of `slicer-runtime` (it depends on
  `slicer-ir` only, so no cycle).
- Emitted from the resolved config via `ResolvedConfig::to_config_map`, so the
  CONFIG_BLOCK reports the value the slice actually validated against. The dedup
  in `emit_config_kv` makes the real key shadow the padding literal; **the padding
  table itself was not edited** (Authoring rule 2).

**Default 250.0 is an intended deviation from canonical's 100.0** — user ruling.
Canonical's 100.0 is a profile fallback every real preset overrides; adopting it
alongside a new hard rejection would newly fail every model over 100 mm tall under
default config, in a tree that had no build-height gate at all. 250.0 matches this
tree's own 250 x 250 mm `printable_area` default and the value the emitter has
always advertised for this key. Filed as **DEV-167** in `docs/DEVIATION_LOG.md`;
the generated deviation table in `docs/15_config_keys_reference.md` independently
detected it (`printable_height | 250.0 | 100.0`).

Second, smaller divergence, recorded in the function's own doc comment: canonical
compares the **last sliced layer's** Z, this check compares the object's
world-space **mesh extent** before slicing. The two agree except within one layer
height of the limit, where this form is the stricter.

### `extruder_printable_area`, `extruder_printable_height` — returned to the queue

Both are per-extruder vectors defaulting to empty / `{0}` and are **inert on a
single-extruder printer**. Their only behaviour-changing canonical paths are
multi-extruder wipe-tower ones: `Print::get_extruder_shared_printable_polygon`
feeding `WipeTower::set_shared_print_bed` (clamps the tower centre), and
`WipeTower::is_valid_last_layer` (skips finish-layer/purge above an extruder's
height), the latter gated on `m_is_multi_extruder`. Everything else canonical does
with them is diagnostics or GUI, and several of those functions
(`PrintObject::detect_extruder_geometric_unprintables`,
`Print::get_extruder_printable_height`,
`GCodeProcessor::check_multi_extruder_gcode_valid`) have no caller at all.

So the emitter is the wrong owner. Under Authoring rule 1 they are left out and
returned to the queue as **unimplemented**, with the missing feature named in the
tier table; their real home is the wipe-tower packet tickets **28–31**, and a
pointer was added to ticket 28. `04-asset-tier-assignment.md` re-tiers both A → B
with the canonical read named; `05-asset-packet-list.md`'s P19 section records
the split.

### Verification

- `cargo test -p slicer-model-io --test printable_height_tdd` — 8 passed, 0 failed
  (validator: under/over/at the limit, world-space translation, empty mesh,
  non-positive limit disables, error fields, stable code).
- `cargo test -p slicer-runtime --test e2e printable_height` — 2 passed, 0 failed
  (wiring: the wedge still slices at the default; `printable_height = 1.0` fails
  the slice with `EXCEEDS_PRINTABLE_HEIGHT`). This is the rule-6(b) obligation —
  a behaviour change asserted at a non-default value.
- `cargo test -p slicer-runtime --test unit host_keys_doc_lock` — 3 passed
  (the doc-lock arm for the new key was added to `resolved_num`).
- `cargo test -p slicer-gcode` — 16 binaries, all green, including
  `golden_emit_tdd`: the CONFIG_BLOCK is byte-stable because
  `ConfigValue::Float(250.0)` renders as `250`, the padding literal's exact text.
- `cargo test -p slicer-ir` — all green. `cargo test -p pnp-cli --test
  e2e_integration_tdd` — 8 passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
  `cargo xtask check-literals` — 0 violations.
  `cargo xtask gen-config-docs` — regenerated.

Not run: `cargo test --workspace`. Per the repo's Test Discipline that is a
packet-closure ceremony command, and this ticket closed without a packet.
