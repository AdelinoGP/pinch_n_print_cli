# clipper2 1.2.0 geometry delta (dependency direction, next bump)

Type: task
Status: open

## Question

[clipper2 cost/output verdict](17-clipper2-cost-output-verdict.md) established
that 1.0.3 → 1.1.0 is output-neutral, and in passing found that
**1.1.0 → 1.2.0 is not**: 2 of 367 fixtures differ, each losing one
collinear / near-collinear vertex, and 1.2.0's `clean_collinear`
(`clipper2-rust` `src/engine.rs`) replaces 1.1.0's re-read of
`outrec.pts` with the tracked `start_op`. Upstream has since also added a
`using_z` feature and example.

Does this repo want 1.2.0, and if so what output/parity work does the bump
imply? Work:

- Decide whether there is a reason to move at all (the current version is
  cost-neutral and DEV-173-carrying in both — 1.2.0's `check_split_owner` is
  byte-identical to 1.1.0's, so an upgrade does not improve the DEV-173
  posture and must not be motivated by it).
- If moving: re-run the ticket-17 harnesses (preserved in
  [`evidence/t17-clipper2-ab/tools/`](../evidence/t17-clipper2-ab/tools/README.md))
  with the 1.2.0 pin, attribute the two changed fixtures to the
  `clean_collinear` change, and run the fairness contract's output disclosure
  over at least the benchy/wedge matched cells.
- Check whether the `using_z` feature or the `PolyFace64` additions open a
  cheaper path for anything this map cares about (e.g. the contour/hole
  reconstruction in `expolygons_from_tree`, whose hand-rolled tree walk is
  what made the flat `union_64` API unattractive for DEV-173).

Not on the timing chain; parallel-takeable when a dependency decision comes
due. The harness-pin trap from ticket 17 applies: pin the harness dep and
assert the resolved version from the harness's own `Cargo.lock`.
