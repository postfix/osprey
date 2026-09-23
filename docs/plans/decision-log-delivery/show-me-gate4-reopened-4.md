## What problem do we have?

Slice 3's feature-level verification failed P-WHEN: records delivered to the log file or the SIEM carry no time of decision, while Gate 1 promises "what was served and when" and "with timestamps" (`evidence/slice-3.md`). Gate 3 C49 designed the fix after slice 3 was proven, so Gate 4 needs a place in the build order for it.

## How will we solve it?

Add a new slice 4 that is only the timestamp (C50). Slices 1-3 are complete history and stay checked. Slice 3's proof line covers its own row, and this change invalidates none of it.

| Slice | Outcome | APIs used or changed | Witness | Temporary limit |
|---|---|---|---|---|
| 1 | File sink: NDJSON per decision, bounded at 2x `log_file_max_bytes`, one `.1` rollover | complete | complete | complete |
| 2 | SIEM sink: batched NDJSON `POST`, optional credential, no redirects | complete | complete | complete |
| 3 | Opt-in `consumer` peer IP, `0o600` file, startup probe | complete | complete | complete |
| **4** | Every decision and summary record reaching the file or SIEM has `timestamp`: UTC RFC 3339, six fractional digits, from the injected clock. Console line unchanged | **New** `delivery::rfc3339`; `timestamp: String` first field of `Decision` and `Summary`; `summarise`, `flush_summary`, `close_window` gain final `clock: &dyn Clock` | `tp20_delivered_records_carry_timestamp` + `tp13` unchanged; full `cargo test`; clippy `-D warnings` | none |

Slice 4 edit surface, as a diff against slice 3's file list. `src/config.rs` and `config.sample.toml` drop out because C49 adds no key:

```diff
  src/delivery/mod.rs          # + rfc3339, + timestamp on Decision and Summary
  src/delivery/file.rs         # tp19 Summary literal at :236 gains timestamp (test-only)
  src/http/logging.rs          # decide, summarise, flush_summary, close_window
  src/lib.rs                   # Running::shutdown passes the clock to flush_summary
  tests/decision_log_delivery.rs  # + tp20
  docs/operations.md           # timestamp format, meaning, console omission
- src/config.rs
- tests/config_validation.rs
- config.sample.toml
```

The shapes that change (`### Slice 4 interfaces`):

```rust
// src/delivery/mod.rs (new)
pub(crate) fn rfc3339(utc_micros: i64) -> String {
    jiff::Timestamp::from_microsecond(utc_micros).map(|t| format!("{t:.6}")).unwrap_or_else(|_| utc_micros.to_string())
}

// src/http/logging.rs (changed)
fn summarise(elapsed: Duration, bytes: u64, was_error: bool, sinks: &Sinks, clock: &dyn Clock)
pub(crate) fn flush_summary(sinks: &Sinks, clock: &dyn Clock)
fn close_window(counters: &mut Counters, open_for: Duration, sinks: &Sinks, clock: &dyn Clock) -> Summary
```

Clock reads, as a call tree:

```text
decide                       reads app.clock after next.run(request) -> Decision.timestamp
  summarise(.., app.clock.as_ref())
    close_window(.., clock)  only summary read -> Summary.timestamp
Running::shutdown
  flush_summary(&self.app.delivery, self.app.clock.as_ref())
    close_window(.., clock)
```

`summarise` and `flush_summary` forward the clock without reading it. `tracing::info!` and `emit` do not name `timestamp`. `decide`'s signature is unchanged.

## How will we confirm it is solved?

All planned. None of these has run for slice 4.

| Scenario | Expected result | Check |
|---|---|---|
| `TestClock::at_rfc3339("2026-09-21T14:13:20.120000Z")`, file + wiremock SIEM answering `200`, one request, `advance_seconds(60)`, shutdown | file `request_decided` `timestamp == "2026-09-21T14:13:20.120000Z"`; **last** `request_summary` `== "2026-09-21T14:14:20.120000Z"`; collector's `request_decided` has the same `timestamp` as the file's | step 1, `tp20` (TP-20). Red today: the test does not exist (C51) |
| stdout decision line | key set unchanged, no `timestamp` | `tp13`, unchanged |
| `tp19` after its `Summary` literal gains `timestamp` | every target passes, `0 failed` | step 2, `cargo test` |
| all changed code | exit 0, no warning lines | step 3, `cargo clippy --all-targets -- -D warnings` (TP-12) |
| `summarise`/`flush_summary` forward their own `clock` to `close_window`; `rfc3339` body is exactly the C49 expression; `file.rs:236` edit only adds the field | confirmed by review | `sf-code-review` |
| feature outcomes including P-WHEN; `docs/operations.md` timestamp text matches the code | confirmed | `sf-verification` (last slice again) |

Reviews: `code-review, qa`. `sf-security-review` is not named: Gate 3 recorded that C49 adds no entry point, trust boundary, destination or input. `sf-red-team: not triggered` for the same reason.

Recommendation: Approve. Slice 4 is one caller outcome, it depends only on slices 1-3 and existing code (`jiff 0.2.37`, `App.clock`, the `TestClock` helpers), it covers all seven sites in `evidence/impact-c49.md`, its witness is red today, and sf-gate-qa returned READY with no blocking findings.

Limits: The periodic summary's clock forwarding (`summarise` -> `close_window`) has no execution witness; code review is the check. Nothing executes a check that the stdout summary line (`emit`) omits `timestamp`: `TP-13` covers only the stdout decision key set, and the slice 4 code review does not name it explicitly. Gate 1's rated-load promise (P-GUARD-LOAD) still has no load test (inherited from earlier gates). QA could not read `00-status.md`, which C50 cites. QA evidence is Manual/Structural: nothing was built or run.

Status: Gate 4, fourth revision (2026-09-22), after Gate 1 reopened (C8) and Gates 2 (C48) and 3 (C49) were re-approved. sf-gate-qa READY (fresh). The prior blocking finding QA-G4-R4-1 (the `Reviews` column) and the minor QA-G4-R4-2 (build-order sentence) are closed.

Sources: `04-slices.md` (slice 4 row, `### Slice 4 interfaces`, C50, C51); `03-program-design.md` (C49, TP-20, `## Invariants & spec dispositions`); `gate-4-qa.md`; `evidence/impact-c49.md`; `evidence/research-jiff-format.md`; `evidence/slice-3.md`.

Approve Gate 4, or what should change?
