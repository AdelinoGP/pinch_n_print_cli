# Data Defaults and Fixtures

**When to read this:** when you are writing or editing test code that constructs
struct literals, or when `cargo xtask check-literals` flags a violation. This is
a test-code conventions page, not a normative architecture doc.

Keywords: struct literal, exhaustive, FRU, functional record update, `..Default::default()`, waiver, watchlist, `check-literals`, `needless_update`, fixtures, `test_support`

---

## 1. The rule

In **test code**, a struct literal of a **watched type** must contain a `..`
rest (any base: `Default::default()`, a fixture call) **OR** an inline waiver
comment with a mandatory reason.

### 1.1. Checker CLI contract

Run `cargo xtask check-literals [--report] [PATH...]`. Positional paths use
component-aware workspace-relative prefix matching, so a path matches itself
or descendants but not a similarly named component. Output is sorted by
workspace-relative path and line, with forward-slash separators. Each
violation is printed as
`<ws-relative-path>:<line>: exhaustive literal of watched type \`<Name>\``;
the final summary is
`check-literals: <N> violation(s) in <M> file(s) (watchlist: <K> types)`.
Exit code 0 means no violations (and is also used by `--report`), 1 means
violations in enforce mode, and 2 means invalid usage such as an unknown flag.

```rust
// OK — functional record update from Default
let p = Point3WithWidth { x: 1.0, y: 2.0, z: 3.0, ..Default::default() };

// OK — fixture base
let p = test_support::point3_with_width(1.0, 2.0, 3.0);

// OK — waived (reason mandatory)
let p = Point3WithWidth { x: 1.0, y: 2.0, z: 3.0, width: 0.0 }; // exhaustive: width is intentionally explicit here
```

An exhaustive literal (all fields spelled, no `..`) of a watched type in test
code is a violation. `cargo xtask check-literals` enforces this and exits 1 on
any violation; `--report` prints the same output and always exits 0. The gate is
**enforced since packet 199**, runs as the `check-literals preflight` in
`cargo xtask test`, and is required before committing.

Operationally, `cargo xtask test` performs the workspace-wide enforce scan
before the guest-freshness check. `cargo xtask test --summary-from` remains
gate-free because it only summarizes an existing test log.

## 2. Production-exemption rationale

The rule applies to **test code only**. Production `src/` literals stay
exhaustive on purpose.

Commit `a579fc18` (packet 193) is the canonical example: `Point3WithWidth`
gained a field, sweeping 165 files — ~90% one-line `overhang_distance_mm: None`
filler in test files. But the production sites (`slicer-wasm-host/src/marshal/*`,
`interpolate_point`, perimeter producers) received *real logic* for the new
field, not filler. Exhaustive literals there are compiler-enforced propagation
checkpoints; FRU there would have silently dropped `overhang_distance_mm` at the
WIT boundary. Production exhaustiveness is a feature, not churn.

Measured scale for context: `a579fc18` touched 165 files; re-derived 2026-08-07,
103 test files still construct `Point3WithWidth` literals.

## 3. Watchlist derivation

The watchlist is derived at run time — there is no manual ledger to keep in
sync. A type is watched when **all** of the following hold:

- it is a `pub` struct (tuple structs excluded; `pub(crate)` and narrower excluded);
- it has **≥ 5 named fields**;
- it is defined under `crates/*/src/**` (including structs in inline mods).

Enum struct-variants cannot fire: the watchlist derives from struct definitions
only, so a variant like `SomeEnum::PrintEntity { … }` is not a watched literal
by itself.

## 4. Waiver format

Use `// exhaustive: <reason>` when a literal must stay exhaustive. The reason is
mandatory (non-empty). Placement:

- on the same line as the literal's opening line, or
- on the line immediately above the literal.

```rust
let p = Point3WithWidth { x: 1.0, y: 2.0, z: 3.0, width: 0.0 }; // exhaustive: width must be explicit here
```

```rust
// exhaustive: this site intentionally pins every field
let p = Point3WithWidth { x: 1.0, y: 2.0, z: 3.0, width: 0.0 };
```

## 5. Fixture policy

The designated home for shared IR fixture bases is `slicer_sdk::test_support`
(fixture bases authored by packet 195). Prefer a fixture call over spelling a
literal with `..Default::default()` when the type has no safe default or when
several fields are meaningful together. Host crates consuming it take a
`slicer-sdk` dev-dependency with `feature = "test"`.

The API provides `print_entity_base(role)`, `wall_loop_base(loop_type,
boundary_type)`, and `ordered_entity_view_base(role)`. Their explicit enum
arguments avoid unsafe blanket `Default` implementations, and the returned
bases are intended for FRU composition.

`SliceRunOptions::default()` is the quiet test baseline. It uses
`MeshIR::default()` for the current schema version, leaves paths and options
empty or `None`, and intentionally keeps `progress_events = false` rather
than using the CLI default.

## 6. `clippy::needless_update` guidance

When converting a site, **omit default-equal fields** rather than spelling all
fields plus `..Default::default()`. Never write spell-all-fields + FRU — that
both trips `clippy::needless_update` and defeats the point of the rest.

```rust
// Prefer: only the fields that differ from the default
let p = Point3WithWidth { x: 1.0, ..Default::default() };

// Avoid: spelling every field then adding a rest
let p = Point3WithWidth { x: 1.0, y: 0.0, z: 0.0, width: 0.0, ..Default::default() };
```

## 7. Known blind spots

The scanner is syn-based and cannot see through everything. Known gaps:

### 7.1. Scanner scope

The enforced scan covers `crates/*/tests/**`,
`modules/core-modules/*/tests/**`, `crates/*/benches/**`, and inline
`#[cfg(test)]` subtrees in `crates/*/src/**`. The tree
`crates/slicer-wasm-host/test-guests/*/src` is intentionally exempt so its
WIT adapter shims remain compiler-enforced.

An out-of-line `#[cfg(test)]` module declaration is not followed into its
separately declared module file; only inline `#[cfg(test)]` subtrees are
scanned.

1. **Macro range expressions.** A macro token tree with a top-level range
   expression (`field: 0..2`) reads the `..` as an FRU rest, suppressing
   detection (locked by test `scan_macro_range_blind_spot_documented`).
2. **Macro-generated struct definitions** are invisible to the watchlist. For
   example, `ResolvedConfig` in `crates/slicer-ir/src/resolved_config.rs` is
   emitted by a macro and is constructed with literals in ≥ 8 test files.
3. **Enum struct-variant name collisions.** `SomeEnum::PrintEntity { … }` would
   fire if the variant name collides with a watched struct name. No such
   collision exists today; the waiver is the escape hatch.
4. **`#[cfg(any(test, feature = "test"))]` mods** are treated as production
   `src/` and therefore exempt — e.g. `slicer_sdk`'s `test_support` declaration.
