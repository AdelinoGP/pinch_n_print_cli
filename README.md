# Pinch 'n Print

This repository contains an experimental, highly-modular slicing engine for 3D printing, written in Rust.

## Lineage and Acknowledgments

This project would not be possible without the profound legacy of the open-source 3D printing community. We are grateful to the pioneers and developers whose years of dedication have paved the way for modern slicing algorithms. 

The core logic and geometric operations in this codebase are **LLM-assisted Rust ports** originally adapted from the C++ engine of **OrcaSlicer**. 

In the same spirit that OrcaSlicer honors its roots, we pay explicit respects to the towering shoulders upon which we stand:
- **[OrcaSlicer](https://github.com/OrcaSlicer/OrcaSlicer)** by SoftFever and its community, which forms the direct basis for our porting efforts.
- **[Bambu Studio](https://github.com/bambulab/BambuStudio)** by BambuLab, from which OrcaSlicer was originally forked.
- **[PrusaSlicer](https://github.com/prusa3d/PrusaSlicer)** by Prusa Research, the robust foundation underlying Bambu Studio.
- **[Slic3r](https://github.com/Slic3r/Slic3r)** by Alessandro Ranellucci and the RepRap community, the pioneering project that started this lineage.

We also incorporate ideas and logic refined by **[SuperSlicer](https://github.com/supermerill/SuperSlicer)** (by @supermerill) and rely on the underlying geometry algorithms from the **Clipper2 Library** (Angus Johnson) and the **Arachne Engine** (Ultimaker B.V.).

This project is licensed under the **GNU Affero General Public License, version 3 (AGPLv3)**, reflecting and honoring the open-source commitment of our upstream ancestors. For more details on the exact porting and attribution guidelines utilized within this repository, please see our [OrcaSlicer Attribution Framework](docs/ORCASLICER_ATTRIBUTION.md).
