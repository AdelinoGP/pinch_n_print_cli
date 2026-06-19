---
when: Read this when running `spec-review --preflight <packet>`, or when Step 1 preflight needs the symbol-existence gate. Defines the seven authoring checks (S1–S7) a draft packet must clear before its files are committed / it is activated.
keywords: preflight, symbol existence, fictional symbol, draft packet gate, authoring defect, S1-S7
---

# Preflight Symbol-Existence Gate (S1–S7)

A spec packet can read flawlessly and still instruct the implementer to extend a
function, field, enum variant, ADR slot, schema constant, WIT type, or test
binary that does not exist or has a different shape. Prose-quality review does
**not** catch this — only resolving each named symbol against the tree does.

This gate is the mechanism that breaks that cycle. It runs against a **draft**
packet and emits a structured FACT pass/fail report. **The authoring agent must
clear every check (or downgrade a flagged item to an explicit, justified
FORWARD-DEP) before the packet's files are committed or the packet is
activated.**

It originates from the 2026-06 review wave that found ≥1 fictional-symbol defect
in all of packets 104–112 (`build_wall_flags`→`build_outer_wall_flags`; WIT
`surface-group` absent; `PaintSemantic::SeamEnforcer` fictional; arachne module
modeled as a stub when it was a 512-line impl; ADR slots 0009/0010/0012 already
taken; schema versions hardcoded against a speculative reservation table;
deviation IDs that live only in the roadmap; new contract tests never registered
in `tests/contract/main.rs`).

## When this runs

- **`--preflight` mode**: this gate is the *entire* review. No AC traces, no
  verification-command runs, no closure verdict. Output is the S1–S7 FACT report
  plus the existing Step-1 AC-command and Doc-Impact checks. Verdict enum is
  `PREFLIGHT PASS` / `PREFLIGHT BLOCKED`.
- **Full / Delta mode**: run S1–S7 as part of Step 1 preflight *before* any AC
  tracing. A packet that fails the gate cannot be `APPROVED`; report the failures
  as Critical (S5/S6 fictional refs, S4 ADR collisions) or High (S1/S2/S3/S7) and
  stop dispatching AC traces until they're acknowledged — tracing an AC whose
  symbols are fictional wastes budget.

## Context discipline for the gate

The gate is context-hostile (4–5 packet files × dozens of tree greps). Obey the
skill's hard limits. The controller reads **only** the 5 packet files directly
(they are the allowed direct reads); **every** tree/grep/existence check is a
sub-agent dispatch. Dispatch one **symbol-inventory** extraction per packet
first (below), then fan out per-check verification. Never read source files,
`docs/` bodies, or `target/` in the controller.

### Symbol-inventory dispatch (run once per packet, first)

```
Question: From the 5 packet files in <packet dir>, extract every CONCRETE
  reference, classified. Do NOT verify anything against the tree — extraction only.
Scope: the packet dir's packet.spec.md, requirements.md, design.md,
  implementation-plan.md, task-map.md only.
Return format: SUMMARY with these labeled lists (omit empty ones):
  - PREREQ-PACKETS: each "Depends on" packet + the status the packet ascribes it
  - DEVIATION-IDS: each D-*/DEV-* token + verb (create | supersede | close | grep) + the file each AC greps for it
  - SCHEMA-VERSIONS: each hardcoded *_SCHEMA_VERSION SemVer + which IR + the AC/grep that pins it
  - NEW-ADRS: each ADR filename the packet authors + the template ADR it cites
  - PREEXISTING-SYMBOLS: each fn/struct/enum/field/trait-method/module/file the packet treats as already-existing (verbs: extend/consume/call/read/rename/reuse/"already has"/"ships"/"placeholder"/"stub") + claimed crate + claimed shape
  - WIT-IR-IDENTIFIERS: each WIT type/record/func + each IR enum-variant the packet names as pre-existing
  - NEW-TEST-FILES: each new test file + its target test binary (e.g. --test contract) + the aggregator it must register in
```

The checks below consume these lists. Each check's verification is itself
dispatched (FACT pass/fail). The controller only adjudicates returned FACTs.

## S0 — Packet structure (runs first, before symbol checks)

**Asserts:** the packet directory contains all five contract files —
`packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`,
**and `task-map.md`** — each non-empty.

**Dispatch / check:** `ls <packet dir>` — confirm all five exist.

