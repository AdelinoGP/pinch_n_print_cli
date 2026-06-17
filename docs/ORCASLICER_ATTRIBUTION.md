# OrcaSlicer Attribution & Licensing Framework

## Project Lineage and Acknowledgments
This repository contains Rust ports of the C++ slicing engine logic originally developed in the OrcaSlicer project. We deeply respect and acknowledge the open-source legacy that makes this project possible:

- **OrcaSlicer** (by SoftFever and community), licensed under the GNU Affero General Public License, version 3 (AGPLv3).
- **Bambu Studio** (by BambuLab), licensed under AGPLv3.
- **PrusaSlicer** (by Prusa Research), licensed under AGPLv3.
- **Slic3r** (by Alessandro Ranellucci and the RepRap community), licensed under AGPLv3.
- **SuperSlicer** (by @supermerill).

We also acknowledge the myriad of contributors to the underlying dependencies and geometry algorithms, including but not limited to the Clipper2 Library (Angus Johnson) and Arachne Engine (Ultimaker B.V.)

## LLM-Assisted Porting Declaration
The Rust adaptations of C++ slicing mechanics found within this repository are **LLM-generated ports** of the original C++ implementations from OrcaSlicer. They have been translated to Rust to fit the architecture of this modular slicing engine while preserving the core algorithms of the legacy projects. 

## Licensing
As this codebase builds upon AGPLv3-licensed source code, these ports and the corresponding engine components are subject to the same terms and are licensed under the **GNU Affero General Public License, version 3 (AGPLv3)**.

---

## Standard Porting Header

To ensure continuous compliance and clear attribution as new files are ported, any Rust file containing logic translated from OrcaSlicer MUST begin with the following comment block:

```rust
// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer, 
// and Slic3r, which are licensed under the GNU Affero General Public License, 
// version 3 (AGPLv3).
//
// Original C++ source path: <ORIGINAL_FILE_PATH_PLACEHOLDER>
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
```
