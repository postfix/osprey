## Delivered

When several people ask this firewall for the same package at the same moment and it has nothing cached, it used to ask the upstream registry once per person. Slice 11 makes it ask once and serve everyone from that one answer, for both npm and Python packages, on the same shared mechanism. It also removes a way the firewall could get permanently stuck on one package if a refresh crashed mid-flight.

The mechanism that already coalesced concurrent requests for package downloads was extracted into a single shared component, and the metadata-lookup path was put on that same component rather than a second copy being built. npm and Python packages were fixed together and now share one table. A crash inside a refresh now frees everyone waiting on it instead of freezing that package forever.

## Proof

- Lint: clean.
- The ten pre-existing concurrency tests pass, their test file untouched.
- 22 npm and 14 Python metadata tests pass.
- 7 new adversarial tests pass (test evidence, not product behaviour).
- Full suite: 224 passed, 0 failed, 13 ignored across 20 targets, up from 211 with 13 tests added.
- Every new test was first watched failing against the old code, recording 6 upstream requests where 1 was required.
- Independently re-run by two separate reviewers, with matching results.
- Reviews: code review returned FIX FIRST on two items, both since resolved. Adversarial testing returned PASS, reproducing no way to get the firewall stuck, including 3,200 racing attempts at one suspected flaw and a check that an npm package and a Python package of the same name cannot be confused for each other. Verification returned VERIFIED WITH FINDINGS, and its finding is resolved.

## Limits

**Accepted limitation, not fixed in this slice.** If the person whose request triggered a refresh disconnects midway, everyone else waiting on it receives a temporary error and has to retry, where they would have succeeded had they never been grouped together. It is temporary and self-correcting — the next request works — and it cannot get the firewall stuck. It was deliberately not fixed here because fixing it requires changing files outside what was approved for slice 11, which the plan treats as a decision for the user rather than something to slip in. It is documented in the code with the fix named.

**Test-coverage finding worth knowing about.** One of the new tests carried a comment claiming it guarded a specific hazard; independent verification proved it did not — it would have stayed green even if the protection were deleted. It has since been strengthened so it now fails when that protection is removed. This is the fourth consecutive check in this plan that caught something the previous one missed.

## Next

Build order after this slice: 12, then 13, 14, and finally 10.

## Recommendation

Proceed. Every witness result was independently reproduced by two reviewers with matching counts, no residual review finding is open, and the one accepted limitation is self-healing and explicitly scoped out rather than overlooked. The test-coverage finding was caught and closed before this handoff, which is a track record worth noting rather than a reason to pause.

Context: this is the completed-slice result for slice 11 of 14 in the Package Firewall MVP plan (build order 9 -> 11 -> 12 -> 13 -> 14 -> 10).

Source: `docs/plans/package-firewall-mvp/00-status.md`, slice 11 entry (line 37) and checklist (lines 36-41).

Continue to slice 12, or re-steer?
