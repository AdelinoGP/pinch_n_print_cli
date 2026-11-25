# Spec Packets

A packet is one coherent slice of work, specified well enough that a fresh agent can implement it without re-planning: what and why in `packet.spec.md`, how in `implementation-plan.md`.

## Layout

- Directory: `docs/spec_packets/<prefix>_<NN>_<packet-name>/`. `<prefix>` is the workstream (for plan-driven packets, the `docs/specs/<prefix>-plan.md` file name minus `-plan`), `<NN>` is the zero-padded plan queue row or the next free number for that prefix, `<packet-name>` is kebab-case.
- Files: `packet.spec.md` (goal, why, scope, design notes, acceptance criteria, verification, docs impact) and `implementation-plan.md` (ordered steps).
- Legacy packets keep their historical names and their older four-or-five-file shape (`requirements.md`, `design.md`, `task-map.md`). Do not migrate them.
- Archived packets are design-only digests at `docs/spec_packets/_OLD/<packet-name>.md`; their full source files remain in Git history. They are not implementable packet directories.

## Lifecycle

`status:` in the `packet.spec.md` front matter is one of:

- `draft`: authored, not yet implemented. Any number may exist.
- `implemented`: closed after a green acceptance run and a full `spec-review`.
- `superseded`: absorbed by a later packet, named in `superseded_by:`.

Legacy packets may use historical statuses such as `active`; interpret those in the context of their own workflow until they are implemented or superseded.

Workflow: `/spec-packet-generator` authors → `/spec-review <packet> --preflight` optionally re-checks it cold → `/swarm <packet>` implements and closes it. Divergences found while implementing are appended to a `## Deviations` section at the end of `packet.spec.md`.

## Implementation gotchas

Every packet skill applies these. Each one caught a real shipped defect.

1. **Fictional symbols.** Resolve every pre-existing function, field, variant, WIT type, ADR slot, and test binary a packet leans on against the tree, by crate-qualified path (AGENTS.md §In-Tree Citation Style). Prose review cannot see a name that does not exist.
2. **Helper passes, driver never calls it.** A helper's unit tests stay green while the production path never invokes it. Confirm a non-test call site: `rg '<helper>\s*\(' --type rust --glob '!**/tests/**'`.
3. **Placeholder tests.** Assertions must exercise the symbols and IR fields the criterion names, not pre-existing fixture data. `docs/22_test_quality.md` lists the false-green patterns.
4. **Unregistered test file.** A new file under an aggregated test binary (`tests/contract/`, `tests/unit/`, `tests/integration/`) without a `mod` line never compiles, and `cargo test --test <bin> <filter>` reports "0 tests" as a pass.
5. **Stale guest WASM.** Guests are not rebuilt by `cargo build`/`cargo test`. Run `cargo xtask build-guests --check` and judge by exit code before attributing any guest or module-dispatch failure (AGENTS.md §Guest WASM Staleness).
6. **Field and version fallout.** Adding a struct field or bumping a schema/WIT version can affect struct-literal sites (`cargo xtask check-literals`, `docs/21_data_defaults_and_fixtures.md`) and tests that assert old values. Land affected changes in the same step. For WIT package changes, inspect the relevant assertions in `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`; some pin package names or versions. Preserve an event's version literal when its wire format is intentionally locked.
7. **Brittle source greps.** Prefer behavioural tests. A criterion that greps for a symbol path must accept re-exported forms (`slicer_sdk::prelude`); fix the grep, never the call site.
8. **Workspace test runs.** Follow AGENTS.md §Test Discipline: workspace tests require an explicit user request or the packet's closure gate, after narrower checks pass, and run through `cargo xtask test --summary --workspace`. Iterate with narrow tests; check required features and use `--all-targets` for check/clippy gates.
9. **Baseline provenance.** A captured baseline (digest, golden, measured value) records `git rev-parse HEAD`, the uncommitted changes present, and determinism-affecting settings (profile, features, `CARGO_TARGET_DIR`), so drift can be attributed later.
