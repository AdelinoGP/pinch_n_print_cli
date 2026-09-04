# Task Map: 242-support-family-orca-closure

Crosswalk between this packet's steps, the allocated task IDs (`TASK-429`..`TASK-440`), and the
backlog rows in `docs/07_implementation_status.md`. Registration of these IDs is Step 1 work
owned by this packet (queue rule: registration deferred to packet-owned closure step); TASK-335
closes here and only here.

| Step | Title | Task IDs | docs/07 row(s) | Backlog anchor |
| --- | --- | --- | --- | --- |
| 1 | Register IDs + amend TASK-335 pointer | TASK-429 | TASK-429..440 (new open rows), TASK-335 (amend) | support family Orca closure row |
| 2 | Cross-packet disposition pre-audit | TASK-430 | TASK-430 | plan §12 brief 242 "register closure"; §10 |
| 3 | Gap-register disposition ledger + mirror tokens (adds the `Disposition` column) | TASK-431, TASK-432 | TASK-431, TASK-432 | `docs/specs/support-parity-gap-register.md` — every live `| G-NN |` row (count re-derived at audit time) |
| 4 | Deviation + divergence dispositions | TASK-433, TASK-434 | TASK-433, TASK-434 | DEV-141..DEV-146; `orca-divergences.md` squash groups 1-8 |
| 5 | Absorbed-218 e2e support-marker test | TASK-435, TASK-436 | TASK-435, TASK-436 | 218-support-gcode-e2e absorption (plan §10) |
| 6 | Re-prove inherited suite + inspection records | TASK-437, TASK-438 | TASK-437, TASK-438 | inherited 224 ACs AC-1..AC-4, AC-6; E2 records |
| 7 | Supersession records + 224 flip | TASK-439 | TASK-439 | plan §10; AC-5 |
| 8 | Closure ceremony + human-gate record | TASK-440 | TASK-440 (+ TASK-335 flip at sign-off) | final human gate (§8 + §12 brief 242); whole-suite green (E5) |

## ID allocation notes

- `TASK-429..TASK-440` is a **backfill into an unused gap**, not an append at the ledger tip:
  higher IDs were registered by later packets after this one was authored (TASK-523..TASK-537 are
  reserved by 239d/240a/240b (240a owns TASK-533..TASK-536; 240b owns TASK-537)). Re-derive immediately before Step 1 writes:
  `grep -oE "TASK-[0-9]{3}" docs/07_implementation_status.md | sort -u | tail -1` — this returns
  the tip, which is expected to be far above TASK-440 and is not a stop condition. The gating
  check is that TASK-429..TASK-440 are **absent**:
  `grep -oE "TASK-4(29|3[0-9]|40)" docs/07_implementation_status.md | sort -u` must return
  nothing.
- `TASK-324..328`, `TASK-330..335`, `TASK-336..343` are historically claimed/closed — never
  reused.
- `TASK-163b-orca-ref` was closed by packet 224 on 2026-08-20; this packet re-confirms that
  disposition against fresh references (AC-6) but does not reopen or re-close the row.
