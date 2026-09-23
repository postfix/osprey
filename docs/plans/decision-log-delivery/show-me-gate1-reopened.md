## What problem do we have?

An operator cannot get the firewall's decision records anywhere useful. Today the records go only to the console. That leaves three questions the operator cannot answer:

- **Who needs cleaning up?** After a package is blocked, which machines already took it?
- **Can we prove what the firewall did?** An auditor wants durable records.
- **Does the security team hear about it?** Blocks never reach their SIEM.

Slices 1-3 now deliver records to a file and to the SIEM. Final verification found two gaps in what this Gate promised:

1. **Records do not say when the decision was made.** Records delivered to the file and the SIEM carry no time. Only the console line has one. This Gate promised "what was served and when" and a list of consumers "with timestamps". Without the time, the operator cannot tell whether a machine took the package before the block or after it.
2. **A promised setting was never built.** C2 offered a setting that refuses a request rather than serving it unlogged. No later Gate designed it, and none deferred it.

## How will we solve it?

This reopening changes only those two points. Everything else in Gate 1 stays the same.

| Change | What the operator gets |
|---|---|
| **C8: the time of the decision is now an explicit promise** | Every record delivered to the file or the SIEM states when the firewall made the decision. The time the SIEM receives the record does not count, because delivery can lag behind the decision. |
| **C7: the refuse setting is deferred out of this feature** (your answer, 2026-09-22) | Records that cannot be delivered are dropped, counted and reported. No setting in this feature refuses a request because its record was not delivered. That setting would make a request wait on delivery, or fail because of it. This would break C3 ("delivery never slows a request"), so it belongs in a plan of its own. |

To match C7, C2's policy, the "Losing a record must be visible" paragraph and a new non-goal ("Not a refuse-when-unlogged mode") now all say the same thing.

Unchanged from the approved Gate 1:
- Everything stays off by default.
- Consumer identification is opt-in and off by default (C1).
- Delivery never slows a request (C3).
- The success metric is under five minutes from a block to the list of consumers, measured with one SIEM query.
- No record is lost silently.
- There is no UI.

## How will we confirm it is solved?

All of these are planned. None has been run yet.

| Scenario | Expected result | Check |
|---|---|---|
| A decision is delivered to the file | The record states the time of the decision | New slice 4 check (to be designed at Gates 2-4) |
| A decision is delivered to the SIEM, possibly late | The record states the time of the decision, not the time the SIEM received it | New slice 4 check |
| The destination is unreachable | The request is still served. The lost record is counted and reported. No request is refused. | Already checked in slices 1-3 (P-C3, drain accounting, TP-19) |
| Final verification is run again | P-WHEN passes. P-C2-OPTIN no longer applies because the setting is deferred. | sf-verification after slice 4 |

Recommendation: Approve. The first change states a promise this Gate already made, so Gates 2-4 can design it. The second records your decision to defer the refuse setting, which keeps "delivery never slows a request" intact. After approval, Gates 2, 3 and 4 reopen in turn to design the time-of-decision field and plan it as slice 4. Slices 1-3 stay checked.

Limits:
- This Gate does not decide how the time is represented. Gates 2-3 decide that.
- A refuse-when-unlogged mode is not available in this feature.
- The guard metric's "rated load" is not quantified, and its check (P-GUARD-LOAD) has not been run under load. This does not block Gate 1.
- While consumer identification is off, the five-minute target does not apply.
- render: not inspected

Gate 1 was reopened on 2026-09-22 after final verification. Gate QA: READY. Slices 1-3 are built and checked.

Sources: docs/plans/decision-log-delivery/01-product.md (C2, C7, C8, Non-goals, Decisions); gate-1-qa.md (READY); evidence/slice-3.md (sf-verification P-WHEN, P-C2-OPTIN, P-GUARD-LOAD)

Approve Gate 1, or what should change?
