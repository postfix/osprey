## What problem do we have?

Slice 3 (opt-in consumer identification) shipped and passed its own tests, but its independent
Security review returned `FIX FIRST` on three findings the pre-implementation Gate 3 design never
modelled: the decision log file is created world-readable (`0o644`) and now holds peer IP
addresses; the `log_consumer_identification` opt-in's startup check only confirms a sink *key* is
present, not that a destination actually works, so a log-path typo starts cleanly and silently
collects-then-drops every peer IP; and the retained "no client data reaches a log line" analyzer
spec cannot see `request.extensions()`, so its earlier zero-findings proof covered less than
claimed. A fourth gap — the console/journal stream is a durable, third-party-readable destination
under systemd/Docker/Kubernetes, not the "non-durable" thing this plan called it in three places —
was found by Red Team while re-reviewing the design itself. Per the playbook, a security surface
Gate 3 did not model reopens Gates 3 and 4; product source writes stopped and slice 3 is unchecked
pending re-implementation against the corrected design below.

## How will we solve it?

**C27 — log file mode.** Every create-capable open sets `.mode(0o600)`: in `Writer::open`
(`src/delivery/file.rs`) via `tokio::fs::OpenOptions`'s own inherent `.mode` on Unix, and in the
startup probe in `build` (C36) via `std::fs::OpenOptions` with
`std::os::unix::fs::OpenOptionsExt`. Applies only on creation, Unix-only, no `cfg` guard (product
targets Linux only). Documentation-only was rejected — the unsafe default is silent.

**C37 — pre-existing permissive file.** `.mode()` cannot fix a file created by an earlier
deployment (slices 1–2 already shipped at `0644`) or one a local user pre-created/symlinked. After
any open, `file::run` emits one `tracing::warn!` when `mode & 0o077 != 0`, naming the path and
mode. It does not re-chmod a file it did not create.

**C28/C36 — startup probe, tally clause struck.** `delivery::build`'s approved contract said it
opens the log file and reads its length into the byte tally, but the implementation deferred that
to `Writer::open` on the first record — so an unopenable path started cleanly and collected peer
IPs while dropping every record. C28 restores a startup-time open; C36 corrects its shape after
Red Team found the "byte tally" clause conflicts with `Writer::open`'s existing length-restoring
behaviour: the startup action is a **probe only** —
`std::fs::OpenOptions::new().create(true).append(true).mode(0o600)`, handle dropped immediately,
`std::fs` (not `tokio::fs`, since `build` is synchronous and pre-runtime; C16's async-blocking
concern binds only the sink task). Its two jobs are failing startup and creating the file at
`0o600`. `Writer::open` keeps its own length-restoring behaviour unchanged.

**C39 — probe failure fatal only when consumer identification is on.** A privacy-justified fatal
startup failure applies to everyone if left as C28 first stated it, turning a logging typo into an
outage for a package firewall whose unavailability blocks CI and builds. Narrowed: fatal only
while `log_consumer_identification` is on (`App::start` returns `StartupError::Delivery`, nothing
serves). With the opt-in off, probe failure logs one `tracing::error!` naming the key and reason,
and the process continues — the file sink then behaves exactly as it does today (drops counted,
`Writer::open` retried per record). This also corrects Gate 2, which said an unopenable log file
leaves the firewall serving with no condition, and rested that on the console being
"non-durable" — refuted by C35. Both are reconciled as Gate 2 C41, re-approved 2026-09-21 before
this Gate.

**C40 (residual, accepted).** `check_config` writes nothing and cannot run the startup probe, so
it can report a configuration valid that then fails at boot once the opt-in is on. A read-only
probe would not close the gap either (a path openable at check time can be unwritable at boot).

**C29/C38 — extension-read guard.** `request.extensions()` is an inbound-request accessor the
retained `client-data-reaches-delivery-sink.yaml` spec's call-name list cannot see, so `TP-14`'s
earlier zero proved less than claimed. A second retained spec,
`.smtc/analyzers/consumer-identity-extension-read.yaml` (`structural_pattern`, names
`["extensions", "extensions_mut"]`, receiver `request|req|parts`), has a pass condition of
**exactly one finding, at `src/http/logging.rs`** rather than zero — adding `extensions` to the
sibling spec was rejected, as it would fire on the one approved read. C38 adds a fourth behavioural
guard the spec structurally cannot give: same-file substitution (delete the approved read, add a
different inbound-extension read) leaves the count, filename and `.ok` unchanged. `TP-4` now sends
`X-Forwarded-For` and `Forwarded:` headers and still asserts `consumer == 127.0.0.2`, testing the
property rather than the call site's shape — the first test in this repository ever to send a
forwarding header. Field access (`parts.extensions`) stays invisible to both specs by construction
(call-site inventories only) — named, not fixed.