**FAIL (HIGH)** if any of the five is missing or empty. The most common miss is
`task-map.md` (symbol-focused regen passes edit the four prose files and silently
omit it — this is a packet-authoring defect of equal severity to a missing AC
command). **Fix:** author the missing file. For `task-map.md`, map every
`task_id` in `packet.spec.md` frontmatter to its backlog/roadmap row. Run S0
before S1–S8: a structurally incomplete packet cannot pass preflight regardless
of symbol cleanliness.

### Re-verify-on-disagreement protocol (mandatory)

The gate exists because symbols must be resolved against the **real tree**, not
asserted from memory — and that discipline applies to the gate's own operator,
not just to the packets it audits. Two rules:

1. **Ground every claim with a grep.** Before recording any FAIL (or accepting
   any PASS), the controller must have a sub-agent's tree grep behind it — not a
   recollection, not a prior agent's summary taken on faith. A claim with no
   grep behind it is not evidence.
2. **When a re-check disagrees with an earlier claim, the re-check wins, and you
   say so.** If your own follow-up grep contradicts something you (or a prior
   sub-agent) reported, openly retract the earlier claim and propagate the
   verified result — never the convenient one. Re-verification failures are
   often operator error (wrong cwd, stale path, relative-vs-absolute), so when a
   result "feels wrong," re-run it with an absolute path from the repo root
   before drawing any conclusion. A single fabricated/false sub-claim in a sweep
   does **not** condemn the whole sweep — re-verify each load-bearing claim
   independently rather than wholesale-accepting or wholesale-rejecting.

This protocol is what turns the gate from "a checklist a confident agent can
rationalize past" into "a thing that only passes when the tree actually agrees."

---

## S1 — Prerequisite-status truth

**Asserts:** no packet is described as a satisfied (`implemented`/`shipped`)
dependency unless its own `packet.spec.md` frontmatter is `status: implemented`.

**Dispatch:** for each PREREQ-PACKET the packet calls implemented →
`FACT: grep '^status:' in .ralph/specs/<dep>/packet.spec.md — is it 'implemented'?`

**FAIL (HIGH)** if any dep claimed implemented is `draft`. **Fix:** replace the
"implemented" claim with an explicit `FORWARD-DEP on draft <dep>` blocker, and
move any AC that consumes that dep's output behind the blocker.

## S2 — Deviation-ID conformance

**Asserts:** every deviation ID the packet references conforms to the
`DEVIATION_LOG.md` ID convention and exists/doesn't-exist as the verb requires.

**Dispatch:**
- `FACT: what ID format does docs/DEVIATION_LOG.md use? (sample 2 recent rows)`
- For each DEVIATION-ID with verb `supersede`/`close`/`grep`:
  `FACT: does '<id>' appear in docs/DEVIATION_LOG.md? Y/N`
- For each with verb `create`: `FACT: is '<id>' absent from the log AND format-conformant? Y/N`

Derive the real convention from the log at runtime (the first dispatch) — do
**not** assume a format. As of 2026-06 the log uses `D-<pkt>-<SLUG>` (e.g.
`D-96-AC8-CUBE-REBASELINE`, `D-103-T041-VORONOI-PORTED`) and bare `D-<n>`; a new
`D-104-OVERHANG-QUARTILE-NONE` is therefore format-conformant.

**FAIL (HIGH)** if: an ID's format doesn't match the convention the log actually
uses; a `supersede`/`close` AC greps `DEVIATION_LOG.md` for an ID that lives only
in the roadmap (or nowhere — the common defect: `D-98`, `D-10/12`, `D-9`,
`D-7/15` are referenced for closure but absent from the log); or a `create` ID
already exists. **Fix:** match the live convention; point supersede/close greps
at the file the ID actually lives in (often the roadmap, not the log); register
to-be-created IDs in the log as part of the packet.

## S3 — Schema-version computed, not hardcoded

**Asserts:** no AC hardcodes a future `*_SCHEMA_VERSION` SemVer; version targets
the correct constant for the IR being changed; targets are internally consistent.

**Dispatch:**
- For each SCHEMA-VERSION: `FACT: which constant governs <IR>? what is its live value in crates/slicer-ir/src/slice_ir.rs?`
- Cross-check the packet's own docs for a single agreed target.

