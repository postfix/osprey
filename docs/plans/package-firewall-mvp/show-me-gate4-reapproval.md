## What problem do we have?

npm 12.0.2 — the current upstream stable release — refuses every install that goes through this
firewall. It fails with `npm error code EALLOWREMOTE` because npm 12 changed its default policy for
tarballs it does not recognize as belonging to the registry it is talking to, and this firewall's
current layout does not qualify: metadata is served under `/npm/` but artifact bytes are served under
a separate, unrelated `/artifacts/` root. This is an observed, reproduced failure against real npm
12.0.2, not a hypothetical: the same test suite passes when npm's stricter check is manually disabled
with `--allow-remote=all`, which confirms the cause and was rejected as the fix because it would turn
off npm 12's supply-chain protection for every tarball in a project — self-defeating for a product
whose entire purpose is a supply-chain firewall.

Gate 3 was already reopened and re-approved on 2026-09-19 to make the routing decision that fixes
this. Gate 4 must now be reopened because no existing slice row owns making that change to the code.

## How will we solve it?

A new slice 14 moves artifact URLs under each ecosystem's own path root — `/npm/artifacts/...` and
`/pypi/artifacts/...` — replacing the single shared `/artifacts/...` root. This gives npm 12 the path
prefix it requires to trust its own tarballs again, with no `.npmrc` override needed on the client
side. Nothing about identity or trust moves: reference IDs, digest pins, and first-seen times are
computed from ecosystem, name, version, filename, and upstream data — never from the public path — so
none of them rotate or invalidate. The one new check added is that the artifact handler now confirms
the ecosystem named in the URL matches the ecosystem recorded on the stored reference before serving
it, closing a decision-log integrity concern (a reference ID is a public, computable value, not an
access credential, so this check protects accurate logging, not authorization).

Slices 1 through 9 are complete and untouched by this revision; slice 10 (delivery) is built but
deliberately left unchecked. This revision changes what remains to be built, not what has already
shipped:

- Build order changes from `9 → 11 → 12 → 13 → 10` to `9 → 11 → 12 → 13 → 14 → 10`. Slice 14 goes
  before slice 10 because slice 10 is the delivery slice and re-runs every client and benchmark
  witness against final, post-14 code.
- Slice 14's file list was produced by tracing the actual call sites in the current tree, not by
  reusing Gate 3's older list, and it is materially larger: Gate 3 assumed most integration tests
  would "follow automatically," but only two test call sites actually do that. Seven test files need
  forced edits — four hardcode the old `/artifacts/` path literally, and five parse the reference ID
  out of the URL by position, which would silently read the wrong path segment rather than fail once
  a new segment is added.
- Row 10 (delivery) is corrected in three places: its dependencies now include slice 14; a factual
  error calling npm 11.16.0 / pip 25.1.1 "the current pair" is replaced with four exact, pinned,
  out-of-tree client versions (npm 10.9.9, npm 12.0.2, pip 25.0.1, pip 26.2.1); and its witnesses are
  now stated to run against final code built after both slice 13 and slice 14.
- `docs/operations.md`'s subsection telling operators to work around this with `.npmrc`
  `allow-remote=all` and to prefer pinning to npm 11 becomes false the moment slice 14 ships, and slice
  14's file list now names that whole subsection for rewriting rather than a line patch, since no
  witness elsewhere in the plan checks prose for staleness.

No product code for slice 14 exists yet. This section describes what is planned; the routing defect
above is the only part of this that has actually been observed against running code.

## How will we confirm it is solved?

Slice 14's witness set was reviewed by a fourth Red Team pass (2026-09-19) that returned NEEDS
REVISION with two MAJOR findings, both resolved directly in the revised document before this
presentation was written:

- The two planned cross-ecosystem tests (an npm reference requested under the PyPI artifact root, and
  vice versa) would have passed today for the wrong reason: neither new route exists yet, so both
  requests already return 404 through the router's generic unknown-route fallback. A bare
  status-code check could never tell a working ecosystem check apart from a route that simply isn't
  registered. Both tests now assert three things together: the 404 status, that the logged ecosystem
  came from the verified database row (not the URL), and that the refusal came from the artifact
  handler itself rather than the router's fallback.
