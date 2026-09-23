# Package Firewall MVP — feature completion

## What problem do we have?

Developers and CI jobs install whatever a public registry published a minute ago. When a
popular package is hijacked, the bad release gets installed hours before any malware feed
names it. Refusing all new releases outright breaks installs that would have been fine.
And once a feed finally does name something bad, nothing stops the next machine from
pulling it anyway.

## How will we solve it?

Put a firewall between developers and the public npm and PyPI registries. Point `npm` and
`pip` at it and install normally. It refuses anything an operator has put on a blocklist,
holds brand-new releases back for a configurable cooldown so a hijacked publish is not
installed the minute it appears, checks the integrity of every file against its published
fingerprint before a single byte reaches a client, and refuses to serve rather than
pretend things are fine when its own view of the world (blocklist, or now cached package
information) has gone stale. It keeps working from its own local cache so repeat requests
are fast.

## How will we confirm it is solved?

Observed, not promised:

- Real clients install through it end to end: npm 12.0.2 (current stable) and npm 10.9.9
  (previous major), pip 26.2.1 (current stable) and pip 25.0.1 (previous minor), each
  driven against a running instance, in tests that fail if the install fails.
- 276 automated tests pass, 0 fail, in a debug build (all counts in this document are debug
  unless stated otherwise).
- All nine of the specification's performance targets (SPEC §12) are met, measured over
  five runs each and reported as ranges rather than single numbers, because a single run on
  shared hardware was shown to be unreliable (see below).
- It builds and runs as a container, and reaches the real npm registry over TLS using the
  container's own certificate roots, with no certificate error.
- Four gates approved, fifteen slices built, each with an independent witness and named
  reviews (code review, adversarial testing, security review, independent verification)
  chosen by what the slice touched.

## Decisions

1. **A concurrency defect survived a passing witness and a clean code review.** A panic
   inside the artifact transfer task unwound past the code that would have released a
   shared per-reference slot, wedging it forever — every present and future request for
   that one reference hung. Only the mandatory adversarial review reproduced it; the fix is
   a drop guard that always releases the slot on any exit path, including a panic.
2. **A slice passed its witness and two clean reviews, then failed independent
   verification.** Slice 7's artifact download path never re-checked that a version was
   still advertised by upstream metadata before serving cached bytes, an obligation stated
   only in the design document, not tested anywhere because no code existed for it to test.
   A version pulled upstream could keep being served indefinitely from cache. Fixed before
   the slice was recorded.
3. **The current npm client refused every install through the firewall**, discovered only
   by testing against the actual current npm (12.0.2), not the version already on the
   build machine. npm 12 requires artifact URLs to sit under the registry's own path
   prefix or it treats them as an untrusted remote dependency and blocks them. The fix
   moved file downloads under each ecosystem's own address (`/npm/artifacts/`,
   `/pypi/artifacts/`); the alternative — telling operators to pass
   `--allow-remote=all` — was rejected because it disables one of npm's own supply-chain
   protections for every dependency, which defeats the point of a supply-chain firewall.
4. **A safeguard depended on two statements being written in the right order**, with
   nothing in the test suite able to catch a future edit that got the order wrong. An
   adversarial pass proved the risk by making the mistake and watching nothing fail. Fixed
   structurally — the API was replaced so the wrong order can no longer be expressed in
   code, not just tested against.
5. **The performance benchmark itself was measuring the host machine, not the firewall.**
   Identical code swung from 56% to 252% of baseline throughput across runs. It took three
   rounds to make honest: a reviewer read the instrument rather than trusting its output
   and found asymmetric warm-up; a first fix didn't settle it; paired interleaved sampling
   (baseline and firewall alternating in one loop, so drift lands on both) collapsed the
   spread to a 4-point band across a 5x range of host load.
6. **Two obligations remain open**, both written into `docs/operations.md` as known gaps
   rather than hidden: the repository has no gate of its own that turns compiler warnings
   into build failures, so a warning-level regression (demonstrated concretely by swapping
   a guarded cache write for an unguarded one) can pass an ordinary `cargo build` or
   `cargo clippy`; and the test suite has never been green in release mode — one timing
   test calibrated to debug-build speed fails under `--release` because optimized code is
   faster than the margin it asserts.
7. **A cached copy of package metadata can be served up to 30 seconds past its configured
   maximum age**, in the narrow case where a request joins a refresh that is already in
   flight. The specification was amended on 2026-09-20 to state that precise bound, after
   the user was shown the tradeoff and chose to accept the bound over changing the code;
   independent verification had recommended closing the gap in code instead. The overshoot
   never advances the stored "last fetched" time and cannot compound across requests.
8. **The benchmark still compares against a baseline that does not read ahead the way the
   firewall does**, which is why measured throughput sits near 110% rather than closer to
   100%. This is disclosed by file and line in `docs/operations.md`, not corrected, because
   changing the instrument until it produces a preferred number is the exact failure mode
   the earlier benchmark rework was fixing.
9. **All test counts recorded across this plan are from a debug build**; the suite has
   never been run to green in release (see decision 6).

Document of record: `docs/plans/package-firewall-mvp/00-status.md`

## Limits

- No free-space check/alarm, no online backup, no administrative API, and state grows
  monotonically with no pruning — all accepted operational tradeoffs documented in
  `docs/operations.md`.
- `SIGTERM` outside the container has no graceful-shutdown handler; only `SIGINT` does. The
  container image works around this with `STOPSIGNAL SIGINT`.
- A database written by an earlier build is refused outright rather than migrated.
- No -D warnings gate in the repository itself (item 6 above); no green release test run
  (item 6 above); metadata age-ceiling joiner overshoot up to one upstream timeout (item 7
  above); benchmark baseline asymmetry (item 8 above).

## Next

Optional logging to a local file and to a SIEM is already agreed as a separate, later
four-gate plan. One question it must settle early: today the firewall deliberately records
nothing a client sends, so it can say a blocked package was served at a given time but not
to which machine. If logs are meant to drive clean-up after an incident, that no-client-data
rule has to be revisited on purpose, not by default.

## Recommendation

Ship it. Every gate is approved, every slice is checked with a recorded, independently
re-run proof line, and every defect this plan found was found before it shipped — by
adversarial testing, by independent verification, or by the user directly — never after.
The remaining open items are disclosed, bounded, and owned in `docs/operations.md`, not
silent.

Complete the feature, or re-steer?