**FAIL (HIGH)** if: the packet pins a hardcoded future version (e.g. AC greps
`major: 4, minor: 6`) against a reservation table while the live constant is
lower and the intervening bumps are unshipped; the bump targets the wrong
constant (e.g. `CURRENT_SLICE_IR_SCHEMA_VERSION` for a `SurfaceClassificationIR`
field, which has its own `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION`); or the
packet's docs disagree on the target (4.4 vs 4.6). **Fix:** the AC should assert
the *field/variant addition*, not a literal version; compute the version bump at
activation from the live constant; one IR → one constant.

## S4 — ADR slot allocation

**Asserts:** any new ADR uses a number not already present in `docs/adr/`, and no
doc references the ADR before it exists.

**Dispatch:**
- `LOCATIONS: ls docs/adr/ — list NNNN prefixes in use; report the highest.`
- For each NEW-ADR filename: `FACT: is prefix <NNNN> already taken? what is the next free number?`
- `FACT: do any docs/ files already reference <new-ADR path>? LOCATIONS` (dangling-ref check)

**FAIL (CRIT)** on number collision (e.g. authoring `0012-...` when
`0012-spatial-indexing-...` exists). **FAIL (HIGH)** on a dangling forward
reference to the not-yet-created ADR. **Fix:** allocate the next free number;
remove premature cross-references until the ADR lands.

## S5 — Shipped-symbol existence & shape (core check)

**Asserts:** every PREEXISTING-SYMBOL exists in the tree, in the named crate,
with the named shape; every "placeholder/stub" claim about a module is true.

**Dispatch** (batch by area to bound dispatch count): for each PREEXISTING-SYMBOL
`FACT/LOCATIONS: does <symbol> exist in <claimed crate/path>? exact signature/fields? if a module is called a 'stub/placeholder', is its lib.rs actually a stub (warn!/Ok(()) only) or a working impl?`

**FAIL (BLOCKER)** on any: fictional fn/struct/field/method; wrong crate of
origin (`variable_width` is `slicer-ir`, not `slicer-core`; `Point2` is
`slicer-ir`, not `slicer_core::geometry`); wrong shape (`Vec<bool>` vs
`Vec<Vec<bool>>`); duplicate symbol the packet "creates" that already exists
elsewhere with live call sites (`WallSequence`); a "new directory/module" that
already exists; a "stub/placeholder" module that is actually a working impl; a
"source fn to promote" that the file doesn't contain. **Fix:** correct the name /
crate / shape; if creating a symbol that already exists, reconcile (rename,
extend in place, or de-duplicate) and account for existing call sites.

> Distinguish PRE-EXISTING (verify) from NET-NEW (expected absent — do **not**
> flag). The verb is the discriminator: extend/consume/call/read/rename/reuse/
> "already has"/"ships"/"stub" ⇒ pre-existing; add/create/introduce/register ⇒
> net-new. A symbol the packet will create that already exists is an S5 FAIL.

## S6 — WIT / IR identifier drift

**Asserts:** every WIT type/record/func and IR enum-variant named as pre-existing
exists with that exact identifier (kebab on the WIT side, the right variant on
the Rust side).

**Dispatch:** for each WIT-IR-IDENTIFIER `FACT: does <wit-or-variant> exist in crates/slicer-schema/wit/** or the named IR enum? exact identifier?`

**FAIL (BLOCKER)** on: absent WIT record (`surface-group`); kebab/snake or
naming drift (`wall-loop-type` vs `loop-type`); a fictional enum variant where
the data is actually `Custom("…")` (`PaintSemantic::SeamEnforcer/SeamBlocker` →
`Custom("seam_enforcer")`); a field-type assumption that's wrong (`WallLoop.path`
is `ExtrusionPath3D`, not `Vec<Point3WithWidth>`). **Fix:** use the real
identifier; for `Custom(...)` paint, match on the string, not a named variant;
route through the real wrapper type.

## S7 — Test-target wiring

**Asserts:** every NEW-TEST-FILE under an aggregated test binary is registered in
the aggregator, and its AC's `cargo test --test <bin> <filter>` references the
binary the file actually lands in.

**Dispatch:**
- `FACT: how is the <bin> test binary aggregated? (tests/<bin>.rs vs tests/<bin>/main.rs mod-list); list current mod declarations.`
- For each NEW-TEST-FILE: `FACT: does the AC's --test target match where the file lands, and is a 'mod <name>;' registration accounted for in the aggregator? Is the aggregator in the step's edit list within the per-step edit cap?`

