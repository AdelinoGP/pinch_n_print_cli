# Implementation Plan: 155-arachne-beading-simplify-parity

## Execution Rules

- One atomic step at a time.
- Each step maps back to the audit gaps G15 and G20 (backlog source
  `docs/18_arachne_parity_audit.md`; no `docs/07` task IDs).
- TDD first (the red gap tests already exist; for G15 the test body
  is rewritten as part of Step 1's call-site test extension), then
  implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by
  `spec-packet-generator`, `swarm`, and `spec-review`. The fields below
  are the budget contract for the step.

## Steps

### Step 1: Extend the `BeadingStrategy` trait + wire forwarding in all 4 decorators

- Gaps: G15.
- Objective: add `get_split_middle_threshold(&self) -> f64` and
  `get_add_middle_threshold(&self) -> f64` to the trait in
  `beading/mod.rs` as **required methods with NO default impl** and **no
  arguments** (Orca: `double getSplitMiddleThreshold() const`,
  `BeadingStrategy.hpp:166`). Then, in the same step, add the forwarding
  impls to the four decorators — `Redistribute` (`redistribute.rs`),
  `Widening` (`widening.rs`), `OuterWallInset` (`outer_wall_inset.rs`),
  `Limited` (`limited.rs`) — each returning `self.parent.<method>()`.
  `Distributed` gets a temporary `todo!()`-free placeholder returning `0.99`
  that Step 2 replaces with the real stored fields (or, preferably, Step 2
  is merged in — see the note below). Confirm the trait remains object-safe.
  **Why forwarding is mandatory:** the four decorators already implement
  every trait method explicitly and forward to `parent`; they inherit
  nothing. A default impl would be picked up at the `Limited` layer and
  shadow `Distributed`'s real value, making AC-2 unsatisfiable.
- Precondition: packet active. `BeadingStrategy` trait def is at
  `crates/slicer-core/src/beading/mod.rs:64-153`.
- Postcondition: workspace compiles; trait object-safe
  (`Box<dyn BeadingStrategy>` at `factory.rs:174` still builds).
- Files allowed to read: `crates/slicer-core/src/beading/mod.rs:64-153`,
  and the `impl BeadingStrategy for …` blocks in
  `{distributed,redistribute,widening,outer_wall_inset,limited}.rs`
  (signatures only, not the `compute` bodies).
- Files allowed to edit: `crates/slicer-core/src/beading/mod.rs`,
  `widening.rs`, `outer_wall_inset.rs`, `limited.rs`, `redistribute.rs`,
  `distributed.rs`. **Cap-bust to 6 files is justified and unavoidable:**
  making a trait method required is atomically breaking — every implementor
  must gain the method in the same commit or the crate does not compile.
  The edits are 4-line forwarding stubs in four of the six files.
- Files out-of-bounds: the factory (Step 5), the test files (Step 6).
- Expected sub-agent dispatches:
  - "Confirm `BeadingStrategy` remains object-safe after adding the
    two required methods; FACT yes/no + ≤5 lines of evidence."
  - "Run `cargo check --workspace --all-targets`; FACT pass/fail or
    SNIPPETS (compile error) on fail."
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: `BeadingStrategy.hpp:166` (signature),
  `BeadingStrategy.hpp:74-78` (thresholds are required ctor params — no
  sentinel), and the four decorator ctors that copy-construct
  `BeadingStrategy(*parent)` (`RedistributeBeadingStrategy.cpp:39-48`,
  `WideningBeadingStrategy.cpp:39-44`,
  `OuterWallInsetBeadingStrategy.cpp:39-43`,
  `LimitedBeadingStrategy.cpp:54-58`) — delegate; never load.
- Verification: `cargo check --workspace --all-targets` clean.
- Exit condition: trait compiles, object-safe, all 5 implementors have both
  methods, the 4 decorators forward to `parent`.

### Step 2: Port `DistributedBeadingStrategy` fields + overrides + 2 impls

- Gaps: G15.
- Objective: add `wall_split_middle_threshold: f64` and
  `wall_add_middle_threshold: f64` to the struct; extend `new(...)` to
  take both as trailing args (preserving the existing 5-arg call
  signature is NOT required — the factory is the only caller and is
  updated in Step 5); return the stored fields from the two trait methods
  added in Step 1; port `optimal_bead_count` from
  `DistributedBeadingStrategy.cpp:132-144` — **integer-truncating**
  `naive_count = (thickness / optimal_width).trunc()` (**not** the current
  `.round()` at `distributed.rs:177-179`), parity-based `minimum_line_width`,
  and `naive_count + (remainder >= minimum_line_width)` with a `>=`; port
  `get_transition_thickness` to the parity-based formula from
  `BeadingStrategy.cpp:90-102`.
- Precondition: Step 1 landed (trait methods exist).
- Postcondition: AC-3 green; `Distributed::optimal_bead_count` returns the
  OrcaSlicer formula's value for parameterised inputs, and the falsifying
  case (`optimal_width = 4000`, `split = add = 0.99`,
  `optimal_bead_count(7500) == 1` where the old `.round()` gave `2`) passes.
- Files allowed to read: `crates/slicer-core/src/beading/distributed.rs`
  (whole file ~227 lines, load directly).
- Files allowed to edit (≤3): `crates/slicer-core/src/beading/distributed.rs`.
- Files out-of-bounds: factory (Step 5), test (Step 6), other implementors
  (Step 3).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --test beading_distributed -- distributed_optimal_bead_count_uses_split_middle_threshold --exact`; FACT pass/fail or SNIPPETS (fail with assertion + ≤20 lines)."
- Context cost: `S`.
- Authoritative docs: `docs/18_arachne_parity_audit.md` (load the G15
  detailed-gap section, lines 324-353).
- OrcaSlicer refs: `BeadingStrategy.cpp:90-102`,
  `DistributedBeadingStrategy.cpp:132-144` — delegate SUMMARY; never load.
- Verification: the FACT dispatch above.
- Exit condition: AC-3 green.

### Step 3: Port `RedistributeBeadingStrategy` 3 methods

- Gaps: G15.
- Objective: port the three methods from `RedistributeBeadingStrategy.cpp:50-85`:
  - `optimal_thickness(bead_count)` — `inner = max(0, bead_count - 2)`,
    `outer = bead_count - inner`,
    `parent.optimal_thickness(inner) + optimal_width_outer * outer`.
  - `get_transition_thickness(lower_bead_count)` — `case 0` →
    `minimum_variable_line_ratio * optimal_width_outer`; **`case 1`** →
    `(1.0 + parent.get_split_middle_threshold()) * optimal_width_outer`
    (this is the ONLY branch that consults the split threshold — it is
    **not** `case 0`); `default` →
    `parent.get_transition_thickness(lower_bead_count - 2) + 2 * optimal_width_outer`.
  - `optimal_bead_count(thickness)` — `thickness < minimum_variable_line_ratio * optimal_width_outer`
    → `0`; `thickness <= 2 * optimal_width_outer` →
    `if thickness > (1.0 + parent.get_split_middle_threshold()) * optimal_width_outer { 2 } else { 1 }`;
    otherwise `parent.optimal_bead_count(thickness - 2 * optimal_width_outer) + 2`.

  Note the 2-bead branch requires `thickness > (1 + split) * W` — with
  `split = 0.5` that means `> 1.5W`, so `0.9W` yields **1**, not 2.
  The `compute` method is UNCHANGED.
- Precondition: Step 2 landed (`Distributed` carries the thresholds
  that `Redistribute` will read via the parent).
- Postcondition: AC-4 + AC-N2 green; existing `redistribute::compute`
  tests still pass.
- Files allowed to read: `crates/slicer-core/src/beading/redistribute.rs`
  (whole file ~186 lines, load directly); existing
  `crates/slicer-core/tests/beading/redistribute.rs` test file.
- Files allowed to edit (≤3): `crates/slicer-core/src/beading/redistribute.rs`.
- Files out-of-bounds: factory (Step 5), test (Step 6), `Distributed`
  (Step 2).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --test beading_redistribute -- redistribute_optimal_bead_count_consults_split_middle --exact`; FACT pass/fail."
  - "Run `cargo test -p slicer-core --test beading_redistribute -- redistribute_compute --exact`; FACT pass/fail (AC-N2 lock)."
- Context cost: `S`.
- Authoritative docs: none new.
- OrcaSlicer refs: `RedistributeBeadingStrategy.cpp:50-85` — delegate
  SUMMARY; never load.
- Verification: the two FACT dispatches above.
- Exit condition: AC-4 + AC-N2 green.

### Step 4: Restructure `simplify_distance_gated` + add 2 helpers + Shoelace

- Gaps: G20.
- Objective: rewrite `simplify_distance_gated` to track `previous` and
  `previous_previous` as **`ExtrusionJunction` value copies** (not indices —
  `ExtrusionLine.cpp:75,79`); add the two new helpers
  `line_intersection_infinite(a, b, c, d) -> Option<(f64, f64)>` and
  `dist_greater(p1, p2, threshold) -> bool` (**three** args — the
  overflow-avoiding component-wise fast-reject then squared-norm compare,
  `ExtrusionLine.cpp:180-188`); port the tier-3 special case
  (`ExtrusionLine.cpp:166-220`: the
  `next_length2 > 4 * smallest_line_segment_squared` branch, the intersection
  computation, the `dist_greater` reject path at `:189-200`, and the
  junction-replacement else-branch at `:201-217` which carries **`curr`'s**
  width and `perimeter_index` verbatim, pops the previously-pushed junction,
  restores `previous = previous_previous`, then re-advances both cursors and
  `continue`s); replace the height calc at both gate sites with the OrcaSlicer
  Shoelace formula `height_2 = (area_removed_so_far)² / base_length_2`
  (`ExtrusionLine.cpp:151`), where `area_removed_so_far` is the
  **per-iteration local** `accumulated_area_removed + negative_area_closing`
  (`:139`) and `accumulated_area_removed` is the **running accumulator**
  (declared `:104`, incremented `:131`, reset to `removed_area_next` at
  `:211` and `:223`).

  **`use_distance_gates` (`simplify.rs:103-104`) is NOT modified.** The
  RED test reaches the dist-gated path through the existing condition once
  its parameters are corrected in Step 6.
- Precondition: packet active. `simplify_distance_gated` exists at
  `crates/slicer-core/src/arachne/simplify.rs:139`;
  `calculate_extrusion_area_deviation_error` at `:286`;
  `point_line_distance_squared` at `:314`.
- Postcondition: AC-7 + AC-8 + AC-9 + AC-N3 + AC-N4 green. (AC-6 is a
  `slicer-runtime` test and closes in Step 6.)
- Files allowed to read: `crates/slicer-core/src/arachne/simplify.rs`
  (whole file ~350 lines, load directly — the entire file is in
  scope for this step).
- Files allowed to edit (≤3):
  `crates/slicer-core/src/arachne/simplify.rs`,
  `crates/slicer-core/tests/arachne_simplify_intersection_distance_gate_tdd.rs`
  (NEW — the 5 G20 unit tests). **No `Cargo.toml` edit** — top-level
  `crates/slicer-core/tests/*.rs` files are auto-discovered by Cargo as
  integration-test binaries (every existing `arachne_*.rs` test in that
  directory is registered this way, with no `[[test]]` entry). The
  `[[test]] path = …` entries at `Cargo.toml:75-97` exist only because
  those files live in the `tests/beading/` **subdirectory**.
- Files out-of-bounds: pipeline (no change), other arachne modules
  (no change), `arachne_parity_round2.rs` (Step 6).
- Expected sub-agent dispatches:
  - "Delegate OrcaSlicer `ExtrusionLine.cpp:56-243` simplify walk;
    return SUMMARY (≤200 words) + at most two 30-line SNIPPETs
    (the tier-3 block at `:162-220` + the `dist_greater` lambda at
    `:180-188`)."
  - "Run `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_intersection_distance_gate_preserves_junction --exact`; FACT pass/fail."
  - "Run `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_junction_replacement_moves_to_intersection --exact`; FACT pass/fail."
  - "Run `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_distance_gated_uses_shoelace_height_2 --exact`; FACT pass/fail."
  - "Run `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_degenerate_two_junctions_unchanged --exact`; FACT pass/fail (AC-N3)."
  - "Run `cargo test -p slicer-core --test arachne_simplify_intersection_distance_gate_tdd -- simplify_closed_line_minimum_size_preserved --exact`; FACT pass/fail (AC-N4)."
- Context cost: `M` (the highest-risk single step in this packet).
- Authoritative docs: none new.
- OrcaSlicer refs: `ExtrusionLine.cpp:56-243` — delegate SUMMARY +
  SNIPPETs (the heaviest dispatch in the packet).
- Verification: the 5 test dispatches above.
- Exit condition: AC-7 + AC-8 + AC-9 + AC-N3 + AC-N4 green.

### Step 5: Thread thresholds through `BeadingFactoryParams` + factory

- Gaps: G15.
- Objective: add `wall_split_middle_threshold: f64` and
  `wall_add_middle_threshold: f64` to `BeadingFactoryParams`; set their
  `Default` values from the **real live field names** (`factory.rs:144-160`
  — there is **no `min_bead_width` field**; the `min_bead_width` config key
  surfaces as `min_output_width`) via the OrcaSlicer clamp formulas
  (`WallToolPaths.cpp:619-640`):
  - `split = clamp(2 * min_output_width / preferred_bead_width_outer - 1, 0.01, 0.99)`
    → `clamp(2*4000/4000 - 1) = 0.99` at the shipped defaults
  - `add = clamp(min_output_width / optimal_width, 0.01, 0.99)`
    → `clamp(4000/4000) = 0.99` at the shipped defaults

  (Orca divides by the **external** perimeter width and the **inner**
  perimeter width respectively — two different, Flow-converted widths.
  `preferred_bead_width_outer` / `optimal_width` are PnP's nearest
  analogues; the residual is D-155.) Update `create_stack`
  (`factory.rs:187-228`) to pass both to `DistributedBeadingStrategy::new`.
  Audit for any call site that constructs `BeadingFactoryParams` literally
  (rather than via `..Default::default()`) and add the two fields there —
  including `crates/slicer-core/src/arachne/pipeline.rs`'s
  `to_beading_factory_params` (`:269-292`).
  **Do not alter the `[0.01, 0.99]` clamp bounds** — AC-N1 locks them.
- Precondition: Step 2 landed (`Distributed::new` takes the new
  args).
- Postcondition: AC-5 + AC-N1 green; `BeadingFactoryParams::default()`
  produces the `0.99/0.99` threshold pair.
- Files allowed to read: `crates/slicer-core/src/beading/factory.rs`
  (whole file ~229 lines, load directly);
  `crates/slicer-core/src/arachne/pipeline.rs:269-292`
  (`to_beading_factory_params`, range-read).
- Files allowed to edit (≤3): `crates/slicer-core/src/beading/factory.rs`,
  `crates/slicer-core/src/arachne/pipeline.rs` (only
  `to_beading_factory_params`, if it constructs the struct literally),
  `crates/slicer-core/tests/beading/factory.rs`.
- Files out-of-bounds: `Distributed` (Step 2), `Redistribute` (Step 3),
  the rest of `pipeline.rs`.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --test beading_factory -- beading_factory_passes_split_middle_thresholds --exact`; FACT pass/fail."
  - "Run `cargo test -p slicer-core --test beading_factory -- beading_factory_threshold_propagates_through_full_stack --exact`; FACT pass/fail (AC-1's stack-forwarding lock)."
  - "Run `cargo test -p slicer-core --test beading_factory -- beading_factory_threshold_clamp_bounds_are_canonical --exact`; FACT pass/fail (AC-N1)."
- Context cost: `S`.
- Authoritative docs: `docs/15_config_keys_reference.md` (load the
  `min_bead_width` and `optimal_width` entries directly).
- OrcaSlicer refs: `WallToolPaths.cpp:619-640` — delegate SUMMARY;
  never load.
- Verification: the three FACT dispatches above.
- Exit condition: AC-1's stack-forwarding lock + AC-5 + AC-N1 green.

### Step 6: Rewrite the two RED test bodies + verify AC-10 regression lock

- Gaps: G15 + G20.
- Objective: two declared, justified test-body exceptions in
  `crates/slicer-runtime/tests/arachne_parity_round2.rs`:
  1. **G15** (`arachne_parity_beading_split_middle_threshold_exposed`,
     body at `:134-160`): replace the unconditional `assert!(false)` with
     real calls — build a stack with **every decorator present**
     (`print_thin_walls = true`, `outer_wall_offset != 0.0`), then call
     `stack.get_split_middle_threshold()` and
     `stack.get_add_middle_threshold()` (**no arguments**) on the `Limited`
     top of the stack and assert both equal the factory-computed `0.99`.
     This is what proves the decorator forwarding chain works end-to-end.
     Sanctioned by the test's own doc note at `:120-132`.
  2. **G20** (`arachne_parity_simplify_intersection_distance_gate_present`,
     `simplify_toolpaths` call at `:192`): change the *parameters* from
     `(…, 0.01, 0.0, f64::INFINITY, f64::INFINITY)` to
     `(…, 0.01, 1e-3, 1.0, f64::INFINITY)` and strengthen the assertion
     from `kept >= 4` to `kept == 4` plus an exact junction-sequence check.
     **Rationale (must be reproduced in the test's doc comment):** with
     `smallest_line_segment_squared = 0.0` the tier-3 gate
     (`ExtrusionLine.cpp:162-164`) reduces to `length2 < 0`, which is
     unsatisfiable for every input because `length2` is a squared norm —
     so the entire intersection/`dist_greater` path (`:166-220`) is
     **dead** and the old test could not have exercised the gate it names.
     The new parameters place junction 2 inside the gate. **The assertion
     is strengthened, never weakened.**

  Then run the full regression-lock suite: 14 round-1 `arachne_parity.rs`
  locks + G3/G10 closures + the `factory_orca_reference.json` golden. Any
  bead-count shift must be adjudicated per `design.md` §Risks (recompute by
  hand from the OrcaSlicer formula, then re-record and log in D-155) — not
  rubber-stamped.
- Precondition: Steps 1-5 landed.
- Postcondition: AC-2 (G15 RED flips green) + AC-6 (G20 RED flips
  green) + AC-10 (regressions adjudicated) green.
- Files allowed to read: `crates/slicer-runtime/tests/arachne_parity_round2.rs`
  (whole file ~218 lines, load directly);
  `crates/slicer-runtime/tests/arachne_parity.rs` (delegate
  SUMMARY of the 14 lock test names).
- Files allowed to edit (≤3): `crates/slicer-runtime/tests/arachne_parity_round2.rs`.
- Files out-of-bounds: the source files (Steps 1-5 are done; no
  source changes in this step).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test arachne_parity_round2 --
    arachne_parity_beading_split_middle_threshold_exposed --exact`;
    FACT pass/fail or SNIPPETS (fail with assertion + ≤20 lines)."
  - "Run `cargo test -p slicer-runtime --test arachne_parity`; return
    FACT pass/fail (AC-10 14-locks regression lock) or SNIPPETS (fail
    with assertion + ≤20 lines)."
- Context cost: `S`.
- Authoritative docs: `docs/18_arachne_parity_audit.md` (the G15 + G20
  detailed sections; the "closing a gap turns its test green with no
  rewrite" promise is acknowledged as an exception for **both** tests —
  G15's body and G20's parameters).
- OrcaSlicer refs: `ExtrusionLine.cpp:162-164` (the tier-3 gate whose
  `length2 < smallest_line_segment_squared` guard is unsatisfiable at
  `smallest_line_segment_squared = 0`) — delegate; never load.
- Verification: the two dispatches above.
- Exit condition: AC-2 + AC-6 + AC-10 green.

### Step 7: Doc updates + `cargo xtask build-guests --check` + final gates

- Gaps: G15 + G20.
- Objective: update `docs/18_arachne_parity_audit.md` Gap summary
  table to mark G15 + G20 closed; update the detailed-gap "PnP
  status" entries to "closed (this packet)"; add D-155 (beading
  threshold parity) and D-156 (simplify intersection gate) entries
  to `docs/DEVIATION_LOG.md`; add *split-middle threshold* and
  *intersection-distance gate* glossary entries to `CONTEXT.md`;
  run `cargo xtask build-guests --check` to confirm the beading
  trait-surface extension did not break guest builds; run the
  final `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  the 4 AC-grep checks from `packet.spec.md` §Doc Impact.
- Precondition: Steps 1-6 landed.
- Postcondition: packet acceptance ceremony green; every AC
  green; every doc grep returns a hit; guest WASM fresh.
- Files allowed to read: `docs/18_arachne_parity_audit.md`,
  `docs/DEVIATION_LOG.md` (the D-105B/C/E entries only),
  `CONTEXT.md` (the current glossary section only).
- Files allowed to edit (≤3): `docs/18_arachne_parity_audit.md`,
  `docs/DEVIATION_LOG.md`, `CONTEXT.md`.
- Files out-of-bounds: source code (no changes in this step).
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; return FACT clean / STALE."
  - "Run `cargo check --workspace --all-targets`; FACT pass/fail."
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`;
    FACT pass/fail."
  - "Run each of the 4 doc-grep checks from `packet.spec.md` §Doc
    Impact; return FACT hit/no-hit for each."
  - "Run `cargo test -p slicer-core`; FACT pass/fail (final unit
    sweep)."