- The stale `docs/operations.md` subsection (the one recommending the `allow-remote=all` workaround)
  is now explicitly named for a full rewrite rather than a partial edit, since it would otherwise keep
  telling operators to defeat the fix this slice ships.

The decisive end-to-end confirmation planned for slice 14 is a real npm 12.0.2 install, with no
`--allow-remote` flag and no `.npmrc` override, succeeding against the new routes — the exact scenario
that fails today. PyPI's route also moves for symmetry (not because pip is broken) and is re-verified
with both a current and a previous pinned pip version.

## Decisions

1. Reopen Gate 4 to add slice 14 (artifact routing under the ecosystem root) between slice 13 and
   slice 10, changing the build order from `9 → 11 → 12 → 13 → 10` to `9 → 11 → 12 → 13 → 14 → 10`.
2. Treat the npm 12.0.2 `EALLOWREMOTE` failure as a design decision to correct (SPEC deviation), not
   an implementation bug — it is reproducible on every deployment and confirmed by the
   `--allow-remote=all` control, which was rejected as a fix rather than adopted.
3. Move artifact URLs from a shared `/artifacts/...` root to per-ecosystem roots,
   `/npm/artifacts/...` and `/pypi/artifacts/...`, with no change to reference-ID computation, digest
   pinning, or first-seen timestamps.
4. Add an ecosystem-match check in the artifact handler: the URL's ecosystem segment must match the
   ecosystem on the stored reference row, or the request is refused with 404 — a decision-log
   integrity fix (TM-R1), not an authorization change, since policy already reads ecosystem from the
   stored row.
5. Accept slice 14's file list as produced by an `sf-impact` trace against current source rather than
   reusing Gate 3's older list, since the trace found the older list materially incomplete (missing
   seven test files with forced edits) and partly wrong (two citation errors).
6. Require slice 14's two cross-ecosystem tests to each assert status, logged ecosystem, and
   handler-vs-fallback refusal together, so the tests cannot pass today for the wrong reason before
   the routes exist.
7. Require slice 14 to fully rewrite the now-stale `docs/operations.md` subsection recommending the
   `allow-remote=all` workaround and pinning to npm 11, rather than leave it in place.
8. Correct row 10 (delivery, already built but unchecked): dependencies now `9, 13, 14`; drop the
   false "current pair" claim about npm 11.16.0 / pip 25.1.1 in favor of four exact pinned client
   versions; and state that its witnesses re-run against code built after both slice 13 and slice 14.
9. Leave slices 1-9 and slice 10's existing (unchecked) build entirely unre-presented and unchanged by
   this revision — only what remains to be built changes.

## Limitations

- No product code for slice 14 exists; the routing fix, the ecosystem-match check, and the rewritten
  cross-ecosystem tests are all planned work, not built or run yet. Only the underlying `EALLOWREMOTE`
  failure and its `--allow-remote=all` control are observed evidence.
- The fourth Red Team pass could not invoke a threat-modeling tool directly from inside its own
  context and reasoned the threat question by hand instead (labeled MODELED in the source document);
  it treated this as an acceptable weaker instrument only because Gate 3's threat model for this exact
  change is already approved and on file.
- The Red Team's check that no already-completed slice's proof lines become false assumes slice 14
  edits existing test assertions in place rather than adding or removing tests — the source document
  states this is implied by row 14's wording but not stated outright.

## Recommendation

Approve. The failure is reproduced against real npm 12.0.2, the fix directly targets the confirmed
cause without disabling npm's supply-chain protection, the routing decision was already vetted through
a Gate 3 reopening, and the fourth Red Team pass's two MAJOR findings are both resolved in the current
document with witnesses that cannot pass before the fix is built.

Context indicator: MODELED — the pre-implementation risk assessment for this revision rests on Red
Team's hand-reasoned threat analysis (no fresh tool invocation) built on Gate 3's separately approved
threat model, and on static tracing of source rather than any run of code that does not yet exist.

Gate document: `docs/plans/package-firewall-mvp/04-slices.md`

Sources: `docs/plans/package-firewall-mvp/04-slices.md` lines 13, 15, 36, 40, 71, 77-84;
`docs/plans/package-firewall-mvp/00-status.md` lines 44-51, 66-69.

Approve Gate 4, or what should change?
