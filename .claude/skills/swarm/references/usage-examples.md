# Swarm — Invocation Examples & Troubleshooting

**When to read this:** the user asks for invocation examples, you need to remind yourself of valid argument shapes, or you hit a known troubleshooting case below.

**Topics:** invocation syntax, draft-vs-active handling, retrofit packets, no-change worker results, planner context pressure recovery, build-failure diagnosis, L-cost-step escape hatch.

## Invocation examples

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement mode:refine-draft
```

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement mode:implement workers:2
```

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement mode:implement workers:3 state_backend:session-memory
```

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement scope:"Step 1,Step 2,Step 3"
```

## Troubleshooting

The core `Error handling` section already covers draft-implement, multiple-active, scope overlap, context pressure, worker overflow, missing verification, and L-step rules. The cases below add operational detail beyond those rules.

**"Packet tasks already show [x] in docs/07."** Treat the packet as a retrofit or reopen slice. Do not blindly toggle backlog rows; reconcile the mismatch in the report.

**"Worker produced no changes."** Verify whether the step was read-only or already satisfied. If neither, treat the step as incomplete.

**"Review keeps finding packet-authoring defects."** Fix the packet docs first; do not keep rerunning code workers against an under-specified packet.

**"Build fails after a worker run."** Map the failure back to the specific packet step, repair that slice, and rerun the same narrow validation before widening scope. Never paste the full build log into the planner — dispatch a SNIPPETS lookup for the failing assertion.