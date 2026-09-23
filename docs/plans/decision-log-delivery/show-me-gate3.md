## What problem do we have?

The Package Firewall MVP ships with logging that only reaches STDOUT and deliberately excludes
anything client-supplied (`src/http/logging.rs:14`: "Nothing a client sends reaches a log line").
A registry firewall can block further fetches but cannot reach a package already installed on a
machine — logs are the only route from "we blocked it" to "here is who needs cleaning up," so
Gate 1/2 approved optional delivery of decision records to a local file and/or a SIEM collector,
with consumer identification (peer IP) as an explicit, off-by-default opt-in.

## How will we solve it?

No new programming entry point and no new HTTP route: the only supported entry point remains the
operator's configuration file, read by the existing `Config::load`.

**New module `src/delivery`** — the one place that knows a decision record exists as a value and
where copies go. Owns `Record`, `Decision`, `Summary`, `Sinks`, `Drops`, bounded queues (capacity
4096), per-sink drop counters, and the spawned sink tasks. `pub(crate)` only; callers are
`src/http/logging.rs` and `src/lib.rs`.

| Interface | Promise |
|---|---|
| `delivery::build(config, drain) -> Result<(Sinks, Vec<JoinHandle<()>>), StartupError>` | Builds enabled sinks from validated config; empty config → empty `Sinks`, nothing spawned. Rejects a bad SIEM client/credential as `StartupError::Delivery` at startup, never after serving begins, and **the error text carries the variable's name and a fixed reason, never the value**. |
| `Sinks::offer(record: Record)` | Non-`async`, no lock, no I/O, no error return — one `try_send` per enabled sink; a full/closed queue increments that sink's drop counter and returns. This is what makes "delivery never slows a request" a type-level fact, not a review item. |
| `Sinks::drops() -> Drops` | Read-and-reset per-sink drop counts, called only from the summary path. |
| `Sinks::is_empty() -> bool` | True when nothing is configured. |
| `file::run(path, max_bytes, rx, drain, drops)` | Appends NDJSON, one syscall per record; rolls over to `<path>.1` at the size cap (disk bounded at 2x cap); retries forever on I/O error via reopen, never propagates to the request path. |
| `siem::run(client, url, auth, rx, drain, drops)` | Batches up to 256 records or 2s, POSTs as `application/x-ndjson`; retries `5xx`/`429`/transport errors 3x (100ms/500ms/2s backoff); drops+counts the whole batch on any other `4xx`; redirects disabled. |

**Changed:** `src/http/logging.rs` builds the record before rendering the unchanged stdout line,
offers it to `Sinks`, and gains `flush_summary`/`flush_drop_tail` for shutdown. `src/config.rs`
gains five optional keys, each absent-means-off, with cross-key and URL-scheme rejections.
`src/lib.rs` builds the sink set, holds it on `App`, creates the drain token, and enables
connect-info only when the opt-in is on. `src/tasks/mod.rs` gains one parameter so delivery's
handles join the existing `Tasks::join`.

**Decisions worth overruling (C13–C21):**

- **C13** — Per-sink drop counts live on their own `Arc<AtomicU64>`, not as fields on the existing
  `Counters` mutex Gate 2 named. Refines Gate 2's mechanism (keeps its constraint — nothing new on
  the request path beyond arithmetic) so a sink task never reaches for a sync mutex the request
  path holds every request; `struct Counters` stays unchanged, shrinking the change surface.
- **C16** — File sink uses `tokio::fs` + `AsyncWriteExt`, not `std::fs` as Gate 2's Fit row named,
  because `std::fs` blocks the async runtime thread it runs on. `tokio`'s `fs` feature is already
  enabled — no new dependency.
- **C19** — reqwest's async client has **no default timeout** (confirmed against current reqwest
  docs), so an untimed POST to a collector that accepts the connection and never answers would
  hang shutdown forever. Two independent bounds: client `.timeout(3s)` + `.connect_timeout(1s)`,
  plus each sink's drain loop wrapped in `tokio::time::timeout(5s, ..)` — the deadline is enforced
  by the caller, not trusted to the callee.
- **C14** — The `Decision` record is always built, even with no sink enabled, to keep one
  rendering path instead of two that could drift. Costs two small allocations per request on the
  default (unchanged) path.
- **C15** — Peer address is read from `request.extensions()` before `next.run(request)` consumes
  the request, in the same block that already resolves `Target::of`.
- **C18** — `delivery::build` spawns its own tasks and returns their `JoinHandle`s; `tasks::spawn`
  gains a fourth parameter to adopt them into the existing `Tasks.handles`/`Tasks::join`. No new
  hook point; receivers never leave `delivery`.
- **C20** — `flush_summary` reads and resets drop counters *before* the drain starts, so anything
  dropped during the drain window would otherwise be counted by nobody. `flush_drop_tail`, called
  after `tasks.join()` and before `App` drops, reads the counters one last time and warns if
  either is non-zero.
- **C17** — `ConnectInfo`/`into_make_service_with_connect_info` availability under axum's default
  features is asserted by policy, verified only by `cargo clippy --all-targets -- -D warnings`
  failing at compile time if unavailable (TP-12) — no design alternative held open.

