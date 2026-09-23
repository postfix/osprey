# Gate 1 — Product: Package Firewall MVP

## What problem do we have?
Teams that install public npm and pip packages on developer machines and CI can pull a newly published, compromised release hours before any malware feed names it. Refusing all new releases outright would also break installs that could have used an older, still-good release. Even after a bad package is finally named, nothing today stops another machine from downloading it again.

## How will we solve it?
- Sit between installs and the public registries; hide any release younger than a chosen waiting period (24 hours by default) and anything on a malware blocklist.
- Let npm and pip keep choosing versions and owning lockfiles; the firewall only decides what is *available* to choose from.
- Never substitute different bytes or a different version for an exact request — an exact request for a refused release fails outright, with a reason.
- Once a release clears the wait, it becomes available on its own on the next request; once a delivered file is later named as bad, it is refused from then on, including through old links.
- Keep the operator surface minimal: a settings file, a blocklist file, and a decision log — no UI, publishing, login, search, audit endpoint, private packages, admin API, or multi-server setup.
- Stop handing out packages entirely if the blocklist goes missing or stale, rather than pretend everything is clean.

## How will we confirm it is solved?
- Planned, not yet observed: run an end-to-end install suite with real npm and pip clients against harmless test packages.
- Target: 0 delivered files younger than the waiting period or on the blocklist, and 100% of installs with an older acceptable release still succeed without any dependency or lockfile edits.
- Measured by the install suite's pass count plus the firewall's decision log for every delivered and refused file.
- Speed (a repeat request answered in a few milliseconds) is measured and reported, not promised as the headline success metric.

## Decisions this gate asks you to make
1. Accept the problem statement above as the product's reason to exist for this MVP.
2. Accept the success metric (0 disallowed files delivered; 100% of eligible installs succeed) as the bar for "done," with speed reported but secondary.
3. Accept the non-goals: no malware scanning/AI analysis, no version picking, no exact-request substitution, no reach into already-installed/cached/other-index packages, no UI/publishing/login/search/audit/admin surfaces, one shared policy per install.
4. Accept the ten product rules table (in `01-product.md`) governing what a client sees in each situation (too-young release, blocked exact request, frozen lockfile naming a blocked file, blocklist missing/stale, etc.) as the fixed client-facing contract.
5. Decide the product name: the specification calls it "Package Firewall"; the repository is named `osprey`. The product document uses "Package Firewall" throughout — confirm or override.
6. Decide whether a speed number should be the headline success metric instead of the reliability-based metric above (the product document currently treats speed as measured-and-reported only).
7. Accept that everything above is a plan: the repository currently contains no source code (only README.md, SPEC.md, LICENSE, .gitignore), so every confirmation is planned, not yet run.

## Gate document
`/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/01-product.md`

## Open product questions (from the gate document)
1. Product name — "Package Firewall" vs. repository name `osprey`.
2. Whether a speed target, not the reliability metric, should be the headline success metric.

## Limitations
- No code exists yet; nothing described above has been built or run. All confirmation steps are planned test-suite runs, not observed results.
- This presentation covers Gate 1 (Product) only. Architecture, storage, module, and endpoint decisions belong to Gates 2 and 3 and are out of scope here.

## Recommendation
The product document is internally consistent with `SPEC.md`'s product-facing sections and states its two open questions plainly. Proceed to a decision on this gate; resolve the two open questions (name, speed-vs-reliability headline) as part of that decision.

## Context indicator
Observed: repository has no source code (README.md, SPEC.md, LICENSE, .gitignore only) — confirmed via `docs/codebase-overview.md`. Everything above describing behavior, the success metric, and the ten product rules is the *plan* recorded in `01-product.md`; none of it has been executed or measured yet.

## Source evidence
- Gate document: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/01-product.md`
- Status/presentation setting: `/home/john/go/src/github.com/postfix/osprey/docs/plans/package-firewall-mvp/00-status.md` (presentation: auto)
- Source of intent: `/home/john/go/src/github.com/postfix/osprey/SPEC.md` (sections 1, 2, 5–8, 11, 13)
- Repository briefing: `/home/john/go/src/github.com/postfix/osprey/docs/codebase-overview.md`

Approve Gate 1, or what should change?
