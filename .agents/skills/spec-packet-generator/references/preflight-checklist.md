# Preflight Checklist

Every packet must clear this checklist before implementation starts. The author runs it inline while writing (`spec-packet-generator` step 6); `spec-review <packet> --preflight` runs the same list cold. Check each item against the tree, not against memory or the packet's own prose.

Each check covers names the packet treats as **pre-existing** (verbs like extend, call, consume, rename, "already has"). During authoring, reconcile a supposedly new name that already exists with live call sites. During implementation review, expect names the packet creates to exist and check their shape and use. A name an earlier draft packet in the same plan will create is acceptable only when that packet plans the same name and shape.

1. **Symbols.** Every cited function, type, field, enum variant, trait method, module, and file exists at its crate-qualified path with the stated shape. A module called a "stub" really is one.
2. **WIT/IR identifiers.** WIT records and functions match exactly (kebab-case); IR variants match the enum, and paint semantics carried as `Custom("…")` are matched by string, not by an imagined variant.
3. **Schema versions.** Criteria assert the field or variant addition, not a literal future version. A bump targets the constant that governs that IR, and the packet agrees with itself on the target.
4. **ADRs.** When proposing a new ADR, check `docs/adr/` for number collisions and re-derive the free number when creating it. The design conforms to every existing ADR's normative content it touches, or records the required amendment and rationale.
5. **Deviation IDs.** IDs follow the convention `docs/DEVIATION_LOG.md` actually uses. IDs to close or supersede exist there; IDs to create do not.
6. **Prerequisites.** Every dependency described as satisfied has `status: implemented` in its own `packet.spec.md`, or in its `_OLD/<packet-name>.md` digest if archived. Recheck any required exports against the current tree.
7. **Test targets.** Each new test file under an aggregated binary is registered in its aggregator. Each criterion's `--test` binary is the one the file lands in, and that binary can drive the asserted behaviour today, or the plan adds the fixture. Check required features and file-level feature gates (AGENTS.md §Test Discipline).
8. **Criteria.** Each criterion is checkable and names exact outputs. Validation, enforcement, contract-boundary, and failure-path changes have a negative criterion. No criterion command is `cargo test --workspace`.
9. **Fallout.** Every step that adds a struct field or bumps a schema or WIT version lists its struct-literal sites and old-value assertions (README gotcha 6).
10. **Docs impact.** Either `none` with an eligible reason, or each doc section to edit with a grep.
11. **Completeness.** No placeholders, `TBD`, or template brackets remain. Every unresolved blocking question is listed under Open questions as `[BLOCK]`; it blocks preflight and implementation until resolved.

## `--preflight` report

```
## Preflight: <packet>

| # | Check | Result | Offending items |
|---|-------|--------|-----------------|
| 1 | Symbols | PASS / FAIL | ... |
...
| 11 | Completeness | PASS / FAIL | ... |

**Verdict:** PREFLIGHT PASS / PREFLIGHT BLOCKED (<n> failures)
```

Failures of items 1, 2, or 4 (fictional or wrong-shape names, ADR collisions) are blockers. The rest are high severity.
