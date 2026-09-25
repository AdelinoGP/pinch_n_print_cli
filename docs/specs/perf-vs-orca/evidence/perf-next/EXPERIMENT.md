# Perimeter refresh after accepted wall-flags fast path

## Approved scope

Measurement-only: repair generator selection/validation, capture independent
supports-off Benchy baselines, and use existing instrumented/fuel attribution.
Temporary source probes and optimization edits require another user confirmation.
The coordinator is the sole build/test/timing owner; subagents implement scratch
tooling and perform read-only source investigation. No automatic commits.

## Acceptance decision

Future KEEP recommendations require repeatable process-CPU and whole-slice wall
improvement with canonical geometry/toolpath semantics preserved. The earlier
CPU-only exception applied only to the already committed wall-flags fast path.
Overlapping/noisy evidence remains inconclusive. Promising changes must also be
checked with tree-support Benchy and repeated base runs before retention.

## Baseline protocol

- Current release host and complete external module tree are isolated under
  `baseline-artifacts/`; `artifact-manifest.json` records source identity and hashes.
- Release build passed and `cargo xtask build-guests --check` returned exit 0
  before artifact capture. Module diagnosis is `module-diagnose.json` and establishes
  availability only.
- Supports-off `tmp/3dbenchy.stl`, 12 Rayon workers, explicit Classic/Arachne configs.
- One warmup per generator; four measured samples per generator in balanced order:
  Classic/Arachne, Arachne/Classic, Classic/Arachne, Arachne/Classic.
- Every accepted sample must agree across explicit config, stderr claim-holder
  evidence, and emitted G-code generator marker. Missing/contradictory evidence is
  rejected. Completion/error/degraded status remains visible.
- Baselines are probe-free and omit report, instrumentation, and fuel metering.
- Process CPU is final kernel + user time from Windows `GetProcessTimes` on
  the retained process handle. Wall is that process's creation-to-exit interval,
  excluding harness startup/cleanup and avoiding the working-set polling tail.
  This is a different timing scope from historical harness Stopwatch values;
  do not treat their difference as a measured optimization.
- Instrumented and fuel-profile captures are separate attribution data, never
  acceptance wall measurements. Worker elapsed, process CPU, process wall, host
  scope elapsed, and guest fuel retain separate units and denominators.
- Environment snapshots are point observations, not proof of continuous quiet.

## Interpretation limits

These are independent generator baselines, not a cross-generator speed comparison.
No candidate is implemented in this scope. G-code hashes and TYPE counts are smoke
evidence; Classic has known same-binary nondeterminism, so neither establishes
fine-grained semantic parity. No historical profile share is assumed current.

The normal paint-segmentation path routes material/fuzzy paint through variants;
effective Material/Fuzzy segment annotations remain accepted by callers/adapters.
A representative live reprojection workload must be demonstrated before selecting
that optimization.
