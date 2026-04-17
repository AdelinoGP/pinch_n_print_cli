---
name: spec-packet-generator
description: Generates a full ModularSlicer spec packet under .ralph/specs from a rough prompt, file, or URL. Produces packet.spec.md, requirements.md, design.md, implementation-plan.md, and task-map.md aligned to docs/07_implementation_status.md.
type: anthropic-skill
version: "1.0"
metadata:
  internal: true
---

# Spec Packet Generator

## Overview

Generate a complete ModularSlicer spec packet from a rough prompt or prompt file.

This skill is repo-specific. It targets the spec-driven Ralph flow in this repository, not generic product planning. The output must be a packet folder under `./.ralph/specs/[spec-slug]/` that Ralph can use as one bounded execution slice.

## When to Use

- The user has a rough implementation prompt and wants it converted into a proper packet.
- A backlog slice in `./docs/07_implementation_status.md` needs to be turned into runnable spec artifacts.
- A task group needs clear scope boundaries, authoritative docs, OrcaSlicer references, and acceptance criteria before implementation.

## Important Notes

These rules apply across all steps:

- `./docs/07_implementation_status.md` is the canonical backlog and prioritization source.
- A packet narrows the backlog to one coherent remediation slice. Do not generate a packet that spans unrelated workstreams.
- Packet output lives under `./.ralph/specs/[spec-slug]/`.
- Each packet must contain:
  - `packet.spec.md`
  - `requirements.md`
  - `design.md`
  - `implementation-plan.md`
  - `task-map.md` when mapping back to `docs/07` needs to be explicit
- `packet.spec.md` is the preflight-visible contract and MUST contain real Given/When/Then acceptance criteria.
- Default new packets to `status: draft`. Only mark a packet `active` if the user explicitly requests it and there is no other active packet.
- Use the normative document map in `./docs/00_project_overview.md` to choose authoritative sources.
- If the packet mirrors or audits OrcaSlicer behavior, cite specific paths under `./OrcaSlicerDocumented/`.
- This skill ends after generating the packet. Do not begin implementation.

## Parameters

- **input** (required): Rough prompt text, a markdown file path, or a URL containing the prompt.
- **task_ids** (optional): One or more `TASK-###` ids from `./docs/07_implementation_status.md`. If omitted, infer the most likely task group and ask the user to confirm.
- **spec_slug** (optional): Packet folder name. If omitted, derive kebab-case from the prompt and task scope.
- **output_dir** (optional, default: `./.ralph/specs/[spec_slug]/`): Where the packet files should be created.
- **status** (optional, default: `draft`): `draft` or `active`.

**Constraints:**

- You MUST ask for any missing required parameters up front in one message.
- You MUST support `input` as direct text, file path, or URL.
- You MUST derive `spec_slug` as kebab-case when not provided.
- You MUST NOT overwrite an existing packet directory without explicit user approval.
- You MUST present the proposed packet scope before generating files and get explicit approval.
- You MUST keep the packet small: a handful of related `docs/07` tasks, not a whole phase.

## Workflow

### 1. Detect Input Mode

- If `input` is a file path, read it.
- If `input` is a URL, fetch and summarize the relevant prompt content.
- Otherwise treat `input` as direct prompt text.

Extract the core remediation slice, likely subsystems, likely authoritative docs, and any stated verification requirements.

### 2. Resolve Backlog Scope

Read `./docs/07_implementation_status.md` and map the prompt to one small, coherent task group.

**Requirements:**

- Confirm every proposed `TASK-###` exists in `docs/07`.
- Prefer one contiguous or tightly related slice.
- If the prompt is too broad, narrow it and explain the cut.
- If the mapping is ambiguous, present 1-3 options and ask the user to choose.

### 3. Resolve Packet Metadata

Determine:

- packet slug
- task ids
- packet goal
- in-scope and out-of-scope boundaries
- target output directory
- desired packet status

Before generating files, present a short plan with:

- packet slug
- grouped task ids
- one-paragraph packet goal
- in-scope boundaries
- out-of-scope boundaries
- expected files to generate

**Gate:** You MUST NOT write packet files until the user approves this packet scope.

### 4. Gather Authoritative References

