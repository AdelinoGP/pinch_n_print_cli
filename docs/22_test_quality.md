# Test Quality

**When to read this:** when writing or reviewing test code in this repository —
especially LLM-authored tests — and when `cargo xtask check-test-quality`
flags a violation. This is the authoring standard for tests, companion to
`docs/21_data_defaults_and_fixtures.md` (which owns the struct-literal rule).
Definitions of the core terms live in `CONTEXT.md` (False green, Compile
witness, Production oracle, Loud skip, Negative control, Roster check);
this doc applies them with in-tree examples.

Keywords: false green, oracle, compile witness, loud skip, negative control,
roster check, vacuous loop, fixture-skip, source-grep, `test-quality`, waiver,
check-test-quality, R1-R8, earn-their-keep

---

## 1. The standard

A test earns its keep (ADR-0064) only if a reviewer can name the specific
regression input that would slip through if the test were deleted. A test that
cannot answer that question is retired, not kept by default. This doc tells you
how to write tests that answer it, and how the gate catches the mechanically
recognizable failure patterns.

**Green is not coverage.** A passing test proves only that its assertion
executed and held. It proves the behavior only if the assertion is falsifiable
by a plausible production defect. When writing a test, run the counterfactual:
"if the production code this name claims to test were broken in the way the name
describes, would this test fail?" If the answer is no, the test is a false green
— worse than a missing test, because it reports a hole as coverage.

## 2. The false-green taxonomy

Each pattern below is one way a test can pass while exercising nothing. The
gate rules R1–R8 (§5) mechanically catch the detectable subset; the rest is
review judgment guided by this section.

### 2.1 Self-referential oracle

The expectation is derived from the code under test (or a test-local mirror of
its algorithm), so the test proves the code agrees with itself. See CONTEXT.md
**Production oracle**.

- Symptom: the test calls the same function to build both sides of the
  comparison, or embeds a re-implementation of the production algorithm.
- Legitimate exceptions: encode/decode roundtrip pairs (different functions
  that must invert each other) and determinism pins (same function called
  twice with identical inputs, claiming stability not correctness). Mark both
  with a `// test-quality:` waiver naming which exception applies.
- In-tree example (repaired shape): `region_mapping_two_semantics_produces_cross_product_cardinality`
  (`crates/slicer-core/tests/algo_region_mapping_tdd.rs`) derives its expected
  cardinality analytically instead of from the mapper.

### 2.2 Success-only assertion

The test's terminal statement is a bare `let _ = f(...)` or an
`is_ok()`/`is_some()` check, with no assertion about the produced value. It
proves the call path didn't panic — nothing more. Gate rule R4. Legitimate as a
marked compile witness only.

### 2.3 Vacuous loop

The test iterates a collection that is empty under the fixture (no skeleton
points, no committed regions, no emitted events), so the property "checked"
inside the loop is never exercised. Classic in-tree case:
`support_plan_has_finite_branch_paths` (`crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`)
reported by the audit as iterating zero skeleton points. Repair by requiring a
nonempty population (`assert!(!xs.is_empty())` before the loop) or by driving
the pipeline with a fixture that actually produces one. An always-failing
production implementation must fail such a test.

### 2.4 Hand-maintained roster

The expected set is an array written in the test (`["pass_a", "pass_b", ...]`),
so an addition to the real set stays green forever. See CONTEXT.md
**Roster check**. Repair by deriving the expectation from an authority (the
schema version, the manifest, the registry) — e.g. the fixed-array literal
report in `dag_validation_tdd.rs` should be replaced by coverage of the
production report. A documented count pin (a wire-format field count) is a
legitimate contract, waived with the pinned format named.

### 2.5 Source-grep assertion

The test greps its own crate's (or another file's) source text and treats a
substring match as behavior: "the feature exists" proved by "the string exists".
This checks the editor buffer, not the compiler output. Two legitimate uses:
(a) a migration guard pinning an API's absence until equivalent compile-fail
coverage exists (e.g. `closure_api_is_fully_removed` in
`crates/slicer-sdk/tests/finalization_builder_tdd.rs`), and (b) generated-source
template checks in `pnp_cli module new`, which assert template text is the
product. Both are waived and name which legitimate use applies. Everything else:
replace with a behavior or compile check.

### 2.6 Silent fixture skip

The test returns early when a fixture is missing, without a loud mechanism.
Green forever regardless of the coverage it claims. See CONTEXT.md **Loud
skip**. Gate rule R3 catches the detectable form.

### 2.7 Assertions about the scaffolding

The test asserts on values the test itself constructed (a hand-built plan
containing the expected answer, a locally formatted log submitted to the
logger, a resolver the test wrote rather than the production resolver). The
WORLD-Z/PRECEDENCE repair items in the remediation plan are this pattern at
scale: scripted plans that inject the answer, local resolver duplicates that
replace production resolution. Repair by driving the production path with
inputs whose expected output is independently derivable.

### 2.8 Self-equality and decorative assertions

`assert!(true)`, asserting equality of a value with itself, or asserting a
property that holds for every possible input. Gate rule R1.

## 3. Legitimate weak forms (waive, don't delete)

The gate flags patterns; the waiver records why the flagged form is correct.
Deleting the body of any of these removes the only check of something real:

