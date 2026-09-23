# Slice 4 — The Outbound Boundary

## What problem do we have?

The firewall's whole job is to stand between an installer and the public package registries. Until
this slice, nothing enforced that the firewall could only ever *reach* the registries it was told to
reach. A registry response, a redirect, or even a crafted package name could in principle steer an
outbound request somewhere else entirely — off the approved registries, onto a port nobody approved,
carrying credentials nobody meant to send, or onto a private address inside the operator's own
network (an address that is not meant to be reachable from the public internet, the kind an attacker
uses to turn a public service into a way of probing or reaching what sits behind it — this class of
attack is called SSRF, server-side request forgery, elsewhere in this project's records). None of
that was closed off before this slice.

## How will we solve it?

Every outbound request now goes through exactly one controlled gate, and that gate re-checks itself
on every hop, not just the first one:

- The firewall only ever calls out through one internal component. There is no second path to the
  network.
- A redirect from a registry is followed only if it still lands inside an approved origin; a
  cross-origin redirect is refused.
- Before any connection is made, the address the registry name resolved to is checked and refused if
  it is a loopback, private, or otherwise non-public address — closing the SSRF class of attack
  above.
- Credentials embedded in a URL, an unexpected port, or a non-HTTPS scheme are all refused.
- A package name cannot smuggle a path escape (`..`), a scheme, or an encoded slash into the request
  that gets built — the URL is assembled segment by segment rather than by pasting text together.
- None of this can be relaxed by a config key, flag, or environment variable in the shipped binary;
  only a test-only construction path can loosen it, and that path is not reachable from the running
  program.

## How will we confirm it is solved?

The automated checks passed — and then three rounds of engineering review, run precisely because the
witness alone is not enough for a boundary this security-sensitive, found nine real defects, every
one of them in code the passing test suite had already accepted. **That is the honest shape of this
slice: the witness passed, and the witness was not enough.**

What the witness itself showed, re-run independently by the main agent after each of the three review
rounds on a forced rebuild: no compiler warnings; the dedicated boundary test suite passed all 17 of
its cases (the 8 originally planned plus 9 added during review); the whole project's test suite
passed across all 8 test files; and the locked dependency versions were confirmed unchanged from the
ones approved back in slice 1.

What the witness could not see, and three rounds of review found instead:

- **A misread of "not changed."** Every upstream signal meaning "this hasn't changed, nothing to
  fetch" was being treated as a refused redirect instead, so that path was silently never taken —
  and the passing test never exercised it correctly either.
- **A package name of exactly `.` or `..` was silently dropped rather than refused**, which let a
  real request leave the process for a name the approved plan says must never leave. The existing
  test happened to pass anyway because it only checked that the request succeeded, not what it
  actually reached.
- **A live path to the operator's own machine.** A whole block of addresses meant to be treated as
  private was being treated as public, and on this kind of system those addresses route back to the
  local host — a real, working SSRF into loopback, not a theoretical one.
- **The private-address check itself was rewritten**, twice, after being found incomplete. Rather
  than patch each newly found gap, it was inverted: instead of naming every bad address range (an
  open-ended list a reviewer must keep re-checking), it now names the one good range and its
  approved exceptions (a closed list). Re-deriving that closed list from the internet's own address
  registry during the rewrite found five further gaps — one of them the same class of mistake in an
  older spelling that no reviewer had caught the first time.
- **That rewrite created a new risk of its own**, checked before it shipped: a private-address list
  that is too narrow doesn't leak, it breaks — it would refuse legitimate traffic outright, which is
  a full outage rather than a security hole. All three of the firewall's real production
  destinations were resolved live during review and confirmed still reachable under the new check.

Verdicts: code review shipped the slice after two reworks; the mandatory security review cleared it
after two reworks; adversarial testing passed with no boundary escape reproduced across a wide set of
encoding and redirect attacks.

**Limitations, stated plainly rather than smoothed over:**

- One specific evasion — a private network deliberately mapped to look like a public address through
  a mechanism called NAT64 — cannot be caught by any check that only looks at DNS answers. It has to
  be stopped by network-level filtering at the operator's boundary, which is not yet written; it is
  recorded as owed to slice 10's operations notes, not done.
- One untouched test file's byte-for-byte sameness could not be proven this slice, because no earlier
  slice ever recorded a baseline to compare against. Confidence that it was not edited rests on its
  own tests still passing plus two reviewers reading it, not on a hash match.
- Three files outside this slice's approved file list had to be edited, each forced by Rust's type
  checker and each judged forced and minimal by code review. This is the third slice in a row where
  that has happened, because the approved slice plan does not name the call sites a change is forced
  to touch.
- Two supporting statements in the approved program-design document (Gate 3) were found false during
  this slice's work and have been corrected in that document directly. Neither correction changes an
  approved decision. The main agent made the call to correct in place rather than reopen that gate;
  the reader may overrule that call.

## Decisions taken

- Outbound traffic has exactly one path out of the process, re-checked on every redirect hop, not
  just the first request.
- A private-address check was rewritten from an open-ended "name every bad range" list into a
  closed "name the one good range" list, because the open form kept being found incomplete.
- The rewritten private-address list's risk of being too strict (an outage) was checked against all
  three live production destinations before shipping, not assumed safe.
- Package names cannot escape into a path segment (`.`/`..`); this is now a hard refusal rather than
  a silent drop.
- A misclassified upstream "not changed" response was fixed so that code path is real again.
- reqwest's default system-proxy behavior, which could have let an environment variable redirect
  outbound traffic, is now explicitly disabled.
- The NAT64 evasion gap is accepted for this slice and formally owed to slice 10's operations notes.
- Three forced, minimal edits outside the approved file list were accepted by review for the third
  slice running; this is flagged as a gap in how the slice plan names forced call sites, not a defect
  in the edits themselves.

Full detail — every witness command and its output, the complete nine-defect record, the scope
deviation list, and the full limitation list — is in the slice 4 proof line of:

`docs/plans/package-firewall-mvp/00-status.md`

**Recommendation:** ship slice 4 as delivered. The boundary is closed on every angle reviewed, the
one open gap (NAT64) is a known, owned, and narrow residual rather than a silent one, and nothing
found required reopening an approved gate.

---

**Context indicator:** presentation setting `auto`, resolved to Markdown (the stored
`## Factory settings` value in `docs/plans/package-firewall-mvp/00-status.md:8-10`).

**Source evidence (compact):**
- `docs/plans/package-firewall-mvp/00-status.md:24-25` — slice 4 checked, with its full proof line:
  witness commands and results, the nine defects and their fixes, the test-integrity record, the
  scope deviations, the over-rejection check against live production origins, the limitations, and
  the two Gate 3 corrections; reviews `sf-code-review=SHIP after two reworks`,
  `sf-security-review=CLEAR after two reworks`, `sf-adversarial-testing=PASS`
- `docs/plans/package-firewall-mvp/04-slices.md:20` — slice 4's approved row: the promise this slice
  had to meet, its exact file list, and its named mandatory reviews
- `docs/plans/package-firewall-mvp/04-slices.md:26-31` — the remaining rows (5-10), not started
- `docs/plans/package-firewall-mvp/03-program-design.md:937-970` — "Post-approval corrections
  (recorded 2026-09-18, during slice 4)": the reqwest system-proxy default-feature hole and the
  `PathSegmentsMut::push` `.`/`..` silent-drop defect, both corrected in that document in place

Durable boundary recorded. To clear context, run /clear (or /new), then software-factory resume. To proceed now, reply continue.
Continue to slice 5, or re-steer?