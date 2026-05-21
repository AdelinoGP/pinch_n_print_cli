# Swarm — Invocation Examples & Troubleshooting

**When to read this:** the user asks for invocation examples, you need to remind yourself of valid argument shapes, or you hit one of the known troubleshooting cases below.

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

**"Packet is draft."** Refine or review by default. Only run implementation from a draft when the user explicitly asks for code changes.

**"Packet tasks already show [x] in docs/07."** Treat the packet as a retrofit or reopen slice. Do not blindly toggle backlog rows; reconcile the mismatch in the report.

**"Worker produced no changes."** Verify whether the step was read-only or already satisfied. If neither, treat the step as incomplete.

**"The planner is running out of context."** Rebuild the compact execution manifest and continue from the step ledger plus deltas instead of reloading the full packet and prior worker transcripts. If past 70%, stop dispatching exploratory workers and finish on the evidence you already have.

**"Review keeps finding packet-authoring defects."** Fix the packet docs first; do not keep rerunning code workers against an under-specified packet.

**"Build fails after a worker run."** Map the failure back to the specific packet step, repair that slice, and rerun the same narrow validation before widening scope. Never paste the full build log into the planner — dispatch a SNIPPETS lookup for the failing assertion.

**"A step is rated context cost L."** Stop. The packet should have been split during generation. Either go back to `spec-packet-generator` to split it, or fall back to manual sequential implementation outside Swarm.
