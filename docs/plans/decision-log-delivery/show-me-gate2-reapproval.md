## What problem do we have?

You approved Gate 2 (Architecture) earlier today, 2026-09-21. It is back only because the reopened Gate 3 found two statements in it that were wrong or incomplete. If they stay as written, Gate 2 and Gate 3 disagree, and a later reader could build from the wrong one.

1. **C11 gave the wrong reason.** It called the console "a channel the product explicitly calls non-durable". That is false wherever this firewall runs. Under systemd, stdout goes to journald's persistent journal. Under a container runtime, it goes to a log file the host keeps. So with consumer identification on, peer IPs already reach a durable destination.
2. **The log-file failure rule had no condition.** Gate 2 said a log file that cannot be opened leaves the firewall serving, whatever the settings. In the Gate 3 reopening you chose to make that depend on consumer identification.

Both changes record decisions you have already made (Gate 3 C35, C28 as narrowed by C39, and C36). This review decides nothing new. It checks that Gate 2 now says what you chose.

## How will we solve it?

Gate 2 is corrected in place, and the correction is recorded as row **C41**. No other Gate 2 decision changes: C4-C10 and C12 stand as approved.

| Where in Gate 2 | Before | Now |
|---|---|---|
| C11, the reason for the rule | The rule stops peer IPs going to a "non-durable" console | The rule is unchanged: consumer identification with no sink configured is still rejected at startup. Its stated purpose is narrower and true. It guarantees a durable destination **the operator chose**. The console is still a durable destination, but not one the operator chose. |
| Failures: log file cannot be opened | One bullet: the firewall keeps serving, with no condition | Two bullets, below |
| Flow step 1 and the `src/delivery/mod.rs` Fit row | No startup action on the file | When `log_file_path` is set, `delivery::build` **probes** it at startup: it opens the file for append, creating it if absent, and closes it again. Both failure bullets depend on this probe. |

**The two failure bullets now say:**

- **Consumer identification on**, and the log file cannot be opened at startup: startup fails and names `log_file_path`. Otherwise the process would collect peer IPs with no working destination the operator chose. They would reach only the console, which the operator did not choose.
- **Consumer identification off**, or the file fails **after** startup: if it failed at startup, one error line is logged. In both cases each unwritten record is counted, the open is retried on the next record, and the firewall keeps serving.

**Accepted cost, which you chose at Gate 3:** `check_config` cannot catch this failure. It writes nothing, and the startup probe may create the file. With consumer identification on, an operator whose log directory is created later by an init system or a mount will fail to boot instead of recovering.

**Module ownership is unchanged.** `src/delivery/mod.rs` still owns the record type, the bounded queues and the per-sink drop accounting, and it is still used by `src/http/logging.rs` and `src/lib.rs`. The only new responsibility is the startup probe. The other modules in the Fit table (`file.rs`, `siem.rs`, `logging.rs`, `config.rs`, `lib.rs`, `tasks/mod.rs`, `main.rs`, the sample config and the operator docs) are not affected.

## How will we confirm it is solved?

Observed by Gate 2 QA (fresh, sealed READY; `router gate-qa verify` returned `{"verdict":"READY","files":5}`):

- C11, C41, Flow step 1, the Fit row and both Failures bullets agree with each other and with Gate 3 C28, C35, C36 and C39 -> **PASS**.
- Only the places C41 names were revised -> **PASS**, checked against the C41 markers and against Gate 3 R6's verbatim quote of the old Failures bullet.
- Still consistent with Gate 1 -> **PASS**. Consumer identification stays off by default (C1). Records are still dropped, counted and surfaced (C2). Delivery still never slows a request (C3). The probe runs only when `log_file_path` is set, so the default still creates no new files on disk.
- `check_config` still writes nothing -> **PASS**, re-checked against current source.

Planned, not yet run: the startup probe's behaviour is tested by Gate 3's `TP-16` and `TP-17`.

Recommendation: Approve. Both edits bring Gate 2 in line with choices you already made at the Gate 3 reopening, QA found nothing else changed, and the C11 rejection itself stays as approved.

Limits:
- There is no byte copy of the Gate 2 you approved, because the plan folder is not tracked by git. QA checked the scope of the revision against the C41 markers and against quotes of the old text, not against a full diff.
- **Possible tension with Gate 1, for you to judge.** Gate 1's problem statement says: "Console output that disappears when the process restarts is not a record." Corrected C11 says console output often does survive a restart. QA judged the two consistent if Gate 1's sentence is read as conditional ("console output *that* disappears"), and no Gate 1 decision rests on it, so Gate 1 is not being reopened. If you read it differently, say so and Gate 1 will be reopened.
- Gate 2's source line references are against base `0f50cec`. Only the `check_config` claim was re-checked against current source.
- QA did not read the reopened Gate 3's Red Team output directly. It relied on Gate 3 rows C35-C39.

Status: re-approval under the backtracking rule, after the Gate 3 reopening. Gates 3 and 4 are still pending and come next. Slice 3's product writes stay stopped until all three gates are approved.

Sources: `02-architecture.md` (C11, C41, Program behavior > Failures, Fit `src/delivery/mod.rs`, Flow step 1); `03-program-design.md` C28, C35, C36, C39; `01-product.md` Problem, C1-C3, Non-goals; `gate-2-qa.md`; `evidence/repository-structure.md` §4 (`check_config` at `src/main.rs:80-96`).

Approve Gate 2, or what should change?
