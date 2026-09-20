# Gate 3 — Program Design: Package Firewall MVP

## What problem do we have?
Gate 2 approved the *shape* of the service — the modules, the storage approach, the rule that no bytes reach a client before a safety check runs — but none of its actual internals existed yet: no list of what files would be written or why, no exact rules for how a request is handled step by step, no chosen version of anything the service would depend on, and no test that could ever prove any of it works. Without deciding all of that before a single line of code is written, there is no way to catch problems like an unproven database engine, an unsupported way of reading a package's filename, or a safety check that looks right on paper but was never actually enforced — until they show up in production.

## How will we solve it?
- Decide the whole implementation's internals on paper, before any code exists: every file the build will contain and why, every piece of information it holds, five step-by-step walkthroughs of the main things the service does, and a named test for every behavior the specification requires.
- Pin the exact version and license of every third-party piece of software the service will use, checked against live, current registry data rather than memory or assumption — including the embedded database engine, which is not yet at a stable 1.0 release, which is exactly why testing its save-and-recover behavior comes early in the build, as Gate 2 already required.
- Verify claims rather than assume them: there is no ready-made, maintained way to read the pieces of a Python package's filename, so the design builds that piece itself — and calls that out as the single place most likely to fail silently, with a plan to test it hard.
- Run a security threat model against the design and an adversarial review of the design itself, and fix everything either one finds before bringing this in front of you.

## How will we confirm it is solved?
- Planned, not yet observed: every test named in this gate — the rule-checking tests, the five review-finding tests, the five security-threat tests, the full request-and-response contract tests, and the two client-installer test suites — is a plan for the next gate to build and run, not something that has executed.
- Planned: an early, dedicated test of the database engine's actual save-and-recover behavior on the exact pinned version, before anything else depends on it.
- Planned: real end-to-end installs with real npm and pip clients, once the client-facing slice is built.
- Observed today: nothing has been built or run. The repository still contains no source code for this service — everything above is this gate's plan for what the next gate must build and prove, not a report of anything working.

## Decisions this gate asks you to make
1. Accept that this gate commits to the full internal design before any code exists — every file, every piece of information it holds, five main request walkthroughs, and a named test for every required behavior — rather than leaving those decisions to be made ad hoc during the build.
2. Accept the pinned versions and licenses of everything the service will depend on, checked against live registry data on 2026-09-17, including the embedded database engine at a pre-1.0 release — a known and accepted risk, managed by testing its recovery behavior early rather than late.
3. Accept that reading a Python package's filename is built in-house rather than bought, because nothing maintained does it — flagged here as the single most likely place for a silent mistake, mitigated with a large, deliberately adversarial set of test filenames rather than a dependency.
4. Accept the security threat model's five retained risks: four are closed by design changes made in this document, and one — a single client tying up every download slot the service has — is deliberately accepted and pushed onto the operator's own reverse proxy, because this service has no concept of "which client is which" by specification.
5. **Needs your explicit sign-off:** one of those four fixes adds a new setting to the settings file that the original specification never listed, to stop one oversized package from delaying every other request, including an urgent block. This is a new operator-visible option beyond what was originally promised and should not be approved by default.
6. Accept that an adversarial review of this design found five problems, two serious enough on their own to block approval — including that the design had claimed a compile-time guarantee that no bytes could reach a client before the final safety check, when the actual code shape shown did not enforce that. All five problems, including that one, are fixed in this version of the document.
7. **Needs your decision:** two questions this document leaves open with no owner named — who is responsible for producing the blocklist feed itself, and who is accepting the earlier product decision that a download already in progress is not interrupted partway through even if the package becomes blocked while it is still streaming. Both are yours to answer, not something this design can decide for itself.
8. Accept that this is still a paper design: no code exists yet, so every confirmation described above is a plan for the next gate to build and prove, not a result already achieved.

## Gate document
`/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/03-program-design.md`

## Limitations
- This presentation covers Gate 3 (Program Design) only; building the service, running the named tests, and measuring anything against a real system is Gate 4's job and is out of scope here.
- Two ownership questions (the blocklist producer, and acceptance of the no-interrupt download decision) and one new configuration setting are called out above because they need your explicit answer — they are not settled by this gate's own analysis and should not be waved through with a general approval.
- The one technical uncertainty this design cannot resolve on paper — whether the pre-1.0 database engine actually behaves as assumed — is unchanged from Gate 2 and is still deferred to the early build slice that tests it directly.

## Recommendation
The design is internally consistent with the approved product and architecture, names its own riskiest assumptions plainly, and closes every problem an adversarial review found against it, including the one that would have let bytes ship before the final safety check actually ran. Proceed to a decision on this gate, but treat the new configuration setting and the two named ownership questions as things this decision must explicitly settle, not details to wave through.

## Context indicator
Observed: the repository still contains no source code for this service — every file, type, and call stack described above is a plan recorded in `03-program-design.md`, not something built, compiled, or run. Nothing in this presentation implies that any part of the design has been exercised against real requests, a real database, or a real registry.

## Source evidence
- Gate document: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/03-program-design.md`
- Approved Gate 1 product contract: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/01-product.md`
- Approved Gate 2 architecture: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/02-architecture.md`
- Prior presentations (voice and shape matched): `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/show-me-product.md`, `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/show-me-architecture.md`

Approve Gate 3, or what should change?
