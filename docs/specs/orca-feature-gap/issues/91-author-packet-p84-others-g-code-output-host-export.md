# 91 — Author packet P84 — Others / G-code output — host-export

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-10) — claimed 2026-09-10, resolved 2026-09-10
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P84 — Others / G-code output — host-export** — 1 keys, Tier B new logic, owner host-export. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P84 — Others / G-code output — host-export):

`gcode_add_line_number`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation, no packet.** The "Author packet P84" title is
a rotted ledger fact — re-sized at claim time under the map's *Packets are for
complex implementation only* rule. The remaining work was one host-export key and
a line-prefix rewrite at the export seam: no new module, IR field, WIT/schema
change, claim seam, or new decision-point geometry.

**Scope (rule 3) — in scope, and the 04 owner column is correct as written.**
`gcode_add_line_number` is `coBool` default `0` (`PrintConfig.cpp`). Its only
*behavioural* read in canonical is the export-time post-processor
`gcode_add_line_number` (`src/slic3r/GUI/PostProcessor.cpp`), called at the end of
export from `BackgroundSlicingProcess.cpp` after the file is written. Every other
touch is non-behavioural: the declaration (`PrintConfig.cpp` / `.hpp`), the
`Print::invalidate_state_by_config_options` `steps_gcode` bookkeeping set
(`Print.cpp` — the same invalidation-only class the map already records for other
keys), preset listing (`Preset.cpp`) and the GUI option row (`Tab.cpp`). Rule 3's
exclusion list names *non-behavioural* read sites (tooltips, preset plumbing,
`ConfigManipulation.cpp`, IGNORE sets); a post-processor that rewrites the
exported artifact is live behaviour, and ticket 04 had already adjudicated the key
in scope with owner "host export orchestration (`crates/slicer-runtime`; GUI
post-processor in canonical)". No re-file, no scope change.

**What landed.**

- `crates/slicer-runtime/src/run.rs` — `DEFAULT_GCODE_ADD_LINE_NUMBER: bool =
  false` (named constant, the `[host_runtime]` convention), read from the
  CLI/JSON config source beside `gcode_flavor` / `use_relative_e_distances`
  (tolerating an `Int` 0/1 via `extract_bool`), and applied as the **last export
  step** to `SliceOutcome::gcode_text` so both the `--output` file and the stdout
  path carry it. `add_line_numbers` ports the canonical loop verbatim: every line
  of the whole artifact — header, comments, and CONFIG_BLOCK included — prefixed
  `N<line> `, numbered from 1 in file order, newline-terminated. Off by default,
  so emitted bytes are unchanged.
- `crates/slicer-scheduler/src/manifest.rs` — a `HOST_RUNTIME_KEYS` row (bool,
  `SCOPE_PRINT`, default `false`) so the key appears in the `module config-schema`
  reply's host array.
- `docs/config/host-keys.toml` — `[host_runtime]` row tying the default to
  `run.rs::DEFAULT_GCODE_ADD_LINE_NUMBER`; `docs/15_config_keys_reference.md`
  regenerated (`cargo xtask gen-config-docs`; 55 → 56 host keys).

**Decision point now driven, at a non-default value.** `gcode_add_line_number =
true` makes every exported G-code line `N<line> `.

**Tests proving it.**

- `run::tests::add_line_numbers_prefixes_every_line_from_one` — comment / command
  / blank / unterminated-final-line / empty-input semantics.
- `host_keys_doc_lock_tdd::host_runtime_keys_match_constants` — the doc row cannot
  drift from the code default.
- `run_slice_api_tdd::gcode_add_line_number_prefixes_every_exported_line` — a real
  slice of `resources/regression_wedge.stl` through the live core-module tree:
  default emits no prefix; `true` numbers every line contiguously from 1; everything
  ahead of the CONFIG_BLOCK is byte-identical once the prefixes are stripped.

**Two observations recorded, neither a defect introduced here.**

1. Setting the key adds `; gcode_add_line_number = true` to the CONFIG_BLOCK — any
   user-set key the resolver does not recognise routes through its `extensions`
   bucket, which `to_config_map` emits — and thereby displaces one count-bounded
   `ORCA_CONFIG_PADDING` row: the block is held at the ≥96-key OrcaSlicer floor,
   so it does not grow. That emitted bool is the word-form `true` which canonical's
   `ConfigOptionBool::deserialize` reads back as `false`; that is the known
   CONFIG_BLOCK spelling defect owned by
   [132](132-author-packet-config-block-reader-contract.md) and is **not**
   spot-fixed here.
2. `cargo xtask build-guests --check` still exits non-zero on this tree for a
   **pre-existing** 5-crate guest-lock divergence
   (`crossbeam-deque`, `crossbeam-epoch`, `crossbeam-utils`, `indexmap`, `syn`).
   It reproduces with this ticket's changes stashed, so it is a HEAD condition,
   not this work (remedy: `cargo xtask build-guests --sync-locks`, out of scope
   here). The stale-*artifact* set is now empty: touching
   `crates/slicer-scheduler` puts every guest in the stale closure (the crate is
   in each guest's lock), so `cargo xtask build-guests` was run.

**Verification.** `cargo check --all-targets` and `cargo clippy --all-targets
-- -D warnings` clean for both touched crates; `cargo xtask check-literals` 0
violations; `cargo xtask gen-config-docs --check` current; `slicer-runtime` unit
bucket 94/94 and `run_slice_api` 2/2; `slicer-scheduler` all buckets green. No
deviation rows (default matches canonical); no packet number consumed; queue count
unchanged.
