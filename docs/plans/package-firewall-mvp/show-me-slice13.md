## Delivered

Slice 13 closed two problems observed directly in the shipped code, both traced to the same area of the metadata cache.

**A correctness hole.** When the firewall learned a package file was blocked — by computing its fingerprint during a download — the product rule says the next listing must hide that version. That held only if nothing else was reading the same package at that moment. A concurrent reader could put its own, slightly older view back into the cache and keep showing the blocked version for up to the metadata refresh interval.

**A speed problem**, and the one item in the whole plan predicted to breach a stated performance target on a real package. A security fix made in an earlier slice forced the firewall to re-read and re-parse the entire package document on every single file request — including requests it could already answer from its own cache.

What changed: the cache now refuses to accept a stale view that was read before a block landed, and the list of files a package advertises is parsed once per version of the document instead of once per request. Both fixes apply identically to npm and PyPI through one shared mechanism.

```
Before: cargo test --test npm_metadata  →  ok. 33 passed
        cargo test --test pypi_metadata →  ok. 24 passed
After:  cargo test --test npm_metadata  →  ok. 33 passed
        cargo test --test pypi_metadata →  ok. 26 passed (+2 new PyPI tests)
        cargo test --test artifacts_verification → ok. 10 passed
        cargo test --test warm_artifact_budget -- --ignored → ok. 1 passed
        Full suite: 248 passed / 0 failed / 14 ignored
```

Changed files: `src/npm/mod.rs`, `src/pypi/mod.rs`, `src/store/cache.rs`, `src/artifacts/mod.rs`, `src/artifacts/download.rs`, `tests/npm_metadata.rs`, `tests/artifacts_verification.rs`, `tests/warm_artifact_budget.rs`. Forced edits outside the approved row file list: `src/store/mod.rs:319-322` (comment only) and `tests/pypi_metadata.rs` (the two new witnesses).

## Proof

Observed, not planned — every number below was measured against running code, re-run independently by the main agent and by code review, green on more than fourteen consecutive runs:

- `cargo clippy --all-targets -- -D warnings`: exit 0, no warnings.
- `npm_metadata` 33 passed, `artifacts_verification` 10 passed, `pypi_metadata` 26 passed (+2 new PyPI tests), `warm_artifact_budget -- --ignored` 1 passed.
- Full suite: 248 passed, 0 failed, 14 ignored.

Warm file request on a 5,000-version package, the one number this slice exists to fix:

| Build | Before | After | SPEC §12 budget |
|---|---|---|---|
| Debug (measured this slice) | +202.833 ms | +1.074 ms | 5 ms |
| Release (slice 10 baseline, not re-measured here) | +37.9 ms | — | 5 ms |
| Release, 1,000 versions (slice 10 baseline, not re-measured here) | +6.8 ms | — | 5 ms |

Only the 5,000-version point was re-measured in this slice; the debug/release distinction is labeled above because it changes the number by roughly 35–200x and the two must not be compared as if equivalent.

This slice did not pass on the first attempt, and the process caught two real things:

- Review found the new correctness safeguard depended on two statements being written in the right order at two places in the code, with nothing in the test suite that would notice if a future edit got the order wrong. An adversarial pass proved it by making that exact mistake and watching the data corrupt while every test stayed green. The fix was structural, not another test: the two separate operations were merged into one (`get_with_seen` replacing `invalidations`), so the wrong order is no longer expressible.
- Both reviews independently found PyPI's half of this work had no test of its own — it was riding entirely on being written the same way as npm's. Two end-to-end PyPI tests were added, which also surfaced that PyPI had been missing this class of coverage for six slices, not just this one.

## Limits

- The core correctness safeguard cannot be tested end-to-end. The only situation that triggers it happens in a code path with no pause point a test can hold, so it is covered by a hand-driven test of the mechanism itself plus the structural guarantee, not by a test of the real request path. Reviewers confirmed this against the source rather than accepting the claim.
- One route to the old mistake remains possible in principle: a future edit could still call the unguarded version of the cache operation. Nothing does today. Closing it completely needs a change reviewers judged outside this slice.
- The 248/0/14 result above is a DEBUG build. Running the suite in release mode fails one unrelated timing test from an earlier slice, reproducibly, because optimized code finishes the work faster than that test's hard-coded margin allows. This was independently confirmed as pre-existing and not caused by this slice. It means no run of this plan has ever exercised the suite in release — recorded as unowned item 7 and left for whoever takes slice 10.

## Next

Slice 14 moves package file downloads under each ecosystem's own address — `/npm/artifacts/…` and `/pypi/artifacts/…` in place of the shared `/artifacts/…` — so that current npm installs work again without the user disabling one of npm's own supply-chain protections.

## Recommendation

Ship slice 13 as complete and proceed to slice 14. The correctness fix is structural (not merely test-covered) and the performance fix is measured well inside budget; the two disclosed limits are bounded, self-healing or pre-existing, and none blocks moving forward.

Document: `docs/plans/package-firewall-mvp/00-status.md`

Context: grounded in `docs/plans/package-firewall-mvp/00-status.md` slice 13's checklist entry and proof line, and its "Notes for a fresh session" entry "UNOWNED AFTER SLICE 13 (2026-09-19)"; `docs/plans/package-firewall-mvp/04-slices.md` row 13; `SPEC.md` §9 and §12.

Continue to slice 14, or re-steer?
