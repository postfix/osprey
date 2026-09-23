## Delivered

Slice 8 closes the concurrency, cancellation, eviction and overload behaviour of the artifact
download path. Many developers asking for the same package file at the same moment now cause
exactly one download from upstream, not one per request. If one of them gives up, the others
still get their file; if all of them give up, the partial download and its reserved disk space
are cleaned up. Once the firewall has started publishing a verified file, it always finishes,
even if nobody is still waiting. A client that stops reading, or holds a response open
indefinitely, is cut off and its resources released, so one slow consumer cannot occupy capacity
forever. The cache clears space when it needs to, but never deletes a file a request is
currently reading. When the firewall is at capacity it refuses immediately rather than queuing
work it cannot do.

A second, smaller fix landed in the same pass: a setting that controls how long a slow client
may hold resources was publicly reachable by any program embedding this code as a library, with
no limit on how large it could be set — and those two deadlines are the only bound on that hold.
It is now compiled out of ordinary builds entirely.

## Proof

**The important part — a real defect was found after everything looked clean.** All of the
slice's own tests passed and the code review found nothing. The mandatory adversarial review
then reproduced, with a running probe rather than an argument, a permanent failure: if the code
handling a download crashed internally for any reason, every developer waiting for that file
hung forever, and so did every later request for the same file, until the whole service was
restarted. One internal crash disabled one package file permanently.

- The cause was specific: the crash skipped both the step that tells waiters the outcome and the
  step that clears the download slot, and the shared channel they waited on stayed open because
  the waiters themselves kept it alive.
- It was correctly bounded by the reviewer: capacity and reserved disk space were still
  released, and no wrong or unverified bytes could be served. It was a denial of service, not a
  correctness failure.
- It is fixed, with a guard that resolves and clears the slot on every way out — normal return,
  crash, or the task being discarded. The crash is still reported rather than hidden. The
  implementer also found and closed a hazard its own fix introduced, which would have been worse
  than the original problem: the guard takes a shared lock, and a crash while holding it would
  have broken every future download for every file.
- A new test covers it, proven to fail before the fix. An independent recheck rebuilt the
  reproduction from scratch rather than reusing that test, added an attack nobody had run, and
  confirmed the defect closed.

**Witness** (all re-run independently by the main agent on a forced rebuild, before and after the
fix):

- `cargo clippy --all-targets -- -D warnings`: exit 0, clean.
- `cargo test --test artifacts_concurrency`: ok, 10 passed — the nine tests named in the approved
  row plus the new one.
- Full suite: 196 passed, 0 failed across 16 targets, up from 180.

**Reviews:** sf-code-review SHIP, and SHIP again on recheck. sf-adversarial-testing returned FIX
with a reproduced defect, then PASS with it ruled closed. sf-verification VERIFIED.

**On the plan's recurring problem:** slices 5, 6 and 7 each left approved work with no owner, and
the count stands at five. Slice 8 added none. Verification ruled every candidate: the one
deferred item is genuinely owned by slice 9, which owns the error-behaviour table, and the eight
gaps the adversarial review listed are extra depth on mechanisms this slice already delivers,
each traced to the specific construct that closes it. The last piece of the approved design that
had never been built anywhere — the capacity check at the front door — was also implemented
here.

Full detail, file paths, test names and command lines: `docs/plans/package-firewall-mvp/04-slices.md`
row 8, and the slice 8 proof line in `docs/plans/package-firewall-mvp/00-status.md`.

## Limits

- One of the nine tests proves the timing bound it claims but not the internal priority rule
  behind it; the test says so itself.
- One known minor issue is accepted and unfixed: a request arriving in a narrow window may
  receive a cached failure rather than starting a fresh attempt. Self-healing, no effect on which
  bytes are served, and it pre-dates this slice's fix.
- Eight areas were reasoned through rather than forced by a test, including a crash during the
  publishing step itself.
- Five approved obligations, all inherited from slices 5 through 7, still have no owning slice
  row. Slice 8 added none of them.

## Next

Continue to slice 9. Slice 8 is complete, its one real defect is closed and independently
confirmed, and it is the first slice since slice 4 to add nothing to the plan's outstanding work.
Slice 9 is the error-behaviour contract, which is also where the one item deferred from this
slice lands, so continuing puts that work in the hands of the slice that owns it. The five older
unowned obligations remain recorded and still have no owner; they are worth raising again before
the final slice, not now.

## Recommendation

Continue to slice 9 as planned in `docs/plans/package-firewall-mvp/04-slices.md`.

Continue to slice 9, or re-steer?
