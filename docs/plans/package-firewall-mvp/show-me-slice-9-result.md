# Slice 9 result — The HTTP contract

## Delivered

Every failure this proxy can have now gets a truthful, specific answer instead of a generic error. A client asking about a package now gets one of a fixed, complete set of outcomes:

| Situation | What the client sees |
|---|---|
| Package is blocked | A refusal that says so, plus *when to check back* (a concrete retry time and a numeric wait) |
| Package name doesn't exist, or was removed | A clear "not found" |
| The request itself was malformed | A clear "bad request" |
| Wrong method or format asked for | A clear "not supported here" |
| This proxy currently can't decide (no policy loaded, over capacity, overloaded, storage trouble) | A clear "try later" |
| The upstream registry misbehaved (failed, sent bad data, or data that didn't match its own claims) | A clear "upstream problem" |
| The upstream registry was too slow | A clear "upstream timed out" |

Nothing this proxy returns is cacheable by anything sitting between it and the client — not the refusals, not the errors, not the successes. The proxy never tells a client "nothing has changed since your last check," and it never hands a client the private caching tokens it received from the upstream registry. A special internal health-check route used to probe the upstream registry is never mistaken for an actual package lookup.

Every request now leaves exactly one operator-facing log line recording the decision made and the same request ID the client was given, so a client's bug report ("I got refused") can be matched to the exact reason the proxy decided that, on that request. A background counter also summarizes decision volume periodically.

## Proof

Reproduced independently by a second and a third reviewer, not only by the implementer:

- `cargo clippy --all-targets -- -D warnings` — clean, no warnings.
- The new HTTP-contract test suite — 11 checks, all passing, 0 failing.
- Full test suite — 211 passing, 0 failing, across 17 test files (up from 196 across 16 before this slice).
- None of the new checks could have passed against the pre-slice code; several wouldn't even compile, because the piece of the system that produces the one-line-per-request log didn't exist yet.

Both a code review and a security review examined this work: code review shipped it with two minor notes, both already fixed; security review cleared it outright. A third, independent pass re-checked every situation in the table above against the actual code that handles it, not against a checklist of intentions.

## Limits

- The security check here was a careful manual read of the code rather than an automated leak-detector, because that automated tool wasn't available for this pass; its main conclusion (that private data literally cannot flow into a client-facing error) rests on the guarantee the type system enforces, which a manual read can verify directly.
- Two of the "try later" causes (over capacity, overloaded) are proven here only as isolated cases in the failure table, not by driving the whole system from a live overload; the end-to-end proof of those two lives in an earlier slice (slice 7), not this one.
- The log line's recorded response size is the size the proxy declared it would send, not necessarily what actually reached the client — a connection that drops partway through would still log the full declared size. Accepted for this slice.
- The periodic counter summary is only emitted when triggered by the next request; a quiet period with zero traffic produces no summary for that period. Accepted for this slice.

All test evidence above (the fake upstream registry and its fixtures used to exercise these situations) is test scaffolding, not something a real client or the production upstream registry does.

## Next

Slice 10 is the only slice remaining unchecked in the plan.

## Recommendation

Ship slice 9 as delivered and move to slice 10.

### Notable decisions and findings

- The "nothing is cacheable" guarantee holds for every one of the twelve possible response statuses because it's applied once, as a single wrapper around the whole request-handling path — not, as first believed, because certain responses happen to share a code path with ordinary successful ones. Code review caught that the originally stated reason was wrong for one status even though the behavior itself was correct; the true reason is now the one recorded in the test.
- Two of the twelve statuses were being produced correctly but were missing from the test that checks "nothing is cacheable," even though the test's own comment claimed full coverage. Both are now covered, and the completeness check was proven load-bearing by deliberately breaking it — removing coverage made the check fail with exactly the missing statuses, not silently pass.
- "The proxy never says nothing changed" is not just untested luck — it's structurally impossible: that specific response is never constructed anywhere in the code, and no code path reads the two request headers that would trigger it.
- "Private upstream caching tokens can't leak to the client" is enforced the same way — that stored value has no code path that ever reads it back out onto a client-facing response, and the complete list of headers this proxy ever sends downstream is closed and fully enumerated.
- Error responses cannot leak an internal upstream address, header, or file path back to a client as a matter of the type system — the fields capable of carrying such text are typed so that a runtime string cannot be assembled into them, not merely policed by convention.
- Only two request headers are ever read from an incoming client request, and requests made to the upstream registry are built from a fixed, separate set of headers that never reuse them — so client credentials cannot reach the upstream registry.
- A local network hiccup during an upstream transfer used to be reported to clients as "we're over capacity," which wrongly blamed the client's request. It's now correctly reported as an internal proxy fault, using the same general "try later" category rather than inventing a new one.
- Three files outside this slice's originally approved file list had to change, each a single call added at an existing decision point, because the per-request log line needs to record whether a response was served from a warm cache or cold, and no file inside the approved list could observe that. This is the sixth consecutive slice where the approved list named files but not every call site those files force a touch to.

Plan document: docs/plans/package-firewall-mvp/00-status.md

Continue to slice 10, or re-steer?
