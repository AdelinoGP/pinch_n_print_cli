# T27 flat-bridge reduction experiment (2026-09-28/29)

Ticket: [Serial host floor](../../issues/27-serial-host-prepass-floor.md).
This is a **DROP for the tested candidate**, not a performance keep/drop of the
entire stage. The phase-B sibling-region memo remains a recommendation for the
human, not an accepted decision.

## Candidate and falsifying comparison

`assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`)
constructs `refilled = difference(closed, support)` and tests each already-
unsupported component against it. The temporary candidate tested the
mathematical set identity `component ∩ (closed \ support) = component ∩ closed`
to avoid the full-layer `difference`. The candidate kept the emitted components
unchanged. A committed narrow mixed-fixture test,
`flat_bridge_gap_and_free_edge_keep_only_the_gap_component`
(`crates/slicer-core/tests/algo_prepass_slice_tdd.rs`), exercises an enclosed gap
and an outward-growing free edge in the same region. That test and the other
tests in the file passed with the candidate (8/8); the synthetic case alone is
**not** an adequate representation oracle.

A temporary differential test (removed after the probe) loaded the user's
`tmp/3dbenchy.stl` with `slicer_model_io::load_model`, ran
`execute_mesh_analysis`, `batch_bottom_surface_footprints` and
`batch_slice_objects_by_layer`, and compared `assemble_flat_bridge_areas`
against the original `refilled` branch on the same consecutive raw slices.
Command: `cargo test -p slicer-core --features host-algos --test
flat_bridge_probe_t27 -- real_layer_bridge_ir_matches_legacy --nocapture`
(output captured in `target/test-output.log` at the time). **RED:** Benchy
layer 1 (the second sampled Z plane) produced 105 candidate bridge polygons
against 104 legacy polygons; the first unequal component was index 23.
This is a changed `SliceIR.bridge_areas` value on a real mesh layer, even
though the underlying set identity seems plausible. Clipper's output
representation and the fractional area discriminator cannot be bypassed by
that algebra. Do not use the candidate, or turn a near-threshold fallback into
a purported proof without another real-layer comparison. The temporary
production edit and differential test were removed; the legacy mask is intact.

## Exploratory ordinary pair (not an acceptance A/B)

Before the real-layer test exposed the mismatch, ordinary guests were rebuilt
and `cargo xtask build-guests --check` exited 0. Baseline and candidate release
hosts had been built from the same working tree on either side of the change;
the original guest snapshot was copied before the rebuild. Two AB/BA
uninstrumented Benchy/classic/supports-off pairs at `RAYON_NUM_THREADS=12`
are under `target/matched-pair/t27-flat-bridge-first/` (gitignored).

| Variant | Wall samples (s) | Median wall (s) | CPU samples (s) | Median CPU (s) |
| --- | --- | ---: | --- | ---: |
| Legacy | 39.592, 36.726 | 38.159 | 136.281, 136.047 | 136.164 |
| Candidate | 39.296, 37.692 | 38.494 | 132.391, 131.750 | 132.070 |

Candidate/legacy median wall = 1.009 and process CPU = 0.970 in this batch;
the measured CPU/wall ratios are 3.37–3.70, so this is load-qualified and
not a quiet-machine wall result. `degraded=false` for all four runs. G-code
sizes were 7,443,969–7,444,885 bytes and TYPE counts matched except the
Inner-wall marker (which also differed within each variant); G-code byte
identity is not an IR oracle. No accelerated pairs or base pairs were run:
the real-layer mismatch already disqualified the candidate.

## Smaller `apply_opening` parity lead (reassessment only)

`apply_opening` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`)
currently uses two Round offsets with arc tolerance 0 and a radius of half
the configured line width. Canonical `opening_ex`'s default parameters
(`ClipperUtils.hpp`) are `jtMiter` with miter limit 3; its calls in the
top/bottom-surface classification in `PrintObject.cpp` use **one tenth of the
external-perimeter flow width**, not half. Thus swapping only Round → Miter
is not a complete parity correction, and no output-equivalence claim follows
from changing join alone. The native helper `opening` (`crates/slicer-core/src/
polygon_ops.rs`) exposes the Miter/3 convention. The matched-job probe in
`FINDINGS-SUBSTAGE.md` bounds `apply_opening` to a parallel worker term rather
than the stage's larger serial phase-B offsets. If promoted, treat this as a
separate geometry/parity experiment with a real layer-output oracle, not a
representation-safe flat-bridge speedup; no performance A/B or keep/drop was
claimed for it here.
