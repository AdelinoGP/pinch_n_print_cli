# Pinch 'n Print (PNP)

A modular 3D-printing slicer whose pipeline runs as WASM modules. This glossary
holds the project's own vocabulary as it crystallises.

## Language

**Matched settings**:
Two compared arms computing the same print job — identical resolved values on every setting that changes emitted geometry.
_Avoid_: fair settings, same config

**Asymmetry**:
A divergence between two compared arms on a job-affecting setting; closed by shared authored values or a canonical port rather than disclosed.
_Avoid_: mismatch, unfairness

**Canonical**:
The pinned OrcaSlicer reference implementation that parity work is ported from and judged against.
_Avoid_: upstream (ambiguous)

**Slice plane**:
The Z at which a layer's cross-section is sampled. Both arms sample at the layer midpoint (`0.5 × (lo + hi)`, canonical `compute_slice_z`); `GlobalLayer.z` is the print **top**, and the sample plane is `z − 0.5 × effective_layer_height`.
_Avoid_: layer Z (ambiguous: the print top is not the sample plane)

**Cell**:
One entry of the slice-performance benchmark matrix (fixture × wall generator × supports), measured on both arms.
_Avoid_: case, scenario

**Arm**:
One of the engines compared in a matched run (PnP or OrcaSlicer).
_Avoid_: side, tool

**Anchor**:
The boundary a brim ring and skirt hull are generated from: the object's layer-0 cross-section, approximated from the outer-wall path plus half its wall width.
_Avoid_: outline, silhouette, footprint (ambiguous: the raw filled region is also a footprint)

**Skirt hull input**:
The points the skirt's convex-hull envelope is built from: each layer's outline up to `skirt_height` (outer-wall path plus half its width) and the **brim area's outer contour** (canonical `m_objectBrimAreasByInstance[...]` `area.contour.points`) — never the emitted brim beads' centreline, which sits half a flow spacing inside it.
_Avoid_: brim footprint, brim outline (both name the filled region, not the hull input)
