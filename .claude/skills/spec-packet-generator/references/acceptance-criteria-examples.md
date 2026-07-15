---
when: Read while writing or reviewing acceptance criteria.
keywords: Given/When/Then, exact fields, verification, delegation
---

# Acceptance Criteria Examples

## Compliant

```markdown
- **Given** a core-module manifest with a shorthand config field (e.g., `wall_count = "int"`),
  **when** `config-schema` CLI is called on that module,
  **then** the JSON output contains an entry with `"key": "wall_count"`, `"type": "int"`,
  and `"min"`, `"max"`, `"default"`, `"display"`, `"group"` all present
  (absent optionals are `null`).
  | `cargo run --package pnp_cli -- module config-schema --module-dir modules/core-modules 2>/dev/null | python3 -c "import json,sys; entries=[e for e in json.load(sys.stdin)['schema'] if e['name']=='classic-perimeters'][0]['fields']; f=[f for f in entries if f['key']=='wall_count'][0]; assert all(k in f for k in ('type','min','max','default','display','group')), f'MISSING: {[k for k in (\"type\",\"min\",\"max\",\"default\",\"display\",\"group\") if k not in f]}'"`
```

This names exact fields, defines absent optionals as `null`, and provides one command that fails and reports missing fields.

## Non-Compliant

```markdown
- **Given** a core-module manifest with a shorthand config field,
  **when** `config-schema` CLI is called,
  **then** all six AC-2 fields are present.
```

It omits field names, observable optional/null behavior, and a runnable verification command.
