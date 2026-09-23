## What problem do we have?

The test meant to prove the file-sink fix (C42) cannot fail. When the five-second shutdown deadline cuts off the file sink, the fix counts every record it did not write. The approved `TP-19` was supposed to show that this works. Here is the approved form:

- queue 3 records
- run `drain(.., Duration::ZERO)`
- assert lines written + `drops` == 3

It assumed a zero deadline fires on the first poll. It does not. tokio rounds the deadline up to the next millisecond tick. All 3 records are written in about 200 microseconds, well before that tick, so no cut-off ever happens and the counting code never runs. Against the **unfixed** code the test passed 300 of 300 runs.

The consequence: Gate 1's promise that "any record that is dropped is counted and reported" (`01-product.md` C2) has no witness that can detect a regression. If someone removed the fix, every check would stay green.

## How will we solve it?

**Only the test's form changes (C46).** The product fix in `src/delivery/file.rs` stays as it is: on a cut-off it adds `rx.len() + 1` to `drops`, which can be one too high and is never too low. No signature, type, file, dependency or trust boundary changes.

Affected-API map:

| Module / operation | Change | Caller impact / visibility |
|---|---|---|
| `delivery::file::drain` (private) | None. It already takes a `deadline: Duration` parameter so a test can pass `Duration::ZERO` | None. Private; `run` calls it with `DRAIN_DEADLINE` |
| `TP-19` (`#[cfg(test)]` in `src/delivery/file.rs`) | New form, shown below | Test only |
| Gate 3 Drain paragraph (`### src/delivery/file.rs`) | Mechanism corrected: a zero deadline fires at the next millisecond tick, not on the first poll | Documentation only |

Unchanged declaration:

```rust
async fn drain(writer: &mut Writer, rx: &mut mpsc::Receiver<Record>,
               drops: &AtomicU64, deadline: Duration)
```

New `TP-19` form:

| Step | Old (approved) | New (C46) |
|---|---|---|
| Records queued | 3 | **10,000**, on a channel with room for all of them |
| Deadline | `Duration::ZERO` | `Duration::ZERO` (unchanged) |
| Assertion 1 | none | lines written < 10,000, else fail with `inconclusive: no cut-off happened` |
| Assertion 2 | lines + drops == 3 | lines + `drops` is **10,000 or 10,001** (the stated one-high residual) |

Why this works: one millisecond tick can write only a handful of records. That leaves thousands queued, so a cut-off is certain. Assertion 1 checks that a cut-off really happened, so the test can never pass without testing anything. If a machine ever drained all 10,000 inside one tick, the test would fail and say so. Before the fix, `drops` stays 0 while lines < 10,000, so assertion 2 fails.

```
old: 3 records  |###|........tick      -> all written, no cut-off, fix never runs
new: 10,000     |##########......tick| cut-off -> 9,9xx queued -> counted by fix
```

Rejected alternatives:

| Option | Why rejected |
|---|---|
| tokio paused time | Its auto-advance around `spawn_blocking` file I/O is timing-sensitive itself, so the test would trade one timing assumption for another |
| Injected-writer seam | Adds an abstraction that only a test would use |

## How will we confirm it is solved?

| Scenario | Expected result | Check |
|---|---|---|
| New `TP-19` against the fixed code | Passes; lines + drops = 10,000 or 10,001 | **Observed** by Gate QA in a scratch copy: 200/200 passed (e.g. 23 written + 9,978 dropped = 10,001; 0 + 10,000 = 10,000) |
| New `TP-19` with the `fetch_add(rx.len() + 1)` line removed | Fails at assertion 2 | **Observed** by Gate QA: 200/200 failed (e.g. 22 written, 0 dropped) |
| Implementer's controlled experiment | Pre-fix 8 + 0; post-fix 10 + 9,991 = 10,001 | Observed during slice 3 Engineering (`00-status.md` Notes) |
| New form in the repository's own test | Same as the first row | **Planned**: the slice 3 implementer rewrites the test after Gate 4 step 6 is re-approved |

Gate QA also confirmed:

- No text still claims a zero deadline fires on the first poll.
- C46 agrees with C42, the Drain residual (one too high, never too low) and T-DROP-BLACKOUT.
- Red Team was not triggered, because the change affects only the form of a test.

Recommendation: Approve. The new form is the smallest change that makes `TP-19` able to fail. It fails 200/200 against the unfixed code and passes 200/200 against the fix, adds no seam or abstraction, and leaves the working product fix untouched.

Limits:

- The tokio timer source could not be put on record as tracked evidence. The mechanism claim rests on the executed pre-fix and post-fix runs instead.
- The repository's `TP-19` is still the old 3-record form. All execution evidence comes from a scratch copy.
- Gate 4 (`04-slices.md`) was not reviewed and may still hold stale `TP-19` wording.
- "About 8 records per tick" is a measurement from one machine. QA observed 0 to 23.
- render: not inspected.

Status: Gate 3 reopened a third time on 2026-09-22 during slice 3 Engineering. Slice 3's product code is otherwise complete, and its other five witness steps pass. Product writes are paused. After this approval, Gate 4 step 6 is updated and re-approved. Then the same implementer rewrites `TP-19`, and slice 3 finishes its reviews. Base `0f50cec`; slices 1-2 and the slice 3 implementation are uncommitted.

Sources: `03-program-design.md` C46, Drain paragraph (`### src/delivery/file.rs`), `TP-19` row, Threat model C46 disposition; `gate-3-qa.md` (READY); `00-status.md` Notes, first bullet; `01-product.md` C2; `02-architecture.md` Flow step 5; `evidence/repository-structure.md`.

Approve Gate 3, or what should change?
