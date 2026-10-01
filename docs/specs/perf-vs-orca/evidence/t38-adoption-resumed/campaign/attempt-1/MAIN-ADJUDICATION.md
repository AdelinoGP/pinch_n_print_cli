# Main-session campaign adjudication

**Authoritative result: INCONCLUSIVE; production adoption did not pass.**
`campaign-summary.json` is the real runner output, not the synthetic control.
The command returned exit 0; adoption is decided by its cell results, not by
that exit alone. The campaign ran once and stopped at supports-off Benchy
Arachne. Four support-enabled cells remain `not-run`; no retry or standalone
follow-up cell ran.

| Supports-off Benchy | Exactness/status | CPU median, ordinary → accelerated | Wall median, ordinary → accelerated | Decision |
|---|---|---|---|---|
| Classic | pass / clean | 145.0625 → 119.60155 s | 20.8286 → 19.90815 s | KEEP |
| Arachne | pass / clean | 72.8672 → 69.59375 s | 17.17315 → 17.11635 s | inconclusive |

These are four retained samples per variant, excluding exactness and warmup
runs. Classic has strict favorable separation in both metrics. Arachne has
strict favorable CPU separation but overlapping wall ranges: ordinary
16.5563–17.4910 s, accelerated 16.9912–17.1664 s. A favorable median is not the
required separated range. All retained rows have matching generator markers,
`ok` completion and zero degraded/non-fatal/fatal counts. Structural exactness
uses the unchanged documented tolerances; it is not a full geometry oracle.

## Reporting-helper discrepancy, not an evidence-loss finding

The original generated `FINDINGS.md` and `results.json` remain unchanged and
their hashes remain checkable. Their top-level `BLOCKED` label and assertion
that preservation failed are incorrect. `write_findings`
(`docs/specs/perf-vs-orca/evidence/t38-adoption-resumed/campaign/run_campaign_once.py`)
checks `tracked_payload_copy_status`, while the successful archive path records
`tracked_campaign_payload_copy_status: "copied"`. The missing key causes a
false integrity override after the runner has already stopped inconclusively.
No helper correction or rerun was performed.

The main session independently verified every recorded raw-archive file hash
and every recorded tracked-evidence file hash, and the archived runner summary
matches `campaign-summary.json`. It also checked the retained rows, actual
range comparisons and successful post-campaign integrity records. See
[main verification](main-verification.json). The recorded before/after frozen
reference gates both exited 0; recursive snapshot and campaign-input identities
match. Therefore this is an inconclusive performance gate, not missing evidence.

Any later authorized helper repair should add a regression for a real
inconclusive runner result with successful post-integrity and copied tracked/raw
payloads: it must remain `INCONCLUSIVE`, while genuinely failed copies must
still produce `BLOCKED`. That repair must not rerun this campaign or alter its
original results.

## Scope and side effects

Runner controls, packet-focused assertions, controlled production snapshots,
and this partial campaign have evidence in the enclosing directory. The full
workspace suite was not run; support-enabled campaign exactness and timing are
unmeasured because the prescribed stop occurred earlier. The earlier caught-
panic summary display conflict remains documented separately, not suppressed.

Build/test artifacts and runner outputs under `target/` were refreshed. Complete
raw campaign outputs and preserved prior acceptance evidence live under the
durable snapshot root named by `hash-inventory.json`; licensed G-code was not
copied into tracked evidence. Ordinary remains the actual default. No
production source, policy, threshold, reference, fixture or lockfile was changed;
no commit or default switch occurred. The next route requires a human decision.
