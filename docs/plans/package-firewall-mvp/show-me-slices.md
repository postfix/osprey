# Gate 4 — Vertical Slices: Package Firewall MVP

## What problem do we have?
Gates 1–3 decided *what* the firewall is, *why* it exists, and its complete paper design — but nothing has been built. A greenfield build of this size cannot be done as one diff: it needs an order that proves the riskiest assumptions early, keeps every intermediate step demonstrable, and never asks you to trust code nobody has reviewed yet. Without that order decided up front, the build could bury the one unproven dependency (the database engine) until it's expensive to change, or hand you a single enormous diff nobody can meaningfully review.

## How will we solve it?
- Cut the build into ten slices, each one a runnable step forward, in the order SPEC §13 fixes: policy first, then persistence, then npm and PyPI metadata, then artifact delivery, then real clients.
- Start with a **tracer bullet** (slice 1): the real binary boots, and a real npm request is refused by the real policy path — with nothing stored, nothing fetched, and no shortcuts standing in for the real wiring.
- Put the one unproven third-party piece — the pre-1.0 database engine — under direct test in slice 3, while a different choice is still cheap to make.
- Give the "only talk to the real registries, never anywhere else" rule its own slice (slice 4), completed and reviewed before any package listing is ever shown to a client.
- Put the highest-stakes slice — where a blocked package must reliably stop being served, including files already downloaded — in slice 7, and require three separate reviews on it rather than one.
- Ship ten reviewable diffs instead of one unreviewable one, each with its own passing tests before the next slice starts.

## How will we confirm it is solved?
- **Nothing here has been observed yet.** The repository still holds no source code — every test and command below is a plan for the build to run, not a result. That is expected and correct for a pre-code gate.
- Each slice carries its own witness: specific commands (`cargo clippy`, `cargo test --test <name>`, and later `docker build`, `cargo bench`) that must pass before that slice counts as done.
- An independent adversarial review (`sf-red-team`) checked this plan itself and found one blocking gap, now fixed (see Findings below).
- Slice 10 is where the SPEC's speed targets get measured and reported — not gated on, matching the product document's own "measured, not promised" stance — and where real `npm`/`pip` installs against the built firewall are the final proof.

## Decisions this gate asks you to make
1. Accept ten slices as the build order, each producing a working, testable increment rather than one large build. Source: build order table, `04-slices.md` "Slices" section.
2. Accept that slice 1 is a tracer bullet: the actual binary starts and refuses an npm request through the real policy-evaluation code — not a stub — with nothing stored or fetched upstream. This proves the whole wire works before any real behavior is added.
3. Accept that the build order follows SPEC §13: policy engine, then persistence, then npm and PyPI metadata, then artifact verification, then client integration — because later slices depend on earlier ones and nothing is built out of dependency order.
4. Accept that the database engine's still-unproven pre-1.0 save-and-recover behavior is tested in slice 3, early in the build, while switching engines would still be a cheap change rather than a rewrite.
5. Accept that the "never talk to anything but the real, fixed registries" boundary is its own slice (slice 4) — completed and reviewed before slice 5 renders the first real package listing to a client.
6. Accept that slice 7 (artifact verification and the content cache) carries the plan's highest concentration of risk: three separate mandatory reviews (code, adversarial, and security), because this is the slice where a blocked file must actually stop being served, including files a client already has a link to. This is the slice worth reading personally.
7. Accept that this plan adds two files beyond Gate 3's list — a dedicated tracer test (`tests/tracer.rs`) and a fixtures directory for invalid-input test cases — and that it splits three of Gate 3's test files whose tests span slices built days apart, so that each test file is complete and runnable at the end of the slice that owns it, per Gate 3's own rule.
8. Accept that two ownership questions Gate 3 left open — who produces the blocklist feed, and who is accountable for accepting that a download in progress isn't interrupted mid-stream — are still unnamed here; slice 10 is where a name would be recorded in the operations documentation, if you want one recorded.
9. Accept that the SPEC §12 performance numbers are measured and reported only in the last slice (slice 10), not treated as a pass/fail gate at any earlier point.
10. Accept the independent review's one blocking finding as resolved: two restart-survival tests required by the product's own "a restart does not reset the clock" rule had been left out of every slice; they are now assigned to slice 5, the first slice able to build them.

## Gate document
`/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/04-slices.md`

## Limitations
- Nothing in this plan can be witnessed until slice 1 exists — every command and test named for slices 1–10 is a plan, not a result, today.
- The SPEC §12 performance numbers are only measured in slice 10, and are reported there, not enforced as a gate at any point in the build.
- Two ownership questions remain unnamed: who produces the blocklist feed, and who accepts the no-interrupt-mid-download decision. Slice 10 is where a name would land in the operations documentation, if this gate's approval should require one.
- This presentation covers Gate 4 (Vertical Slices) only; actually building and running any of it is out of scope here.

## Recommendation
Approve as written and begin slice 1.

## Context indicator
Observed: the repository still contains no source code — commit `1acede7` holds only `README.md`, `SPEC.md`, `LICENSE`, and `.gitignore`. Every outcome, test, and witness command described above is a plan recorded in `04-slices.md` for the next phase of work to build and run; none of it has executed. This is expected for a pre-implementation gate.

## Findings
An independent adversarial review (`sf-red-team`) was run against this plan and returned one blocking finding, now resolved in `04-slices.md`: two tests required by the product's restart guarantee (`first_seen_survives_restart`, `upstream_timestamp_supersedes_first_seen`) had been left out of every slice's witness. They are now assigned to slice 5's test file, the first slice able to build them. Two non-blocking findings were also recorded and accepted as written: slice 7 concentrates three mandatory reviews in one diff (accepted — splitting it would cut a single capability in half rather than reduce risk), and most witness cells list tests non-exhaustively (accepted, except slice 5's cell, which is now complete).

## Source evidence
- Gate document: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/04-slices.md`
- Approved Gate 1 product contract: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/01-product.md`
- Approved Gate 3 program design: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/03-program-design.md`
- Status and ten-slice checklist: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/00-status.md` (presentation: auto)
- Prior presentations (voice and shape matched): `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/show-me-product.md`, `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/show-me-architecture.md`, `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/show-me-program-design.md`

Approve Gate 4, or what should change?