- Context cost: `S`.
- Authoritative docs: `docs/07_implementation_status.md` (delegate
  SUMMARY of the current M2 chain status; the implementer updates
  the M2 entry to mark P155 (or whatever packet number is assigned)
  complete).
- OrcaSlicer refs: none.
- Verification: all 5 dispatches above.
- Exit condition: every AC green, every doc grep hits, clippy
  clean, guests fresh, `docs/07` updated.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Trait extension; audit 5 impls by signature only. |
| Step 2 | S | `Distributed` field additions + 2 impl ports. |
| Step 3 | S | `Redistribute` 3-method port. |
| Step 4 | M | The highest-risk step (simplify restructure + Shoelace + 2 new helpers + tier-3 special case). |
| Step 5 | S | `BeadingFactoryParams` field additions + factory wiring. |
| Step 6 | S | Test rewrite + AC-10 regression sweep. |
| Step 7 | S | Doc updates + final gates. |

Aggregate: M. Largest single step: M (Step 4). No step is L.

## Packet Completion Gate

- All 7 steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command
  dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for the M2 chain
  (via worker dispatch — never edited by loading the full backlog
  into the implementer's context).
- `docs/18_arachne_parity_audit.md` Gap summary table updated.
- `docs/DEVIATION_LOG.md` D-155 + D-156 entries added.
- `CONTEXT.md` glossary entries added.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from
  `packet.spec.md` (AC-1 through AC-10 + AC-N1 through AC-N4 =
  14 commands).
- Confirm packet-level verification commands are green (the 3 gate
  commands in `packet.spec.md` §Verification).
- Record any remaining packet-local risk explicitly before moving
  to `status: implemented`. Likely residual: the
  `is_closed = true` OrcaSlicer special case is deferred (FWD
  question in `design.md`).
- Confirm the implementer's peak context usage stayed within its
  declared band (≤150k standard; ≤300k only with a logged
  ESCALATION block); if not, log it as a packet-authoring lesson
  for future `spec-packet-generator` runs.
