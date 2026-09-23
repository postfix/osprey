## What problem do we have?

Slice 7 taught the firewall to serve real package files, hashing and verifying every byte before delivery. The first independent check of that work found it still had a gap identical in shape to the thing this whole product exists to prevent: a version withdrawn upstream — unpublished or yanked, including for being found malicious — kept serving its cached bytes indefinitely, because an upstream withdrawal is a different signal from this firewall's own blocklist. Worse, the address for a cached file is computed from public package information rather than issued as a secret, so anyone who knew a package's details before withdrawal could work out the address and fetch the file, having never been handed a link.

That gap is now closed and independently confirmed shut. What remains is a decision about what to build next: slice 8 adds heavy concurrency on top of exactly the code this fix just changed, and five pieces of work this plan approved have no slice row willing to own them — one of them created by this slice's own fix.

## How will we solve it?

The missing check — confirming a version is still offered by its project — now runs at both points the design always required: immediately before a download starts, and again right after any transfer completes, so a withdrawal landing mid-download is still caught. A second, smaller gap closed in the same pass: a version blocked only by a hash this firewall itself computed was still being listed as available, even though its bytes were already correctly refused.

The decision on the table is whether to carry five unowned obligations into slice 8 as-is, or to pause and give them owners first — reopening Gate 4 before continuing. Four were already known after slices 5 and 6; the fifth is new, and it is a direct cost of closing this slice's security hole: the firewall now re-parses a project's stored metadata on every artifact request, including ones already warm in cache, which test fixtures estimate as negligible on small packages but potentially tens of milliseconds on large real ones — enough to miss the latency target. No later slice row currently owns fixing that.

## How will we confirm it is solved?

- `cargo clippy --all-targets -- -D warnings`: exit 0, clean.
- `cargo test --test artifacts_verification`: ok, 9 passed. `cargo test --test blocklist_revocation`: ok, 8 passed. `cargo test --test persistence_content`: ok, 3 passed — all fifteen tests named in the approved slice row present byte for byte.
- Full suite: 180 passed, 0 failed, across 15 targets.
- All of the above re-run independently by the main agent on a forced rebuild, both before and after the fix.
- Three new tests were added for the fix, each proven to fail against the pre-fix code first, and probes confirmed each new mechanism is individually load-bearing on its own — not incidentally covered by something else.
- Six review verdicts, three mandatory for this slice: sf-code-review SHIP, and SHIP again on recheck. sf-adversarial-testing PASS. sf-security-review CLEAR on the revocation boundary, then CLOSED on the blocking finding after rework. sf-verification FAILED, then VERIFIED after rework. Two independent security reads concluded the guarantee behind the final check is structural — a code path that skips it does not compile.

These witnesses prove the delivered outcome (verified file serving, permanent hashes, working revocation) and prove the specific defect found and fixed (withdrawn versions no longer served). They do not, and are not meant to, resolve the five unowned obligations below — that is a planning decision, not a code defect.

## Decisions

- Slice 7 is complete and its outcome — verified, hash-pinned, revocable file delivery — is achieved; the security defect found in review is closed and independently confirmed closed, not merely claimed fixed.
- Do not read this as the product being currently vulnerable: the gap was found and closed within the same slice, before merge.
- A withdrawn version can still be served for up to the metadata refresh interval (5 minutes by default) — an intended staleness bound from the specification, not a residual defect.
- One known minor race was deliberately left unfixed: a listing can keep showing a newly blocked version for at most one refresh interval. It is self-healing and never affects which bytes are served.
- Two of slice 7's witnesses carry methodology caveats worth knowing, but they are limitations of the evidence, not part of the five unowned obligations: the durability-order test observes the sequence the code records rather than the operating system's own view, and the disk-full test approximates a full disk with a permissions trick.
- Five obligations now have no owning slice: three inherited from slices 5–6 (coalescing concurrent metadata refreshes, upstream conditional revalidation, four named Gate 3 tests never placed in a row), one minor uncommitted race from this slice, and one new performance cost this slice's own fix introduced (re-parsing project metadata on every artifact request, including warm ones).
- Slice 8 builds concurrency directly on top of the exact code this slice's fix just changed, so retrofitting the unowned items later gets structurally harder the longer they wait.
- This is the first slice in the plan where the witness passed and named reviews came back clean, yet independent verification still found a blocking security defect on its own — because the missing behaviour had no test (it had no code), and the slice row's own outcome text never mentioned it. Only the program design and the specification named it.
- Recommendation: re-steer — reopen Gate 4 before slice 8 and give all five obligations owners. Continuing to slice 8 is defensible on its own terms, but it carries five known obligations into the most concurrency-sensitive slice in the plan.

Full detail: `docs/plans/package-firewall-mvp/04-slices.md` (slice 7 row and rows 8–10); unowned-obligation ledger in `docs/plans/package-firewall-mvp/00-status.md`.

Continue to slice 8, or re-steer?
