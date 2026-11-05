# Snippets

Copy an applicable block into `packet.spec.md` with its `<!-- snippet: … -->` marker, filling only the bracketed parts. Leave out a block that does not apply.

## orca-delegation

Applies when the packet borrows, or deliberately declines, OrcaSlicer behaviour. Its own section, after Design notes:

```markdown
<!-- snippet: orca-delegation -->
## OrcaSlicer references

Read `OrcaSlicerDocumented/` through a subagent. Cite by function name, never line number.

- `OrcaSlicerDocumented/[path]` `[function]`: [behaviour, constant, or edge case borrowed or deliberately not borrowed]
```

## parity-evidence

Applies when the packet implements OrcaSlicer config keys. Inside Design notes:

```markdown
<!-- snippet: parity-evidence -->
Parity evidence, per key: cite the canonical consumer (file + function) and describe its behaviour. Pin behaviour with invariant or property tests, or with ported `OrcaSlicerDocumented/tests/fff_print/` assertions (porting header per `docs/ORCASLICER_ATTRIBUTION.md`), never golden G-code. A plumbing key needs the canonical default plus a test that the value reaches its consumer. Behaviour that cannot be verified goes to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row.
```

## wasm-staleness

Applies when the change surface feeds a guest build: `crates/slicer-schema/wit/**`, `crates/slicer-{macros,sdk,ir,schema,core}/**`, `modules/core-modules/*/{src,wit-guest}/**` or their `Cargo.toml`, `crates/slicer-wasm-host/test-guests/*/src/**`. One Design notes bullet:

```markdown
<!-- snippet: wasm-staleness -->
- Feeds guest WASM: run `cargo xtask build-guests --check` after edits and judge by exit code (0 fresh, 1 stale, 3 infrastructure error) before interpreting any guest test failure (AGENTS.md §Guest WASM Staleness).
```

## coord-system

Applies to geometry or mm/unit conversion work. One Design notes bullet:

```markdown
<!-- snippet: coord-system -->
- Units: 1 unit = 100 nm (10⁻⁴ mm), not OrcaSlicer's 1 nm; divide OrcaSlicer constants by 100 and convert with `Point2::from_mm` / `mm_to_units()` (`docs/08_coordinate_system.md`).
```
