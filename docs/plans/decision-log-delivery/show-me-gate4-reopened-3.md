## What problem do we have?
On shutdown the file sink stops draining after a 5-second deadline. Any records still queued at
that point must be added to the drop counter, so the operator can see that records were lost
(Gate 3 C42). Slice 3 contains that fix, and `TP-19` was approved as its witness. That witness can
never fail. It assumed `timeout(Duration::ZERO)` fires on the first poll. In fact tokio fires it at
the next millisecond tick. Three records are written in about 200µs, so the test passed 300 of 300
runs against the code **without** the fix (Gate 3 C46). As things stand, slice 3 could ship with
"records are never discarded silently" backed by a check that would still pass if the fix were
removed.

## How will we solve it?
Slices 1 and 2 have shipped and are history, not part of this decision. This revision changes only
slice 3's `TP-19` step. Nothing changes in its APIs, product code, file list or signatures.

| Slice | Caller outcome | APIs used/changed | Available dependency | Deciding check | Temporary limit / removal |
|---|---|---|---|---|---|
| 3 (live) | Opt-in peer IP on every record. File drain cut-offs are counted. | Unchanged from the 2026-09-22 approval. Private `drain(writer, rx, drops, deadline)` in `src/delivery/file.rs` adds `rx.len() + 1` to `drops` on timeout | tokio 1.53.1. The `rsync`, `sed` and `grep` used by 6b are present | Step 6 is now 6a + 6b (below) | 6b needs a full scratch build in `/tmp`, deleted by the same command |

**C47: witness step 6 is split in two**

- **6a** `cargo test --lib tp19_file_drain_deadline_counts_cut_off_records`, which must report
  `1 passed`.
- **6b** Run the same test on a copy of the tree with the fix removed. `rsync` the tree to
  `/tmp/osprey-tp19-prefix`. `sed '/if drained.is_err() {/,/^    }$/d'` deletes the three-line
  accounting branch (a dry run confirmed this is exactly `file.rs` lines 74-76). A
  `grep -c drained.is_err` = 0 guard follows. The build then goes to `/tmp/osprey-tp19-target`.
  The command prints `TP19-DISCRIMINATES` only when the output contains `1 failed` **and** does
  not contain `inconclusive`. After that it prints `test result: FAILED. 0 passed; 1 failed` and
  deletes all three `/tmp` paths.

**New test body (Gate 3 C46 form)**

The test queues **10,000** records on a channel large enough to hold them all, then awaits
`drain(.., Duration::ZERO)`. It checks, in order:
1. lines < 10,000, with the message `inconclusive: no cut-off happened` if not. This guards
   against a vacuous pass.
2. lines + `drops` ∈ {10000, 10001}. The one-high residual is the accepted overcount.

Gate 3 measured this with 10,000 records. Without the fix: 8 written + 0 dropped, so (2) fails.
With the fix: 10 + 9,991 = 10,001.

**Cleanups from QA (no new decisions)**
- `tp16`, `tp17` and `tp5c` are restated as regression checks. They were red at the first slice 3
  build and are now in the tree.
- C34 and C44 are marked superseded by C47.
- The slice 3 `Design:` line now lists `#### build`, the four threat rows and TP-16/17/18.

Red Team was not triggered because only a test's form changes (Gate 3 recorded the same for C46).

## How will we confirm it is solved?

| Scenario | Expected result | Check | State |
|---|---|---|---|
| 6b on today's tree (old 3-record test) | red: no `TP19-DISCRIMINATES` | main agent ran it 2026-09-22 | **observed:** `test result: ok. 1 passed`, no `TP19-DISCRIMINATES`. Step 6 is red today |
| `sed` targets only the fix | removes `file.rs` 74-76 only | dry run on a copy; QA read lines 74-76 | **observed** |
| 6a after the test rewrite | `1 passed` | `cargo test --lib tp19_...` | planned |
| 6b after the test rewrite | `TP19-DISCRIMINATES`, `1 failed`, not `inconclusive` | 6b command | planned (rests on Gate 3's 8-written/0-dropped measurement) |
| Steps 1-5 (`tp4`, `tp13`, `tp16`, `tp17`, `tp5c`, clippy, TP-14, TP-18) | pass | unchanged from the prior approval | planned regression |

Gate QA re-ran fresh and returned READY (`gate-4-qa.md`). It found F1-F4 closed and no new
findings. `router gate-qa verify` returned `{"verdict":"READY","files":18}`.

Recommendation: approve. C47 makes step 6 red today and fail-closed: the `grep` guard catches a
failed mutation, the `1 failed` match catches a compile error, and the `inconclusive` exclusion
catches a vacuous run. It turns `TP-19` from a check that always passes into one that proves the
C42 accounting is load-bearing. It adds no product scope.

Limits: The claim that 6b fails after the rewrite rests on Gate 3's measurement (8 written,
0 dropped without the fix). It has not yet been run with the new 10,000-record test. QA ran no
witness command itself. The 6b red result and the `sed` dry run are main-agent execution facts
that QA checked against the source. Minor citation drift: `src/config.rs:295` is really `:296`.
render: not inspected.

Slice 3's product code is written but paused. Product writes resume after Gate 4 approval.

Sources: `04-slices.md` (C47, C34, C44, slice 3 row step 6, `### Slice 3 interfaces`);
`03-program-design.md` C42, C46, TP-19 (`:32`, `:34`, `:746`); `gate-4-qa.md`;
`show-me-gate4-reopened-2.md`; `evidence/repository-structure.md`.

Approve Gate 4, or what should change?
