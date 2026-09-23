## What problem do we have?

The firewall keeps a local copy of each package's details. When that copy gets old, the firewall
asks the registry "has this changed?" — and if the registry answers "no," the firewall keeps the
old copy and resets its own clock. There is no limit on how many times that can repeat. A registry
that keeps answering "no change" — whether because nothing changed, or because it is broken or
compromised — can hold this firewall's picture of a package frozen forever.

This is not a bug: the current behaviour matches exactly what the written contract said to do. The
contract itself was incomplete, and the user asked for a configurable maximum age on cached items.

This is a **pre-implementation** gate: no code for this change exists yet. Nothing below is
observed behaviour; it is a reviewed plan awaiting approval.

## How will we solve it?

- A new setting caps how old a cached copy of package details may get, defaulting to 24 hours;
  setting it to zero turns the cap off, restoring today's unbounded behaviour.
- The firewall will record when it last downloaded a copy **in full**, kept separate from when it
  last merely checked for a change. A "no change" answer updates only the check time, never the
  full-download time — this distinction is the entire mechanism.
- Once a copy passes its cap, the next request downloads it in full and deliberately asks no
  "has this changed?" question, so the registry gets no chance to answer "no change" again.
- An over-age copy is never served. If the registry cannot be reached, the request is refused
  rather than answered from the over-age copy — this reaches a rule the contract already had, by a
  second route.
- Each package's cap is shortened slightly by a fixed amount derived from its own name, so not
  every cached item expires at the same instant. The shortening only ever makes the cap tighter, so
  the configured value stays a true maximum.
- This needs a new column in the stored database, which moves the storage format forward one
  version. The firewall refuses to open a database written by a different version rather than
  converting it — acceptable only because nothing has been deployed yet; the same tradeoff was
  accepted twice before in this project.

**Deliberately excluded, by user decision:**
- The cap covers package details only.
- It does **not** cover verified package files. Those are stored under a name derived from their
  own verified content, so they cannot go stale; ageing them out would delete correct data and
  re-download it for no safety benefit. They already have their own disk-space limit.
- It does **not** cover the block list, which already carries its own expiry inside the document.
- It does **not** cover cached "this package does not exist" marks. The first draft claimed it
  did; that claim was wrong and has been removed (see below).

**What the adversarial review changed** — this is presented honestly, not as polish. The review
returned NEEDS REVISION with one blocking finding and one major finding, both now fixed:

- **Blocking**: the first draft also applied the cap to cached "does not exist" marks. That could
  never have worked — those marks already expire on a separate, faster timer, and they carry no
  timestamp of the kind the cap needs. A test written to prove the claim could only have passed by
  coincidence with behaviour that already existed. This is the fourth time in this project that a
  check has been caught proving something other than what its name claimed. The claim and its test
  were removed rather than propped up.
- **Major**: with every cached item on one shared clock, a bulk load or a restore from backup would
  leave every copy expiring at the same moment, turning a registry outage into one simultaneous
  refusal across the whole instance instead of a gradual lapse. It also hands anyone who can
  disrupt the network path a new lever: sustain that disruption past one cap and warm packages
  start refusing, where before they served indefinitely. Answered with the per-package shortening
  described above. The remaining exposure is recorded, accepted, and assigned to operations to
  document, including a warning that turning the cap off restores exactly the problem it was added
  to fix.
- Two smaller points were also closed: the cap's dependence on the wall clock is now written down,
  including what happens on a clock jump in each direction, and the contract gained a written
  review record for this revision.

**What the review confirmed as planned, not yet observed:** it searched every place the stored
package record gets written and found only two write paths, one of which touches an unrelated
field. So no restart, recovery, eviction, or reload path can quietly advance the new full-download
time behind the cap's back. This is planned confirmation from a design-time search, not a test run
against running code.

## How will we confirm it is solved?

The single most important planned check is aimed at the one way this control could fail silently:
if any code path ever writes the current time into the full-download record on a "no change"
answer, the cap stops existing while every visible behaviour stays identical — until a copy ages
past it. The plan therefore requires:

- A check that inspects the stored value directly and fails if that field is ever advanced by a
  "no change" answer — not merely a check that fails only once the cap is visibly gone.
- A mandatory adversarial review of exactly that field, and of what happens under wall-clock
  manipulation in both directions.
- Planned tests (not yet run, since no code exists) covering: an over-age copy triggers a full
  download with no validators sent; an over-age copy is refused rather than served when the
  registry is unreachable; repeated "no change" answers cannot extend a copy's age past the cap;
  a zero setting restores today's unbounded behaviour; two packages loaded together do not expire
  at the same instant; and the shortening never lengthens a cap past its configured maximum.

**Decisions for approval**

1. Add a configurable maximum age for cached package details, default 24 hours, zero disables it.
2. Track "last full download" separately from "last check," changed only by a real full download.
3. On expiry, download in full and skip the change-check question entirely.
4. Never serve an over-age copy; refuse the request instead if the registry can't be reached.
5. Shorten each package's cap slightly and deterministically by name, to avoid synchronized expiry.
6. Explicitly exclude verified package files and the block list from this cap (both already have
   their own correct staleness handling).
7. Correct the blocking review finding by removing the "does not exist" marks from cap coverage.
8. Accept, and hand to operations to document, the residual risk that a sustained network
   disruption can now push warm packages into refusing state once the cap lapses.
9. Bump the stored database format forward one version and refuse to open an older one rather than
   converting it, on the same accepted-twice-before tradeoff.
10. Require a check on the stored field itself, plus a mandatory adversarial review of that field
    and of clock manipulation, before this can be called done.

Gate document: `docs/plans/package-firewall-mvp/03-program-design.md`
(revision section: "Revision: bounded maximum age for cached metadata (reopened 2026-09-19)")

**Limitations**

- No code exists for this change; everything above is a reviewed design, not observed behaviour.
- The "only two write paths" finding and the four-clock-manipulation-test set are the result of a
  design-time source search and self-review, not a test run — the adversarial and self-review
  passes on this revision were conducted as a single reviewer rather than an independent pass, the
  same limitation recorded on the two prior reviews in this project.
- The residual denial-of-service exposure from a sustained network disruption is accepted, not
  eliminated, and its written documentation is still owed by slice 15.

**Recommendation:** Approve. The blocking finding was a real logic error and has been removed
rather than patched over; the major finding has a concrete, bounded mitigation with the residual
risk named and owned; the plan is honest about what remains unverified because no code exists yet.

**Context:** pre-implementation Gate 3 re-approval, second reopening of this gate, triggered by a
user product decision rather than a defect. Approval re-opens Gate 4's slice plan for re-approval,
after which the work builds as slice 15, ordered after slices 13 and 14 and before slice 10 (which
stays last as the performance acceptance for everything before it).

**Sources:** `docs/plans/package-firewall-mvp/03-program-design.md` (lines 1139–1190, "Revision:
bounded maximum age for cached metadata"; lines 675–726 for the mechanism and call-stack detail);
`docs/plans/package-firewall-mvp/00-status.md` (line 65, Red Team Pass 5 record; line 62, the
load-bearing inventory finding; line 66, the write-path search); `SPEC.md` (§16, "Review record for
revision 3", lines 462–491; §10, lines 311–315).

Approve Gate 3, or what should change?
