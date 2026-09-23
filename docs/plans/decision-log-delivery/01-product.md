# Product: Decision log delivery

## Clarifications and decisions

| ID | Class | Status | Owning Gate | Target Gate | Question or disposition | Selected value or policy | Decision source |
|---|---|---|---|---|---|---|---|
| C1 | current-Gate decision | resolved | 1 | none | Does the firewall record which consumer asked for which package? Today it deliberately records nothing a caller supplies, which is why it can say a package was served at a given time but not to whom. Answering the headline "which machines already took it" question requires reversing that rule. | Consumer identification is an opt-in and is OFF by default. While it is off the success metric below is unreachable and the records say what was served and when, never to whom. | `## Decisions recorded here rather than left to a later gate`, first decision |
| C2 | current-Gate decision | resolved | 1 | none | When delivery cannot keep up or the destination is unreachable, does the firewall drop records, or refuse to serve rather than serve unlogged? | Drop, count and surface every dropped record. A refuse-rather-than-serve-unlogged setting is not offered in this feature (C7); it would trade availability for completeness and belongs in a plan of its own. | `## Decisions recorded here rather than left to a later gate`, second decision |
| C3 | current-Gate decision | resolved | 1 | none | May delivery slow or fail a request? | No. A developer waiting on an install never waits on a log write or a network round trip, whatever the file or the SIEM is doing. | `## Decisions recorded here rather than left to a later gate`, third decision |
| C4 | current-Gate decision | deferred | 1 | 2 | Which wire format and transport carries records to the SIEM, and how does the operator name a destination? | Not selected at Gate 1. Product requires only that records reach the operator's SIEM; the transport is an architecture choice. | Deferred to Gate 2 |
| C5 | current-Gate decision | deferred | 1 | 2 | How does the local file behave over time — size, rollover, retention — given the non-goal that this is not a storage product? | Not selected at Gate 1. Product requires only that records land in a file the operator names and that the firewall does not fill the disk silently. | Deferred to Gate 2 |
| C6 | current-Gate decision | deferred | 1 | 2 | How does any credential the SIEM destination requires reach the process without being written into the configuration file? | Not selected at Gate 1. Product requires only that a credential is never stored in the configuration file and never written to a record. | Deferred to Gate 2 |
| C7 | user clarification | resolved | 1 | none | Build the refuse-rather-than-serve-unlogged setting C2 promises in this feature, or defer it? Final verification found no gate designed it and none deferred it. | Deferred out of this feature. Records are dropped, counted and surfaced, and no setting in this feature refuses a request because its record was not delivered. The setting reverses C3 — a request would wait on, or fail because of, delivery — and that trade-off belongs in a plan of its own. | user answer, Gate 1 grilling Q1 (2026-09-22) |
| C8 | current-Gate decision | resolved | 1 | none | Does every record delivered to the file and the SIEM say *when* the decision was made? This Gate already promises it: "what was served and when", consumers "with timestamps". Final verification found the delivered records carry no time, and only the console line does. | Yes. Every record delivered to the file or the SIEM states the time of the decision. A collector's own receipt time does not count, because delivery may lag the decision. The promise was already in this Gate; this row makes it explicit so Gates 2-4 design it. | `## Problem`, `## Success metric`, `## Announcement` of this document; `evidence/slice-3.md` (sf-verification P-WHEN) |

## Problem

An operator running this firewall cannot get its record of decisions anywhere useful.

The firewall already decides, for every single request, whether a package was allowed, held, blocked or refused, and why. It already writes that decision down. But it writes it to its own console output and nowhere else. An operator who wants to keep those records, search them later, or have their security team alerted when the firewall blocks something has to capture console output themselves and build the rest.

That matters for three reasons the operator feels directly.

**They cannot answer "who needs cleaning up".** This is the sharpest one. When a package turns out to be malicious and gets blocked, the firewall stops any further copies being fetched — CI runs, container builds, fresh checkouts, new machines. It cannot reach a copy already installed somewhere. The only route from "we blocked it" to "here are the machines that already took it" is the firewall's own record of what it served before the block. Today that record exists, but it says a bad package was served at a given time without saying to whom, so the question stays unanswerable.

