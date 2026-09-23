# Gate 4 (revised): a maximum age for cached package metadata

## What problem do we have?

The firewall already limits how often it re-*checks* the upstream registry for
a package's information (`metadata_ttl_seconds`). It does not limit how *old*
its stored copy of that information may become. Every time upstream answers
"nothing changed," the check timer resets — so an upstream that never changes
its answer, or that is lying, can hold this firewall's view of a package fixed
indefinitely. This is a real, already-built property of the shipped code (the
team recorded it as a known tradeoff of the completed conditional-revalidation
work), not a hypothetical risk.

## How will we solve it?

Add a new, separately configurable ceiling on the true age of a stored copy.
Once a copy passes that ceiling, the firewall performs a full refetch and gives
upstream no chance to answer "nothing changed"; if that refetch cannot
complete, the request is **refused** rather than answered from the over-age
copy (fail closed). Setting the ceiling to zero turns it off. The ceiling
applies only to package information — never to downloaded package files
themselves (those are identified by a verified content fingerprint and cannot
go stale), and never to the blocklist (which already expires on its own).

The accepted cost, stated plainly: fail-closed means an upstream outage that
outlasts the ceiling now takes previously-warm packages offline, where before
they were served indefinitely. Operations is the named owner of that cost, and
this work item must write it into the operations runbook together with the
operator's two levers (raise the ceiling, or set it to zero to go back to the
old unbounded behavior). A small, deterministic per-package offset (up to a
tenth of the ceiling) spreads expiry times across a fleet so packages fetched
together don't all lapse at the same instant — it only ever shortens an
individual package's ceiling, so the configured number stays a true maximum.

## How will we confirm it is solved?

This is **planned**, not yet built — nothing here has shipped. Confirmation
will rest on a set of tests plus two mandatory reviews. Most important is a
test that asserts the stored "last full fetch" value directly, because a bug
in threading that value through would leave every visible behavior identical
until a copy silently aged past the ceiling — a passing suite would otherwise
prove nothing. Beyond that: a test that an over-age copy triggers a validator-free
refetch, a test that an over-age copy is never served when upstream cannot be
reached, tests that reject invalid configuration, tests that the behavior
survives a restart, and mandatory security and adversarial reviews asking
specifically whether the ceiling can be defeated and how it behaves under
clock manipulation in either direction.

## Decisions in front of you

- New build order: **13 → 14 → 15 → 10** (was 9 → 11 → 12 → 13 → 14 → 10, now
  advanced past completed work). Slices 1–9, 11 and 12 are already complete
  and unchanged by this revision; Gates 1 and 2 were approved 2026-09-17 and
  Gate 3 was re-approved 2026-09-19.
- New slice 15 is the owning row for this ceiling: adds `metadata_max_age_seconds`
  (config), a new "last full fetch" column, fail-closed refusal, the
  deterministic per-package offset, and zero-disables semantics — for both
  npm and PyPI.
- Lead witness: `a_304_does_not_advance_the_full_fetch_time`, asserting the
  stored column value directly, not just externally visible behavior.
  Supporting witnesses: over-age refetch without validators, never-served-when-unreachable,
  configuration-rejection tests, restart tests, and mandatory `sf-security-review`
  and `sf-adversarial-testing` on whether the ceiling can be defeated and on
  clock manipulation in both directions.
- Accepted cost: an upstream outage crossing the ceiling now produces
  refusals it previously would not have (TM-15-2). Owner: **operations**, via
  `docs/operations.md`, with the two operator levers stated explicitly.
- Scope boundary, stated as a decision: the ceiling covers project metadata
  only. It explicitly does **not** cover downloaded package bytes (keyed by
  verified content hash, cannot go stale) or the blocklist (already has its
  own expiry). An earlier draft also tried to apply it to the "package name
  not found" cache and that was a real logic error — removed rather than
  patched, because that cache already has its own unconditional recheck and
  the ceiling could never bind there.
- Schema limitation to accept: this adds a database column and moves the
  schema version from 3 to 4, which **refuses** an older database rather than
  migrating it. Acceptable only because nothing has shipped yet — the
  repository can prove no deployed database exists in it, but cannot prove
  none exists anywhere in the world.
- Review history on this exact revision: an independent review pass returned
  "needs revision" once, citing one blocker (the name-not-found cache error
  above) and three lesser findings (fleet-wide correlated outage risk, missing
  documentation of the clock dependency, missing change-log entry in the
  specification). All four are resolved in the current documents.
- Recommendation: approve. The problem is real and already observed in
  shipped behavior, the fix is scoped narrowly (metadata only, one new
  column), the fail-closed cost is named and owned, and the review findings
  against the plan are closed.
- Gate document: `docs/plans/package-firewall-mvp/04-slices.md`.

Context: revision plan, not yet implemented; grounded in `04-slices.md`,
`03-program-design.md`, `00-status.md`, and `SPEC.md` revision 3.

Approve Gate 4, or what should change?
