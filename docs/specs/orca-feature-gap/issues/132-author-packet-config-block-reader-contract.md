# 132 — Author packet — the CONFIG_BLOCK is a contract with OrcaSlicer's reader

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 112, which measured the CONFIG_BLOCK surface and found the work
larger than the padding table it was scoped to — and found a live correctness
bug on the *other* side of the seam. Read
[112](112-derive-config-block-padding-from-resolved-config.md) first; it holds
every measurement and the canonical grounding, and this ticket does not repeat
them.

Author a packet covering `serialize_config_block` / `emit_config_kv` /
`ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) as one seam: the
block is not a debug dump, it is the input to canonical
`ConfigBase::load_from_gcode_file` (`Config.cpp`), and that reader has three
behaviours this port does not currently respect.

**Half A — value spelling (the correctness half; ticket 112 rates this the
higher severity).** Canonical `ConfigOptionBool::deserialize` accepts only
`"1"` / `"0"`. The port emits word-form `true` / `false`. Under the G-code
viewer's `ForwardCompatibilitySubstitutionRule::EnableSilent`, the coBool
fallback in `ConfigBase::set_deserialize_raw` routes the value through
`ConfigHelpers::enum_looks_like_true_value`, which matches only `"enabled"` /
`"on"` — so **every `true` this port emits is read back by OrcaSlicer as
`false`**, silently, with no warning logged and the pair still counted. The
same fallback's `else` arm resets a `coEnum` with an unrecognised keyword to
`optdef->default_value`, which is what `wall_generator = Classic` (a
`format!("{:?}")` of the Rust variant, against canonical's lowercase
`classic` / `arachne`) does today. Fix the formatter, not the call sites: a
type-aware canonical serializer is the deliverable, and `SupportType::as_canonical_str`
(`crates/slicer-ir/src/slice_ir.rs`) is the in-tree precedent — the same bug was
found and fixed for `support_type` alone, and its sibling enums never got the
treatment.

**Half B — derivation and the floor.** Ticket 112's ruling: derive what can be
derived from module manifest defaults (`ConfigFieldEntry.default`, which carries
a string for *every* field type, unlike the percent-only `parsed_default`), keep
an explicitly canonically-grounded floor list for what cannot, and delete the
rows that are dead or that canonical drops. The threading is the work:
`ConfigBoundsIndex` is built from manifests and already rides `PipelineConfig`,
but `run_postpass_with_thumbnail` (`crates/slicer-runtime/src/pipeline.rs`) does
not receive it and `ThumbnailAwareSerializer` has no access to manifest state.

**Obligations the packet must carry:**

- **Measure the accepted-pair count before changing anything.** Ticket 112
  established that canonical's `key_value_pairs` counter increments **inside**
  the `try`, only after `set_deserialize` returns without throwing — so a key
  canonical does not recognise contributes **nothing** to the ≥80 floor. The
  port's current margin is therefore unknown: ticket 112 measured 96 emitted
  lines and identified 6 canonical-unknown keys among them, but did **not**
  classify the whole emitted key set, so the true accepted count is unmeasured.
  Classify every emitted key against `PrintConfigDef` and report the number
  before and after.
- **Replace the floor test with one that asserts the right quantity.** The
  existing assertion in
  `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  counts *lines*, not accepted pairs, so it does not test canonical's condition
  and would stay green through a regression that takes the real count below 80.
- **The anti-drift property the Q5 ruling actually wanted.** For the residual
  hardcoded floor list, a test asserting each entry is (a) a live key in
  canonical's `PrintConfigDef` and (b) declared by no loaded module — so a
  hardcoded row can never shadow a real value. That is the guarantee; "no
  hardcoded values at all" is not achievable for keys nothing in-tree owns.
- **Rule the three multi-module default disagreements** ticket 112 measured
  (`inner_wall_speed` 45/60, `line_width` 0/0.4, `outer_wall_speed` 30/60) and
  the 26 declarations carrying no `default` at all. A derivation needs a
  collision rule and a missing-default rule, and both must be honest rather than
  first-wins-by-accident.
- **The e2e canary moves.** `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs`
  asserts exactly 95 distinct CONFIG_BLOCK keys. That number changes; re-derive
  it, do not guess it, and keep the assertion exact rather than loosening it to
  a range.

Not a queue key; changes no queue count. Authoring rules 1–6 bind as usual.
Ledger facts (next packet number, the 95, line counts) re-derived from disk at
authoring time.

## Answer
