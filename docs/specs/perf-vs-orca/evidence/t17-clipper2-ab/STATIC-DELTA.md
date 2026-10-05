# t17 — clipper2-rust 1.0.3 → 1.1.0 static delta (re-derived)

Ticket: [clipper2 cost/output verdict](../../issues/17-clipper2-cost-output-verdict.md).
Run 2026-09-24. This re-derives [clipper2 1.1.0 upstream
evidence](../../issues/16-clipper2-1-1-0-upstream-evidence.md)'s source table
against the registry trees this box actually builds from, because that ticket's
`diff -rq`-based table cannot distinguish content changes from line-ending
churn and this ticket's measured A/B leans on the distinction.

Sources (exact trees cargo built against):

```
%USERPROFILE%\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\
  clipper2-rust-1.0.3\src
  clipper2-rust-1.1.0\src
```

Method: per file, raw SHA-256 vs newline-normalized SHA-256 (`tr -d '\r'`), plus
a normalized line diff to count additions/removals. Raw artifacts:
`target/t17-clipper2-ab/static/per-file-compare.txt` (gitignored, regenerable).

| File | raw SHA equal | normalized equal | changed lines | removed |
| --- | --- | --- | ---: | ---: |
| `clipper.rs` | NO | NO | 58 | **0** |
| `clipper_tests.rs` | NO | NO | 47 | **0** |
| `core.rs` | NO | yes | 0 | 0 |
| `core_tests.rs` | NO | yes | 0 | 0 |
| `engine.rs` | yes | yes | 0 | 0 |
| `engine_fns.rs` | NO | yes | 0 | 0 |
| `engine_public.rs` | yes | yes | 0 | 0 |
| `engine_tests.rs` | yes | yes | 0 | 0 |
| `lib.rs` | yes | yes | 0 | 0 |
| `main.rs` | NO | yes | 0 | 0 |
| `minkowski.rs` | NO | yes | 0 | 0 |
| `minkowski_tests.rs` | NO | yes | 0 | 0 |
| `offset.rs` | NO | yes | 0 | 0 |
| `offset_tests.rs` | NO | yes | 0 | 0 |
| `rectclip.rs` | NO | yes | 0 | 0 |
| `rectclip_tests.rs` | NO | yes | 0 | 0 |
| `utils/colors.rs` | NO | yes | 0 | 0 |
| `utils/file_io.rs` | NO | yes | 0 | 0 |
| `utils/mod.rs` | NO | yes | 0 | 0 |
| `utils/svg.rs` | yes | yes | 0 | 0 |
| `utils/timer.rs` | yes | yes | 0 | 0 |
| `version.rs` | NO | yes | 0 | 0 |

Consequences for this ticket's question:

- **Geometry code is content-identical.** Every module on the offset/boolean
  path (`engine.rs`, `engine_fns.rs`, `core.rs`, `offset.rs`, `rectclip.rs`,
  `minkowski.rs`) is byte-identical or line-ending-only; `engine.rs` — which
  holds `check_split_owner` — is raw byte-identical.
- **The only content deltas are pure additions.** `clipper.rs` (+58) and
  `clipper_tests.rs` (+47) have zero removed lines; the additions are the
  `PolyFace64` struct and `poly_path_to_faces64` / `poly_tree_to_faces64`. The
  repo does not call them (searched `crates/`), so they cannot perturb any call
  site. This matches ticket 16's finding; the difference here is that the
  line-ending-only files are now positively identified rather than being lumped
  with the content diffs.
- `src/version.rs`'s `CLIPPER2_VERSION` string is unchanged ("1.5.4" in both) —
  do not use it to identify the crate version; use `Cargo.lock`.
