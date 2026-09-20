# Slice 6 result: PyPI metadata end to end

## Delivered

PyPI now works the same way npm has since slice 5. A request for a project's file listing comes
back filtered, in HTML or JSON depending on what the client asks for, with held or blocked
versions never appearing.

- Requesting a project under a non-canonical spelling of its name is redirected once to the
  canonical address.
- Every file is judged by its own upload time, not the release it belongs to — a new file added
  to an old release serves its own waiting period.
- A file whose identity the parser cannot pin down is left out of the listing and logged, never
  guessed at.
- A project that exists but currently has nothing eligible returns a valid empty listing, not an
  error.

## Proof

Re-run independently by the main agent on a forced rebuild, not taken from the implementer's report:

- `cargo clippy --all-targets -- -D warnings` — exit 0, clean.
- `cargo test --test pypi_metadata` — ok, 12 passed, including all six tests named in the approved
  slice row (`html_and_json_list_the_same_files`, `new_wheel_does_not_inherit_old_sdist_age`,
  `unsupported_filename_excluded_and_logged`, `html_escapes_filenames_and_attributes`,
  `existing_project_with_no_eligible_files_returns_empty_listing`,
  `adversarial_multi_separator_filenames`).

Reviews: `sf-code-review` SHIP, no defects. `sf-adversarial-testing` PASS, no defects reproduced.
`sf-verification` VERIFIED.

The threat this slice existed to survive (`03-program-design.md` TM-2): the PyPI filename parser
is a hand-rolled splitter with no maintained crate to lean on, and the program design named it as
the plan's most likely place for a silent identity error — a file attributed to the wrong version
would let a version-specific block be escaped silently. Two independent lines of attack failed to
break it: two reviewers separately derived, on different grounds, why an ambiguous filename cannot
be silently resolved, and a direct adversarial corpus — project names that are prefixes of other
project names, project names shaped like version numbers, build tags against local version
segments, epoch versions, uppercase extensions, embedded whitespace — either resolved to the one
correct project or was excluded outright. No input attributed a file to a different version.

Test evidence, not product behaviour, that reviewers checked was load-bearing rather than
decorative:
- The HTML-escaping test was proven load-bearing by mutation: making both escaping functions
  return their input unchanged made `html_escapes_filenames_and_attributes` fail.
- Every hostile-path assertion carries forward slice 4's rule — it bypasses the ordinary HTTP
  client via a raw `TcpStream` helper, because `reqwest` would otherwise clean the attack up
  before it reaches the server and silently test nothing.

One risk closed outright: a lookalike-Unicode project name cannot impersonate another, because
non-ASCII characters are rejected before normalisation ever runs.

## Limits

- The "no ambiguous filename is silently resolved" argument is a hand proof, independently
  re-derived by two reviewers on different grounds — not a machine-checked property test.
- The three obligations carried forward after slice 5 (concurrent-refresh coalescing, upstream
  conditional revalidation, four named Gate 3 tests) were confirmed to apply to both npm and PyPI
  by the same root cause, but the PyPI half of that confirmation was established by reading the
  code, not by re-measuring live traffic the way slice 5's npm side was measured.
- Slice 6 added no fourth unowned obligation; the plan's notes have been corrected to record the
  three as ecosystem-wide rather than npm-only.

Full detail: `docs/plans/package-firewall-mvp/04-slices.md` (slice 6 row), status notes in
`docs/plans/package-firewall-mvp/00-status.md`.

## Next

Slice 7 is artifact verification and the content cache — the first slice that serves actual file
bytes and pins digests, carrying three mandatory reviews (code review on pin permanence,
adversarial testing on durability and commit order, security review on the revocation boundary).
It is the highest-consequence slice remaining in the plan.

## Recommendation

Continue to slice 7. Slice 6 is complete and clean, it left nothing new unowned, and the three
carried-forward obligations are durably recorded rather than lost.

Continue to slice 7, or re-steer?
