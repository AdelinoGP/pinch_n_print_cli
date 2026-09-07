# Task Map: 242-support-family-orca-closure

Crosswalk between this packet's steps, the allocated task IDs (`TASK-538`..`TASK-549`), and the
backlog rows in `docs/07_implementation_status.md`. Registration of these IDs is Step 1 work
owned by this packet (queue rule: registration deferred to packet-owned closure step); TASK-335
closes here and only here.

| Step | Title | Task IDs | docs/07 row(s) | Backlog anchor |
| --- | --- | --- | --- | --- |
| 1 | Register IDs + amend TASK-335 pointer | TASK-538 | TASK-538..549 (new open rows), TASK-335 (amend) | support family Orca closure row |
| 2 | Cross-packet disposition pre-audit | TASK-539 | TASK-539 | plan §12 brief 242 "register closure"; §10 |
| 3 | Gap-register disposition ledger + mirror tokens (adds the `Disposition` column) | TASK-540, TASK-541 | TASK-540, TASK-541 | `docs/specs/support-parity-gap-register.md` — every live `| G-NN |` row (count re-derived at audit time) |
| 4 | Deviation + divergence dispositions | TASK-542, TASK-543 | TASK-542, TASK-543 | DEV-141..DEV-146; `orca-divergences.md` squash groups 1-8 |
| 5 | Absorbed-218 e2e support-marker test | TASK-544, TASK-545 | TASK-544, TASK-545 | 218-support-gcode-e2e absorption (plan §10) |
| 6 | Re-prove inherited suite + inspection records | TASK-546, TASK-547 | TASK-546, TASK-547 | inherited 224 ACs AC-1..AC-4, AC-6; E2 records |
| 7 | Supersession records + 224 flip | TASK-548 | TASK-548 | plan §10; AC-5 |
| 8 | Closure ceremony + human-gate record | TASK-549 | TASK-549 (+ TASK-335 flip at sign-off) | final human gate (§8 + §12 brief 242); whole-suite green (E5) |

## ID allocation notes

- `TASK-538..TASK-549` was the next contiguous range after the live `TASK-537` high-water mark
  when re-derived on 2026-09-07. Immediately before Step 1 writes, re-derive with
  `grep -oE 'TASK-[0-9]{3}' docs/07_implementation_status.md | sort -Vu | tail -1` and verify the
  proposed range remains absent with
  `grep -oE 'TASK-(53[8-9]|54[0-9])' docs/07_implementation_status.md | sort -Vu`. If the tip moved
  or any proposed ID appears, allocate a fresh contiguous range and update all five packet files
  before registration.
- `TASK-324..328`, `TASK-330..335`, `TASK-336..343` are historically claimed/closed — never
  reused.
- `TASK-163b-orca-ref` was closed by packet 224 on 2026-08-20; this packet re-confirms that
  disposition against fresh references (AC-6) but does not reopen or re-close the row.
- All eleven direct dependencies and remediation packet 241b were `implemented` when re-derived
  on 2026-09-07; 241b restored packet 241 AC-N2 to green. Step 2 re-checks all twelve statuses.
