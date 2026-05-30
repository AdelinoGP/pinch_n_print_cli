# PrePass host built-ins commit in-stage; the runner unifies only the bracket

**Status:** accepted (packet 75, Phase 1 / TASK-216)

The six PrePass host built-ins previously hand-rolled an identical instrument
bracket (`should_run` guard → `estimated_size` snapshot → `StageInstrumentationGuard`
start → execute → commit → finish). We unified that bracket behind one
`run_builtin_stage` helper (`prepass.rs`), but **kept each stage's commit inside its
own `execute` closure** rather than routing built-ins through `commit_stage_output`
like the guest path does.

## Why not a single commit path

The obvious "clean up" — make every stage funnel its output through
`commit_stage_output` over the `PrepassStageOutput` enum — is **infeasible**:
`PrePass::ShellClassification` commits via `Blackboard::replace_slice_ir`
(an *in-place refinement* of an already-committed `SliceIR`), which has no
`PrepassStageOutput` variant and is not a fresh commit. Three other built-ins
(`RegionMapping`, `Slice`, `SupportGeometry`) already commit *inside* their
`commit_*_builtin` functions, which take `&mut Blackboard`. Forcing a single commit
path would mean inventing a fake `Replace(SliceIR)` output shape and moving the
commit relative to the instrument bracket — risk for an aesthetic win.

## Consequence

A future architecture review will likely re-suggest "route all six built-ins through
`commit_stage_output` for symmetry with the guest path." This ADR records that we
considered and rejected it: the built-ins commit in-stage by design. The runner owns
the *bracket* (the genuine duplication), not the *commit* (which is not uniform).
