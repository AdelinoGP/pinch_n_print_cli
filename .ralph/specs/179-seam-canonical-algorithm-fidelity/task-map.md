# Task Map: 179-seam-canonical-algorithm-fidelity

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-292` | `Step 1` | `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md` | `seam-planner-default/src/{comparator.rs,visibility.rs}`, new comparator test | `SeamPlacer.cpp::SeamComparator`, `SeamPlacer.hpp` constants | M | Canonical comparator, `layer_angle`, paint priority. |
| `TASK-292` | `Step 2` | `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md` | `seam-planner-default/src/visibility.rs`, new visibility test, `lib.rs` wire-up | `SeamPlacer.cpp::raycast_visibility`, `calculate_overhangs_and_layer_embedding` | M | Canonical 30000×25 visibility and resolved flow width. |
| `TASK-292` | `Step 3` | `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md` | `seam-planner-default/src/align.rs`, new alignment test | `SeamPlacer.cpp::align_seam_points`, `find_seam_string` | M | Alternative-start retry and bounded gap anchor. |
| `TASK-292` | `Step 4` | `docs/11_operational_governance_and_acceptance_gate.md`, `docs/ORCASLICER_ATTRIBUTION.md` | `seam-planner-default/src/align.rs`, `Cargo.toml`, new spline test | `Curves.hpp::fit_curve`, `Bicubic.hpp::CubicBSplineKernel` | M | Full-pivot QR solver via `faer` or local fallback. |