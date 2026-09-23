## What problem do we have?

Gate 3 was approved 2026-09-21. On 2026-09-22 the user answered two questions slice 2's review had
left open, and both answers change Gate 3's design text, so Gate 3 needs re-approval before slice 3
(not yet built) can be assigned this work. Nothing else in Gate 3 changed — the rest of the
2026-09-21 approval stands as is. This presentation covers only the two 2026-09-22 changes.

**C42 — the file sink's drain deadline undercounted a cut-off.** Slice 2's review found that when
the file sink's five-second shutdown drain gets cut off, the queued records and the one record in
hand simply vanish from every count — breaking the operator promise in `docs/operations.md` that "a
record is never discarded silently." The SIEM sink already had this same bug and slice 2 fixed it
with an `Unsent` guard; the file sink's sibling case was deliberately left out of slice 2 to keep it
small. User answer: fix it in slice 3, which already touches this file.

**C43 — the SIEM client doesn't ignore proxy environment variables, unlike the upstream client.**
Slice 2's review found `src/upstream/reqwest_transport.rs:101` calls `.no_proxy()` on the registry
client, but the SIEM client builder never got the same call, so it obeys `HTTP_PROXY`/`HTTPS_PROXY`.
Security judged this crosses no new trust boundary. User answer: match upstream and add the call.

## How will we solve it?

**C42 fix, in `src/delivery/file.rs`.** The drain loop moves into a new private function:

```rust
async fn drain(
    writer: &mut Writer,
    rx: &mut mpsc::Receiver<Record>,
    drops: &AtomicU64,
    deadline: Duration,
)
```

`run` calls it with the real `DRAIN_DEADLINE` (5s); the `deadline` parameter exists only so the test
can pass `Duration::ZERO`. When `tokio::time::timeout` returns `Err` (the deadline fires), `drain`
adds `rx.len() as u64 + 1` to `drops` — every record still queued, plus the one already pulled off
the channel and in hand. There is always exactly one "in hand" at a cut-off because `try_recv` never
blocks, so the only place the drain future can be stuck is inside the `append` write itself.

Cost, stated plainly: the count can be **one too high**, never too low. If the cut lands right after
that in-hand record's `write_all` actually finished, it is both written to disk *and* counted as
dropped — an over-count by one, not a lost record. `flush_drop_tail` (unchanged, runs after
`tasks.join()`) reports these drops on the console.

Witness — `TP-19`, a new unit test inside `src/delivery/file.rs` (`#[cfg(test)]`, because `drain` is
private): three records queued on a fresh channel, a `Writer` on a temp path, `drain(.., Duration::ZERO)`
awaited. Expected: lines-written + `drops` == 3. With a zero deadline, the first `timeout` poll finds
the drain future still pending inside tokio's blocking file-open, so the deadline fires immediately —
before this fix, nothing was written and `drops` stayed 0, so the sum was 0, not 3.

**C43 fix, in `delivery::build`.** The SIEM `reqwest::Client` builder gains `.no_proxy()` alongside
its existing `redirect::Policy::none()`, `.timeout(3s)` and `.connect_timeout(1s)`:

```
Also constructs the reqwest::Client with redirect::Policy::none() ... and with .no_proxy(), so the
SIEM client ignores HTTP_PROXY/HTTPS_PROXY exactly as the upstream registry client at
src/upstream/reqwest_transport.rs:95-105 does and both clients leave the host by the same route (C43)
```

— `03-program-design.md`, `delivery::build` Effects. `siem::run`'s Accepts clause is updated to
match: "a client already built with `redirect::Policy::none()` and `.no_proxy()`".

There is no execution witness for C43: a proxy test would have to set process-wide environment
variables and rely on reqwest's loopback-proxy rules, which the design judges not worth the fragility
for a one-line builder call. Disposition: code review checks the call against the named upstream line
(`src/upstream/reqwest_transport.rs:101`), the same proportionate treatment already given to the
three sibling builder calls covered by `TP-9`/`TP-15`.

**What did not change.** Red Team was not re-triggered for this reopening. The design's own stated
reason (`## Threat model` intro): C42 closes an already-modelled threat, T-DROP-BLACKOUT, with a
counting mechanism checked against tokio 1.53.1's own vendored source (`Receiver::len`, and
`Timeout::poll` polling the inner future before the delay); C43 removes an egress path and adds none.
The threat table row for T-DROP-BLACKOUT was updated to point at this fix:

> Records a sink's own drain deadline cuts off are counted before the task returns: the SIEM sink
> through its `Unsent` guard (slice 2), the file sink through `drain` (C42, `TP-19`).

## How will we confirm it is solved?

| Scenario | Expected result | Check |
|---|---|---|
| File drain cut off at `Duration::ZERO` with 3 queued records | Lines written + `drops` == 3 (was 0 before the fix) | `TP-19`; unit test, execution planned |
| SIEM client builder vs. upstream registry client | Both call `.no_proxy()`; no behavioral test, checked by code review against `src/upstream/reqwest_transport.rs:101` | code review; no execution witness |

Gate QA (fresh, independent review, verdict READY) additionally confirmed, by direct source reading
rather than by trusting the design text:
- `src/delivery/file.rs`'s current `run` has no `drain` seam yet and silently discards the timeout
  result on cut-off — TP-19's stated defect is real against current source.
- The tokio 1.53.1 mechanism C42 relies on (`Timeout::poll` polls the inner future first;
  `mpsc::Receiver::len` is a public method; `tokio::fs::OpenOptions::open` goes through
  `spawn_blocking`) was checked against the pinned crate's own vendored source, not assumed.
- `src/upstream/reqwest_transport.rs:101` is confirmed to read exactly `.no_proxy()`, and the current
  `delivery::build` SIEM client builder (`src/delivery/mod.rs:216-226`) does not yet call it —
  consistent with C43 being an unshipped slice-3 change, not a missed one.

Gate QA found no REVISE-level defect, no open user-owned question, and no missing evidence across the
whole document, with particular attention to this reopening's scope.

**Limits.** Neither C42 nor C43 is implemented yet — slice 3 is not done (router status). This
presentation covers only what the 2026-09-22 reopening changed; the rest of Gate 3's 2026-09-21
content is unchanged and not re-litigated here. Render was not inspected in a browser; this is a
Markdown/terminal-safe presentation, not HTML.

Gate document: `docs/plans/decision-log-delivery/03-program-design.md`.

Recommendation: **Approve.** Both changes are narrowly scoped (one function extracted, one builder
call added), both trace to a named user answer, and Gate QA independently verified the underlying
code and dependency claims rather than accepting the design's prose. Red Team's decision not to
re-trigger is itself reasoned and grounded, not a bare skip.

Context: Gate 3, reopened 2026-09-22, upstream Gates 1 and 2 both approved 2026-09-21.

Sources: `docs/plans/decision-log-delivery/03-program-design.md` (C42, C43; `### src/delivery/file.rs`
Drain paragraph; `delivery::build` Effects; `## Threat model` intro and T-DROP-BLACKOUT row; `TP-19`
row in `## Test plan`); `docs/plans/decision-log-delivery/gate-3-qa.md` (READY verdict, subject list,
TP-19/tokio/C43 verification); `docs/plans/decision-log-delivery/01-product.md`;
`docs/plans/decision-log-delivery/02-architecture.md`;
`docs/plans/decision-log-delivery/evidence/repository-structure.md`.


Approve Gate 3, or what should change?
