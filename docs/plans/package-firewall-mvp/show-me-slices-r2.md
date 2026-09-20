# Gate 4, second look — Vertical Slices: Package Firewall MVP

This is a re-presentation of `04-slices.md` after the 2026-09-18 reopen and a second Red Team pass run against the reopened document today. It does not repeat the first presentation (`show-me-slices.md`, rendered 2026-09-17 for the pre-reopen version); read this as "what changed and why it's still sound," not as a first look.

## What problem do we have?
The gate you approved yesterday quietly left two dependency decisions for whoever wrote the code to make on the spot — which TLS backend the upstream HTTP client uses, and which of two competing version-ordering libraries the npm logic relies on. Both are load-bearing: get either wrong and the firewall either can't reach the registries at all or silently misorders package versions, which is exactly the kind of mistake this product exists to prevent in other software. Work had already started against the un-settled plan before that was caught, so the reopen also has to answer: is anything now in the repository that was built on top of an unsettled decision?

## How will we solve it?
- Settle both dependency facts inside the plan itself, not in code review after the fact: the TLS backend and its build-time consequences, and the version-ordering library's lineage and correctness.
- Roll back the work that had started against the unsettled plan. Nothing built on the open question survives; the repository returns to the last approved commit plus this session's planning documents.
- Trace every consequence of the two now-settled facts forward into the slice plan, not just into the slice where the fact was first needed — including the slices that only inherit the consequence later (the build toolchain a C-based TLS provider needs, and a container trust-store requirement).
- Re-run an independent adversarial review against the revised plan, scoped to exactly what the reopen touched, so the parts of the plan the reopen didn't touch aren't re-litigated for no reason.
- Carry forward, rather than quietly drop, the one open question from the prior gate that only you can close: who is accountable for the blocklist feed and for accepting that a file already mid-download won't be interrupted.

## How will we confirm it is solved?
- **Still nothing has been observed running.** The repository holds no product source at commit `1acede7` — no `Cargo.toml` exists — so every witness command in every slice fails today, and is required to. That is the plan working as intended for a pre-code gate, not a gap.
- The rolled-back work is not silently gone: it's preserved outside the repository, in this session's evidence, in case anyone needs to check what was stopped and why.
- Each slice still carries its own concrete pass/fail commands (`cargo clippy`, `cargo test --test <name>`, and in the last slice `docker build`, `cargo bench`) — those didn't change in kind, only in a few of their prerequisites and one row's witness, as the settled facts propagated.
- A second independent adversarial review ran today against the revised document, found two more issues (both non-blocking), and both are already fixed in the document you're being asked to approve — not deferred.

## What changed since yesterday's presentation
- **Decisions delegated, now settled.** Gate 3 had left the TLS backend/feature choice and the version-ordering library's lineage as open questions. Both are now fixed facts recorded in the design; slice 1 states plainly there is "no open dependency question" left for the implementer to make on the fly.
- **The false start is undone.** Slice 1 had been dispatched and began writing `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` and a modified `.gitignore` before the open questions were caught. That output was removed from the repository (kept as evidence outside it), and the tree is back to the approved commit. Nothing in the current repository was built on an unsettled decision.
- **The consequences were traced forward, not just fixed at the point of decision.** Settling the TLS choice means the very first `cargo clippy` in slice 1 already compiles a C-based crypto provider — so slice 1 now needs a C compiler, cmake and perl from the start, not only by slice 10 as the plan previously said. Settling the version-ordering library added a dedicated test in slice 5 that checks its ordering against a corpus of real published npm precedence, rather than trusting the library's own claims.
- **A second adversarial pass ran against the revised plan and found two more things, both already fixed:** slice 1 was missing that same C build toolchain requirement (now added, with the exact tool versions confirmed present); and slice 10 asserted a container trust-store requirement that no test actually exercised (the slice's witness now includes an offline check that the trust store is non-empty, plus one live-network request that proves TLS verification is using it — this is the plan's only step needing outbound network).
- **One ownership question is still open, and it's yours.** Nobody is yet named as accountable for producing the blocklist feed, or for accepting that a download already in progress won't be cut off mid-stream. Slice 10 is where a name would be recorded in the operations documentation — if you want one recorded before this ships.

## Gate document
`/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/04-slices.md`

## Limitations
- This presentation only covers what the reopen changed and what the second review found; the slice order, the tracer-bullet approach, and the case for slice 7's triple review (from the 2026-09-17 presentation) are unchanged and not repeated here.
- The trust-store fix's live-network check is, by the document's own account, the plan's only witness step that touches outbound network; every other witness in every slice runs offline.
- The second review recorded one limitation of its own: it relied on the settled dependency table's own description of the TLS provider's build requirements rather than independently re-checking that description against upstream documentation.
- The blocklist-producer and no-interrupt-mid-download ownership question remains genuinely unresolved — approving this gate does not resolve it.

## Recommendation
Approve as written. The reopen did what it needed to: it removed a delegated decision, rolled back the work that predated its resolution, and traced the resolution's consequences into every slice that inherits them. The second review's two findings are already closed in the document. The only outstanding item is the ownership naming, which is a decision for you, not a defect in the plan.

## Context indicator
Observed: the repository at commit `1acede7` holds only `README.md`, `SPEC.md`, `LICENSE`, `.gitignore`, and (untracked) this session's `docs/` folder — no `Cargo.toml`, no Rust source. Every test name, tool version and command in `04-slices.md` is a plan for work not yet started; none of it has executed. This matches the document's own statement of its required property.

## Source evidence
- Gate document (subject of this presentation): `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/04-slices.md`
- Reopen record and rollback note: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/00-status.md`
- Approved Gate 3 program design (post-reopen, 2026-09-18): `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/03-program-design.md`
- Approved Gate 2 architecture (2026-09-17): `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/02-architecture.md`
- Approved Gate 1 product contract (2026-09-17): `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/01-product.md`
- Prior Gate 4 presentation, pre-reopen (voice and shape matched, not repeated here): `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/show-me-slices.md`

Approve Gate 4, or what should change?
