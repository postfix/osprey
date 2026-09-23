# Slice 3 — The Firewall Remembers

## What problem do we have?

Until this slice, the firewall started every run with an empty memory. It kept nothing between
runs, so each restart began with no blocklist at all — and the product rule for "no valid
blocklist" is that **all package delivery stops**. Every restart therefore meant a gap in which
installs got a refusal instead of packages, lasting until a blocklist was read and accepted again.

Three more gaps sat beside it. Nothing stopped a second copy of the firewall being started against
the same data folder. Nothing said what should happen when the firewall's own stored state came
back unreadable — repair it, start fresh, or stop. And a replaced blocklist file was only noticed
by luck of timing, when the product promises an update "takes effect within seconds with no
restart".

## How will we solve it?

Four behaviours, all of them now working:

- **It remembers its blocklist across a restart.** When the firewall comes back up it enforces the
  blocklist it already accepted — while that blocklist is still within its expiry — without waiting
  to re-fetch or re-read anything first. A blocklist is stored durably *before* the firewall begins
  enforcing from it, so there is no moment where it is enforcing something it would forget.
- **It refuses to run twice against one data folder.** A second copy started against the same data
  folder is turned away rather than allowed to interleave with the first.
- **When its stored state cannot be trusted, it stops handing out packages and stops there.** The
  process stays up and answers the alive signal, its ready signal stays false, every package request
  is refused, and it leaves the stored bytes exactly as it found them. It never quietly starts
  fresh — because starting fresh would throw away a blocklist an operator may still be able to
  recover.
- **It picks up a replaced blocklist within two polling intervals** — including the awkward case of
  a replacement written in the same second, at the same length, as the file it replaced.

## How will we confirm it is solved?

It is already confirmed. These were **observed**, not planned: the witnesses for this slice ran and
passed, and the whole result was then re-run independently rather than taken on the implementer's
word.

| What the slice promised | What was observed |
| --- | --- |
| State survives a restart; stored before it is enforced | Passed, including a case run against both a healthy and a deliberately unopenable store (test evidence) |
| Failed recovery leaves the process alive, not ready, and never recreates the store | Passed, including a case proving an emptied store beside a recoverable log is refused, not rebuilt (test evidence) |
| A replaced blocklist file is noticed within two intervals | Passed, including the same-second, same-length replacement and an in-place rewrite |
| Everything still green | 50 checks passed, 0 failed, across the whole project — no earlier slice broken |

**Two real defects were found by review after the implementation had reported success. Both were
fixed and re-verified.**

- One would have let a **blocked package stay available for up to a minute** after a momentary disk
  problem: the firewall marked an accepted blocklist as handled before it was actually stored, so a
  failed store silently dropped it until the next cycle. The fix makes it keep retrying the same
  blocklist, and the fix was confirmed by re-breaking the code and watching the check object.
- The other would have **silently destroyed a still-recoverable blocklist** when the main store file
  was emptied or deleted while its write-ahead log survived — the old behaviour rebuilt from scratch
  and logged nothing. This one was proven rather than argued: the state was successfully recovered
  before the fix was in place, and correctly refused afterwards.

**Limits worth knowing.** Durability against a real power cut is **not** proven — no torn-write or
power-loss simulation was possible on this machine. What *was* shown is that the firewall forces its
writes to disk before it reports a blocklist committed, and that it survived 28 real process kills
(test evidence). Separately, one internal check recognises a retryable condition by matching an
error message rather than a proper signal; it fails safe — the worst case is one delayed cycle — but
it is owed a sturdier fix in a later slice.

## Decisions taken

- A blocklist is committed to durable storage **before** it is published for enforcement, never
  after.
- Untrustworthy stored state means **refuse and preserve**, never repair-in-place and never rebuild;
  operator recovery is worth more than an automatic fresh start.
- A failed store **retries the same accepted blocklist** instead of treating it as handled.
- **One process per data folder**, enforced by an exclusive lock taken at startup, not by convention.
- A firewall that cannot trust its state **stays up and refuses**, rather than exiting — so the
  failure is visible to whoever is watching it, and is not mistaken for a crash loop.
- Change detection on the blocklist file uses several independent file properties plus a periodic
  forced re-read, so the same-second, same-length replacement cannot slip through.
- Database access is confined to one area of the code, and that confinement is checked by a
  repeatable search rather than trusted.
- Store health is **not** part of the ready signal yet; that check is explicitly owed by a later
  slice (artifact verification and content cache).
- The pre-1.0 storage engine's open question from the design gate was answered empirically against
  the exact pinned version rather than from documentation.

## Where the detail lives

Full detail — every witness command and its output, the complete defect record, the scope deviation
on one shared file, and the full limitation list — is in the slice 3 proof line of:

`docs/plans/package-firewall-mvp/00-status.md`

Next up is slice 4, the outbound boundary: the firewall reaches a public registry only through one
controlled path, and every attempt to send it somewhere else — a redirect off-origin, credentials in
a URL, an odd port, a private address, a package name carrying a path or a scheme — is refused
before a byte leaves the process.

---

**Context indicator:** presentation setting `auto`, resolved to Markdown (the stored
`## Factory settings` value in `docs/plans/package-firewall-mvp/00-status.md:8-10`).

**Source evidence (compact):**
- `docs/plans/package-firewall-mvp/00-status.md:23` — slice 3 checked, with its full proof line:
  witness commands and results, the SMTC analyzer-spec verdict, the MAJOR and MEDIUM defects and
  their fixes, and the limitations quoted above; reviews `sf-code-review=SHIP after rework`,
  `sf-adversarial-testing=closed and clean`
- `docs/plans/package-firewall-mvp/04-slices.md:19` — slice 3's approved row: the promise this slice
  had to meet, and its named witnesses
- `docs/plans/package-firewall-mvp/04-slices.md:20` — slice 4's approved row, the source for "next up"
- `docs/plans/package-firewall-mvp/01-product.md:26,41-42` — the product language used here:
  "all package delivery stops until a valid one is present", "takes effect within seconds with no
  restart", "the last good one keeps working and the problem is logged"
- `SPEC.md:8-16` — product intent

Durable boundary recorded. To clear context, run /clear (or /new), then software-factory resume. To proceed now, reply continue.
Continue to slice 4, or re-steer?
