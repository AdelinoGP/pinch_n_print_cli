# Accelerated residual query attribution

Type: task
Status: open
Blocked by: 12

## Question

Split the residual per-vertex query cost in accelerated mode — the hot queries
shrank only ~1.9× while total guest fuel fell 35.8%
([Accelerated-mode pair](09-accelerated-mode-pair.md)) — into three parts and
recommend which is the next optimization candidate:

1. **Wasm spatial-tree query internals** (the suspected u128 envelope math) in
   `crates/slicer-core/src/perimeter_spatial.rs`'s accelerated path.
2. **Fallback paths firing**: `bridge_arithmetic_safe` and
   `IndexState::Linear` (same file) — count firings and attribute their cost;
   these should be rare in accelerated mode and may not be.
3. **Any remaining linear scans** reachable in the accelerated build (the
   2026-09-22 attribution found the scans dominating in ordinary mode).

Attribution runs only (ADR-0055 user scopes / `--instrument-stderr`; never
wall claims). Runs use the complete accelerated snapshot recipe.

Deliverable: the measured three-way split plus a recommendation — this scopes
[Consume-only-when-read annotations](20-consume-only-when-read-annotations.md)
against accelerated mode (what remains worth avoiding once the query path and
fallbacks are accounted for). Re-rank or drop leads per
[Gap budget per cell](12-gap-budget-per-cell.md)'s budgets where they conflict.
