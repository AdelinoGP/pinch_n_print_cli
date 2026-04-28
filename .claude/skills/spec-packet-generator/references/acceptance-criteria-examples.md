# Acceptance Criteria Examples

**When to read:** while writing or reviewing ACs in Step 5 / Step 7. Shows the rule "name exact assertion content" applied to a real example, plus the most common failure shape.

**Topics:** Given/When/Then, exact field names, runnable verification command, delegation-friendly output.

## Compliant AC

```
- **Given** a core-module manifest with a shorthand config field (e.g., `wall_count = "int"`),
  **when** `config-schema` CLI is called on that module,
  **then** the JSON output contains an entry with `"key": "wall_count"`, `"type": "int"`,
  and `"min"`, `"max"`, `"default"`, `"display"`, `"group"` all present
  (absent optionals are `null`).
  | `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules 2>/dev/null | python3 -c "import json,sys; entries=[e for e in json.load(sys.stdin)['schema'] if e['name']=='classic-perimeters'][0]['fields']; f=[f for f in entries if f['key']=='wall_count'][0]; assert all(k in f for k in ('type','min','max','default','display','group')), f'MISSING: {[k for k in (\"type\",\"min\",\"max\",\"default\",\"display\",\"group\") if k not in f]}'"`
```

Why this passes:
- Names the exact field names (`wall_count`, `key`, `type`, `min`, `max`, `default`, `display`, `group`).
- Specifies what counts as present (absent optionals must be `null`, not missing).
- Ends with a single runnable command that exits non-zero on failure and prints what was missing — a sub-agent can return a one-line FACT.

## Non-Compliant AC (do not use)

```
- **Given** a core-module manifest with a shorthand config field,
  **when** `config-schema` CLI is called,
  **then** all six AC-2 fields are present.
```

Why this fails:
- "All six AC-2 fields" — the field names are not in the criterion text; the implementer or reviewer must hunt elsewhere.
- No verification command, so the criterion is not falsifiable from the AC alone.
- "Are present" — silent on the optional/null distinction.
