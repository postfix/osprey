# Decision log delivery — Gate 2: Architecture

## What problem do we have?

Gate 1 already decided the firewall must be able to deliver its decision records to a file and to
a SIEM, optionally, without ever slowing a request or serving unlogged. Gate 1 also decided that
recording *which consumer* asked (peer IP) is an opt-in, off by default. Three questions Gate 1
deliberately left open — wire format and transport to the SIEM (C4), how the local file behaves
over time (C5), and how a SIEM credential reaches the process without living in the config file
(C6) — are architecture choices, not product choices, and this Gate answers them. Gate 2 also owns
six further decisions the code needs settled before anyone writes it: what "consumer" actually
means (C7), one pipeline or two (C8), whether the existing stdout line changes (C9), what happens
to in-flight records at shutdown (C10), which key combinations are nonsensical or harmful enough
to reject at startup (C11), and how the drop counts Gate 1 required actually reach the operator
(C12).

## How will we solve it?

### The nine decisions

| ID | Owning Gate | Question | Answer |
|---|---|---|---|
| C4 (imported Gate 1 deferral) | 1 | SIEM wire format and transport | HTTP `POST` of newline-delimited JSON to an operator-set `siem_url`, absent means off. Built on the already-present `reqwest 0.13.5` over rustls. `https://` required; `http://` accepted only at a loopback host. The client is built with `redirect::Policy::none()` — without that, a `3xx` from the collector could send decision records, peer IPs included when C7 is on, to an unvalidated destination. |
| C5 (imported Gate 1 deferral) | 1 | Local file behaviour over time | Operator sets `log_file_path`, absent means off. Newline-delimited JSON appended. At `log_file_max_bytes` (default 100 MiB) the file is renamed to `<path>.1`, replacing any previous `.1`, and a fresh file opened. Disk use bounded at twice the cap; no timer, no retention schedule. |
| C6 (imported Gate 1 deferral) | 1 | SIEM credential without it living in the config file | Read from environment variable `OSPREY_SIEM_AUTH`, sent as the header named by `siem_auth_header` (default `Authorization`). The config file holds only the header name, never a value. Unset variable means the sink runs unauthenticated, with one startup line stating whether it is authenticated. The value is never written to a record, log line, or error. |
| C7 | 2 | What identifies a consumer, when C1 is on | The peer IP of the accepted TCP connection — nothing else, no port, no header, nothing client-supplied. `X-Forwarded-For` and `User-Agent` were rejected: reversing "nothing a client sends reaches a log line" for a spoofable value would produce a clean-up list nobody can trust. |
| C8 | 2 | One delivery pipeline or one per destination | One each. Each enabled sink owns its own bounded queue, background task, and drop counter, so a SIEM retry cannot stall a local file write and an operator can see which sink is losing records. Costs one extra record clone per request when both sinks are on. |
| C9 | 2 | Does the existing stdout line change | It becomes one record rendered identically to stdout, file, and SIEM. With consumer identification off, the stdout field set is exactly today's twelve fields — no visible change. With it on, the consumer field appears on all three. |
| C10 | 2 | What happens to in-flight/queued records at shutdown | A second, separate cancellation token, watched only by sink tasks and cancelled only after the HTTP server join returns — not the main `shutdown` token. Once it fires, no further records can be produced; each sink drains until empty or a five-second deadline, flushes its tail, and exits. |
| C11 | 2 | Is any key combination silently meaningless or harmful | Three combinations rejected at startup, naming the offending key: `log_consumer_identification = true` with no sink configured; `log_file_max_bytes` with no `log_file_path`; `siem_auth_header` with no `siem_url`. The first matters most — without it, an operator could start writing peer IPs to the console with no stated destination. |
| C12 | 2 | How drop counts actually get surfaced | The periodic summary becomes a record like any other and is offered to every enabled sink. A final summary is flushed once at drain, before sinks finish draining, so counts accumulated after the last periodic summary still reach a destination. |

### Zero new Cargo dependencies

The whole feature adds no new crate. Local file writing and background-channel mechanics use
`std::fs`, `serde_json` (already direct), `tokio::sync::mpsc` (already available via tokio's
`sync` feature), and `tokio_util::sync::CancellationToken` (already a direct dependency, and
already the shutdown-coordination primitive in this codebase). Outbound HTTP delivery reuses the
already-direct, rustls-backed `reqwest 0.13.5` stack. `tracing-appender` — the crate that would
normally do rotating file writes on the `tracing` writer path — is absent from the tree and was
rejected: this design deliberately does not put delivery on the tracing writer path at all: it
builds record values in `src/http/logging.rs` and hands copies to independent sinks, rather than
adding a `tracing` `Layer`.

