# clipper2 cost/output verdict

Type: task
Status: open
Blocked by: 15, 16

## Question

Decide the dependency direction on `clipper2-rust` — stay on 1.1.0, roll back
to 1.0.3, or pin with a documented reason — from measured cost and output
deltas (DEV-173's question: the 1.0.3 → 1.1.0 bump landed without a parity
comparison).

Work:

- **Cost A/B**: run the refreshed `polygon_ops` bench
  ([Criterion bench refresh](15-criterion-bench-refresh.md)) at both crate
  versions, interleaved with the recipe's discipline; `union` / `difference` /
  `intersection` / `offset` are the surfaces every hot call site uses.
- **Output diff**: run identical polygon fixtures through both versions and
  classify any output deltas (vertices, winding, collinear handling) — small
  real-model G-code deltas are within known noise (§3.6), so unit-level polygon
  comparison carries this, not byte diffs.
- Fold in [clipper2 1.1.0 upstream evidence](16-clipper2-1-1-0-upstream-evidence.md)'s
  findings (incl. whether `check_split_owner`'s recursion reach changed between
  the versions) into the recommendation.

The verdict fixes which clipper cost profile the optimization tickets
([offset2_ex call reduction](19-offset2-ex-call-reduction.md) onward) measure
against. Keep/drop to the human; nothing auto-committed.
