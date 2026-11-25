# Implementation Plan: [packet-name]

[Optional: ordering constraints, state shared across steps, TDD expectations. Delete when there are none.]

## Step 1: [title]

- Objective: [what changes and why, in one or two sentences]
- Surface:
  - `crates/[crate]/src/[file].rs` `[fn_or_type]`: [the change; the approach when it is not obvious]
- New contract: [exact signature or field, only for WIT, IR, or config schema additions; otherwise delete]
- Fallout: [struct-literal sites and tests asserting an old constant or version, when the step adds a field or bumps a version; otherwise delete]
- Verify: `cargo test -p [crate] --test [bin] -- [name]`
- Done when: [observable condition]

## Step 2: [title]

[Same fields. Steps leave the tree compiling, and each step's Verify proves its own change.]
