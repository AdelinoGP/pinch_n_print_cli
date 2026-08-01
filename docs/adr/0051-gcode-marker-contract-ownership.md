# ADR-0051 — `crates/slicer-gcode` owns the `;LAYER_CHANGE` / `;Z:` / `;HEIGHT:` marker contract, and it is unversioned

<!-- filename: 0051-gcode-marker-contract-ownership -->

## Status

Accepted (2026-07-25). Authored because
`.ralph/specs/187-custom-gcode-injection-registry` makes a WASM guest module a
consumer of comment text emitted by the host serializer, turning an incidental
output detail into a real cross-crate interface that nothing currently governs.

## Context

`DefaultGCodeEmitter` (`crates/slicer-gcode/src/emit.rs`) pushes three
consecutive `GCodeCommand::Raw` markers immediately before each emitted layer's
first command:

```rust
commands.push(GCodeCommand::Raw { text: ";LAYER_CHANGE".to_string() });
commands.push(GCodeCommand::Raw { text: format!(";Z:{}", format_xyz(layer_z, self.gcode_xy_decimals)) });
commands.push(GCodeCommand::Raw { text: format!(";HEIGHT:{}", format_xyz(height_delta, self.gcode_xy_decimals)) });
```

Layers with no output are skipped entirely, so the marker count equals the
header's `; total layer number:`. The spellings are ported from OrcaSlicer's
layer-change tags.

These were written as output decoration. They are not decoration any more.
**Verified live consumers, by crate:**

| Consumer | Crate | What it reads |
| --- | --- | --- |
| `crates/slicer-gcode/src/m73.rs` | producer's own crate | matches `GCodeCommand::Raw { text }` where `text == ";LAYER_CHANGE"` to place M73 progress |
| `crates/pnp-cli/src/visual_debug_gcode.rs` | `pnp-cli` | `line.starts_with(";LAYER_CHANGE")` and `line.strip_prefix(";Z:")` to build a layer table and populate `layer_z` from standalone G-code |
| `crates/slicer-runtime/src/run.rs` | `slicer-runtime` | `l.starts_with(";LAYER_CHANGE")` to derive `layer_count` as a best-effort proxy for the `slice_stats` event |
| `modules/core-modules/machine-gcode-emit` (packet 187, not yet landed) | a WASM guest | on `Raw(";LAYER_CHANGE")`, looks ahead at most two commands for a `Raw` starting with `";Z:"`, to splice `before_layer_change_gcode` / `time_lapse_gcode` / `layer_change_gcode` |

So the marker set is already a three-crate interface today, and packet 187 makes
it a **four**-party one whose newest consumer sits behind the WASM component
boundary. It has no owner, no version, no schema, and no single place where its
spelling is defined — each site carries its own string literal.

## Decision

**`crates/slicer-gcode` owns the layer-marker wire format.** Concretely, the
producing code in `DefaultGCodeEmitter`'s layer-boundary block is the normative
definition of the marker triple: which markers exist, their exact spelling,
their order, their payload formatting (`format_xyz` at `gcode_xy_decimals`), and
the rule that a layer with no output emits no marker. The goldens in
`crates/slicer-gcode/tests/golden_emit_tdd.rs` are the pin on the producer side.

**The format is unversioned, and stays unversioned.** No marker carries a
version token, there is no `;PNP_MARKERS:` preamble, and none is being added.
Consumers detect nothing and negotiate nothing; they pattern-match literals.

**What being unversioned costs, stated rather than assumed:** a consumer cannot
tell a *changed* format from a *malformed* stream, and cannot degrade to an
older behaviour. The only available response to an unrecognised stream is to
fail. That is accepted because all four consumers ship from this repository at
the same revision — there is no cross-version compatibility problem to solve,
and a version token would be ceremony that is never read. It stops being
accepted the moment a marker stream is persisted and re-consumed across
revisions; `visual_debug_gcode.rs` parsing a *standalone* G-code file is the
closest this gets today, and it is the boundary to watch.

**Obligations, which are the operative part of this ADR.**

1. **Changing a marker's spelling, order, payload format, or emission condition
   is a coordinated change.** Every consumer in the table above must be updated
   in the **same commit**, and the table in this ADR must be updated in that
   commit too. A marker change that lands alone is a defect regardless of
   whether any test happens to be red.
2. **Adding a third-party consumer requires adding a row to the table above, in
   the same commit as the consumer.** This ADR is the registry. If the table is
   not updated, the consumer is invisible to the next person changing the
   emitter, and the coordination rule in obligation 1 silently fails to protect
   it.
3. **A consumer must fail loudly on an unrecognised stream, never guess.**
   Packet 187's `ERR_MALFORMED_LAYER_MARKER` is the pattern: a `;LAYER_CHANGE`
   without a `;Z:` within two commands is a fatal module error naming the
   command index. The rejected alternative — reuse the previous layer's Z —
   would put a plausible wrong number into printer G-code, which is exactly the
   class of failure the custom-G-code trilogy exists to remove.
4. **Consumers outside `slicer-gcode` do not emit these markers.** Packet 187's
   module splices *around* the triple and does not add to it; the emitter stays
   the sole producer.

## Consequences

- **The producer now has a real interface, and its tests are load-bearing for
  code in three other crates.** `golden_emit_tdd.rs` is no longer only a
  regression pin on output aesthetics; it is the contract test for a
  cross-crate format. Weakening or regenerating it to accommodate a marker
  change is the wrong move — see obligation 1.

## Amendment — 2026-08-01 (packet 187)

