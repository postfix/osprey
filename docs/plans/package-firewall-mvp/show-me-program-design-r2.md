# Gate 3 (re-approval) — Program Design correction: Package Firewall MVP

## What problem do we have?
Gate 3 was approved on 2026-09-17. It was reopened the next day because the approved document left two facts unresolved and handed them to whoever wrote the code instead: which TLS setup the HTTPS library would use, and which of two similarly-named open-source libraries for comparing package version numbers was the one actually still maintained. Both facts change what the code has to do, so the correction: a design gate settles a fact like that from the library's own current documentation, its published source, or a quick real test — or it stops and says so. It does not leave the decision for later. Nothing had been built yet when this was caught — implementation had been told to start, but was stopped before it wrote a single line of product code — so this correction changes paper only.

## How will we solve it?
- Pin four things that were previously vague or missing, each from a source you can check, not from guesswork: the HTTPS/TLS setup, which of the two compression-and-decompression features are actually turned on, which version-comparison library, and five small supporting libraries ("crates" — Rust's term for a published software package; a "feature" is an optional, named piece of a crate you turn on or off when you pull it in) that had no chosen version at all.
- Re-run the same real test that caught the TLS gap — a throwaway program that made one real HTTPS request and printed its result — as the evidence for the fix, not just as a diagnosis of the problem.
- Carry the two consequences of these fixes into the build plan: a new automated test for version-ordering correctness, and two things the container image build now needs to have installed.
- Change nothing else. The modules, the data each one holds, the request walkthroughs, the threat findings, and the test plan are exactly as you approved them on 2026-09-17.

## How will we confirm it is solved?
- Observed today (2026-09-18), before any product code was written: the throwaway HTTPS probe actually ran and printed `PROBE https status = 200 OK`; a direct attempt to request the old TLS feature name really did fail with `error: unrecognized feature for crate reqwest: rustls-tls`; and a dry-run dependency check really did list the compression features as off unless asked for.
- Observed today: the version-comparison library's fork history and last-updated dates were read from its own project page and the two projects' publication records, not recalled from memory.
- Planned, not yet observed: the new version-ordering test (`npm_version_precedence_matches_a_published_corpus`) and everything else this gate already planned for the next phase to build and run.
- Still true and unchanged: no product code exists. The stopped implementation attempt's leftover files were removed from the repository and kept elsewhere as a record of what happened; the repository is back to the commit before that attempt.

## Decisions this gate asks you to make
1. Accept the reason the gate reopened: leaving an implementation-affecting fact for whoever writes the code, instead of resolving it here, was a defect in the approved gate — not an acceptable shortcut. It won't recur; the design now settles such facts itself. Source: revision record and dependency notes 3 and 4, `03-program-design.md`.
2. Accept the corrected TLS setup: the HTTPS library's newest version changed its secure-connection defaults. The design now uses the new default setup rather than the name the previous draft would have asked for, which does not exist any more and fails immediately if used. This was confirmed by a real HTTPS request, not just by reading the changelog. Source: dependency note 3.
3. Accept that this new default setup needs two things added to the container image that build it: a small C compiler toolchain (because the new default's cryptography component is a C library) and the operating system's list of trusted certificate authorities (because the new default reads the certificates already installed on the machine, instead of shipping its own). Without the second one, every outbound connection would fail. Source: dependency note 3; consequence recorded in slice 10's container requirements, `04-slices.md`.
4. Accept the corrected compression setting: the same real test showed that the previous draft's plan — "one path decompresses responses, the other path turns decompression off" — would not actually have worked, because decompression was never turned on in the first place for either path. The design now turns on exactly the three compression formats actually in use (and deliberately leaves a fourth, uncommon one off, since each one turned on is one more thing that could be abused as a decompression bomb), so the "turn it off" side of that pair now has something real to turn off. This is the one correction that would have caused a real difference in behavior — data delivered uncompressed rather than compressed — if it had reached working code; nothing else in this gate would have. Source: dependency note 2.
5. Accept the version-comparison library choice is now backed by evidence, not judgment. It's the same library the approved design already named; what changed is why. Its own documentation now states outright that it is the actively maintained continuation of an older, similarly named library that has gone quiet, and the publication dates confirm it. Because this choice affects whether a package release is correctly recognized as "newest," the design also adds a dedicated automated test that checks its ordering behavior against an independent reference list, so a defect in the library — not just the team's belief in it — would be caught rather than trusted. Source: dependency note 4; test named in slice 5, `04-slices.md`.
6. Accept the five previously unpinned supporting libraries are now pinned to specific versions, each with its license checked and confirmed compatible with how this service will be distributed. Leaving them unpinned was the same category of defect as items 2 and 5 above — a decision, not a detail — even though none of the five turned out to raise a concern once checked. Source: pinned-crate table, `03-program-design.md`.
7. Accept that nothing else changed: the modules, their responsibilities, the step-by-step request walkthroughs, the five threat findings and their fixes, and the full test plan you approved on 2026-09-17 stand exactly as they were.

## Gate document
`/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/03-program-design.md`

## Limitations
- This is a correction to Gate 3 only. It does not re-present the architecture, the module list, the request walkthroughs, or the threat model — those are unchanged from the version you already approved.
- Both library-choice facts were resolved from the libraries' own current documentation and one independent real-world test, not from a second, independent source; that is the same standard the rest of this gate's dependency table already uses.
- The version-comparison library's "actively maintained" claim comes from that library's own project page, which is one-sided; and neither library's correctness at comparing prerelease version numbers was independently verified — which is exactly why item 5 above adds a dedicated test rather than resting on the library's word.
- The two open ownership questions and the new configuration setting from the original Gate 3 approval are unaffected by this correction and remain as you already decided them on 2026-09-17.

## Recommendation
Approve the corrected Gate 3. Doing so reopens Gate 4 (the build-order plan) for its own approval immediately afterward, since it references this document.

## Context indicator
Observed today, before any product code existed: a real HTTPS request through the corrected setup, and a real failed request through the old, no-longer-valid setup name. Everything else above is still a paper plan for the next phase to build; no product behavior has been built or run.

## Source evidence
- Corrected gate document: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/03-program-design.md` — revision record, `## Dependencies`, the pinned-crate table, and dependency notes 2, 3 and 4.
- Consequences carried into the build plan: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/04-slices.md` — slice 5's added test, slice 10's added container requirements.
- Status and reopened boundary: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/00-status.md`.
- Prior presentations (voice and shape matched): `show-me-program-design.md`, `show-me-slices.md`.

Approve Gate 3, or what should change?
