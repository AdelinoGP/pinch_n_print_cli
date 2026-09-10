# 92 — Author packet P85 — Others / Post-processing Scripts — host-export

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-10) — claimed 2026-09-10, resolved 2026-09-10
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P85 — Others / Post-processing Scripts — host-export** — 1 keys, Tier B new logic, owner host-export. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P85 — Others / Post-processing Scripts — host-export):

`post_process`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation, no packet.** The "Author packet P85" title is a
rotted ledger fact — re-sized at claim time under the map's *Packets are for complex
implementation only* rule. The remaining work was one host-export key plus a shell-out
at this port's export seam: no new module, IR field, WIT/schema change, claim seam, or
new decision-point geometry.

**Scope (rule 3) — in scope, and the 04 owner column is correct as written.**
`post_process` is canonical `coStrings` (`PrintConfig.cpp`, default `{}`) and its only
behavioural read is the export-time post-processor
`PostProcessor.cpp::run_post_process_scripts`, called from
`BackgroundSlicingProcess::finalize_gcode` after the file is written. That is the same
GUI-layer post-processor class ticket 91 adjudicated in scope for
`gcode_add_line_number` — live behaviour that rewrites the exported artifact, not a
tooltip / preset / `IGNORE` read — and ticket 04's Pass 3 had already assigned the key
to host-export orchestration with `crates/slicer-runtime` as owner. No re-file, no
scope change.

**What landed.**

- `crates/slicer-runtime/src/run.rs`
  - `DEFAULT_POST_PROCESS: &[&str] = &[]`.
  - `parse_post_process_scripts` — the canonical `coStrings` list spelling
    (`["cmd1", "cmd2"]`) plus the bare-string form; a non-string list entry or a
    non-list/non-string value is a loud parse error rather than a skipped command.
  - `run_post_process_scripts` — writes the artifact to the sibling `<output>.pp`
    working copy (canonical's own File-host name, so a script never touches a file the
    caller keeps), splits each entry on `\r`/`\n`, trims, skips empty lines, runs each
    command through the platform shell with the working-copy path appended as the final
    argument (`$SHELL -c "<cmd> '<path>'"` on POSIX, `cmd /C` on Windows), exports
    `SLIC3R_PP_HOST=File` / `SLIC3R_PP_OUTPUT_NAME` into the child environment, treats a
    non-zero exit or a script-deleted file as fatal (canonical's error text), reads the
    rewritten file back and removes the working copy on every path.
  - Both export keys now compose in canonical order: scripts first,
    `gcode_add_line_number` last (`BackgroundSlicingProcess::finalize_gcode`).
  - A run with no output file (stdout) and a non-empty `post_process` fails loudly
    ("…needs `--output`") instead of silently skipping configured scripts.
- `crates/slicer-scheduler/src/manifest.rs` — a `HOST_RUNTIME_KEYS` row
  (`string-list`, print scope, empty default) so the key appears in the
  `module config-schema` reply's host array.
- `docs/config/host-keys.toml` — `[host_runtime]` row locking the default to
  `run.rs::DEFAULT_POST_PROCESS`; `docs/15_config_keys_reference.md` regenerated
  (56 → 57 host keys). `xtask/src/gen_config_docs.rs` gained list-default arms
  (`Array` → type `list`, empty array renders `[]`) so the row is legible.

**Decision point now driven, at a non-default value.** A configured command runs
against the exported artifact and its rewrite is what the output carries:
`pnp_cli slice --output out.gcode` with `{"post_process": ["echo POST_PROCESSED>>"]}`
ends `out.gcode` with `POST_PROCESSED` and leaves no `out.gcode.pp`; a failing command
fails the slice and the caller's output file is never written.

**Security divergence (deliberate).** Canonical honours a `post_process` carried in
*model* metadata — an Orca 3MF stores the full print config in
`project_settings.config`, so a downloaded model can arm arbitrary host commands. Here
only an explicit `--config` value is honoured; a model-supplied value is dropped with a
warning in `run_slice`'s override merge. Filed as **DEV-197** (a), together with the
three unported mechanism clauses: the `SLIC3R_<KEY>` environment export, the
`.output_name` rename sidecar + `slicing_pipeline_plugin` step, and canonical's
Windows `CommandLineToArgvW` + `CreateProcess` launch semantics (this port runs the
entry through `cmd /C`).

**Tests proving it.**

- `run::tests::post_process_scripts_parse_string_list_and_reject_other_shapes`
- `run::tests::post_process_scripts_rewrite_the_working_copy_and_clean_up` — a
  newline-packed entry runs both commands in order; the working copy is removed; the
  caller's output path is untouched.
- `run::tests::post_process_script_failure_is_fatal_and_removes_the_working_copy`
- `run::tests::post_process_script_deleting_the_artifact_is_fatal`
- `host_keys_doc_lock_tdd::host_runtime_keys_match_constants` — new list arm locking
  the doc row to `DEFAULT_POST_PROCESS`.
- `run_slice_api_tdd::post_process_scripts_rewrite_the_exported_artifact` — real slice:
  absent key → no marker; set → the marker is the artifact's last line; with
  `gcode_add_line_number` → every line is numbered including the script-appended one,
  proving canonical order.
- `run_slice_api_tdd::post_process_from_model_metadata_is_ignored`.

**Observations.** Setting the key adds `; post_process = …` to the CONFIG_BLOCK
through the resolver's `extensions` bucket and displaces one count-bounded padding row
(the block stays at the ≥96 floor) — ticket 91's finding, unchanged; the value is
`;`-joined by the existing `ConfigValue::List` renderer. `cargo xtask build-guests
--check` still reports the **pre-existing** 5-crate guest-lock divergence ticket 91
recorded (`crossbeam-deque`, `crossbeam-epoch`, `crossbeam-utils`, `indexmap`, `syn`);
no Cargo manifest or lock file is touched by this ticket, and the stale-*artifact* set
is empty after the rebuild below.

**Verification.** `cargo check --workspace --all-targets` and
`cargo clippy -p slicer-runtime -p slicer-scheduler -p xtask --all-targets -- -D warnings`
clean; `cargo xtask check-literals` 0 violations; `gen-config-docs --check` and
`check-deviations --check` current (72 open deviations); `slicer-runtime` lib 116/116,
unit bucket 94/94, e2e bucket `post_process` 2/2; `slicer-scheduler` all buckets green
(90+45+43+24+6+3+2+1 passed); the 36 stale guests rebuilt with
`cargo xtask build-guests`. No packet number consumed; queue count unchanged.