**One accepted risk rather than mitigated (C21), owner: product.** A non-retryable `4xx` drops the
*whole* 256-record batch, not just the offending record — one rejected record costs up to 255 good
ones beside it, and a recurring record shape (e.g. an oversized field, or a collector that answers
a generic `400` to what is really throttling — a documented behaviour of both Splunk HEC and
Elastic bulk) repeats that loss on every batch containing it. Isolating the bad record instead
would mean either a burst of up to 256 individual POSTs at an already-failing collector, or a
bisection ladder — both judged to make a bad moment worse. The drop is counted, and an operator
needing completeness is told to use the file sink instead, which has no equivalent failure mode.
This is the entry most likely to deserve a different answer.

## How will we confirm it is solved?

| Scenario | Expected result | Check |
|---|---|---|
| Decision line delivered to file sink | One NDJSON line per request, fields matching stdout's | TP-1 (execution) |
| File crosses size cap | `<path>.1` exists, both files bounded at 2x cap | TP-2 (execution) |
| Default config (all five keys absent) | `Sinks::is_empty()` true, no file, no task spawned | TP-3 (execution) |
| Consumer identification opt-in off/on | Off: no `consumer` field. On: peer IP, no port | TP-4 (execution) |
| The three C11 invalid combinations | Each rejected by exact key name | TP-5 (execution) |
| `siem_url` scheme rule (https / loopback exceptions) | First three accepted, plain-http-to-remote-host rejected | TP-6 (execution) |
| Drop counted and durably surfaced | Summary record **read back from the file sink's own NDJSON**, not stdout, shows `dropped_siem` > 0 | TP-7 (execution) |
| SIEM batch shape + optional credential | NDJSON body, correct content-type, auth header present/absent as configured | TP-8 (execution) |
| Redirects not followed | Second server receives nothing; batch counted dropped | TP-9 (execution) |
| In-flight request survives shutdown | Its record is in the file after `shutdown()` returns | TP-10 (execution) |
| Credential never leaks — runtime failure path | Sentinel absent from file and stdout after forced SIEM failure | TP-11a (execution) |
| Credential never leaks — startup rejection path | Error text names the variable and reason, contains no part of the sentinel (the path a naive `format!` would leak) | TP-11b (execution) |
| Lint/typecheck | `cargo clippy --all-targets -- -D warnings` clean; also settles C17 | TP-12 (typecheck) |
| Shutdown bounded even when a sink is wedged | Returns within ~5s against a listener that never responds; final tail drops reported via `tracing::warn!` | TP-15 (execution) |
| No client-supplied value reaches a delivered record | Retained taint-query spec `.smtc/analyzers/client-data-reaches-delivery-sink.yaml` — product root: zero findings **with `find_sinks` > 0**; unsafe control: fires at `offer` call site | TP-14 (retained analyzer spec) |
| Stdout decision line does not drift | New assertion parses the emitted JSON and compares the **complete key set** (12 fields off, 13 with `consumer`) — the existing `assert_decided` only checks 3 substrings and cannot catch a renamed field on its own | TP-13 (execution) |

Recommendation: Approve — the design is internally consistent, every promise has a check that
could actually fail, the two refinements of Gate 2 mechanisms (C13, C16) keep Gate 2's stated
constraints rather than override them, and the one deliberately accepted risk (C21) is stated
plainly with a named owner and an alternative already available to operators who need it (the file
sink). C19's shutdown-hang fix is grounded in current reqwest documentation, not assumption.

Limits: Two temporary limits are real, not implementation slop to fix later.
1. The retained analyzer spec (TP-14 / T-INVARIANT-FIRST-BREACH) passes **vacuously** against the
   product root today — `find_sinks` matches 0 because `delivery::Sinks::offer` doesn't exist yet.
   A zero only becomes evidence once `src/delivery` lands; the slice that creates it is the first
   point TP-14 can mean anything.
2. The final drop-count tail at shutdown (`flush_drop_tail`, C20) reaches the console only — by
   the time it runs, every sink task has returned, so a SIEM-only operator whose SIEM is down
   learns of those last drops from stdout or not at all. Same residual Gate 2 C12 already recorded
   for the ordinary drop path, now also true of the shutdown tail.

Also flagged in the Gate's own `## Least confident decisions` as open to being overruled, beyond
C13/C16/C21 above: always building the record even unused (C14); the duplicated `axum::serve`
branch for connect-info vs. one path with gated recording; no `BufWriter` on the file sink (one
syscall per record); one rollover generation only (`<path>.1`, SIEM is the retention answer); the
final drop tail being console-only (C20, above); and the specific numbers chosen for C19 — 3s per
SIEM attempt, 5s drain — where a collector that habitually takes 4s to answer would see every
attempt time out and look like a rejecting collector rather than a slow one.

Gate 3 QA verdict: READY (fresh review, both upstream Gates approved 2026-09-21, all 18 subject
files hash-verified against the receipt, including this Gate document, both upstream Gates, the
grounding evidence, and the retained analyzer spec). READY permits presentation only; it does not
constitute approval.

Sources: `docs/plans/decision-log-delivery/03-program-design.md` (`## Clarifications and
decisions`, `## Modules and interfaces`, `## Test plan`, `## Threat model`, `## Least confident
decisions`); `docs/plans/decision-log-delivery/evidence/repository-structure.md`;
`.smtc/analyzers/client-data-reaches-delivery-sink.yaml`; `docs/plans/decision-log-delivery/gate-3-qa.md`.


Approve Gate 3, or what should change?
