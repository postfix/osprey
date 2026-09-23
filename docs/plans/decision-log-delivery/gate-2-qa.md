# Gate 2 QA
## Verdict
READY
## Questions and findings
No blocking findings.

- F1 (prior, blocking) — resolved. `02-architecture.md` rows C4, C5, C6 now carry Class `current-Gate decision`, an allowed value, matching their origin rows in `01-product.md`. They keep their original IDs, Owning Gate 1 and Target Gate 2, and are resolved with a selected policy and a deciding section.
- N1 (prior, non-blocking) — resolved, with no inconsistency introduced. The `docs/operations.md` Fit row now lists "the `timestamp` field on file and SIEM records and that the console line omits it (C48)". This agrees with C9 (corrected), C48, the Program behavior capability bullet, Flow steps 2 and 5, and the `src/http/logging.rs` Fit row.
- N2 (non-blocking, carried) — section order still differs from the Gate 2 template: `## Repository evidence` and the ID note come before `## Clarifications and decisions`. All required sections are present, so this is left for Gate 3 as advised.
- N3 (non-blocking, carried) — Flow step 5 still does not name which component flushes the final summary. The Fit rows for `src/http/logging.rs` (a one-shot summary flush) and `src/lib.rs` (cancels the drain token) together imply the owner. Gate 3 should name the call.
- N4 (non-blocking, new observation) — plan-local ID collision. Gate 1 now uses C7 and C8, and Gate 2 also uses C7 and C8 with different meanings. The collision comes from approved Gate 1. Renumbering the Gate 2 rows would break IDs that Gates 3 and 4 already cite. Gate 2's explicit disambiguation note ("Gate 1 rows are always cited as 'Gate 1 C<n>'") handles it, and C48 cites "Gate 1 C8" correctly. C48 does not collide with any ID in 03-program-design.md or 04-slices.md.
- N5 (non-blocking, for Gate 3) — the decision-record timestamp is read after `next.run(request)` returns. For a long-lived response, which can run up to `RESPONSE_LIFETIME` of 15 minutes (`src/http/limits.rs`), it marks completion rather than the start of the request. Gate 2 selects this explicitly, and it matches the time the console formatter already stamps on the same line. Gate 3's operator-facing contract should say which instant the field means.

## Checked dimensions
- Subject completeness, readable regular paths and freshness: pass. Every tracked path is a regular file under the repository root. 02-architecture.md is the post-correction revision.
- Clarification inventory: pass.
  - Every row has a stable ID, and every class is allowed: `current-Gate decision` for C4–C12 and C41; `user clarification` for C48.
  - Every row is resolved and names its owner, target, selected policy and decision source.
  - C48 uses the valid user-answer source "user answer, Gate 2 grilling Q1 (2026-09-22)".
  - No row is open, and no deferral to a later Gate remains.
- Gate 1 deferrals imported under their original IDs and resolved: pass. Gate 1's deferrals are C4, C5 and C6, all targeting Gate 2. Gate 1 C1–C3, C7 and C8 are resolved at Gate 1 and are not deferrals.
- Gate 2 fit, interfaces, flow and constraints: pass.
  - Gate 1 C8 is fully designed. Every record offered to a sink, decision and summary alike, carries a UTC RFC 3339 `timestamp` from the injected `App.clock`.
  - The read points are fixed: `decide()` after `next.run`, `summarise()` at build time, and the drain flush.
  - The stdout render omits the field, and C9 is corrected to match.
  - The owners are named: the `src/http/logging.rs` Fit row, and Flow steps 2 and 5.
  - A collector's receipt time is explicitly rejected, consistent with Gate 1 C8.
  - The only choice left to Gate 3 is which formatter renders the value, a Gate 3 implementation detail; the format itself (UTC, RFC 3339) is selected.
- Grounding of the new C48 claims: pass.
  - `src/clock.rs:8-9` reads "Every time reading in the application goes through this trait, so tests can control both the wall clock and the monotonic clock." Read with SMTC from the current file, which is unmodified from the base.
  - `git show 0f50cec:src/lib.rs` has `pub struct App` at :52 with `pub clock: Arc<dyn Clock>` at :54.
  - `decide()` already receives `State(app)`.
  - `summarise(elapsed, bytes, was_error)` has no clock today, which is consistent with the Fit row's stated change.
  - `jiff = "0.2.37"` is already in `Cargo.toml` at the base, so "no dependency is added" holds.
  - The drain token is cancelled while `self.app` is still alive, between :257 and :265, so the flush can reach the clock.
- Base citations reused from the prior review (C10, Constraints; `src/lib.rs:256/257/264/265`, `src/http/limits.rs:24`): spot-checked against `git show 0f50cec`. Consistent.
- Program behavior, Endpoints, Data, External: pass. The capability bullet for the timestamp was added. No route, table, external surface or environment variable was added for C48.
- Red Team: pass. The 2026-09-22 revision records "not triggered" with a reason: one field from the existing injected clock, and the user owned the only choice. The earlier C41 revision records its reason and cites the Gate 3 Red Team that preceded it.
- C41 decision sources in Gate 3 (C28, C35, C36, C39): present and resolved in 03-program-design.md.
- Gate 1, Gate 3 and Gate 4 stage-specific dimensions: not applicable.

## Limitations
- Base-line source citations in `src/lib.rs`, `src/http/logging.rs` and `Cargo.toml` were checked through `git show 0f50cec`, because the working-tree `src/lib.rs` is modified. The helper tracks working-tree files only, so these base reads are not helper-tracked. `src/clock.rs` and `src/http/limits.rs` are unmodified and tracked.
- C41's source "Gate 3 QA (reopened), finding R6" was not re-read. C41 is unchanged since the prior review, and the user answers it relies on (Gate 3 C28, C39) are present in the tracked 03-program-design.md.
- Gate 3 and Gate 4 documents are stale by design. They were used only for the IDs C41 cites and for ID-collision checks.
