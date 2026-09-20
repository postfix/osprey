# Gate 4 — 2026-09-19 revision: artifact routing under the ecosystem root

## What problem do we have?

npm 12.0.2 — the current stable release of npm — cannot install anything through this firewall. Every install fails, on every project, with `npm error code EALLOWREMOTE`. This is not a quirk of the test machine and not an implementation bug in code we wrote: npm 12 changed a supply-chain default (`allow-remote` now defaults to `none`), and it only trusts a registry's own downloads when their URL path starts with that registry's own path prefix. This firewall currently serves package metadata at `/npm/…` but the downloaded files at a separate top-level `/artifacts/…`, so the prefix never matches and npm 12 refuses every download as if it came from an untrusted third party.

This affects every deployment of the firewall against a current npm client, not just this repository's test environment. It was found by measurement (`npm error code EALLOWREMOTE` reproduced against real npm 12.0.2), confirmed by control (the same installs succeed with the workaround `--allow-remote=all`, which is rejected below), and is not yet fixed — the fix is planned in this revision (row 14) and has not been built.

## How will we solve it?

Move where downloaded package files are served from: instead of one shared `/artifacts/{id}/{filename}` for both ecosystems, each ecosystem serves its own artifacts under its own path — `/npm/artifacts/{id}/{filename}` and `/pypi/artifacts/{id}/{filename}`. That satisfies npm 12's prefix rule because the download path now starts with the same `/npm/` prefix as the metadata that named it.

Because a download URL now claims an ecosystem, the firewall adds a check that didn't exist before: when a file is requested, it confirms the ecosystem named in the URL matches the ecosystem recorded for that file internally, and refuses the request (`404`) if they disagree. Without this check, a reference copied from an npm download link and pasted under the PyPI path would be served and logged as if it were a PyPI file — not a security bypass (the firewall's allow/block decision doesn't depend on the URL), but a bookkeeping error worth closing while the path shape is already changing.

Nothing about the firewall's stored records moves or changes: which files are known, their pinned checksums, and when they were first seen are computed independently of the public path and are untouched by this revision — this is a routing and address change only, not a data migration.

## How will we confirm it is solved?

The decisive, falsifiable witness: a real npm 12.0.2 client, run with no `--allow-remote` flag and no `.npmrc` override, must successfully install a package through this firewall. That exact install fails today (observed) and is the one behavior this revision exists to restore (planned, not yet built).

Supporting witnesses, once built:
- A PyPI install with real pip clients against the new `/pypi/artifacts/…` route, because PyPI's move is for symmetry, not necessity, and none of the existing pip evidence was gathered against this new layout.
- Two cross-ecosystem tests confirming a file requested under the wrong ecosystem's path is refused, is logged with the ecosystem it actually belongs to, and is refused by the new check specifically — not by the router simply not recognizing the route (see decisions below).
- A package or project literally named `artifacts`, on both ecosystems, still resolves correctly and doesn't collide with the new route shape.

Test fixture packages and probes used to build this confidence are test evidence only — none of it is a product guarantee on its own; the npm 12.0.2 install is the load-bearing witness.

## Decisions

- Build order for this revision is fixed: slice 9 → 11 → 12 → 13 → 14 → 10, with slice 14 (this routing fix) built before slice 10 (delivery) so slice 10's final benchmarks and client installs run against the finished code, not an intermediate state.
- Both npm and PyPI move to the new artifact path shape, even though only npm's client behavior requires it — this avoids a permanent, accidental shape difference between the two ecosystems and fixes an existing decision-log defect (artifact requests currently can't be reliably attributed to the ecosystem that made the policy decision).
- The obvious client-side workaround — telling operators to set `allow-remote=all` in `.npmrc` — was rejected. It doesn't scope to this firewall: it disables npm 12's supply-chain protection for every tarball dependency in the whole project, which defeats the purpose of running a supply-chain firewall in the first place.
- Red Team found that the two planned cross-ecosystem refusal tests would have passed today for the wrong reason: neither new route exists yet, so both would simply hit the router's generic "unknown route" fallback and return 404 regardless of whether the new ecosystem check works. The tests were rewritten to also assert the request was refused by the new check itself, not the fallback.
- This revision amends the URL form the specification (`SPEC.md`) states literally for artifact downloads. That is the one open question in this revision the reader alone can settle — see below.
- Nothing about stored reference records, pins, or first-seen timestamps changes or migrates; this is purely a change to where files are addressed and served from.
- Gate document under review: `docs/plans/package-firewall-mvp/04-slices.md` (the 2026-09-19 revision-record paragraphs, new row 14, corrected row 10 cells, and the Red Team pass 4 record). Rows 1–13 were approved earlier and are not reopened by this decision.

## Limitations

- The npm 12 failure is directly observed and measured; the fix itself (row 14) is planned, not yet built or witnessed.
- Red Team's threat reasoning for this revision was done by hand rather than through the dedicated threat-modeling tool, because that tool wasn't available in its session — recorded as a weaker instrument, not treated as a gap, since Gate 3's original threat model for this exact change is already approved.

## Recommendation

Approve. The problem is real and affects every deployment on a current npm client; the fix is narrowly scoped, doesn't touch stored state, and its main design risk (the cross-ecosystem attribution gap) was caught and closed by Red Team before this was presented. The only decision left for the reader is whether to amend `SPEC.md`'s stated artifact URL form to match, or record this as a dated deviation instead.

**Context:** grounded in `docs/plans/package-firewall-mvp/04-slices.md` (revision-record paragraphs, row 14, corrected row 10, Red Team pass 4 and its four-row findings table) and `docs/plans/package-firewall-mvp/03-program-design.md:972-1110` (the approved design this implements), cross-checked against `docs/plans/package-firewall-mvp/00-status.md`.

Approve Gate 4, or what should change?