- **Compile witnesses** (CONTEXT.md): the sole compile/link check of a public
  type, re-export, constructor, or WIT export. Waive with the protected surface
  named. If behavior is also claimed, add a real assertion instead of
  deleting.
- **Encode/decode roundtrip pairs** — mark with the pair's two function names.
- **Determinism pins** — mark as determinism, not correctness.
- **Negative controls** for shared oracles (e.g. the parity-invariant
  self-tests in `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs`):
  prove the comparator rejects a known defect. Keep them; they are the
  comparator's own test.
- **Opt-in probes and benchmarks** (timing, foreign-language feasibility):
  feature- or env-gated tooling, never counted as correctness coverage. Gate
  rule R8 does not fire inside these when the gating is explicit.

## 4. Derivation rule (the authoring checklist)

Before writing or accepting a test, answer:

1. **What regression input does this test catch?** If you cannot name one, do
   not write the test.
2. **Where does the expected value come from?** It must be an independent
   derivation (contract doc, analytic computation, recorded reference) — not
   the function under test, not a mirror of it. Roundtrip pairs and
   determinism pins are the two exceptions; mark them.
3. **Does the population exist?** Any loop over production output must assert
   the population is nonempty first.
4. **What happens when the fixture is missing?** Fail loudly or gate
   explicitly; never return silently.
5. **Is any expected set hand-maintained?** Derive it from an authority, or
   waive it as a pinned contract with the format named.
6. **Could this test pass with the named production defect present?** Run the
   counterfactual from §1. If yes, redesign before committing.

A PR that adds a test which fails questions 1, 2, or 6 is rejected in review,
not waived: waivers are for legitimate weak forms (§3), not for tests that
claim more than they check.

## 5. The gate: `cargo xtask check-test-quality`

Runs over the same file scope as `check-literals` (crate `tests/` and
`benches/` directories whole-file; crate `src/` files scanned for inline
test scopes). Rules and implementation status:

| Rule | Detects | Legitimate form (waive) | Implemented |
|---|---|---|---|
| R1 | `assert!(true)`; `x == x` self-comparison | none — retire instead | yes (macro-path + token scan) |
| R2 | `#[test] fn` with an empty body | none — retire instead | planned (first crate waves) |
| R3 | bare-return guard on a fixture-lookup condition inside a `#[test]` fn | intentional env-gated skip (name the gate) | yes (guard conditions matching `exists`/`is_file`/`fixture`/`metadata`/`try_open`/`read_to_string`) |
| R4 | body ends in a bare `is_ok()`/`is_some()`/`let _ =` with no value assertion | compile witness (name the surface) | planned (review-guided until then) |
| R5 | count literal compared against a same-test enumerated roster | documented wire-format pin (name the format) | planned (review-guided until then) |
| R6 | same test-fn name twice in one file | none — one registration wins; keep the intentional one | yes (target-level duplicate registration is a wave review item) |
| R7 | both sides of an `assert!` derive from one call of the same symbol | encode/decode pair or determinism pin (name which) | planned (review-guided; the token scan catches the bare `x == x` form) |
| R8 | `std::thread::sleep` in test code | opt-in probe (name the gate) | yes (AST `...::sleep` call paths + token scan inside macros) |

False-positive calibrations, recorded so future rules don't re-introduce them:
a right-hand ident preceded by `.` is a field access, not a self-comparison
(`.object_id == object_id` compares a field to a local of the same name);
absence-of-output guards in non-test helpers
(`assert_no_bundle_written`'s `if !output.exists() { return; }`) are
negative-path control flow — R3 fires only inside `#[test]` fns; and
`std::thread::sleep` is an associated-function call (`Expr::Call` with a
`...::sleep` path), never a method call.

Waiver syntax (mirrors `docs/21_data_defaults_and_fixtures.md` §Waiver):

```rust
// test-quality: compile witness — sole compile check of <surface>
#[test]
fn prelude_exports_all_types() { let _ = prelude::Everything; }
```

A waiver must name the protected surface, contract, or gate. An empty reason is
itself a violation.

Modes: `cargo xtask check-test-quality --report` prints findings and exits 0;
without `--report` it exits 1 on unwaived findings. The gate ships in report
mode and `cargo xtask test` runs it in its preflight without blocking;
**enforce mode is switched on by the remediation program's final wave**
(ADR-0065) — flipping the `TEST_QUALITY_ENFORCED` constant in
`xtask/src/test.rs`.

## 6. Relations to other docs

- `docs/21_data_defaults_and_fixtures.md` — owns struct-literal discipline and
  the fixture-bases policy; this doc owns oracle/coverage discipline. The two
  gates share the xtask preflight.
- `AGENTS.md` Test Discipline — owns run mechanics (narrow runs, feature-gate
  blindness, log teeing, census reconciliation). This doc owns what a passing
  test is allowed to claim.
- `CONTEXT.md` — owns the vocabulary (False green, Compile witness, Production
  oracle, Loud skip, Negative control, Roster check).
- `docs/specs/test-quality-remediation-plan.md` — owns the wave plan, the
  census baseline, and the disposition ledger for the audited tests.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` — the retirement
  standard.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` — the
  enforcement rollout.