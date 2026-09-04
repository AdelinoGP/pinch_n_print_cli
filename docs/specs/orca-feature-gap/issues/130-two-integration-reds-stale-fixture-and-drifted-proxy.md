# 130 — The two integration reds: a fixture that enabled two producers, and a proxy that drifted

Type: bug
Status: resolved
Assignee: wayfinder session (2026-09-04)
Blocked by: —
Map: ../map.md

## Question

Ticket 129's gating found `cargo test -p slicer-runtime --test integration` at
338/340 on `wayfinder/ticket-100-wipe-tower-rename`, reproduced on a stashed
`71a4c832` baseline through `cargo xtask test`:

- `region_partition_tdd::shell_band_excludes_exposed_seed_but_keeps_propagated_under_top_fill`
  — `assertion failed: intersection(lower, &[exposed_half]).is_empty()`
- `no_linker_module_degraded_raw_output_tdd::no_linker_module_degraded_raw_output`
  — "mean G1 moves per sparse-infill block should be at the raw baseline
  (< 28.0); got 28.05"

Both were undiagnosed. **Neither turned out to be a defect in production code.**

## Answer — red 1: two producers now write one field

`internal_solid_fill` has had a single producer for as long as this test existed:
the shell-band marker `difference(top_solid_fill, top_solid_seed)` in
`commit_shell_classification_builtin`. The test asserts the marker excludes the
depth-0 exposed top seed.

`71a4c832` (five-way fill partition) gave the field a **second** producer. Its
only behavioural change in `slice_postprocess_prepass.rs` is one line in
`convert_small_sparse_islands`:

```
-            region.bottom_solid_fill = union(&region.bottom_solid_fill, &small);
-            if region.bottom_shell_index.is_none() {
-                region.bottom_shell_index = Some(1);
-            }
+            region.internal_solid_fill = union(&region.internal_solid_fill, &small);
```

The fixture leaves `minimum_sparse_infill_area` at its canonical default of
15 mm², so conversion was silently on. Measured (instrumented, this tree):

| | polygons | area | extent |
|---|---|---|---|
| marker alone | 1 | 38.6400 mm² | x ∈ [5.400, 9.600], y ∈ [0.400, 9.600] |
| after conversion | 3 | 50.0347 mm² | + two 0.0087 mm² slivers at x ∈ [0, 0.2] |

The two slivers are rounding leftovers at the outer corners of the exposed seed —
genuine sub-threshold sparse islands, correctly converted to internal solid.
They are `difference(infill_areas, solids)` and so lie strictly **outside**
`top_solid_fill`; the exposed top surface itself stays top solid. The leak the
assertion caught was 0.0347 mm² of island geometry, not shell-band geometry.

So the code is right and the fixture was stale — it had stopped testing one
thing.

**Fix.** The fixture sets `minimum_sparse_infill_area: 0.0` — canonical's own
"feature off" value for the threshold — which isolates the marker. Both original
assertions are kept **verbatim and unrelaxed**: the exposed-half intersection is
empty, and the propagated area is 38.64 mm². Verified: marker = exactly one
polygon, 38.6400 mm², x ∈ [5.400, 9.600], leak 0.000000.

A companion test, `converted_islands_join_the_shell_band_marker_in_the_internal_bucket`,
pins the other half at the canonical default so the interaction cannot be lost:
conversion must add area beyond the marker, islands must reach the exposed half,
and — load-bearing — whatever lands in the exposed half must not overlap
`top_solid_fill`. That last assertion is the invariant the original `is_empty()`
carried while the bucket had one producer.

## Answer — red 2: the discriminator drifted off its own claim

AC-N1's claim is about **path shape**: without the linker, sparse infill is the
raw disjoint form, "mean points-per-path ≤ 2". The test proxied that with mean
G1 moves per `;TYPE:Sparse infill` **block**. A block is a run under one `;TYPE:`
header and holds many paths, so the proxy tracks how paths are *grouped and
ordered*, not how long they are — and grouping changes on every packet that
reshapes infill islands.

Its recorded history is the drift: 4.68 (authored) → 11.36 (packet 233 D11/F7)
→ 21.48 (packets 234/235 bridge gating) → 28.05, against a threshold walked
6.0 → 12.0 → 28.0. A fourth recalibration was the obvious move and would have
been wrong.

Measured with a differential loop (both arms, same wedge, repeated runs
bit-identical), 2026-09-04:

| arm | blocks | mean moves/block | paths | mean moves/path | median | max |
|---|---|---|---|---|---|---|
| without linker | 202 | **28.05** | 5294 | **1.070** | 1 | 3 |
| with linker | 199 | 40.34 | 2454 | **3.271** | 3 | 8 |

**The degraded output is raw disjoint.** 1.070 G1 moves per path is the AC's
"≤ 2 points per path" — a 2-point line is 1 move. Nothing was linking; the block
proxy had eroded to 1.44x separation and went red on a grouping change.

Two things were ruled out along the way, each with a falsifiable probe:

- `minimum_sparse_infill_area` (the ticket-35 feature, which erases small sparse
  islands and would raise a per-block mean): setting it to `0.0` changed nothing
  — 202 blocks, 28.05, identical. Falsified.
- `sparse_infill_density` (ticket 107 renamed it): sweeping 15/20/25 and then
  5/90 changed nothing. That turned out to be a **separate live defect**, filed
  as evidence on ticket 128 below, and not the cause here — the test passes no
  config, so both arms run at the default density.

**Fix.** Assert the AC's actual quantity. New `parse_sparse_infill_path_moves`
splits blocks into paths at travels (`G0`, or `G1` with no `E`) and the
assertion is `mean G1 moves per path < 2.0`. This is **stronger** than what it
replaces on every axis: it is what the AC text says, it separates the arms by
3.1x instead of 1.44x, it sits 1.9x above the raw baseline and 1.6x below the
linked one, and it does not move when infill islands are reshaped. The
superseded block proxy and its four calibrations are recorded in the test's
comment so the erosion is not re-learned.

## Side finding — evidence for ticket 128 (`sparse_infill_density` is inert, not just mis-united)

Found while probing red 2; recorded on ticket 128, which already owns the
`percent` / `float_or_percent` numeric-spelling defect.

Through the `pnp_cli --config` sidecar on `resources/regression_wedge.stl`, a
**bare JSON number** for `sparse_infill_density` has no effect whatsoever, while
the **percent-string** spelling works:

| config | `gcode_filament_length_mm` |
|---|---|
| `{"sparse_infill_density": 5}` | 11054.1767578125 |
| `{"sparse_infill_density": 25}` | 11054.1767578125 |
| `{"sparse_infill_density": 90}` | 11054.1767578125 |
| `{"sparse_infill_density": "5%"}` | 6210.55859375 |
| `{"sparse_infill_density": "90%"}` | 33465.38671875 |

Control, same sidecar, same run: `{"layer_height": 0.3}` → 133 layers and
11332.9765625 mm, versus 200 layers at the default. So the sidecar route is
live and this key specifically is dead on the numeric spelling.

This sharpens ticket 128's table, which predicted a bare number would be read as
an absolute value (silently ~100x over-dense). On this path it is **inert** —
the module keeps its fallback — which is a quieter failure than the one that
ticket predicted, and it is a live user-facing defect: the documented CLI key
silently does nothing for the spelling most users would reach for. Not fixed
here; it is ticket 128's scope.

## Records

- Map's "two integration tests are red at HEAD" bullet removed.
- Ticket 128 gains a measured-evidence section pointing here.
