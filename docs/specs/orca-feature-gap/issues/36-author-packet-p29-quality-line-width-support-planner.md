# 36 — Author packet P29 — Quality / Line width — support-planner

Type: task
Status: resolved
Assignee: wayfinder session (ses_f90d3a695ffeSWYYLS4R5Z3Syt) — claimed 2026-09-05, resolved 2026-09-05
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P29 — Quality / Line width — support-planner** — 1 keys, Tier B new logic, owner support-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P29 — Quality / Line width — support-planner):

`support_line_width`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation, no packet** — under the map's "Packets are for
complex implementation only" rule. Re-sized at claim time from the tree: every
decision point the key drives already exists, so the remaining work is two
manifest declarations plus fallback alignment, all in one session.

### Sizing

Rule 3 (dead-in-canonical, checked against the designated oracle
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`, never the GUI-only
fork): the key is live — `PrintConfig.cpp` declares it `coFloatOrPercent`
(default `0.0`, `ratio_over = "nozzle_diameter"`, min 0, max 1000,
max_literal 10), and the slicing pipeline reads it in `Flow.cpp`
(`support_material_flow` / `support_material_1st_layer_flow` /
`support_material_interface_flow`), `SupportParameters.hpp`, `TreeSupport.cpp`,
`TreeSupport3D.cpp`, `TreeSupportCommon.hpp`, and `Print.cpp` (validation).
Canonical auto rule (`Flow.cpp`): a non-positive value falls back to
`line_width`; percents resolve over the nozzle diameter.

Owner re-derivation (ticket-27 lesson — 04's `support-planner` column is
narrow): the key's decision points span **four** sites, all pre-existing:

| Site | State at claim | This ticket |
| --- | --- | --- |
| `tree-support-planner` smoothing (`smooth_nodes` max-move, `get_max_move_dist` caps) | declared + read | align auto fallback to canonical |
| `tree-support` renderer extrusion width | **read but not declared** — dead on the production path (`ConfigView::from_declared` whitelist, `crates/slicer-scheduler/src/execution_plan.rs`, `bind_module_config_view`) | declare + align |
| `traditional-support` renderer extrusion width | **read but not declared** — same silent fallback | declare + align |
| host `resolve_support_line_width_mm` (prepass territory clearance, CONFIG_BLOCK emission) | resolved 0 to the nozzle, not `line_width` | align to canonical chain |

The two renderer reads are the ticket-34 shape exactly (`thick_bridges` in
`wave-overhangs`): module tests pass because `ConfigViewBuilder`/`from_map`
bypass the manifest filter, while production filters the key out.

### Changes

- `tree-support.toml`, `traditional-support.toml`: new
  `[config.schema.support_line_width]` (`float_or_percent`, default `0.0`,
  min `0.0`, max `1000.0`, planner-row precedent). No deviation: canonical
  default is identically `0.0`; doc-15 deviation block stays at 26.
- All three modules + host now implement the canonical auto chain: explicit
  positive value wins; `0`/absent falls back to `line_width`; non-positive
  `line_width` (Orca's auto `0`) resolves on to the nozzle diameter. This is
  `Flow::support_material_flow` (`Flow.cpp`) composed with
  `Flow::new_from_config_width`: the 0 case forwards the `line_width` option,
  and a 0 `line_width` reaches `Flow::auto_extrusion_width`, whose role switch
  returns the bare nozzle diameter for `frSupportMaterial`/`Interface`/
  `Transition` (the `1.125 ×` factor in that same switch is wall roles only).
  Retired three misapplications of the wall-role auto to this support key: the
  renderers' `1.125 × nozzle` (0.45), the planner's `0.35`
  (`DEFAULT_SUPPORT_LINE_WIDTH_MM`, now a test-only width), and the host's
  0-to-nozzle shortcut (right answer at defaults, wrong as soon as
  `line_width` is set — it skipped the `line_width` step). Do NOT "unify" this
  with the port's shared `resolve_role_width`
  (`crates/slicer-core/src/flow.rs`): that resolver implements the wall-role
  auto unconditionally and has no role axis, so routing the support key
  through it would reintroduce exactly this bug.
- `docs/15_config_keys_reference.md` regenerated (+2 manifest rows);
  `docs/config/host-keys.toml` auto note corrected to
  "0 = auto line_width, then nozzle".

### Default-path output change (intended, canonical alignment)

At defaults the port extruded support at 0.45 mm (renderers) and smoothed the
planner at 0.35 mm; canonical prints both at `line_width` (0.4 here) — the
tree renderer's own F-7 comment even cited the Orca-measured 0.757 mm pitch
that assumes a 0.4 width while the code produced 0.807. All three sites now
resolve 0.4 at defaults. The host header emission is byte-stable
(`resolve(0, line_width 0, nozzle 0.4)` is still 0.4); `slicer-gcode` 16/16
green unchanged.

### Tests (rule 6b: behaviour change at non-default, plus the auto chain)

- Manifest guards (net-new, `toml` dev-dep add-if-absent precedent):
  `support_line_width_config_schema_tdd.rs` in both renderer modules — exact
  key-set assert + canonical shape (type/default/min/max).
- Geometry two-run comparisons (ticket-34 shape):
  `support_line_width_non_default_drives_extruded_width` in both
  `*_support_tdd.rs` — 0.4 vs 0.8 interface widths differ end to end.
- Auto-chain unit tests in all three `src/lib.rs` (`explicit zero →
  line_width`; `line_width 0 → nozzle`; percent → nozzle fraction) and the
  host `resolve_support_line_width_mm` test.
- Baselines updated with measured justification (ticket-102 precedent):
  tree `from_config_defaults` 0.45 → 0.4 (×2: lib + integration),
  tree F-7 pitch 0.807 → 0.757 (now identical to the traditional renderer's
  Orca-measured pin); traditional `non_positive_spacing_yields_no_paths`
  rewritten as `zero_line_width_resolves_to_nozzle_auto` (0 is auto, not an
  error — the `pitches_mm` width guard itself is kept covered by a new
  `pitches_mm_rejects_nonpositive_width` unit test on a directly-constructed
  struct).

Gates: `cargo check --workspace --all-targets` clean; `cargo clippy
--workspace --all-targets -- -D warnings` clean; `cargo xtask check-literals`
clean (0 violations); `cargo xtask gen-config-docs --check` clean;
`cargo xtask check-deviations --check` clean (58 open, 26 in doc 15 —
unchanged); `cargo xtask build-guests --check` clean after rebuild (slicer-ir
sits in every guest closure, 46 rebuilt); module suites green
(tree-support, traditional-support, tree-support-planner); `slicer-ir`,
`slicer-gcode` (CONFIG_BLOCK byte-stable), `slicer-scheduler` (incl.
`scheduler_integration` 90/90 after a `pnp_cli` rebuild), `slicer-runtime
--lib` 100/100 green; support integrated-parity contract tests green on
rebuilt guests; full `--test contract` 296/296, `--test integration`
345/345, `--test e2e` 144/144 green — no suite pinned the retired
0.45/0.35 default widths.

Not touched (out of scope for this key): the bare-number-vs-percent question
(ticket 128 — `get_abs_value` reads a bare `Float` as absolute; unchanged);
CONFIG_BLOCK spelling of the auto value (ticket 132's reader-contract
territory — emission value at defaults is unchanged).

