## Delivered

Before this slice, when the firewall's stored copy of a package's details went stale, it discarded it and re-downloaded the whole thing — even if nothing upstream had changed. It now sends the registry its stored freshness markers and asks "has this changed?" When the registry answers no, the firewall keeps what it has instead of throwing it away.

The critical part: a "no change" answer does not mean "do nothing." The no-change path deliberately rejoins the same rebuild the firewall always runs after a full download, so the operator's *current* block list is re-applied to the kept copy every time. Without this, a version blocked *after* the last full download could keep quietly being served forever.

Both npm and Python packages were handled by one shared mechanism, not two separate ones. Four tests that an earlier design stage had called for — and that had never actually been written for either ecosystem — were written in this slice.

## Proof

Independently re-run by three separate reviewers, all reporting matching counts:

| Check | Result |
|---|---|
| Lint | Clean |
| npm metadata tests | 32 passed |
| Python metadata tests | 24 passed |
| Origin-guard security tests | 17 passed, unchanged |
| Full suite | 244 passed, 0 failed, 13 ignored, across 20 targets (up from 224; 20 tests added) |

The first twelve of the new tests were watched failing against the pre-slice code before the fix, confirming they exercise real behavior rather than passing vacuously.

**Test evidence for the block-list guarantee (not a description of product behavior):** the guarantee that a kept copy still gets the current block list re-applied was proved by deliberately breaking that rebuild step in a throwaway copy of the code. With the rebuild skipped, both ecosystems' tests failed by showing the blocked version being served — while the separate check confirming a "no change" answer had occurred still passed. That means the test failed on the security property itself, not on some unrelated precondition. An independent reviewer ran their own version of this same kind of check on a different test and got the same kind of failure.

**A stronger result than this slice claimed, found by the security reviewer (also test evidence):** blocking a package version takes effect regardless of any of the above. A change to the block list by itself invalidates the firewall's already-rendered copies, so a newly blocked version stops being served even with zero upstream contact. Block-list enforcement does not depend on the refresh machinery at all.

## Limits

**The one recorded tradeoff — the main thing worth reconsidering before continuing:** if upstream keeps answering "no change," the firewall keeps its copy indefinitely and never forces a full re-download. There is no limit and no periodic forced refresh. Two reviewers examined this independently.

- It is exactly what the product contract specifies (the contract imposes no limit), so it is not being treated as a defect.
- Its consequence is narrow: the firewall may be slow to notice that upstream itself withdrew or replaced a version the operator never separately blocked.
- It **cannot** cause a blocked version to be served — that guarantee is independent of this tradeoff (see Proof).
- Closing it would require a full re-download every so many checks, which means changing the product contract and re-opening an earlier approval gate. That was judged out of scope for this slice.

Also worth knowing: if the firewall has no stored copy at all but gets a "no change" answer, that is treated as the registry misbehaving and is refused outright — never guessed at. Left alone deliberately: two of the new tests are near-duplicates that a reviewer confirmed could be merged with no loss of coverage; harmless to keep as is.

## Next

Build order after this slice is: slice 13, then slice 14, then slice 10.

## Recommendation

Proceed. The delivered mechanism is proven independently by three reviewers with matching results, the security-critical guarantee (block list still applies to kept copies) was proven by deliberately breaking it and watching the test catch that break, and a reviewer found the block-list enforcement is even stronger than claimed. The one open tradeoff (no forced periodic re-download) is contract-conformant, cannot expose a blocked version, and is clearly disclosed rather than buried.

Context indicator: full context available — status ledger (`00-status.md`, slice 12 entry) supplied complete proof, findings, and tradeoff record directly; no gaps required inference.

Source evidence:
- `docs/plans/package-firewall-mvp/00-status.md` — Slice 12 checklist line and its proof entry (upstream conditional revalidation, both clauses of SPEC §10; test counts; mutation-proof description; security reviewer's stronger finding; recorded tradeoff; reviews sf-code-review=SHIP, sf-security-review=CLEAR, sf-verification=VERIFIED)
- `docs/plans/package-firewall-mvp/00-status.md` — build order line ("BUILD ORDER REPAIR" / Gate 4 revision entries) confirming order 13 -> 14 -> 10 after slice 12

Continue to slice 13, or re-steer?
