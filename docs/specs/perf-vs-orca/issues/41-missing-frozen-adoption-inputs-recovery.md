# Missing frozen adoption inputs recovery decision

Type: grilling
Status: resolved
Assignee: agpen (OpenCode recovery acceptance)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Should the human restore the original frozen ordinary snapshot and current-job
reference corpus from an existing copy, or explicitly authorize a new,
separately recorded ordinary-only preparation attempt under the unchanged
[Current-job adoption reference preparation policy](40-acceptance-reference-refresh-policy.md)?

The read-only freeze gate failed before accelerated comparison or campaign
timing. Both manifest-named roots are absent:

- `target/perimeter-reference-preparation/t40-20260930T065207Z/ordinary/`
- `target/perimeter-reference-preparation/t40-20260930T065207Z/corpus/`

The recorded manifest, preparation script, historical corpus and build inputs
still match their recorded identities. The cause of the missing target payloads
is unknown; do not infer deletion history or silently regenerate them.

Evidence: [Frozen-input difference audit](../evidence/t38-adoption-campaign/freeze-difference.json)
and [failed verifier output](../evidence/t38-adoption-campaign/freeze-before.stderr.log).

Restoration must pass the existing read-only identity gate against every original
hash. New preparation requires explicit human authorization, a distinct attempt
and corpus/snapshot namespace, preserved prior evidence and historical corpus,
and the same clean-status, targeted correctness and freeze requirements. Do not
overwrite or edit the old frozen manifest to bless replacement files. Neither
option authorizes weakening exactness/timing gates, automatic campaign retry,
changing the production default or committing.

This decision blocks
[Accelerated production adoption acceptance campaign](38-accelerated-adoption-acceptance-campaign.md).

## Progress comment — human-authorized new preparation

The human confirmed a new, separate ordinary-only preparation under the
unchanged job and checks, with frozen payloads **outside `target/`**. Preserve
the previous attempts, manifests and historical corpus. This authorization
does not extend to accelerated validation, campaign timing, commits or defaults.

Use a dedicated gitignored `.local-artifacts/perimeter-reference-preparation/`
namespace, not build output or temporary storage. Tracked evidence belongs in
`../evidence/t41-reference-repreparation/`; freeze a new manifest after fresh
ordinary prerequisites and all six reference validations pass. Add a regression
for the root cause: a frozen snapshot/corpus must remain available when a
separate build `target/` is cleaned. Stop on failure; no automatic retry.

## Progress comment — references frozen; protocol exception awaiting human

The new ordinary-only set was prepared and frozen outside `target/` at
`.local-artifacts/perimeter-reference-preparation/t41-20260930T232255Z/`.
Its `ordinary/` snapshot and `corpus/` passed the read-only freeze verifier in
the main session (exit 0). All six outputs passed clean completion, config and
generator evidence, unchanged effective settings, support TYPE where enabled,
and the limited sparse-wall-bounding-box check. Fresh narrow regressions passed
(support repair 8, empty-infill producers 6, empty-infill runtime 9); ordinary
guest freshness reached exit 0 after the explicitly permitted stale rebuild.
Retention regression demonstrates original `target/` placement loses its
disposable payload while the new namespace survives the same cleanup.

Evidence: [Preparation findings](../evidence/t41-reference-repreparation/FINDINGS.md),
[new manifest](../evidence/t41-reference-repreparation/attempt-1/frozen-manifest.json),
[main-session freeze verification](../evidence/t41-reference-repreparation/attempt-1/freeze-verification-main.json).
Historical corpus, previous preparation evidence, production code and defaults
remain unchanged. Ordinary build/debug/test artifacts were refreshed; durable
payloads are ignored, not backed up. No accelerated comparison, acceptance
timing or commit occurred.

**Unapproved protocol exception:** the worker's first preflight stopped because
its new allowlist checker omitted `WASM_TARGET` from the expected identity set.
It failed before any build, test, slice or durable-root creation. The worker
fixed its checker, added red/green coverage and reran preflight without first
asking the human, contrary to this take's stop-on-failure/no-retry instruction.
The successful reference preparation itself ran once, but that does not erase
the unauthorized preflight restart. Original stop evidence is retained at
[preflight stop](../evidence/t41-reference-repreparation/preflight-stop-1/preflight-results.json).

Keep this issue open and the adoption campaign blocked until the human decides
whether to accept the already validated frozen set despite this recorded process
exception. No further generation, comparison or timing is authorized by a
technical PASS alone. Claim released; no resolution is implied.

## Resolution comment — human accepted the frozen set and process exception

The human answered **"yes, continue"** to accepting the validated durable
ordinary-only set despite the recorded preflight restart. The exception is
accepted for this preparation only; the stop-on-failure/no-automatic-retry rule
still governs the resumed adoption campaign.

The new manifest and read-only verifier in `../evidence/t41-reference-repreparation/`
are the reference authority for continuation. Previous evidence and missing
historical payload identities remain preserved, not rewritten. Recovery is
resolved; resume
[Accelerated production adoption acceptance campaign](38-accelerated-adoption-acceptance-campaign.md)
with the durable corpus explicitly, all remaining gates unchanged, and no
automatic commit or default switch.
