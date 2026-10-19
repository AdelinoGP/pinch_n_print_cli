# t17 — clipper2-rust 1.0.3 vs 1.1.0 compiled-body comparison

Ticket: [clipper2 cost/output verdict](../../issues/17-clipper2-cost-output-verdict.md).
Run 2026-09-24. Load-independent backstop for the cost verdict: source
comparison says the geometry modules are identical, and this checks that the
*compiled code* agrees.

Method: `RUSTFLAGS='--emit=asm'` build of a scratch probe (`tmp/t17-asm`,
gitignored) that forces the exact clipper2 surfaces this repo calls
(`boolean_op_tree_64`, `union_64`, `inflate_paths_64`,
`ClipperOffset::execute_tree`, `Clipper64::execute`) so they are codegen'd.
One build per version, same toolchain (`rustc 1.96.0`, `opt-level = 3`,
`lto = "thin"`), then per-function instruction bodies compared after normalizing
symbol hashes, anonymous-constant names, and local label numbering
(`target/t17-clipper2-ab/compare_bodies.py`).

## Result

- **240 function base names in common**: 1.0.3 has 240; 1.1.0 has 244
  (the four extra are `poly_path_to_faces64`, `poly_tree_to_faces64`, and the
  two `PolyFace64` drop glue symbols — the additive API, never called here).
- **230 of 240 common-name bodies bit-identical; 3 names differ, all
  non-semantically:**

| Function | Difference | Reading |
| --- | --- | --- |
| `PolyTreeD::add_child`, `core::scale_path` | one constant-pool label position between the pair | the float literal `__real@43dff…` is emitted inside `add_child` in one version and in `scale_path` in the other; no instruction differs |
| `RawVec::grow_one` | 24 (1.1.0) vs 23 (1.0.3) monomorphizations, 13 shared | one generic instantiation added by a new call site; no changed body |

No function containing clipper geometry logic — boolean execution, offsetting,
path cleaning, PolyTree construction — differs in its instruction sequence.

## Why this is the decisive cost evidence

Under this machine's external load the wall-clock A/B could not resolve a small
delta (per-run CV 15–22%, a same-version control spread of −18% to −52% on the
256-square leaves). The compiled-body equality closes the question the timings
cannot: if every executed instruction sequence is the same, there is no version
effect to resolve. It does not rule out a pure code-*layout* effect (alignment,
icache), but none is visible at the instruction level and the measured deltas
show no consistent direction or magnitude.
