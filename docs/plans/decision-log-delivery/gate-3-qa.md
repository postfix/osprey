# Gate 3 QA
## Verdict
READY
## Questions and findings
None blocking. All five prior findings are closed:

- R-C49-1 CLOSED. `close_window` now takes `clock: &dyn Clock` (03-program-design.md:548) and is the only summary clock read. `summarise` (:546-550) and `flush_summary` (:565-566) pass the clock on without reading it. The invariant bullet (:856) names the two read sites, `decide` and `close_window`, and records the residual with no execution witness: `summarise` could forward a different clock. Code review is assigned to check it, and the bullet explains why it cannot be witnessed: the periodic window runs on a real `Instant` with a 60 s default, and `set_summary_window` is process-global. TP-20 drives `flush_summary -> close_window`, which is the same read the periodic path uses. So a wrong clock inside `close_window` fails TP-20.
- R-C49-2 CLOSED. The call-stack rows at :756 (`decide -> summarise`) and :757 (`Running::shutdown -> flush_summary`) now carry `app.clock.as_ref()` / `self.app.clock.as_ref()`. The new row at :761 connects `summarise`/`flush_summary -> close_window -> clock.now_utc_micros() -> rfc3339`. The `Running::shutdown` snippet (:706) matches.
- R-C49-3 CLOSED. C49 (:38) and the `rfc3339` Returns (:182) give the same single expression: `jiff::Timestamp::from_microsecond(utc_micros).map(|t| format!("{t:.6}")).unwrap_or_else(|_| utc_micros.to_string())`.
- R-C49-4 CLOSED. evidence/research-jiff-format.md records the jiff 0.2.37 source lines (`from_microsecond` is fallible, `:728`; the `Display` precision doctest `{ts:.6}` gives `…49.123000Z`) and executed output. It is cited from C49 and from `rfc3339`. The fallback mirrors src/main.rs:109-111, which I checked against the current bytes.
- R-C49-5 CLOSED. TP-20 (:787) asserts on the **last** `request_summary` record. That covers a periodic summary emitted early because `COUNTERS` is shared by all tests in the binary.

Non-blocking observations, no revision required:
- O-1. C48 requires the decision stamp to be read after `next.run(request)` returns. The design states this (`decide` step 2, :523-524; call stack :760). TP-20 does not advance the clock during the request, so it cannot tell a read before `next.run` from a read after it. TP-20 does not claim that ordering, so this is an unclaimed property, not a false witness claim.
- O-2. Some line references are anchored to the Git base (`summarise` "one caller (`decide` at `:188`)", declaration `:294`). The current tree has `summarise` at src/http/logging.rs:349 and its call at `:243`. The document states its grounding base, so these references are not wrong.
## Checked dimensions
- Subject completeness, readability, freshness: PASS. I tracked and read 03-program-design.md (958 lines, all of it), 01-product.md, 02-architecture.md, all four evidence files, and the relied-on source: src/http/logging.rs, src/lib.rs, src/clock.rs, src/main.rs, Cargo.toml, tests/common/mod.rs, src/delivery/mod.rs, src/delivery/file.rs, tests/decision_log_delivery.rs.
- Clarification inventory: PASS. Every row C13-C49 has an ID, class, `resolved` status, owner 3, target `none`, a selected value and a decision source. C49 imports Gate 2 C48 by ID. Gate 1 deferrals C4-C6 were resolved at Gate 2, and Gate 1 C7/C8 are resolved (C8 through Gate 2 C48). No open or Engineering-targeted deferral remains.
- Gate 1 technical choices resolved at Gate 2: PASS. They are not applicable to this revision beyond the C48 import.
- Gate 2 C48 obligations designed: PASS.
  - Every record offered to a sink gets a field (`Decision.timestamp`, `Summary.timestamp`, :110, :129).
  - Format is UTC RFC 3339 (`rfc3339`, :174-190).
  - The time comes from the injected clock: `App.clock: Arc<dyn Clock>`, confirmed at src/lib.rs:55-57, borrowed as `&dyn Clock`.
  - The clock is read when each record is built: decision after `next.run` (:523); periodic and final summary in `close_window` (:548-550).
  - Stdout omits the field: both `tracing::info!` calls name their fields, confirmed in the current src/http/logging.rs `decide` and `emit`, and TP-13 is unchanged.
  - No new dependency: `jiff = "0.2.37"` is at Cargo.toml:52, and `SystemClock` uses it at src/clock.rs:22.
  - docs/operations.md is named in Files (:50).
  - The formatter, type and plumbing that Gate 2 left to this Gate are all selected, each with owner, signature and effects.
- Gate 3 files, interfaces, flow: PASS.
  - Signatures are given for `rfc3339`, `summarise`, `flush_summary` and `close_window`, and they match the current code shapes (src/http/logging.rs:349, :376, :404).
  - Every changed operation is connected in the call stack (:756-761).
  - The edit surface matches evidence/impact-c49.md's seven sites, including the test-only `Summary` literal at src/delivery/file.rs:236.
  - Existing TP-1 compares only `DECISION_FIELDS` from the record into stdout (tests/decision_log_delivery.rs:92-100), so the extra `timestamp` key does not break it.
- Gate 3 tests: PASS. TP-20 has a name, a seam and an assertion.
  - Seam: `TestServer::start_with(Config, Arc<dyn Clock>)` at tests/common/mod.rs:146, `TestClock::at_rfc3339` (:71, parsed by jiff, fractional seconds accepted), `advance_seconds` (:79) and `shared` (:95).
  - Assertion: exact strings for the decision record and the last summary record, and the collector's timestamp equal to the file's.
  - It distinguishes: a `SystemClock`/`Timestamp::now()` read at either site, a summary that copies the decision's time, and trimmed `Display` output (`.12Z`).
  - It fails on today's code, which has no `timestamp` key.
- Invariant dispositions: PASS. The injected-`Clock` invariant (:856) names the correct read sites, and its residual is stated with an owner, code review.
- Threat model: PASS. C49's effect on A2 (a peer IP paired with a time) is assessed under T-PEER-IP-RETENTION (:870).
- Red Team: PASS. The document records why none was triggered for C49 (:870): it adds no entry point, trust boundary, destination or input. Gate 2 records the same for C48.
- No contradiction with approved Gates 1-2: PASS. Gate 2's Fit row says `summarise()` and the drain flush "receive the same `Arc<dyn Clock>` from their caller". Borrowing it as `&dyn Clock` satisfies that.
- Gate 4 dimensions: not applicable.
## Limitations
- docs/plans/decision-log-delivery/00-status.md was read for context but could not be tracked: the helper refused it as machine state. It is bookkeeping, not a decision source.
- Code shapes were checked through SMTC `file read`/`file grep` (Manual tier). No compile or `verify` run was made, because the design's new signatures are not implemented yet, so the claim that the expression compiles rests on evidence/research-jiff-format.md's executed scratch crate.
- The completeness of the seven-site edit surface rests on evidence/impact-c49.md. Its struct-literal sites are Manual tier, because the graph has no struct-literal edges. I did not reproduce it.
