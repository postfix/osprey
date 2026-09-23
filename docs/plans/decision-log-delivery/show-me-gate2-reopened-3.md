## What problem do we have?

**Why you are asked again:** you answered "Approve" to the previous presentation of this revision, but the approval could not be recorded. `router gate-qa approve` refused with "Path outside allowed roots", because that QA run tracked a workflow reference file (gate-documents.md) that lies outside the repository. A fresh QA run then found one more defect, now fixed: rows C4, C5 and C6 had the Class `imported deferral`, which is not an allowed value. They now read `current-Gate decision`. The `docs/operations.md` Fit row also now lists the `timestamp` field and says the console line leaves it out. **No decision changed.** The design below is the one you approved before.

The problem: approved Gate 1 C8 promises that every record delivered to the file or the SIEM states when the decision was made. Final verification of slice 3 found that promise unmet (P-WHEN). Delivered records carry no time. Only the console line has one, and the console formatter supplies it.

Consequence: an auditor or a SIEM query cannot tell from a delivered record when a package was served. The collector's receipt time cannot stand in for it, because a SIEM batch may arrive up to 2 s plus retries after the decision.

## How will we solve it?

**Capability added (C48):** every record offered to a sink carries a `timestamp` field, in UTC, formatted as RFC 3339. This covers decision records and summary records. The console line does not change. That scope was your answer to Gate 2 grilling Q1: file and SIEM only.

```text
decide(): next.run(request) returns
   |
   +-- build record, stamped from app.clock.now_utc_micros()
   |      |
   |      +-- stdout: tracing::info!, WITHOUT timestamp (today's 12 fields)
   |      +-- file sink queue   : record WITH timestamp
   |      +-- SIEM sink queue   : record WITH timestamp
   |
summarise()        -> periodic summary, stamped when built -> sinks
drain flush        -> final summary, stamped when built    -> sinks
```

- **Time source:** the clock `App` already injects (`App.clock`, `src/lib.rs:52-54` at Git base `0f50cec`). `src/clock.rs:8-9` requires every time reading to go through that trait, so tests can control it.
- **When the time is taken:** when the record is built. A decision record is stamped in `decide()`, after `next.run(request)` returns. A periodic summary is stamped in `summarise()`, and the final summary at the drain flush.
- **Dependencies:** none added. Gate 3 chooses the formatter. QA confirmed that `jiff 0.2.37` is already a direct dependency.

| Module | Change in this revision | Owns | Callers / dependencies |
|---|---|---|---|
| `src/http/logging.rs` | Decision and summary records gain `timestamp`, which the stdout render leaves out. `decide()` reads the clock from the `App` it already holds. `summarise()` and the drain flush get the same `Arc<dyn Clock>` from their caller | Decision fields, counters | Uses `src/delivery/mod.rs`, `App.clock` |
| `docs/operations.md` | Also documents the `timestamp` field on file and SIEM records, and that the console line omits it | - | - |
| `src/delivery/*`, `src/config.rs`, `src/lib.rs`, `src/tasks/mod.rs`, `config.sample.toml`, `src/main.rs` | Unchanged from the previous Gate 2 approval | as before | as before |

**Other edits in this revision (none changes a decision):**
- **C4, C5, C6:** Class corrected from `imported deferral` to `current-Gate decision`, which matches their origin rows in `01-product.md`. Their IDs, owner, target and policies are unchanged.
- **C9:** corrected in place. The three renders are identical except that the file and SIEM copies carry `timestamp`. With consumer identification off, stdout still has exactly its twelve fields.
- **C4:** its citation now reads "consumer identification (Gate 1 C1)".
- **Structure:** a `## Repository evidence` section and an ID-overlap note were added. Gate 1 rows are cited as "Gate 1 C<n>".
- **Stamping points:** now named in the Program behavior capability line, the `src/http/logging.rs` Fit row, and Flow steps 2 and 5.

Every other part of Gate 2 is unchanged from its previous approval, including the policies of C4-C8, C10-C12 and C41, the sinks, the drain token and the three config rejections.

## How will we confirm it is solved?

All checks below are planned. Gate 4 designs them as new slice 4. None has run yet.

| Scenario | Expected result | Check (planned) |
|---|---|---|
| Request decided with the file sink on | File record has `timestamp` = injected clock's time, UTC RFC 3339 | Test with a controlled clock reads the file line |
| Request decided with the SIEM sink on | SIEM payload record has the same `timestamp` | Test collector inspects the POST body |
| Summary window elapses; process shuts down | Periodic and final summaries delivered to the sinks carry `timestamp` | Test drives the summary and drain paths |
| Any request, all options off | Console line has exactly today's twelve fields and no `timestamp` | Test asserts the stdout field set |
| Build | No new crate in `Cargo.toml` | Review the dependency diff |

Observed so far: sf-verification P-WHEN **FAILED** on the slice 3 tree. That failure is the problem this revision fixes. Every truth in slice 3's own row was VERIFIED.

Recommendation: Approve. C48 fulfils a promise Gate 1 already approved. It adds one field read from the existing injected clock and no new dependency, and the default console output stays the same. The design is the one you approved before. Only a classification defect and one documentation line were fixed, and fresh Gate 2 QA returned READY with no blocking findings.

Limits: Gate 3 still has to choose the formatter and design where the field sits. Four non-blocking QA notes carry to Gate 3:
- **N2:** the section order differs from the template. This is formatting only.
- **N3:** Flow step 5 does not name which component flushes the final summary.
- **N4:** Gate 1 and Gate 2 both use the IDs C7 and C8. The ID note in Gate 2 tells them apart, and renumbering would break the Gate 3 and Gate 4 citations.
- **N5:** the decision timestamp is read after `next.run` returns. For a long response (up to 15 min), it marks when the response completed, which matches the console's time. Gate 3's operator contract must say which moment the field means.

The previous residuals are unchanged. For example, dropped records are gone, and a SIEM-only deployment cannot report its own outage. Presentation render: not inspected.

Status: Gate 2 was reopened 2026-09-22 to design approved Gate 1 C8, which slice 3 final verification (P-WHEN) found missing. This is the third presentation of the reopened Gate. Red Team was not triggered: the revision adds one field from the existing injected clock, and the only choice in it was yours (the reason is recorded at the top of 02-architecture.md). Fresh sf-gate-qa returned READY, and `router gate-qa verify --gate 2` returned READY (8 files). After approval, Gates 3 and 4 reopen to design and slice the timestamp as new slice 4.

Sources: `02-architecture.md` (C4-C6, C9, C48, Program behavior :48, Fit :71 and :77, Flow 2 and 5); `01-product.md` (Gate 1 C8, APPROVED 2026-09-22); `gate-2-qa.md` (READY, F1 resolved, N2-N5); `evidence/slice-3.md` (P-WHEN); `evidence/repository-structure.md` (Git base `0f50cec`); prior presentation `show-me-gate2-reopened-2.md`.

Approve Gate 2 — Architecture as revised with C48 (file and SIEM records carry a UTC RFC 3339 `timestamp` from the injected clock; the console line is unchanged)?