This amendment retires the obligation #3 clause below verbatim, in both of its
prose forms, and replaces it with the warn-and-pass behaviour
`.ralph/specs/187-custom-gcode-injection-registry` lands for
`machine-gcode-emit`. The amendment is recorded in
`docs/DEVIATION_LOG.md` as `D-285-ADR-0051-AMENDED`.

### Retired clause (verbatim, lines 78–83 of the original)

> 3. **A consumer must fail loudly on an unrecognised stream, never guess.**
>    Packet 187's `ERR_MALFORMED_LAYER_MARKER` is the pattern: a `;LAYER_CHANGE`
>    without a `;Z:` within two commands is a fatal module error naming the
>    command index. The rejected alternative — reuse the previous layer's Z —
>    would put a plausible wrong number into printer G-code, which is exactly the
>    class of failure the custom-G-code trilogy exists to remove.

### What stands

The architecture of obligation #3 stands: a consumer must surface a malformed
marker stream to the user, never silently guess at a Z it cannot observe. The
user-facing **"no silent guess"** half of the obligation is preserved. The
mechanism for surfacing the malformed stream, and the rule for what to do
instead of guessing, are the parts that change.

Obligations #1, #2 and #4 are unchanged: a marker-spelling change is still a
coordinated cross-crate change, every consumer must be registered in the table
above, and consumers outside `slicer-gcode` still do not emit these markers.

### Replacement text

A consumer must surface a malformed stream to the user, never silently guess.
For `machine-gcode-emit`, surface means: emit exactly one warning named
`ERR_MALFORMED_LAYER_MARKER` per occurrence, continue with the prior layer's Z
(or, for layer 1, with layer 1's own initial Z context), and return `Ok` from
`run_gcode_postprocess`. The previous layer's Z is the documented fallback; the
warning is the obligation.

A future consumer that cannot degrade gracefully — that is, one for which a
plausible wrong number is *worse* than aborting the slice — must fail loud
instead, and must record that choice here. The warn-and-pass policy is
specific to the postpass injection-point model, where the walk can fall back to
a Z already in scope. Consumers that see the marker stream as their only
source of Z do not have that fallback and remain on the original "fail loudly"
rule.
- **Two consumers parse text; two match structured commands.** `m73.rs` and the
  187 module see `GCodeCommand::Raw` variants before serialization;
  `visual_debug_gcode.rs` and `run.rs` see serialized lines. A change that
  alters serialization without altering the `Raw` text (or vice versa) breaks
  one class and not the other. Both classes are in scope of obligation 1.
- **`run.rs`'s `layer_count` derivation is knowingly loose** — it accepts either
  `;LAYER_CHANGE` or `; layer` and is commented as a best-effort proxy. It is
  listed as a consumer because it *is* one, not because its precision is part of
  the contract. Do not tighten the marker format on its behalf, and do not treat
  its tolerance as licence to emit either spelling.
- **The `;Z:` payload is source text, not a number to re-render.** Its value is
  whatever `format_xyz(layer_z, gcode_xy_decimals)` produced. A consumer that
  parses it to `f32` and re-renders would emit `0.20000000298023224` for
  `;Z:0.2` and would silently disagree with the Z the surrounding G-code
  carries. Parse for comparison if you must; substitute the original text.
- **The BBL spelling is out of scope and stays out.** Canonical picks between a
  `; Z_HEIGHT: ` form and `;Z:` with a runtime ternary per printer. PnP emits
  only the latter. A future BBL flavour extends the producer and every consumer
  together, under obligation 1 — see
  [ADR-0050](./0050-custom-gcode-architecture.md)'s scoping consequence.
- **This ADR is the only thing governing the format.** If it is deleted or
  allowed to go stale, the coupling reverts to invisible.

## Alternatives considered

- **Give the module its own structured channel instead of comment text** — a WIT
  accessor handing the postpass guest layer index and Z directly. Rejected for
  now, not on principle: it is the *right* long-term shape, but it is a
  `world-gcode-postprocess` WIT change plus a rebuild of every guest, for a
  consumer that can get the same two values from text the emitter already
  produces. Revisit if a second postpass module needs layer context, or if
  obligation 1 is ever violated in practice — a real break is the evidence that
  buys the WIT change.
- **Version the markers** (e.g. a `;PNP_GCODE_MARKERS:1` preamble). Rejected:
  all consumers ship at the same revision from this repository, so no consumer
  would ever branch on the version, and an unread version token is worse than
  none — it implies a compatibility guarantee nothing enforces.
- **Centralise the literals in one `pub const`** in `slicer-gcode`, imported by
  every consumer. Rejected as insufficient rather than wrong: it cannot reach
  the guest module (which does not depend on `slicer-gcode`) or the standalone
  G-code parser (which sees text from a file, not from this process), so it
  would give the appearance of a single definition while two of four consumers
  still carried their own literal. The coordination obligation is the real
  mechanism; a shared const would only tidy two of the four sites.
- **Declare `visual_debug_gcode.rs` the owner**, since it has the most elaborate
  parser. Rejected: ownership belongs with the producer. A consumer cannot
  unilaterally decide what gets emitted.

## Cross-references

- ADR-0050 (custom G-code architecture) — the trilogy that adds the fourth
  consumer; its `;Z:`-only scoping consequence is the same fact seen from the
  module side.
- `.ralph/specs/187-custom-gcode-injection-registry` — the packet that splices
  at the marker triple and introduces `ERR_MALFORMED_LAYER_MARKER`.
- `crates/slicer-gcode/tests/golden_emit_tdd.rs` — the producer-side pin.
- Canonical OrcaSlicer `GCode::process_layer` — origin of the tag spellings and
  of the BBL / non-BBL runtime ternary.