**C35 — console/journal is a fourth destination.** Red Team: `consumer` renders to stdout as well
as into the record, and stdout is journald's persistent journal under systemd, or a
cluster-agent-shipped node file under Docker/Kubernetes — durable and third-party-readable, never
inventoried. `T-CONSOLE-JOURNAL-DESTINATION` is added; `T-PEER-IP-RETENTION` is amended to four
destinations; operator documentation's purge bullet names all four; and the
`log_consumer_identification` reason string at `src/config.rs:295` stops claiming the rejection
gives addresses "a destination" (the console already is one) — it now reads it buys a *second,
operator-chosen* one. No technical control is proposed over the journal; its permissions and
retention belong to the host.

## How will we confirm it is solved?

| Scenario | Expected result | Check |
|---|---|---|
| Log path unopenable, opt-in **on** | `App::start` returns `StartupError::Delivery`; nothing serves; text names the key and reason, no peer address in it. Fails today (starts cleanly, collects peer IPs, drops every record) | `TP-16`(a); planned, execution |
| Log path unopenable, opt-in **off** | `App::start` succeeds, request served normally, exactly one `tracing::error!` names the key | `TP-16`(b); planned, execution |
| Fresh run, one request; forced rollover | Live file and `.1` generation both mode `0o600` | `TP-17`(a)/(b); planned, execution |
| File pre-created `0o644` before app starts, one request | Mode stays `0o644` (not re-chmod'd); exactly one `tracing::warn!` names path and mode | `TP-17`(c); planned, execution — the case that matters for every slice-1/2 deployment already running |
| `App::start` returns, no request sent, no shutdown | File exists at mode `0o600` — the only case witnessing the startup probe's create verb rather than `Writer::open`'s lazy create | `TP-17`(d); planned, execution |
| Opt-in on/off, client at distinct loopback source, sends `X-Forwarded-For`/`Forwarded:` | Off: no `consumer` field. On: `consumer == 127.0.0.2` (peer, not the claimed forwarding-header value); neither header value appears anywhere in record or stdout | `TP-4`; planned, execution |
| `smtc spec run` against `consumer-identity-extension-read.yaml` | Exit 0; `.ok == true`; exactly one finding; finding's file ends `/src/http/logging.rs` | `TP-18`; already re-run live by Gate QA — 1 finding at `src/http/logging.rs:154`, matching exactly |
| `smtc spec run` against `client-data-reaches-delivery-sink.yaml` | Zero findings in product root | Already re-run live by Gate QA — 0 findings, confirmed |

Recommendation: **Approve** — Gate QA returned READY on a fresh, independent review that
re-examined all nine findings from prior rounds (R1–R9) including the one residual (R8, the
`.mode` API attribution across `tokio::fs`/`std::fs`, independently confirmed against live
`tokio` 1.49 docs) and found no new defect. Every clarification (C27–C31, C35–C40) is resolved
with a named decision source; the threat model's four new rows each map to a resolved
clarification and a named check; both retained analyzer specs were re-run live against the
current tree with the exact stated results. Gate 2's own divergence this reopening exposed (the
"non-durable console" premise, and the unopenable-log-file fatality condition) is reconciled as
Gate 2 C41 and already re-approved 2026-09-21.

Limits: current source does not yet implement C27/C28/C35–C39 — this is expected, not a Gate
defect (confirmed at `src/config.rs:295` still carrying the pre-correction reason string, and
`Writer::open`/`build` not yet calling `.mode(0o600)` or running a startup probe); slice 3 stays
unchecked pending re-implementation once Gate 4 is re-approved. Three named residuals stay open by
design rather than by omission: a SIEM-only deployment with an unreachable or permanently-rejecting
collector reproduces the same collect-and-drop state the startup probe cannot see
(`T-SILENT-DESTINATIONLESS-COLLECTION`); a file sink can enter that same state *after* a successful
startup (volume unmounted, fills, or file deleted); and `check_config` cannot see either because it
writes nothing (C40). No technical control exists over the console/journal destination (C35). Gate
QA did not run `cargo test`/`cargo clippy` and did not judge whether shipped or future source fully
implements this revision — only that the design document itself is internally consistent and
grounded.

Status: slices 1 and 2 are done and unaffected; slice 3 is unchecked, awaiting re-implementation
against this revised design once Gate 4 is re-approved. Gate QA verdict: READY (fresh, independent
review; report `gate-3-qa.md`, 24 files tracked).

Sources: docs/plans/decision-log-delivery/03-program-design.md (C13–C21, C27–C31, C35–C40; Test
plan TP-4, TP-16, TP-17, TP-18; Threat model T-EXTENSION-ROUTE, T-LOGFILE-WORLD-READABLE,
T-SILENT-DESTINATIONLESS-COLLECTION, T-CONSOLE-JOURNAL-DESTINATION); 02-architecture.md (C41,
re-approved 2026-09-21); 01-product.md; 00-status.md ("Notes for a fresh session"); gate-3-qa.md
(READY verdict, R1–R9 re-examined); .smtc/analyzers/client-data-reaches-delivery-sink.yaml;
.smtc/analyzers/consumer-identity-extension-read.yaml; evidence/repository-structure.md.

Approve Gate 3, or what should change?
