# 98 — Author packet P91 — Multimaterial / Multimaterial advanced — new: mmu-segmented-region

Type: task
Status: resolved
Assignee: Adelino Penedo (wayfinder session 2026-09-10)
Blocked by: 06
Map: ../map.md

## Question

> **Read before claiming (added by ticket 96, 2026-09-10).** Both of this ticket's keys
> appear to be **already live** in this tree as host-side `ResolvedConfig` fields
> (`crates/slicer-ir/src/resolved_config.rs`), driving `run_phase5_width_limit`
> (`crates/slicer-core/src/algos/paint_segmentation/width_limit.rs`) — canonical's
> `cut_segmented_layers` parity — with end-to-end coverage in
> `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs`. They landed in
> `b18c00b3` (2026-06-13), before ticket 01's asset was generated, and read `live=no` there
> only because ticket 01's probe does not scrape `ResolvedConfig` `cli` declarations — see
> [149](./149-measure-resolvedconfig-blind-spot-in-gap-inventory.md). **First act on claiming
> this ticket: verify that against the tree.** If it holds, P91 is a coverage confirmation
> (plus whatever gap remains between `run_phase5_width_limit` and canonical), not a new
> module, and the queue count drops by 2. The third member of that trio, the beam bool, was
> renamed to canonical `interlocking_beam` and re-homed by packet 306 under ticket 96; this
> ticket does not own it and must not re-home Phase 5.


Author the spec packet for **P91 — Multimaterial / Multimaterial advanced — new: mmu-segmented-region** — 2 keys, Tier C new module, owner new module mmu-segmented-region. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P91 — Multimaterial / Multimaterial advanced — new: mmu-segmented-region):

`mmu_segmented_region_interlocking_depth`, `mmu_segmented_region_max_width`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.
- **Authors an ADR** for the seam decision: guest module vs host-side wiring into the existing paint_segmentation pipeline (ADR-0033 warns undocumented host-bridge instances repeat).

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Resolved 2026-09-10. Both keys were already live. No packet, no new module, no
new claim seam. Queue target 409 -> 407.** One Tier-A declaration gap closed in
this session; one parity defect filed as
[150](./150-phase5-width-limit-drops-area-instead-of-returning-it-to-base.md).

### 1. The premise held — verified against the tree

`mmu_segmented_region_max_width` and `mmu_segmented_region_interlocking_depth`
are `cli`-bound `ResolvedConfig` fields (`crates/slicer-ir/src/resolved_config.rs`),
both defaulting to `0.0`, which matches canonical: `PrintConfig.cpp` declares
each as `coFloat` with `set_default_value(new ConfigOptionFloat(0.))`.

They drive `run_phase5_width_limit`
(`crates/slicer-core/src/algos/paint_segmentation/mod.rs`), called from
`execute_paint_segmentation` between the Phase 7 compose block and Phase 6
top/bottom propagation, which reads them through `RegionMapIR::config_for`, guards
on `!interlocking_beam`, short-circuits when both are zero, and invokes
`width_limit::cut_segmented_layers`
(`crates/slicer-core/src/algos/paint_segmentation/width_limit.rs`) — this tree's
port of canonical `cut_segmented_layers` (`MultiMaterialSegmentation.cpp`).

Behaviour coverage at non-default values (map preflight gate (b)) already ships in
`crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs`: AC-5 asserts
`max_width = 2.0` changes the sliced toolpath, AC-6 asserts `interlocking_depth =
0.5` differs from *both* the default slice and a uniform `max_width = 0.5` slice
(proving the even/odd alternation), AC-7 asserts `interlocking_beam = true` skips
the pass. Unit coverage of the kernel's erosion, alternation, degeneracy and
negative-value rejection is in `width_limit.rs`'s own test module.

So Authoring rule 1 is satisfied by shipped code. **Ticket 04's per-key tier rows
("C — new mmu-segmented-region module") are wrong**, and contradict ticket 04's
own "ResolvedConfig-only keys" special ruling, which already listed both keys as
"implemented via typed fields and consumed at decision points". The tier table
row was what P91 was sized from; the special ruling was right. Both rows are
corrected to A, and P91 is dissolved in
[05-asset-packet-list.md](./05-asset-packet-list.md).