Use `./docs/00_project_overview.md` as the normative document map and identify only the decisive docs for this slice.

At minimum, determine whether the packet depends on:

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md`
- `docs/04_host_scheduler.md`
- `docs/05_module_sdk.md`
- `docs/08_coordinate_system.md`
- `docs/09_progress_events.md`
- `docs/11_operational_governance_and_acceptance_gate.md`
- `docs/12_architecture_gate_metrics.md`

If OrcaSlicer parity or reference behavior matters, inspect `./OrcaSlicerDocumented/` and record exact paths.

### 5. Create Packet Structure

Create `./.ralph/specs/[spec_slug]/` and generate:

- `packet.spec.md`
- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` when useful

Use `./.ralph/specs/_templates/` as the starting structure, but replace placeholders with packet-specific content.

### 6. Populate `packet.spec.md`

`packet.spec.md` MUST include:

- YAML frontmatter with:
  - `status: draft|active`
  - `packet: [spec-slug]`
  - `task_ids:` list
  - `backlog_source: docs/07_implementation_status.md`
- packet goal
- scope boundaries
- Given/When/Then acceptance criteria
- verification commands
- authoritative docs
- OrcaSlicer reference obligations

The acceptance criteria must be concrete enough for Ralph preflight and later verification.

### 7. Populate `requirements.md`

Capture:

- problem statement
- grouped task ids
- in-scope and out-of-scope boundaries
- authoritative docs
- OrcaSlicer reference obligations
- acceptance summary
- verification commands

### 8. Populate `design.md`

Document the implementation shape without doing the implementation.

Include:

- controlling code paths or likely implementation surfaces
- neighboring tests or fixtures
- architecture constraints
- proposed change shape
- data and contract notes
- risks and tradeoffs
- open questions that must be resolved before the packet becomes active

### 9. Populate `implementation-plan.md`

Break the packet into atomic steps.

Each step should include:

- step title
- linked task ids
- objective
- likely files or subsystems touched
- authoritative docs
- OrcaSlicer refs
- narrow verification commands

**Requirements:**

- Steps must be ordered.
- Steps must stay inside the packet boundary.
- Steps must reflect TDD and narrow validation.
- Include a packet completion gate at the end.

### 10. Populate `task-map.md`

Add `task-map.md` when it clarifies how packet steps map back to `docs/07`.

Use it especially when:

- the packet spans more than one task id
- multiple docs are authoritative for different steps
- OrcaSlicer refs differ by step

### 11. Report Results

List generated files with paths and summarize:

- packet slug
- packet status
- task ids covered
- authoritative docs chosen
- OrcaSlicer refs chosen
- any open questions or assumptions

### 12. Offer Activation Guidance

If the packet is still `draft`, ask whether the user wants you to mark it `active`.

If the user asks for activation:

- confirm there is no other active packet
- update `packet.spec.md` to `status: active`
- remind them the next step is `ralph preflight` and then `ralph run -c ralph.yml`

## Output Contract

The generated packet should be sufficient for a later Ralph run to understand:

- what exact backlog slice is in scope
- which docs govern the behavior
- which OrcaSlicer references must be checked
- what acceptance looks like
- what order of implementation steps to follow

## Usage Examples

```text
/spec-packet-generator input:"Rework TASK-121 and TASK-122 into one manifest contract packet" task_ids:TASK-121,TASK-122
```

```text
/spec-packet-generator input:notes/task-121-prompt.md spec_slug:task-121-contract status:draft
```

```text
/spec-packet-generator input:"Create a Benchy parity packet for supports and seam placement" status:draft
```

## Troubleshooting

**Prompt too broad:** Narrow it to one remediation slice and explain the cut before generating files.

**Task mapping unclear:** Present candidate task groups from `docs/07` and ask the user to confirm one.

**No relevant task ids in docs/07:** Stop and tell the user the prompt is outside the canonical backlog.

**Another packet is already active:** Keep the new packet as `draft` and call out the conflict.

**OrcaSlicer reference missing:** Note that the packet has no OrcaSlicer dependency instead of inventing one.

**Existing packet directory already present:** Ask whether to overwrite, revise in place, or choose a new slug.