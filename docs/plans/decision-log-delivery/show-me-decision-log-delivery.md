# Gate 1 — Decision log delivery: is this the right problem?

Context: the Package Firewall MVP shipped at commit `0f50cec`. The user's very next request was "we need to add optional logging local to file and to SIEM." Nothing is built and nothing is designed yet. This gate asks only whether the problem is real, stated correctly, and bounded correctly — not how to build it.

## What problem do we have?

**Observed today** (confirmed by reading the shipped code): the firewall already decides, for every request, whether a package is allowed, held, blocked or refused — and already writes that decision down. `src/main.rs:58-63` sends that output through `tracing_subscriber::fmt().json()` to STDOUT and nowhere else. `src/http/logging.rs` emits one decision event per request (request_id, method, ecosystem, package, version, status, result, reason, blocklist_revision, cache, duration_micros, bytes) plus a periodic counter summary. There is no file sink, no network sink, and no logging configuration anywhere in `src/` or `config.sample.toml`.

That gap costs the operator in three concrete ways:

1. **They cannot answer "who needs cleaning up".** This is the sharpest one. When a package turns out malicious and gets blocked, the firewall stops further copies being fetched — but it has no reach into a machine that already installed a copy before the block. The only route from "we blocked it" to "here are the machines that already took it" is the firewall's own record of what it served beforehand. That record exists today, but it says a bad package was served at a given time without saying to whom, so the question stays unanswerable.
2. **They cannot prove what the firewall did.** An auditor asking "show me this was never served after the block" needs durable records. Console output that disappears on restart is not a record.
3. **Their security team never hears about it.** A SIEM exists so security-relevant events reach the people watching for them. A firewall that blocks a hijacked package and tells nobody outside its own console has done half its job.

**Why #1 is the boundary that matters:** a registry firewall can only stop packages not yet fetched — CI runs, container builds, fresh checkouts, cache misses. It has no way to reach an already-installed copy. That makes the firewall's own decision record the *only* asset that can ever answer "who needs cleaning up," which is why this problem is worth solving rather than merely convenient.

## How will we solve it?

**The proposal:** deliver the firewall's existing decision records to a file on disk and to the operator's SIEM. Each destination is optional and off unless the operator turns it on. An operator who wants today's behaviour unchanged sees no new files and no new network connections.

**The decision that needs the user's eyes most:** the firewall was deliberately built to record nothing a caller supplies — no identifying detail about who is asking. That rule is why the firewall can say a package was served at 14:03 but not to which machine, and it is the direct cause of problem #1 above. Solving "who needs cleaning up" requires reversing that rule for at least some deployments.

The draft proposes: consumer identification becomes an **opt-in, off by default**. Reasoning given — turning it on means the firewall starts holding a record of which internal consumer asked for which package, and that is data an operator should choose to acquire, not something that shows up automatically on an upgrade. When it is off, records still say what was served and when, just not to whom.

> If the user disagrees in either direction — that this should default ON because incident response is the whole point of shipping this feature, or that consumer identification should not exist at all — this is the moment to say so. Every downstream gate is shaped by this call.

**Two further decisions recorded here, given less weight but still load-bearing:**

- **A dropped record must be visible.** Delivery may drop records under load or when a destination is unreachable, rather than slowing or failing the firewall's real job of serving packages — but every drop is counted and surfaced, never silent. An operator who needs the stronger guarantee (refuse rather than serve unlogged) can opt into that as a separate, deliberate trade against availability.
- **Delivery never slows a request.** Whatever the file or the SIEM sink is doing, a developer waiting on `npm install` never waits on a log write or a network round trip.

**Non-goals**, which bound the work and keep it from growing:

- Not a replacement for endpoint remediation — these records tell an operator which machines to fix; fixing them stays outside this product.
- Not a log search or storage product — delivery only, no indexing, retention scheduling, or UI.
- Not on by default — an operator who wants nothing but console output sees no behavioural change at all.
- Not a general-purpose log pipeline — this delivers the firewall's own decision records, not arbitrary application logs.
- Not a metrics or dashboard feature — counters and summaries already exist; this is about the per-decision record.

## How will we confirm it is solved?

**Planned, not yet measured** — no delivery mechanism exists to measure against today:

- **Primary target:** time from a block taking effect to the operator holding the list of internal consumers already served that package — from impossible today to under five minutes, measured by one query against the operator's SIEM for the blocked package name, returning consumers and timestamps.
- **Guard measure:** no decision record lost while the firewall serves at its rated load, and any record that is dropped is counted and reported rather than discarded quietly.

Both are targets this gate proposes to hold the design to at Gate 2 and beyond — they are not results, because nothing is built.

## Decisions on the table

1. Ship optional decision-record delivery to a file and to a SIEM, each independently off by default.
2. Root justification: the firewall's own record is the only route from "we blocked it" to "here is who needs cleaning up," because a registry firewall cannot reach an already-installed copy.
3. Secondary justifications: durable records for audit, and reaching the security team that a console-only log never reaches.
4. **Consumer identification (who asked for the package) is opt-in and off by default** — the one decision most needing user sign-off, since it reverses an existing deliberate no-client-data rule.
5. A dropped delivery record must be counted and surfaced, never silently discarded; a stronger refuse-rather-than-serve-unlogged mode is available as a separate opt-in.
6. Delivery must never add latency to a package request — no waiting on a log write or a network call.
7. Success is measured as time-to-answer for "who needs cleaning up," target under five minutes via one SIEM query, guarded by zero silent record loss at rated load.
8. Explicitly out of scope: endpoint remediation, log search/storage, on-by-default behaviour, a general log pipeline, and metrics/dashboards.
9. Protocols, config keys, file paths, crate names, and architecture are deliberately excluded from this gate; they belong to Gates 2 and 3.

Gate document: `docs/plans/decision-log-delivery/01-product.md`

## Limitations

- No Gate QA receipt was supplied or found for this plan (`docs/plans/decision-log-delivery/` contains no `gate-1-qa.md`); this presentation renders the Gate 1 document and status notes as given, without an independent QA verdict to check against.
- No prior gates exist to cross-check against — Gate 1 is the first gate for this plan.

## Recommendation

The problem is grounded in the shipped product (not hypothetical), the boundary reasoning for why records are the only remediation route is sound, and the scope is tightly bounded by explicit non-goals. The one open question is whether opt-in/off-by-default is the right default for consumer identification — everything else reads as ready to proceed.

Approve Gate 1, or what should change?
