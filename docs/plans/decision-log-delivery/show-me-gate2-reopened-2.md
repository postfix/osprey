## What problem do we have?

Approved Gate 1 C8 promises that every record delivered to the file or the SIEM states when the decision was made ("what was served and when", consumers "with timestamps"). Final verification of slice 3 found this is not true (P-WHEN): delivered records carry no time. Only the console line has one, and that time comes from the console formatter.

Consequence: an auditor or a SIEM query cannot tell from a delivered record when a package was served. The collector's receipt time cannot stand in for it, because a SIEM batch may arrive up to 2 s plus retries after the decision.

## How will we solve it?

**Capability added (C48):** every record offered to a sink carries a `timestamp` field, in UTC, formatted as RFC 3339. This covers decision records and summary records. The console line is unchanged. That scope is the user's answer to Gate 2 grilling Q1: file and SIEM only.

```text
decide(): next.run(request) returns
   |
   +-- build record, stamped from app.clock.now_utc_micros()
   |      |
   |      +-- stdout: tracing::info!, WITHOUT timestamp (same 12 fields as today)
   |      +-- file sink queue   : record WITH timestamp
   |      +-- SIEM sink queue   : record WITH timestamp
   |
summarise()        -> periodic summary, stamped when built -> sinks
drain flush        -> final summary, stamped when built    -> sinks
```

- **Time source:** the clock that `App` already injects (`App.clock`, `src/lib.rs:52-54` at Git base `0f50cec`). `src/clock.rs:8-9` requires every time reading to go through that trait, so tests can control it.
- **When the time is taken:** when the record is built. For a decision record that is in `decide()`, after `next.run(request)` returns. A periodic summary is stamped in `summarise()`, and the final summary at the drain flush.
- **Dependencies:** none added. Gate 3 chooses the formatter. QA notes that `jiff 0.2.37` is already a direct dependency.

| Module | Change in this revision | Owns | Callers / dependencies |
|---|---|---|---|
| `src/http/logging.rs` | Decision and summary records gain `timestamp`, which the stdout render leaves out. `decide()` reads the clock from the `App` it already holds. `summarise()` and the drain flush get the same `Arc<dyn Clock>` from their caller | Decision fields, counters | Uses `src/delivery/mod.rs`, `App.clock` |
| `src/delivery/*`, `src/config.rs`, `src/lib.rs`, `src/tasks/mod.rs`, `config.sample.toml`, `docs/operations.md`, `src/main.rs` | Unchanged from the previous Gate 2 approval | as before | as before |

**Other edits in this revision (no decision changes):**
- **C9:** corrected in place. The three renders are identical except that the file and SIEM copies carry `timestamp`. With consumer identification off, stdout still has exactly its twelve fields.
- **C4:** its citation now reads "consumer identification (Gate 1 C1)".
- **Structure:** added a `## Repository evidence` section and a note on ID overlap. Gate 1 rows are cited as "Gate 1 C<n>".
- **Stamping points:** now named in the Program behavior capability line, the `src/http/logging.rs` Fit row, and Flow steps 2 and 5.

Everything else in Gate 2 is unchanged from its previous approval. That includes C4-C8, C10-C12, C41, the sinks, the drain token and the three config rejections.

## How will we confirm it is solved?

All checks below are planned. Gate 4 designs them as new slice 4. None has run yet.

| Scenario | Expected result | Check (planned) |
|---|---|---|
| Request decided with the file sink on | File record has `timestamp` = injected clock's time, UTC RFC 3339 | Test with a controlled clock reads the file line |
| Request decided with the SIEM sink on | SIEM payload record has the same `timestamp` | Test collector inspects the POST body |
| Summary window elapses; process shuts down | Periodic and final summaries delivered to the sinks carry `timestamp` | Test drives the summary and drain paths |
| Any request, all options off | Console line has exactly today's twelve fields and no `timestamp` | Test asserts the stdout field set |
| Build | No new crate in `Cargo.toml` | Review the dependency diff |

Observed so far: sf-verification P-WHEN **FAILED** on the slice 3 tree (the problem). Every truth in slice 3's own row was VERIFIED.

Recommendation: Approve. C48 fulfils an already-approved Gate 1 promise. It adds one field read from the existing injected clock and no new dependency, and the default console output stays unchanged. Fresh Gate 2 QA returned READY with no blocking findings.

Limits: Gate 3 still has to choose the formatter and design the exact field placement. Three non-blocking QA notes carry to Gate 3. N1: the `docs/operations.md` Fit row does not yet list `timestamp`, or say that stdout omits it. N2: section order differs from the template, which is formatting only. N3: Flow step 5 wording on flush/cancel order; the code flushes first and then cancels. The previous residuals are unchanged, for example that dropped records are gone and that a SIEM-only deployment cannot report its own outage. Presentation render: not inspected.

Status: Gate 2 reopened 2026-09-22 to design approved Gate 1 C8, which slice 3 final verification (P-WHEN) found missing. Red Team was not triggered: the revision adds one field from the existing injected clock, and its only choice was the user's (recorded at the top of 02-architecture.md). Fresh sf-gate-qa returned READY and `router gate-qa verify --gate 2` returned READY. After approval, Gates 3 and 4 reopen to design and slice the timestamp as new slice 4.

Sources: `02-architecture.md` (C9, C48, Program behavior :48, Fit :71, Flow 2 and 5); `01-product.md` (Gate 1 C8, APPROVED 2026-09-22); `gate-2-qa.md` (READY, N1-N3); `evidence/slice-3.md` (P-WHEN); `evidence/repository-structure.md` (Git base `0f50cec`).

Approve Gate 2 — Architecture as revised with C48 (file and SIEM records carry a UTC RFC 3339 `timestamp` from the injected clock; the console line is unchanged)?
