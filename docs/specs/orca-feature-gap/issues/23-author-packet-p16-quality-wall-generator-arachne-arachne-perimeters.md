# 23 — Author packet P16 — Quality / Wall generator — Arachne — arachne-perimeters

Type: task
Status: resolved
Assignee: wayfinder session (ses_f9bf4666affeSN3TSEIDgLo5ip) — claimed 2026-09-02
Blocked by: 06, 107
Map: ../map.md

## Question

Author the spec packet for **P16 — Quality / Wall generator — Arachne — arachne-perimeters** — 1 keys, Tier A plumbing, owner arachne-perimeters. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P16 — Quality / Wall generator — Arachne — arachne-perimeters):

`min_feature_size`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Resolved 2026-09-02 by direct implementation instead of creating a packet, per the
user's ruling that this item was small enough to complete in-session.

The production path was already live and canonical-shaped: the
`arachne-perimeters` manifest declares `min_feature_size` as `percent` with a
`25%` default, `arachne_params_from_config` resolves it against
`nozzle_diameter`, and `to_beading_factory_params` passes the resolved value to
the widening strategy's `min_input_width`. Added
`percent_min_feature_size_reaches_widening_threshold` in
`modules/core-modules/arachne-perimeters/tests/min_feature_size_config_tdd.rs`.
It proves the real module path emits a `0.15 mm` strip at the canonical `25%`
threshold and emits no wall when the configured threshold is `50%` of a
`0.4 mm` nozzle.

Validation passed:

- `cargo test -p arachne-perimeters --test min_feature_size_config_tdd`
  (`1 passed, 0 failed`), with output captured in `target/test-output.log`.
- `cargo fmt -p arachne-perimeters -- --check`.
- `cargo clippy -p arachne-perimeters --all-targets -- -D warnings`.
- `cargo xtask check-literals`.

No packet directory was created because the key's production behavior was
already implemented; this change adds the missing non-default regression
evidence directly.