Root cause of the mis-sizing is [149](./149-measure-resolvedconfig-blind-spot-in-gap-inventory.md):
ticket 01's probe does not scrape `ResolvedConfig` `cli` declarations, so both
keys read `live=no`. They landed in `b18c00b3` (2026-06-13), two months before the
asset was generated. This ticket confirms the third and fourth of 149's provable
original false negatives.

### 2. Tier-A residue closed in this session (direct implementation)

Ticket 04's ruling that these keys are declared in no manifest — "a contract
violation" — still held: they appeared in **no** module manifest and **not** in
`docs/config/host-keys.toml`. Old packet 96's plan to declare them in
`modules/core-modules/mesh-segmentation/mesh-segmentation.toml` never landed, and
should not: per the map's "re-derive the owner" rule the consumer is a **host
built-in** (`slicer-core`'s paint-segmentation prepass), not a guest module, so
`docs/config/host-keys.toml` `[resolved_config]` is the correct home — the table
whose whole purpose is "host-registered config keys NOT in any module manifest".

Landed here:
- two rows in `docs/config/host-keys.toml` `[resolved_config]`, defaults `0.0`,
  range `>= 0`, notes carrying the band semantics, the even-layer replacement
  rule, and the canonical tooltip divergence recorded in §3;
- two `resolved_num` arms in
  `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`, so
  `resolved_config_keys_match_default` now locks both documented defaults against
  the live `ResolvedConfig::default()` and cannot drift;
- `cargo xtask gen-config-docs` regenerated the host-key table in
  `docs/15_config_keys_reference.md` (59 host keys, up from 57).

Verification run: `cargo test -p slicer-runtime --test unit host_keys` — 3
passed, 0 failed. `cargo xtask check-literals` — 0 violations.
`cargo clippy -p slicer-runtime --all-targets -- -D warnings` — clean.

### 3. Residual gap vs canonical

Three divergences from canonical `cut_segmented_layers` were found. Only the
first is a defect.

**(a) The port deletes slice area; canonical does not.** Filed as
[150](./150-phase5-width-limit-drops-area-instead-of-returning-it-to-base.md).
Both the driver's write-back and the kernel skip the BASE chain, and BASE was
already computed as the unpainted **residual**, so the painted area deeper than
`max_width` ends up owned by no region. Canonical cuts every index including the
unpainted state 0, but its segmented regions are overrides on a parent
`LayerRegion` covering the whole slice (`apply_mm_segmentation`,
`PrintObjectSlice.cpp`), so that area falls back to the object's default
filament. Measured on `cube_4color.3mf` at `max_width = 2.0`: sparse infill
-66%, internal solid infill -55%, inner wall +109%, outer wall +34%. The two
existing e2e gates assert only that the toolpath *differs*, so they pass on a
slice that has lost most of its infill. Full evidence table in 150.

**(b) Negative values error here, no-op in canonical — intended, and it is the
port that is right.** The kernel returns
`PaintSegmentationError::InvalidPhase5Config` for a negative width or depth;
canonical's caller guards on `> 0.f`, so a negative value silently disables the
pass. This is the map's standing "a canonical `min` is a GUI hint" divergence
(ticket 113) — canonical declares `def->min = 0` for both keys and never
consults it. No action; recorded here so a future audit does not read it as a
parity break.

**(c) Canonical's tooltip promises two clauses its code does not implement, and
the port matches the code.** The tooltip on
`mmu_segmented_region_interlocking_depth` says it "will be ignored if
`mmu_segmented_region_max_width` is zero or if
`mmu_segmented_region_interlocking_depth` is bigger than
`mmu_segmented_region_max_width`". `cut_segmented_layers` implements neither: its
caller activates on `max_width > 0 || interlocking_depth > 0`, and the
`interlocking_cut_width` local that looks like the intended implementation
(`interlocking_depth > 0 ? max(cut_width - interlocking_depth, 0) : 0`) is
**never used** — the lambda captures `cut_width` and `interlocking_depth`
directly. The port mirrors the code. Recorded in the `host-keys.toml` note so it
is not "fixed" toward the tooltip later.

Noted but **not** filed: `polygon_ops::offset` hardcodes miter limit `2.0`
(Clipper2's default) where canonical's `offset_ex` defaults to
`DefaultMiterLimit = 3.0`. This is a tree-wide property of the shared primitive —
`ORCA_MORPH_MITER_LIMIT = 3.0` exists and is applied only in `opening`/`closing_ex`
— so it is a deliberate pre-existing choice, not P91's to change, and not a
config-key gap.
