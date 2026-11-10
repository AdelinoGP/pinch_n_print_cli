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

**Cell**:
One entry of the slice-performance benchmark matrix (fixture × wall generator × supports), measured on both arms.
_Avoid_: case, scenario

**Arm**:
One of the engines compared in a matched run (PnP or OrcaSlicer).
_Avoid_: side, tool
