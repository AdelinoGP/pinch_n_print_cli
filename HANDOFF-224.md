# Handoff — packet 224 `support-family-orca-closure` (2026-08-18, session 2)

Branch `parity/support-planners`, HEAD `6f983b56`. **Working tree clean; everything below is
committed.** Packet status remains `draft`.

## How to read this document

The previous handoff recorded work as "Completed and verified" that was not. Its measurements were
taken against Orca references that have since been regenerated, and against narrow `cargo test`
runs that stop at the first failing binary. **Nothing here is called verified unless it names the
command, the commit, and — for module crates — a `--no-fail-fast` run.**

Four measurement traps, each of which cost this packet real time:

1. `cargo test -p tree-support-planner` builds **8** test binaries. Without `--no-fail-fast` it
   stops at the first failure and reports a false, smaller failure set.
2. `cargo test -p slicer-runtime --test integration support_family_closure` runs **ZERO** tests.
   The closure tests are bare `#[test] fn` wrappers with no module prefix, so that filter matches
   nothing. Every AC command in the packet used it. Use `-- <name> --exact`.
3. Guest WASM is not rebuilt by cargo. Run `cargo xtask build-guests --check` before **every**
   measurement. One stale-guest measurement already produced a completely wrong conclusion.
4. `eprintln!` from guest code does not reach the test harness. Use `push_diagnostic`, or a
   temporary `ModuleError` — fatal-error text does reach the harness.

## Commits this session

```
6f983b56  docs: punch-list audit, corrected plan, Steps 3a/3b
289a2056  docs: correct void figures, record RC-14..RC-17, amend ACs, add gap register
647a7d0a  docs: attribute tree-family failures to 9f4540bd
4d1848eb  fix: reconcile config keys; repair three wrong-reason tests
2afa4cf9  fix: routing cells bounded by extent, not absolute grid position
4c67ccd9  wip: honest tests + tree density diagnosis (2 open failures)
d97fb2b8  fix: tree honours support_top_z_distance_mm (RC-11)
4d245486  test+docs: closure tests, matched Orca config, re-authored packet
437176d4  fix(traditional-support): carve interface from body, plate-aware bottom interface
9f4540bd  fix(tree-support): render geometry, swept capsules, interface roles   <-- SEE RC-17
a62b5de9  fix(support): carry interface role end-to-end through marshal
e85f2517  fix(support): route support family to region config at plan promotion
```

`e85f2517..437176d4` are the previous session's uncommitted work, committed unchanged.

## Decisions locked with the human (do not relitigate)

| Topic | Decision |
|---|---|
| 224 scope | Closes on correctness + honest tests. Feature gaps route via the gap register |
| Parity gate | Structural invariants + human/LLM `/visual-debug` inspection with side-by-side Orca renders. **No test reads Orca G-code** |
| Orca references | Regenerated 2026-08-18 with many settings disabled. Inspection aids only, gitignored in `tmp/` |
| RC-15 contact sampling | Classified GAP, **implemented in 224** anyway (it is the dominant cause) |
| Bug cluster | Fixed together with RC-15, measured **once** at the end. Fixing them separately makes parity look worse — two errors currently cancel |
| RC-17 regressions | **Fix forward** as a punch list; do not revert `9f4540bd` |
| Benchy golden | Regenerate **last**, after all fixes, renamed off `orca_parity`, with a provenance header saying it is a PnP self-capture and not parity evidence |
| Baseline metric | Deposited material + XY path length. Never total E — it counts de-retraction primes |
| Gap routing | `docs/specs/support-parity-gap-register.md` plus packets 224a / 225 / 226 / 227 |

## Closed this session, with evidence

- **RC-11 tree top-Z gap** (`d97fb2b8`). The root cause was **not** the "unexplained contradiction"
  the last handoff described: `from_config` never read `support_top_z_distance_mm`, and the key is
  absent from `crates/slicer-schema/wit/` entirely. The 125/90/89 measurements behind the mystery
  were stale-guest artifacts. Red-first test in `orca_parity_tdd.rs`.
- **RC-14 host routing-cell defect** (`2afa4cf9`). `in_routing_cell` rejected any body whose bbox
  crossed an absolute 104.8576 mm grid line. The fixture's edge sits on y=0, so a 0.4 mm tip disc
  reached y = -0.4 mm: 528 rejections at gap 0.2 against zero at gap 0.0, destroying both
  TopInterface layers. Canonical `generate_contact_points` emits contour vertices directly, so it
  hits the same case — the validator would have rejected canonical's own output.
