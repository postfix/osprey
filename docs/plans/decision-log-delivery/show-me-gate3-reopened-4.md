## What problem do we have?

Gate 1 promises operators "what was served and when". Slice 3 verification found that delivered
records say what was served, but not when: no file or SIEM record has a time of decision
(P-WHEN failed). An operator reading the log file or the collector cannot tell when a package was
served, or when a summary window closed, except from the order records arrived in. Gate 2 C48
(re-approved 2026-09-22) decided the fix: every file and SIEM record carries a UTC RFC 3339
`timestamp`, read from the injected clock when the record is built. The console line stays as it
is. Gate 3 must now choose the type, the formatter, and how the summary path reaches the clock.

## How will we solve it?

C49 adds one field, one helper, and one clock parameter along the summary path.

Affected-API map (these seven edit sites are the whole edit surface, per `evidence/impact-c49.md`):

| Module / operation / type | Change | Caller impact / visibility |
|---|---|---|
| `delivery::Decision` | new first field `timestamp: String` | `pub(crate)`; one literal, in `decide` |
| `delivery::Summary` | new first field `timestamp: String` | `pub(crate)`; literal in `close_window`, plus the test-only literal in TP-19 (`src/delivery/file.rs:236`), a mechanical edit |
| `delivery::rfc3339` | **new** `pub(crate) fn rfc3339(utc_micros: i64) -> String` | callers: `decide`, `close_window` only |
| `http::logging::decide` | body only: stamps the decision, passes the clock to `summarise` | signature unchanged |
| `http::logging::summarise` | gains final `clock: &dyn Clock`; forwards it unread | private, one caller (`decide`) |
| `http::logging::flush_summary` | gains final `clock: &dyn Clock`; forwards it unread | `pub(crate)`, one caller (`Running::shutdown`) |
| `http::logging::close_window` | gains `clock: &dyn Clock`; the **only** place a summary reads the time | private; used by `summarise` and `flush_summary` |
| stdout `tracing::info!` (decision, summary) | unchanged; `timestamp` is not named, so it does not appear | TP-13 key set unchanged |

No new dependency: `jiff = "0.2.37"` is already present (`Cargo.toml:52`). No public Rust API.

Deciding declarations:

```rust
pub(crate) fn rfc3339(utc_micros: i64) -> String {
    jiff::Timestamp::from_microsecond(utc_micros)
        .map(|t| format!("{t:.6}"))
        .unwrap_or_else(|_| utc_micros.to_string())
}

fn summarise(elapsed: Duration, bytes: u64, was_error: bool, sinks: &Sinks, clock: &dyn Clock)
pub(crate) fn flush_summary(sinks: &Sinks, clock: &dyn Clock)
fn close_window(counters: &mut Counters, open_for: Duration, sinks: &Sinks, clock: &dyn Clock) -> Summary
```

- **Value:** always six fractional digits and a `Z`, e.g. `2026-09-21T14:13:20.120000Z`. Plain
  `Display` would trim trailing zeros (`...:20.12Z`, `...:20Z`). A fixed width sorts as text and is
  easier for a collector to parse (measured in `evidence/research-jiff-format.md`).
- **Errors:** none reach the caller. `from_microsecond` fails only outside years -9999..9999,
  which the production clock cannot reach. In that case the output is the decimal microsecond
  count, the same fallback as `src/main.rs:109-111`.
- **Type:** a pre-rendered `String`, not a `jiff::Timestamp`. Serialising a `jiff::Timestamp`
  directly would need jiff's `serde` feature, which is not enabled. The cost is one allocation
  per record.
- **When the clock is read:** in `decide`, after `next.run(request)` returns (step 2), so the
  stamp is the time the decision completed. For summaries, inside `close_window`, which both the
  periodic and the shutdown summary go through.

Caller examples:

```rust
// decide, step 2 and step 5
timestamp: delivery::rfc3339(app.clock.now_utc_micros()),
summarise(elapsed, bytes, error.is_some(), &app.delivery, app.clock.as_ref());

// Running::shutdown
http::logging::flush_summary(&self.app.delivery, self.app.clock.as_ref());
```

Connections: `decide -> app.clock.now_utc_micros() -> rfc3339`;
`summarise / flush_summary -> close_window -> clock.now_utc_micros() -> rfc3339`. Both start from the
`Arc<dyn Clock>` already on `App`, borrowed as `&dyn Clock`.

## How will we confirm it is solved?

| Scenario | Expected result | Check (planned) |
|---|---|---|
| `TestClock` pinned at `2026-09-21T14:13:20.120000Z`; file sink plus a wiremock SIEM answering 200; one request; clock advanced 60 s; `shutdown()` | file `request_decided` has `timestamp == "2026-09-21T14:13:20.120000Z"`; the **last** file `request_summary` has `"2026-09-21T14:14:20.120000Z"`; the SIEM copy of the decision has the same timestamp as the file's | `TP-20`, execution. It fails today because no record has `timestamp` |
| The stdout decision line | key set unchanged, no `timestamp` | `TP-13`, unchanged |
| Struct and arity edits | compiles with no missing-field errors | `cargo clippy --all-targets -- -D warnings` (`TP-12`) |

TP-20 fails if either read site uses `SystemClock` or `Timestamp::now()`, if the summary copies
the decision's time, or if the fraction is trimmed. It checks the last summary record because an
earlier periodic summary may appear. All of these are planned checks. None has run yet.

Recommendation: approve C49. It meets every obligation in Gate 2 C48 with one small helper and a
borrowed clock parameter, adds no dependency, and leaves stdout unchanged. Because every summary
reads the clock in one place, one execution check covers both the periodic and the shutdown summary.

Limits:
- One residual has no execution witness: `summarise` could forward some other clock (for example
  `&SystemClock`) instead of the one it receives. Code review checks this. A test cannot reach it,
  because the periodic window runs on a real `Instant` with a 60 s default, and its only setter is
  process-global.
- TP-20 does not prove the decision clock is read after `next.run` (QA note O-1). The design
  requires that ordering, but the test does not claim it.
- Red Team was not triggered. C49 adds no entry point, trust boundary, destination or input. With
  consumer identification on, A2 now pairs a peer IP with a time. That is already accepted under
  T-PEER-IP-RETENTION (Gate 1 C1), and the operator documentation gains the field.
- Grounding: the claim that the helper compiles rests on the executed scratch crate in the jiff
  evidence. That the seven sites are the whole edit surface rests on the impact evidence, whose
  struct-literal sites are Manual tier.

Status: Gate 3 (Program Design) reopened a fourth time for Gate 1's P-WHEN promise. Gates 1 and 2
re-approved 2026-09-22. Gate QA is fresh and READY (16 files). The prior round was REVISE with five
findings, all closed. Next: Gate 4 adds slice 4 to deliver this.

Sources: `03-program-design.md` (C49; `Decision`/`Summary`; `rfc3339`; `decide` steps 2/3/5;
`summarise`; `flush_summary`; `Running::shutdown`; Call stack rows for the clock; TP-20; Clock
invariant; Threat model C49 sentence; Least confident 12) - `gate-3-qa.md` - `evidence/impact-c49.md` -
`evidence/research-jiff-format.md` - `evidence/slice-3.md` - `02-architecture.md` C48

Approve Gate 3, or what should change?
