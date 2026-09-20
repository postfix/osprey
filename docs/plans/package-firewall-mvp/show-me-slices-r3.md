## What problem do we have?

Three slices in a row finished with approved work that nobody owned. Each time it was written into the plan's notes as it was found, and each time the user chose to keep going rather than stop. The count has now reached five, and the user has asked that all five be assigned — which changes the approved slice plan and is why this gate is reopened.

The five, in plain terms:

1. When several developers ask for the same package's information at the same moment, the firewall should fetch it from upstream once. It fetches once per request instead — measured at 21 separate upstream calls for 25 simultaneous first-time requests. The design named the function meant to prevent this; it exists nowhere in the code.
2. The firewall should ask upstream whether its copy has changed rather than downloading it again, and treat "not changed" as confirmation. It currently treats that answer as an upstream failure.
3. Four tests the design named — covering a package removed upstream, refreshing a package already known, and clock movement — exist nowhere and were assigned to no slice.
4. Closing a security hole in an earlier slice added a cost: the firewall now re-reads and re-parses a project's stored information on every file request, including ones already cached. Estimated harmless on small packages, but several to tens of milliseconds on large real ones — which would miss the product's own latency target.
5. Two known minor races, both self-healing and neither affecting which bytes are served, were accepted but left unassigned.

## How will we solve it?

Three new slices — 11, 12 and 13 — built before the existing delivery slice rather than after it, since that slice's benchmarks are the evidence that the performance fix in this revision worked, and run first they would measure exactly what this revision removes. The new slices keep their higher numbers because slice numbers already appear in eight recorded proof lines, in the plan's notes, and in source comments naming the owning slice; renumbering would make all of those false.

- **Slice 11** extracts the safety mechanism a completed slice already built into one shared piece, and uses it for both package files and package information, so the "fetch once" behaviour is built on a mechanism that already survived a dedicated attack rather than a second hand-built one.
- **Slice 12** does the ask-upstream-first work, in both halves the specification requires.
- **Slice 13** fixes the re-parse cost and the two known races.

This revision was itself put through an adversarial review, whose first draft came back NEEDS REVISION. Three of its findings changed the plan materially:

- The draft claimed one fix covered both package ecosystems while naming a test for only one of them — by the plan's own rule, work stated in prose without a test is not assigned, so the revision meant to stop unassigned work had itself left half of one item unassigned. Fixed.
- The specification behind item 2 has two required halves and the draft carried only one: a "not changed" answer must renew freshness AND still re-apply the current blocklist. Without the second half, a package confirmed unchanged could keep serving a version that has since been blocked — a blocked version reaching a developer. This is the most important change the review produced; the revision now names a test for it and requires a security review.
- The draft gave the new coalescing work its own new safety mechanism. The equivalent mechanism built earlier had a permanent failure mode that a passing test suite and a clean code review both missed, and only a dedicated attack found. Building a second one invited the same outcome, so the new slice now reuses the existing mechanism, with its ten existing tests as the check that extracting it changed nothing.

The review also found the original performance check could not be falsified once the fix shipped, because the old code being measured against would no longer exist. It is now a threshold test that fails automatically past the target, with the before-measurement recorded first.

## How will we confirm it is solved?

None of the following has run yet — these are the checks this revision commits to, not results:

- **Slice 11**: the ten existing concurrency tests must still pass unchanged, proving the shared mechanism preserved what it already proved; plus new tests asserting that simultaneous first-time requests cause exactly one upstream fetch, and that a crash during a refresh does not wedge the package.
- **Slice 12**: tests that a "not changed" answer avoids a refetch, and that it still re-applies a blocklist that changed in the meantime; plus the four previously-unassigned design tests, written for both ecosystems.
- **Slice 13**: a test that a warm file request no longer re-parses the project document, and a threshold test that fails automatically if a warm-request time exceeds the target.
- All three new slices carry a mandatory adversarial or security review, named against each row.

These checks were chosen because they are falsifiable against the specific failure each obligation names — a call count, a response code, a re-fetch that must not happen, a timing threshold with a hard ceiling — rather than a description of intended behaviour.

## Decisions

- Assign all five previously-unowned obligations to three new, explicitly ordered slices (11, 12, 13) rather than folding them into existing rows.
- Keep slice numbers 11-13 higher than the delivery slice's number even though they are built first, because renumbering would falsify recorded proof lines, status notes, and source comments.
- Build the new slices before the delivery slice, since its benchmarks must measure the post-fix code to be meaningful evidence.
- Build the coalescing slice (11) on the existing proven single-flight mechanism rather than a new one, using its ten tests as a regression check.
- Require the conditional-revalidation slice (12) to satisfy both halves of the specification clause — freshness renewal and blocklist re-application — not just the half a first draft covered.
- Require a security review specifically on the blocklist re-application path, since missing it means a blocked version could keep being served.
- Replace the original performance check with a threshold-asserting test recorded against a before-measurement, since the original check could not be falsified after the fix shipped.
- Leave slices 1-10 and their recorded proof lines untouched; slice 9 continues in progress unaffected by this revision.
- Accept the review's stated limitations rather than blocking on them (see below).

**Document**: `docs/plans/package-firewall-mvp/04-slices.md`

**Limitations**:
- The adversarial review ran as a self-review without the dedicated threat-modelling tool, which was not installed in this session; it reasoned the threat question itself and concluded the new upstream requests open no new attack surface.
- Its check for "what is still unowned" came from re-reading the specification, not from re-tracing every named design element against the current source — the plan has twice been caught by exactly that kind of gap.
- Slice 9 is being implemented while this gate is open; its row is unchanged by this revision.

**Recommendation**: Approve. The revision assigns all five obligations to named slices with falsifiable checks, was challenged by an adversarial review and materially improved as a result, and disturbs none of the eight already-recorded slices or the one in progress. The alternative — continuing with five unassigned obligations into delivery — is what the last three slices already demonstrated goes wrong.

Approve Gate 4, or what should change?