- **Config-key reconciliation** (`4d1848eb`), including `support_layer_height_mm`, whose
  `default = 0.0` violated its own `min = 0.05` and blocked two tests.
- **RC-16, three wrong-reason tests** (`4d1848eb`). `invalid_body_degraded`'s occupancy path was
  dead three ways: a coplanar triangle at z=100; `..ObjectMesh::default()`, whose
  `Transform3d::default()` is an all-zeros matrix collapsing every vertex to the origin; and then a
  winding that still produced an empty cross-section.
- **Test theatre deleted, four invariants added** (`4c67ccd9`). One found a real defect on its
  first run.
- **All AC commands de-vacuumed** (`289a2056`, `6f983b56`).

## Open work, in the order to do it

1. **Step 3a — RC-17 punch list.** See `tree-regression-punch-list.md`. Five of the eight
   regressions are ONE bug: `plan_for_object`'s family lookup became
   `let Some(..) else { continue }`, so an empty `family_assignments` discards every entry with no
   diagnostic. The fix distinguishes *no assignments at all* (fall back to the module's configured
   `support_family`, plus a diagnostic) from *assigned to another family* (skip, as RC-5 intends).
   Do **not** migrate the fixtures — that deletes the only coverage of the no-assignment path.
   Re-measure afterwards: several of the five may already be green.
2. **Step 2 — interface layer counts.** PnP normal emits 1 `;TYPE:Support interface` block against
   Orca's 3 at `support_interface_top_layers = 2` / `bottom_layers = 2`.
3. **The orphan-interface defect.** `interface_is_topmost_and_carved_out` is RED: tree emits a
   TopInterface at layer 119 for a column whose geometry ends at layer 79.
4. **Step 3b — RC-15 contact-sampling port.** Canonical spec is in `design.md` RC-15.
   **Read the port hazard first**: all six planner fixtures are coplanar plates with an empty
   cross-section at every Z, and pass only because the planner never slices
   (`detect_overhang_facets` reads triangles directly). A naive port turns five green tests red in
   a way that looks like the port is wrong.
   Also answer the punch list's UNKNOWN on canonical tree-base infill — it claims no OrcaSlicer
   checkout is available, which is **false**; `OrcaSlicerDocumented/` is present in this checkout.
5. **Steps 6, 7 (remainder), 8** — inspection checklist, `docs/07` rows, golden regeneration,
   acceptance ceremony.

## Current test state (measured at `4d1848eb`, `--no-fail-fast`, guests clean)

- 10 failures in the tree family: 8 introduced by `9f4540bd`, 2 inherited. The baseline at
  `5a38fdce` was 3. **The tree family is worse than before this packet began.**
- `interface_is_topmost_and_carved_out` RED (genuine defect, assertion not weakened).
- `final_gcode_roles` PASSES. `invalid_body_degraded` and both `invalid_body_rejected` PASS, now
  testing what they claim.
- `slicer-wasm-host` contract: 97 passed / 1 failed
  (`support_plan_aggregation_diagnoses_duplicate_identity`, pre-existing, unrelated).
- `cargo check --workspace --all-targets` clean; clippy clean on every touched crate.
- `cargo xtask check-literals` exits 1 on **61 inherited violations across 34 files, 0 attributable
  to this branch** — verified two ways: no added line opens a watched-type literal, and the
  watchlist did not grow. The human approved committing over it.

## Measured baseline (2026-08-18, regenerated references, guests clean)

|  | PnP tree | Orca tree |
|---|---|---|
| deposited support filament | 388.73 mm | 683.96 mm (PnP = 56.8%, a **1.76x** deficit) |
| support XY path length | 11,687.5 mm | 22,774.9 mm (1.949x short) |

PnP over-extrudes 1.107x per mm; 1.949 / 1.107 = 1.76. **Do not quote 31.6% / 486.33 / 1538.36** —
those summed de-retraction primes, which penalises PnP twice for the same defect (Orca's prime
count scales with its loop count). Extruding-move counts are not a parity metric either: Orca's
segments are ~15x shorter, so the count measures polygon granularity, not material.

Both families and both references now sit on the same 150-layer grid. The old "205 distinct print Z"
divergence is gone from the reference and is no longer a 224 concern; independent support-layer Z
remains a feature PnP lacks, routed to packet 225.