### The five new configuration keys

All five keys are optional; absence means the corresponding behaviour is off. `#[serde(deny_unknown_fields)]`
on `RawConfig` means each key must be registered in three places — `RawConfig` field, `Config`
field, and the name in `OPTIONAL_KEYS` — or it is rejected outright. Three combinations are
rejected at startup as `ConfigError::Invalid`, naming the offending key (this is C11, restated):

- `log_consumer_identification = true` with no sink (`log_file_path` or `siem_url`) configured.
- `log_file_max_bytes` set with no `log_file_path`.
- `siem_auth_header` set with no `siem_url`.

### Consumer identification

What it records: the peer IP address of the accepted TCP connection — nothing the client sends,
never a header, never a port. What it unlocks: with it on, every record additionally carries that
peer IP, making "which machines already took the blocked package" answerable by one SIEM query.
Two stated costs: behind a proxy, NAT, or load balancer the recorded value is that hop, not the
machine that ran the install; and turning the setting off later does not erase what was already
recorded — peer IPs already written to the log file, its `.1` rollover, or shipped to the SIEM stay
there, with no purge path in scope.

### The shutdown decision (C10)

The existing `shutdown` token is cancelled before in-flight requests finish, and a request may
legitimately run for the full `RESPONSE_LIFETIME` of 15 minutes. A sink watching that token would
exit, and its five-second drain deadline would expire, long before those requests emit their
records — losing every slow request active at shutdown, which is exactly the audit-completeness
case this feature exists to serve. Waiting for sender closure instead is not available either:
`drop(self.app)` happens *after* the existing task-join step, so that wait would deadlock. A
second, separate drain token, cancelled only after the HTTP server join returns, is the design
that avoids both failure modes.

### Accepted limits

- **A record whose delivery is dropped is gone.** No spool, no disk buffer, no replay — a direct
  consequence of the Gate 1 decision to drop rather than slow or fail a request.
- **Drop counts cannot be reported through a channel that is down.** An operator running the SIEM
  sink alone, whose SIEM is unreachable, learns about the resulting drops only from the console;
  the file sink is the answer for anyone who needs the stronger guarantee.
- **The file sink assumes it is the only writer of its path.** No interprocess lock: two instances
  pointed at one file, or an external `logrotate` renaming the file out from under the process,
  will interleave or lose records, and a crash can leave a partially written final line.

### The unchanged default

An operator who sets none of the five keys sees no change at all: no file is opened, no connection
is made, and the console output is unchanged — this is the default.

## How will we confirm it is solved?

| Scenario | Expected result | Check |
|---|---|---|
| Operator sets neither `log_file_path` nor `siem_url` | No file opened, no client constructed, sender set empty, stdout unchanged | Planned — `## Program behavior` Capabilities, `## Flow` step 1 |
| Operator sets `log_file_path` only | Every decision record appended as one JSON line; file capped at twice `log_file_max_bytes` | Planned — `## Flow` step 3 |
| Operator sets `siem_url` only | Records batched (256 or 2s) and POSTed as NDJSON; retried on transport error/5xx/429, dropped on other 4xx | Planned — `## Flow` step 4 |
| `log_consumer_identification = true` with no sink configured | Startup fails with `ConfigError::Invalid` naming the key | Planned — C11, `## Program behavior` Failures |
| Shutdown while a 15-minute request is still in flight | Drain token cancelled only after server join; record still reaches its sink if produced before the drain deadline | Planned — C10, `## Flow` step 5, `## Constraints` |
| Both sinks enabled, one destination unreachable | The unreachable sink drops and counts independently; the other sink is unaffected | Planned — C8, `## Flow` |

Recommendation: none — this presentation does not decide or recommend; the reader (the operator-owner who approved Gate 1) approves or redirects.
Limits: none beyond those stated above as accepted limits; all nine clarification rows carry a selected value, none is deferred past this Gate.
Status: Gate 2 architecture, currently in progress in the router; Gate 1 (product) approved 2026-09-21; Gate 2 passed a fresh `sf-gate-qa` review with verdict READY (presentation only, not approval) after `sf-red-team` returned five findings against an earlier draft, all five confirmed resolved in the document text under review.
Sources: `docs/plans/decision-log-delivery/02-architecture.md`, `docs/plans/decision-log-delivery/01-product.md`, `docs/plans/decision-log-delivery/evidence/repository-structure.md`


Approve Gate 2, or what should change?