**They cannot prove what the firewall did.** An auditor asking "show me that this blocked package was never served after the block" needs durable records. Console output that disappears when the process restarts is not a record.

**Their security team does not find out.** Organisations run a SIEM precisely so that security-relevant events reach the people who watch for them. A supply-chain firewall that blocks a hijacked package and tells nobody outside its own console has done half its job.

## Success metric

**Time from a block taking effect to the operator holding the list of internal consumers that already received that package: from impossible today, to under five minutes.**

Measured by the operator running one query against their SIEM for the blocked package name and getting back the set of consumers that were served it, with timestamps.

**This number is reachable only when the operator has turned consumer identification on (C1), and it is off by default.** With it off, the same query returns what was served and when, but not to whom, and the five-minute target does not apply — the operator gets an audit trail rather than a clean-up list. That is a deliberate choice about data an operator should hold on purpose rather than acquire on an upgrade, and it is the one decision in this document most worth overruling if you disagree.

A second number guards the first, because a log that silently loses records is worse than no log: **no decision record is lost while the firewall is serving at its rated load, and any record that is dropped is counted and reported rather than discarded quietly.** This guard applies whether or not consumer identification is on — an operator running this purely for audit still needs to know their records are complete.

## Non-goals

- **Not a replacement for endpoint remediation.** These records tell an operator which machines to go and fix. Fixing them is outside this product entirely, and shipping logs does not change that boundary.
- **Not a log search or storage product.** The firewall delivers records to a file and to the operator's SIEM. It does not index them, query them, retain them on a schedule, or provide a UI over them.
- **Not on by default.** An operator who wants nothing but console output today must see no change in behaviour, no new network connections, and no new files on disk.
- **Not a general-purpose log pipeline.** This delivers the firewall's own decision records. It is not a place to route arbitrary application logs.
- **Not a refuse-when-unlogged mode.** No setting in this feature refuses a request because its record could not be delivered (C7).
- **Not a metrics or dashboard feature.** Counters and summaries already exist; this is about the per-decision record.

## Screens

No UI. The operator's interface is the configuration file and, afterwards, their own SIEM.

## Announcement — the blog post before the feature

The package firewall can now deliver its decision records where you actually need them: to a file on disk, and to your SIEM.

Every request the firewall handles already produces one record saying what was asked for, what was decided, and why. Until now those records went to the console and no further. From this release you can point them at a log file, or forward them to your SIEM, or both — each optional, each off unless you turn it on.

The reason this matters is the question you get asked after a supply-chain incident: *which of our machines already pulled that package before we blocked it?* The firewall cannot uninstall anything, but it knows exactly what it served and when. If you turn on consumer identification, those records will tell you who to go and fix, and your SIEM will let you ask in one query instead of reading log files by hand.

Consumer identification is off by default and always will be. The firewall was deliberately built to record nothing about who is calling it, and turning that on is a decision with a privacy cost that belongs to you, not to us. When it is off, the records still say what was served and when — just not to whom.

## Decisions recorded here rather than left to a later gate

**Consumer identification is an opt-in, and off by default.** The firewall today deliberately records nothing a caller supplies. Reversing that unlocks the headline problem above — answering "who needs cleaning up" — but it means the firewall starts keeping a record of which internal consumer asked for which package, which is exactly the kind of data an operator must choose to hold rather than acquire by accident on an upgrade. So: off by default, on when the operator says so, and stated plainly in the operator documentation including what is recorded and what it is for.

**Losing a record must be visible.** When delivery cannot keep up or the destination is unreachable, records may be dropped rather than slowing or failing the firewall's real job of serving packages. Every dropped record is counted and surfaced. The stronger guarantee — refusing a request rather than serving it unlogged — is a separate, deliberate choice with its own cost in availability. It is not part of this feature (C7).

**Delivery never slows a request.** Whatever the file or the SIEM is doing, a developer waiting on `npm install` must not wait on a log write or a network round trip.

sf-red-team: not triggered — Gate 1 has no plan to challenge
