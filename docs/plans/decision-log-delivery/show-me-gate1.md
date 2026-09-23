# Gate 1 — Decision log delivery: is this the right problem, stated correctly?

## What problem do we have?

The package firewall already decides, for every request, whether a package was allowed, held, blocked or refused — and already writes that decision down. But today it writes it only to its own console output, and nowhere else.

That costs the operator in three concrete ways:

- **They cannot answer "which of our machines already took that package before we blocked it".** This is the sharpest of the three. A registry firewall can stop further copies being fetched — CI runs, container builds, fresh checkouts, new machines — but it cannot reach a copy already installed somewhere. The firewall's own record of what it served before the block is the *only* route from "we blocked it" to "here is who needs cleaning up." Today that record says a package was served, and when — never to whom.
- **They cannot prove to an auditor what the firewall did.** Console output does not survive a restart. There is no durable record to point to.
- **Their security team is never told a block happened.** Nothing reaches a SIEM.

**Why the first consequence is sharpest:** it defines the exact boundary of what a registry proxy can and cannot do. Blocking is a boundary at the point of fetch, not at the point of use — everything on the other side of that boundary is invisible to the firewall unless its own record of "served to whom" is preserved and delivered somewhere durable.

## How will we solve it?

Deliver those existing per-request decision records to two optional destinations: a file on disk, and the operator's SIEM. Each is independently off unless the operator turns it on. An operator who wants exactly today's behavior sees no change at all — no new files, no new network connections.

**The decision that most needs your eyes:** the firewall was deliberately built to record *nothing a caller supplies* — no source address, nothing client-identifying. That design rule is exactly why it cannot say who received a package today. Answering the clean-up question means reversing that rule for the first time. The document proposes doing this as **consumer identification, opt-in, off by default** — on the reasoning that "which internal consumer asked for which package" is data an operator should choose to hold on purpose, not acquire by accident on an upgrade. If you think this should default to *on* (because incident response is the entire point of shipping this) or that it shouldn't exist at all, this is the moment to say so — everything downstream is shaped by this call.

Two other decisions are settled in the same document, with less at stake:

- **A dropped record must be visible.** If delivery can't keep up or a destination is unreachable, the firewall drops rather than blocks — but every drop is counted and surfaced, never silent. An operator who wants the stronger guarantee (refuse rather than serve unlogged) can opt into that separately, at its own availability cost.
- **Delivery never slows a request.** Whatever the file or the SIEM is doing, nobody waiting on an install waits on a log write or a network round trip.

**Deliberately not decided here** — recorded as deferrals to the architecture gate, not oversights: how records reach the SIEM (wire format, transport), how the local file behaves over time (size, rollover, retention), and how a SIEM credential reaches the process without landing in the configuration file. None of those choices changes what's being decided today.

**Non-goals**, to keep the boundary tight: not a replacement for endpoint remediation (these records tell an operator who to go fix, not how); not a log search or storage product; not on by default; not a general-purpose log pipeline for arbitrary application logs; not a metrics or dashboard feature.

## How will we confirm it is solved?

> The claims below observed against the shipped product are marked **Observed**. Everything about the proposed feature is **Planned** — nothing described here is built yet, and Gate 1 approval does not change that.

**Observed, today's product (independently verified against source by Gate QA):**
- `src/main.rs:58-63` initializes JSON logging to STDOUT only — no file sink, no network sink, no logging configuration exists anywhere in the codebase.
- `src/http/logging.rs:14` states the deliberate rule "nothing a client sends reaches a log line," and one decision event is already emitted per request with request_id, method, ecosystem, package, version, status, result, reason, blocklist_revision, cache, duration_micros and bytes, plus a periodic counter summary.

**Planned, the proposed success measure (a target, not an achievement):**

> Time from a block taking effect to the operator holding the list of internal consumers that already received that blocked package: from **impossible today** to **under five minutes**, measured by one query against their SIEM for the package name.

This number is reachable **only when consumer identification (the opt-in above) is turned on** — which it is not, by default. With it off, the same query still returns what was served and when, just not to whom: an audit trail rather than a clean-up list.

A second measure guards the first, and applies regardless of that setting: **no decision record is lost while the firewall serves at its rated load, and any record that is dropped is counted and surfaced rather than discarded quietly.**

## Decisions in this document

1. Deliver decision records to a file and to a SIEM, each optional and independently off by default — no behavior change for an operator who does nothing.
2. **Consumer identification is opt-in, off by default** (the load-bearing, most-worth-overruling call — see above).
3. With consumer identification off, the firewall can never answer "who received this package" — only "what was served, and when."
4. A dropped record must be counted and surfaced, never silently discarded; refuse-rather-than-serve-unlogged is available as a separate, deliberate opt-in with its own availability cost.
5. Delivery must never slow or fail a request — no log write or network round trip sits on the request path.
6. Wire format/transport to the SIEM is explicitly deferred to Gate 2, not decided here.
7. Local file rollover/retention behavior is explicitly deferred to Gate 2, not decided here.
8. How a SIEM credential reaches the process without entering the config file is explicitly deferred to Gate 2, not decided here.
9. Non-goals fix the boundary: not endpoint remediation, not a log search/storage product, not on by default, not a general log pipeline, not a metrics/dashboard feature.
10. Success is defined as an operator-felt time-to-answer metric with a stated measurement method and a stated contingency on setting C1, not a restatement of the feature.

Gate document: `docs/plans/decision-log-delivery/01-product.md`

## Limitations

None. All claims above trace to the Gate 1 document or to source files independently re-verified by Gate QA immediately before this presentation was written.

## Recommendation

Approve. The document was not self-certified: an independent Gate QA pass first returned REVISE over two gaps — a missing decision inventory and a headline metric stated without its own contingency — and both were corrected before this fresh QA pass returned READY. The one call genuinely worth a second look is the opt-in default for consumer identification; every other decision here is a direct, low-risk consequence of the existing product's own "record nothing a caller supplies" rule.


Approve Gate 1, or what should change?
