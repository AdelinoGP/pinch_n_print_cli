# Preflight Report: top-bottom-surface-keys

## Preflight Gate: 264-top-bottom-surface-keys

Reviewed: 2026-09-01 · Mode: --preflight · Symbol-inventory dispatched: 1 packet

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files)     | PASS | — |
| S1 Prerequisite-status truth      | PASS | — |
| S2 Deviation-ID conformance       | PASS | — |
| S3 Schema-version computed        | PASS | — |
| S4 ADR slot allocation            | PASS | — |
| S5 Shipped-symbol existence/shape | PASS | — |
| S6 WIT/IR identifier drift         | PASS | — |
| S7 Test-target wiring             | PASS | — |
| S8 ADR conformance                | PASS | — |
| (existing) AC runnable command    | PASS | every AC ends with its own delegation-friendly command; the AC-6 deviation probe was live-tested against `docs/15_config_keys_reference.md` (26 data rows) with the double-quoted form that survives this Windows shell |
| (existing) Doc Impact Statement   | PASS | `docs/15_config_keys_reference.md` +4 rows, deviation block unchanged at 26; verification greps listed |

### Blockers (S4/S5/S6) — fix before any commit

None.

### High (S1/S2/S3/S7/S8) — fix or convert to justified FORWARD-DEP

None.

### Accepted FORWARD-DEPs (consumer name/shape matches the producer packet's plan)

- `infill_config_schema_tdd` (guard binary) ← produced by draft packet 262, names reconciled ✓
- `infill_pattern_specific_config_schema_tdd` (guard binary) ← produced by draft packet 263, names reconciled ✓
- `toml = "0.8"` dev-dep in `rectilinear-infill/Cargo.toml` ← add-if-absent; first lander (262/263/264) wins ✓

**Verdict:** PREFLIGHT PASS (0 blockers, 0 high)
