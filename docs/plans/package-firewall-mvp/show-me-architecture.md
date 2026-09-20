# Gate 2 — Architecture: Package Firewall MVP

## What problem do we have?
The product decided at Gate 1 (hide too-young releases and blocklisted packages, never substitute bytes, keep serving fast once something is known) has no design yet for how one running service would actually do that safely: how it stores what it has already seen, how it decides trust for every request, how it keeps a large file it is still downloading from being handed to a client before it is fully checked, and how it survives a crash or a restart without quietly losing what it already knew.

## How will we solve it?
- One process, one Rust binary, sitting on loopback behind the operator's own TLS proxy — no fleet, no second server to keep in sync.
- Split the work into small one-way modules: something that only decides yes/no/wait-until (never touching the network or a clock itself), something that only talks to the three registries, something that only remembers state, and something that only speaks HTTP — each depending only on the modules below it, never sideways or back up.
- Keep exactly one place that owns the database and exactly one open connection to it, and keep every request that is already "warm" (already decided once, still valid) answerable from memory alone, with no database round trip.
- Treat an artifact download as unsafe until it is fully fetched and its fingerprint verified — no bytes reach a client before that finishes, and the safety decision is checked again immediately before the response is built, not just when the download started.
- Make persistence and recovery an early, dedicated slice of the build, not something bolted on later, precisely because the chosen storage engine is still new and unproven at this exact pinned version.
- Give the six problems the specification's own review already found (STATE-01, REL-01, FLOW-01, PERF-01, TEST-01, OPS-01) a name and a place — each is written into this design and is required to come back as a named test in the next gate, so none of them can quietly disappear.

## How will we confirm it is solved?
- Planned, not yet observed: a persistence-and-recovery slice, run early in the build, that actually exercises the storage engine's save/crash/restart behavior on the exact pinned version before anything else depends on it.
- Planned: the six named review findings each become a specific, named test in Gate 3 — not a general assurance that "testing will happen."
- Planned: the fake test registry proves it can only reach the service through the same constructor-level door the real registries use, with no separate switch that a real deployment could accidently leave open.
- Observed today: none of this exists yet. The repository holds no manifest, no source file, and no test — this is a design on paper, confirmed against the repository's actual current contents, not a report of anything built or run.

## Decisions this gate asks you to make
1. Accept Rust as the implementation language, even though the checkout sits on a Go-style path and the current `.gitignore` is a Go template — the specification's explicit decision record fixes Rust, and the mismatched path/ignore file are treated as leftover repository setup to be corrected in the first build slice, not a reason to reopen the language choice.
2. Accept the embedded, pre-1.0 storage engine as a known and managed risk: because it is not yet a 1.0 release, all of its SQL is confined behind one single module, its exact version is pinned before any code trusts it, and a persistence-and-recovery slice is scheduled early — rather than treating "it might not do everything expected" as something to discover late.
3. Accept the single-process shape: one storage task, one database connection, one bounded work queue for it — anything that would need a database round trip on an already-warm, previously-seen request is treated as a design defect to avoid, not an acceptable slow path.
4. Accept that the three upstream registries the service is allowed to talk to are fixed in the binary, with no configuration key, flag, or environment variable able to relax that or the checks against private/loopback addresses — including inside the test setup, which must reach the service through the same door a real deployment would use.
5. Accept the eight-module split and its one-way dependency shape (a pure decision module with no I/O; separate modules per registry; one module owning all storage; one module owning outbound calls; one module owning HTTP) as the structure later gates build against.
6. Accept that the six named findings already recorded in the specification's own review (STATE-01 permanent fingerprint pins, REL-01 save-and-commit ordering, FLOW-01 abandoned-download cleanup, PERF-01 checking local blocks before the network, TEST-01 the test-only door, OPS-01 not confusing the download-cache budget with total storage capacity) are carried forward as-is, each owing a named test at the next gate rather than being re-litigated now.
7. Accept that no separate adversarial architecture review was run at this gate: the specification's own review already covered these six findings, the one open technical uncertainty (whether the pinned storage engine behaves as expected) is something only running code can answer and is scheduled as the earliest build slice, and the remaining security-sensitive surface (untrusted data from the registries, outbound-request restrictions) is deferred to a dedicated review at the next gate once the relevant code exists to review.
8. Accept that this is a paper design only: the repository holds no manifest, source, or test today, so nothing in this architecture has been built, run, or measured — every claim above about speed, safety, or recovery is a plan for the next gates to build and prove, not a result.

## Gate document
`/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/02-architecture.md`

## Limitations
- No code, build, or test exists yet; every mechanism described above (module split, storage shape, download-safety ordering, recovery behavior) is a plan, not something exercised against real requests, real registries, or a real crash.
- This presentation covers Gate 2 (Architecture) only. Exact endpoint contracts, dependency versions and licenses, and the six findings' specific test names belong to Gate 3 and are out of scope here.
- The one open technical uncertainty this architecture flags — whether the pinned storage engine actually supports the durability and recovery behavior the design depends on — is explicitly unresolved at this gate; the architecture document treats it as something only the early persistence slice, not further planning, can answer.

## Recommendation
The architecture document is internally consistent with the approved Gate 1 product contract and the specification, states its one real open uncertainty (the pinned storage engine's behavior) plainly, and gives each of the specification's own six review findings a concrete forward destination (a named Gate 3 test) rather than leaving them as prose. Proceed to a decision on this gate; treat the storage engine's actual behavior as something the early persistence-and-recovery slice, not this document, must confirm.

## Context indicator
Observed: the repository at commit `1acede7` on `main` holds only `README.md`, `SPEC.md`, `LICENSE`, and `.gitignore` — no manifest, source, test, or build automation exists (confirmed via `docs/codebase-overview.md`). Everything above describing modules, storage shape, request flow, and recovery behavior is the *plan* recorded in `02-architecture.md`; none of it has been built, run, or measured. Nothing in this presentation implies the problem described in Gate 1 is already solved.

## Source evidence
- Gate document: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/02-architecture.md`
- Approved Gate 1 product contract: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/01-product.md`
- Source of intent: `/home/john/go/src/github.com/postfix/osprey/SPEC.md` (revision 2, 2026-09-17; sections 3, 4, 8–11, 13, 15)
- Repository briefing: `/home/john/go/src/github.com/postfix/osprey/docs/codebase-overview.md`

Approve Gate 2, or what should change?
