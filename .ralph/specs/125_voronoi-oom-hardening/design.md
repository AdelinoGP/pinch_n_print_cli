# Design: 125_voronoi-oom-hardening (rescoped — region_id↔tool conflation)

## Controlling Code Paths

- **Conflation source:** `crates/slicer-runtime/src/layer_executor.rs` — `paint_tool =
  dominant_tool_index(&wl.feature_flags)` (~:727); the two tool-resolution chains ending in
  `.unwrap_or(region.region_id)` (walls ~:739-743, paths ~:773); the result stored as
  `RegionKey.region_id` (the tool slot, ~:747/:777).
- **OOM site:** `crates/slicer-gcode/src/emit.rs` — `required_tool = first_entity.region_key.region_id
  as u32` (~:268) and ~10 sibling `region_key.region_id as u32` sites; `max_tool =
  filament_per_tool.keys().max()` (~:637); `vec![0.0f32; max_tool + 1]` (~:638).
- **Root (read-only, do not change):** `crates/slicer-core/src/algos/paint_segmentation/mod.rs` —
  `paint_variant_region_id` (~:169-178) deriving the 64-bit identity.
- **Tripwire (in tree, keep):** `crates/slicer-runtime/tests/executor/main.rs` — the guarded
  `#[global_allocator]` from WI-1.
- **Test:** `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs`
  (`cube_fuzzy_painted_face_jitter`, `cube_4color_paint`).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- The wasm-staleness rule applies **only if** fix (A) lands in a guest perimeter module (i.e. the painted tool must be encoded into `wl.feature_flags` by `{classic,arachne}-perimeters`). If (A) is a host-side propagation fix (paint-seg prepass → region feature flags), it is host-only and no guest rebuild is needed — see Open Questions.
- **`region_id` is overloaded:** paint-seg writes a paint-variant identity into `region_id`; the
  resolver chain is meant to *overwrite* it with a tool index before emit; `emit.rs` reads it as a tool.
  This packet conforms to that existing convention (does not rename the field).

## Code Change Surface

- **Selected approach (user-chosen: correct parity fix A+B + guard):**
  - **(B)** `layer_executor.rs`: both `.unwrap_or(region.region_id)` → `.unwrap_or(0)` (or a named
    `DEFAULT_TOOL` const). Region identity can never reach the tool slot.
  - **(A)** Make `dominant_tool_index(&wl.feature_flags)` resolve for painted entities: trace why
    `feature_flags` lack the painted tool and populate it (location per Open Questions) so painted
    regions carry their real tool index and the fallback never fires for painted geometry.
  - **(Guard)** `slicer-gcode/src/emit.rs`: before `vec![0.0f32; max_tool + 1]`, reject/clamp a tool id
    that exceeds the configured extruder count with a typed error (defense-in-depth).
- **Rejected alternatives:** (a) crash-stop only (B + guard, defer A) — user chose the full parity fix,
  since tool-0-for-painted is a colour regression on a parity branch; (b) rename the `region_id`/tool
  field — correct long-term but touches all `as u32` sites + `RegionKey`, out of scope; (c) the original
  discretize cap — wrong bug.

## Files in Scope (read + edit)

Per-step files stay ≤3 (see `implementation-plan.md`).

- `crates/slicer-runtime/src/layer_executor.rs` — (B) safe fallback; possibly (A) host-side feature-flag population.
- `crates/slicer-gcode/src/emit.rs` — (Guard) bound-check; WI-6 remove the temporary diagnostic dumps.
- `<paint→tool source for (A)>` — TBD by Open Questions: either host paint-seg propagation, or a guest perimeter module's `feature_flags` population.
- `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` — non-vacuous fuzzy test.
- New tests: `crates/slicer-runtime/tests/integration/*` (AC-1), `crates/slicer-gcode/tests/*` (AC-N1).

## Read-Only Context

- `crates/slicer-core/src/algos/paint_segmentation/mod.rs:169-178` — region_id derivation (why it's huge; do not change).
- `OOM_FINDINGS.md` + the WI-1 capture — the confirmed chain + the exact value arithmetic.
- `docs/02_ir_schemas.md` — `PaintValue`/`RegionKey` fields (delegate a FACT).

## Out-of-Bounds Files

- `target/`, `Cargo.lock`, generated bindgen — never load.
- `boostvoronoi`/`cpp_map` crates — irrelevant to this bug; do not browse.
- `OrcaSlicerDocumented/` — not used.
- The full `emit.rs` / `layer_executor.rs` bodies — symbol-locate and range-read; do not load whole.

## Expected Sub-Agent Dispatches

- "In `layer_executor.rs`, where does `wl.feature_flags` get its tool, and why does
  `dominant_tool_index` return `None` for a painted entity? Trace to the population site; return
  `LOCATIONS` + a ≤5-line FACT." — resolves the (A) Open Question.
- "Return all `region_key.region_id as u32` sites in `slicer-gcode/src/emit.rs`; `LOCATIONS`." — confirm
  the source-fix covers them all.
- "Run `cargo test -p slicer-runtime --test executor cube_4color_paint`; FACT pass/fail + count." — AC-3/AC-6.
- "Run `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_face_jitter`; FACT + which
  assertion on fail." — AC-4.

## Data and Contract Notes

- No IR/WIT/manifest change. `RegionKey.region_id`-as-tool-slot is the existing host convention; the fix
  makes the resolver honor it (valid tool, not identity).
- The captured identity's low 32 bits (`as u32`) happen to equal `max_tool` — i.e. the truncation, not a
  separate corruption. The fix removes the identity from the slot entirely, so truncation is moot.

## Locked Assumptions and Invariants

- After the fix, every `RegionKey.region_id` consumed by `emit.rs` is a valid tool index `< extruder
  count` (the invariant emit.rs already assumes). AC-1/AC-3 + the emit guard (AC-N1) enforce it from both
  ends.
- `region_id`-as-paint-identity in paint-seg is unchanged (by design); only the *tool-resolution* output
  is corrected.
- The WI-1 guarded allocator stays wired (AC-5 surface).

## Risks and Tradeoffs

- **Fix (A) depth is unknown until traced** — could be a one-line missing propagation or a deeper
  feature-flag plumbing gap. If deep, AC-2/AC-3 may need more than one step; do not regress to tool-0
  fallback as a "pass."
- **Guest rebuild** needed iff (A) lands in a perimeter module (Open Questions).
- The `region_id`/tool field overload remains (out of scope) — a future reader can still misuse it; the
  emit guard (AC-N1) is the safety net until a rename refactor.

## Context Cost Estimate

- Aggregate: `M`. Largest single step: `M` (fix A — trace + populate the painted tool).
- Highest-risk dispatch: the (A) trace — must return `LOCATIONS` + a ≤5-line FACT, not file bodies.

## Open Questions

- `[FWD]` **Where does fix (A) land?** Trace why `dominant_tool_index(&wl.feature_flags)` is `None` for a
  painted entity: host-side (paint-seg prepass should propagate the tool into region/wall feature flags)
  vs guest-side (perimeter module should encode it). Resolvable in WI-3 Step start; determines whether
  `cargo xtask build-guests` is required.
- `[FWD]` Confirm the `DEFAULT_TOOL` for the (B) fallback is `0` (base extruder) per the host's
  extruder-indexing convention; verify no path legitimately relies on the old identity fallback.
