# Pinch 'n Print

This repository contains an experimental, highly-modular slicing engine for 3D printing, written in Rust.

> [!WARNING]
> ## ⚠️ Mostly "Vibe-Coded" ⚠️
>
> This project was developed with heavy AI assistance through iterative natural-language prompting. That means:
> - The code was written iteratively through natural language descriptions with loose human oversight
> - Testing coverage can be uneven.
> - You may encounter unconventional naming, odd refactors, or irregular coding patterns.
> - Performance and numerical stability have not been rigorously validated.
>
> **While functional, this codebase should be treated as experimental.** We recommend:
> - Thoroughly testing in your own development environment first.
> - Avoiding actual printing usage without extensive gcode review.
> - Being prepared for edge cases or unusual outputs.
> - Contributing fixes if you encounter issues—we appreciate the help!

## Architecture Overview

Pinch 'n Print is designed from the ground up as a host/runner for independently compiled WebAssembly (WASM) modules. Instead of tying features like infill or wall generation directly to the core, these features are implemented as isolated plugins.

### Key Architectural Concepts

- **Modular Pipeline**: Every slicing feature is a separate compiled module assigned to a specific pipeline stage.
- **DAG Execution**: Modules execute in a topologically sorted Directed Acyclic Graph (DAG) derived from strict Intermediate Representation (IR) read/write declarations.
- **ECS-inside-Blackboard State**: Global print state (mesh, region maps, layer plans) lives in a host-owned read-only Blackboard. Per-layer state is modeled as an Entity Component System (ECS) world in isolated arenas.
- **Safe Parallelism**: Per-layer execution is heavily parallelized via `rayon`. Because modules only receive scoped borrow tokens for their layer's data, data races are structurally impossible.
- **Zero GUI Concern**: The engine is purely a backend slicer. All UI or visualization concerns must be handled by a separate frontend process communicating via CLI or UNIX socket.

## Slicing Pipeline Tiers

The slicing lifecycle is divided into three fixed-order tiers:

1. **Tier 1 — PrePass (Sequential)**: Whole-model geometry processing. Responsible for mesh analysis, layer planning, overhang classification, and semantic paint segmentation. Results are committed to the immutable Blackboard.
2. **Tier 2 — Per-Layer (Parallel)**: The heavy lifting. Every layer runs independently across threads. Stages within this tier generate slice polygons, compute perimeters, generate infill, and create support structures.
3. **Tier 3 — PostPass (Sequential)**: Cross-layer finalization. Responsible for sorting travel moves, inserting skirt/brim/wipe tower entities, handling toolchanges, and finally emitting serialized G-code.

## Extensibility via WebAssembly (WASM)

Pinch 'n Print utilizes the WASM Component Model to guarantee a stable ABI and fast iteration.
- Modules are loaded at startup and require only a `.wasm` file and a `.toml` manifest.
- Developers can write modules in Rust, C, C++, or any language targeting WASM.
- Modules declare strict data access contracts (reads/writes) in their manifest, allowing the host to detect DAG conflicts and enforce zero-surprise memory isolation.
- Config keys added by modules automatically merge into the system configuration tree.

## Building and Running

You'll need the standard Rust toolchain installed, including the `wasm32-wasip1` target for building guest modules.

```bash
# Compile all host components and libraries
cargo build --workspace

# Build all guest WASMs (core modules)
cargo xtask build-guests

# Build the release CLI and bundle guest modules into target/dist/
cargo xtask dist

# Run the slicer
cargo run --bin pnp_cli --release -- slice --input resources/benchy.stl --output /tmp/out.gcode
```

## Lineage and Acknowledgments

This project would not be possible without the profound legacy of the open-source 3D printing community. We are grateful to the pioneers and developers whose years of dedication have paved the way for modern slicing algorithms. 

The core logic and geometric operations in this codebase are **LLM-assisted Rust ports** originally adapted from the C++ engine of **OrcaSlicer**. 

In the same spirit that OrcaSlicer honors its roots, we pay explicit respects to the towering shoulders upon which we stand:
- **[OrcaSlicer](https://github.com/OrcaSlicer/OrcaSlicer)** by SoftFever and its community, which forms the direct basis for our porting efforts.
- **[Bambu Studio](https://github.com/bambulab/BambuStudio)** by BambuLab, from which OrcaSlicer was originally forked.
- **[PrusaSlicer](https://github.com/prusa3d/PrusaSlicer)** by Prusa Research, the robust foundation underlying Bambu Studio.
- **[Slic3r](https://github.com/Slic3r/Slic3r)** by Alessandro Ranellucci and the RepRap community, the pioneering project that started this lineage.

We also incorporate ideas and logic refined by **[SuperSlicer](https://github.com/supermerill/SuperSlicer)** (by @supermerill) and rely on the underlying geometry algorithms from the **Clipper2 Library** (Angus Johnson) and the **Arachne Engine** (Ultimaker B.V.).

All credit for the original algorithms and design belongs to the upstream authors and contributors. Conversely, any bugs, logical misinterpretations, or general awkwardness introduced in this Rust port—whether by human oversight or LLM hallucination—are unequivocally our own.

This project is licensed under the **GNU Affero General Public License, version 3 (AGPLv3)**, reflecting and honoring the open-source commitment of our upstream ancestors. For more details on the exact porting and attribution guidelines utilized within this repository, please see our [OrcaSlicer Attribution Framework](docs/ORCASLICER_ATTRIBUTION.md).