**FAIL (HIGH)** if: a new `tests/contract/*.rs` (or `unit`/`integration`) file
has no `mod` registration planned (it silently won't compile →
`cargo test --test <bin> <name>` reports "0 tests run", a false pass); the AC's
`--test` binary or path doesn't match where the file goes
(`tests/contract/dag_validation.rs` when DAG tests live at
`tests/unit/dag_validation_tdd.rs`); or adding the aggregator breaks the step's
≤3-edit cap. **Fix:** add the aggregator to the step's edit list (split the
sub-step if it busts the cap); point the AC at the real binary/path.

## S8 — ADR conformance (no silent ADR rewrites)

**Asserts:** a packet whose `design.md` touches behavior governed by an existing
ADR's *normative content* (the field shape, mechanism, ordering, or constraint
the ADR locked) either **conforms** to that ADR or **explicitly amends** it via a
deviation — it never silently contradicts it. Silently rewriting an
architecture-level decision is precisely what produced the defect cascade this
gate was built to stop.

**Dispatch:** for each ADR the packet's design surface plausibly touches
(cross-reference PREEXISTING-SYMBOLS / SCHEMA / WIT lists against `docs/adr/`):
`FACT: does the packet's design contradict <ADR>'s normative clause on <topic>? quote the ADR clause and the packet clause.`

**FAIL (HIGH)** if the packet's design conflicts with an existing ADR's normative
content **and** the packet neither conforms nor carries a
`D-<pkt>-ADR-<NNNN>-AMENDED` deviation that (a) references the ADR by slot
number and (b) quotes the contested clause. Example: ADR-0013-mmu locks
`bisector_edge_skip_mask: Vec<bool>` (flat, per-edge, WIT-boundary perf
rationale); a packet specifying `Vec<Vec<bool>>` must either conform to
`Vec<bool>` or carry `D-105-ADR-0013-AMENDED` quoting the clause. **Fix:**
conform the packet (default — cheapest, keeps the ADR authoritative); or, only if
the change is genuinely warranted, author the amendment deviation **and** a
superseding ADR — an ADR edit requires its own decision record, never a quiet
packet-driven rewrite.

> S4 covers "don't invent/collide ADR *slots*"; S8 covers "don't silently
> contradict an ADR's *content*." Both halves are needed.

---

## Gate report format (`--preflight` output)

Emit exactly this, then the verdict. One row per check; list offending items
inline (≤ 5 per check; if more, give count + the worst 5).

```
## Preflight Gate: <packet>

Reviewed: YYYY-MM-DD · Mode: --preflight · Symbol-inventory dispatched: <N packets>

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files)     | PASS / FAIL | ... |
| S1 Prerequisite-status truth      | PASS / FAIL | ... |
| S2 Deviation-ID conformance       | PASS / FAIL | ... |
| S3 Schema-version computed        | PASS / FAIL | ... |
| S4 ADR slot allocation            | PASS / FAIL | ... |
| S5 Shipped-symbol existence/shape | PASS / FAIL | ... |
| S6 WIT/IR identifier drift         | PASS / FAIL | ... |
| S7 Test-target wiring             | PASS / FAIL | ... |
| S8 ADR conformance                | PASS / FAIL | ... |
| (existing) AC runnable command    | PASS / FAIL | ... |
| (existing) Doc Impact Statement   | PASS / FAIL | ... |

### Blockers (S4/S5/S6) — fix before any commit
1. ...

### High (S1/S2/S3/S7/S8) — fix or convert to justified FORWARD-DEP
1. ...

### Accepted FORWARD-DEPs (consumer name/shape matches the producer packet's plan)
- <symbol> ← produced by <draft packet>, names reconciled ✓

**Verdict:** PREFLIGHT PASS / PREFLIGHT BLOCKED (<n> blockers, <m> high)
```

A `FORWARD-DEP` is **only** acceptable when the producing draft packet's spec
actually plans to create the symbol with the **same name and shape** the consumer
assumes. If the producer's plan differs (or doesn't exist), it is an S5/S6 FAIL,
not a FORWARD-DEP. Reconcile the name/shape in **both** specs before either
activates.
