# Slice 4 — every delivered record says when it was made

## Delivered

Every record written to the log file or sent to the SIEM now carries `timestamp` as its
first field: UTC RFC 3339, six fractional digits, `Z` suffix, read from the app's injected
clock. The console line is unchanged.

```diff
  {
    "event": "request_decided",
+   "timestamp": "2026-09-21T14:13:20.120000Z",
    "request_id": "...", "method": "...", "ecosystem": "...", "package": "...",
    "version": "...", "status": ..., "result": "...", "reason": "...",
    "blocklist_revision": ..., "cache": "...", "duration_micros": ..., "bytes": ...
  }
```

The files that changed (the seven C49 edit sites, and nothing else):

```diff
  src/delivery/mod.rs      + pub(crate) fn rfc3339(utc_micros: i64) -> String   (:91-95)
                           + pub timestamp: String  first field of Decision (:53), Summary (:78)
                           ~ Decision doc comment corrected (:48-52)
  src/http/logging.rs      ~ decide: timestamp = rfc3339(app.clock.now_utc_micros()) (:208),
                             read after next.run(request) returns
                           ~ summarise (:357), flush_summary (:384), close_window (:415)
                             each gain a final `clock: &dyn Clock`
  src/lib.rs:321           ~ flush_summary(&self.app.delivery, self.app.clock.as_ref())
  src/delivery/file.rs:237 ~ tp19's Summary literal gains timestamp: String::new()
  tests/decision_log_delivery.rs:620-683
                           + tp20_delivered_records_carry_timestamp
  docs/operations.md:453-459, :512-514
                           ~ operator text for the field
```

Public behaviour otherwise unchanged: neither `tracing::info!` names `timestamp`, so the
stdout decision line keeps exactly its twelve keys (`tp13`). The signature changes above
are internal (`summarise`, `flush_summary`, `close_window`); no configuration key was
added, and `config.sample.toml` and `src/config.rs` are untouched.

**Why this slice existed.** Slice 3's final verification passed the slice but failed the
*feature* on P-WHEN — the Gate 1 promise that records say what was served **and when**.
Only the console line had a time. This slice closes that.

## Proof

| Promise | Executed witness | Observed result |
|---|---|---|
| P-WHEN: delivered records state the time | `tp20_delivered_records_carry_timestamp` | Red before: `left: Null  right: "2026-09-21T14:13:20.120000Z"`. Green after; file decision record, file's last summary record (`…14:14:20.120000Z`, clock advanced 60 s) and the collector's received body all carry it. |
| Console line unchanged | `tp13_stdout_decision_key_set` | `ok` — a `timestamp` key on stdout would fail it. |
| Nothing else broke | `cargo test --test decision_log_delivery`; `cargo test` | `17 passed; 0 failed`, every slice 1–3 test `ok`; all 22 targets `0 failed`, including the lib target holding `tp19`. |
| No lint regressions | `cargo clippy --all-targets -- -D warnings` | exit 0, no warning lines. |

All three commands were run by the main agent from the repository root, **after** the one
post-verification correction: the `Decision` doc comment still claimed the struct holds
exactly the stdout line's fields, which `timestamp` made untrue. Fixed, witness re-run.

Reviews: `sf-code-review` **SHIP**, no findings — including the two checks that have no
execution witness (both summary paths forward their own `clock` to `close_window`;
`rfc3339`'s body is exactly the C49 expression). `sf-verification` **VERIFIED P-WHEN**
plus P-FILE, P-SIEM, P-C1, P-DROP, P-C3, C5, C6 and the default-off non-goal, confirmed
P-C2-OPTIN is recorded as deferred, and found `docs/operations.md` current against the
code. `sf-security-review` was not named: Gate 3 recorded that C49 adds no entry point,
trust boundary, destination or input.

**The feature, end to end** — slices 1–4, all four checked:

| Slice | What an operator gets | Status |
|---|---|---|
| 1 | `log_file_path` appends one NDJSON record per decision, bounded at twice the cap with a single `.1` rollover, drained at shutdown | proven |
| 2 | `siem_url` batches records and POSTs them as NDJSON, authenticated from `OSPREY_SIEM_AUTH`, never blocking a request or hanging shutdown | proven |
| 3 | opt-in `log_consumer_identification` records the peer IP on every delivered record, file created `0o600`, refused at startup with nowhere to write | proven |
| 4 | every delivered record states when the decision was made | proven (this slice) |

## Limits

- **P-GUARD-LOAD has no executed witness.** This is the feature's one accepted
  verification gap (C52, user decision 2026-09-23). The counted half holds and is
  witnessed: a record the queue cannot take is counted and reported as `dropped_file` /
  `dropped_siem` (`tp7`, `tp15b`). The no-loss-at-rated-load half has no load test,
  because "rated load" has no number anywhere in the firewall; accepted rather than
  guessing one. An operator learns about loss from the drop counters.
- The periodic `summarise → close_window` clock pass has no execution witness; code
  review is its check, and it passed.
- The stdout *summary* line's omission of `timestamp` is checked by reading `emit`, not by
  a test; `tp13` covers the stdout decision line.
- Pre-existing and outside this slice: the delivered `package` value is escaped as
  `"\"left-pad\""`, so a collector query must match that form — same as stdout today.
- `tp20` was flaky in the whole-file run until it took the env lock
  (`AuthEnv::set(None)`) that every other SIEM test uses. Test-only fix; product code
  unchanged; the file then passed 3 of 3 runs plus the witness run.

## Next

Slice 4 is the feature's last slice — no approved slice remains after it. The next step is
completing the feature: the timestamp promise that failed at slice 3 is now verified, and
the only open item is the accepted P-GUARD-LOAD limitation above.

## Recommendation

Complete. Every Gate 1 promise this feature owns is verified by an executed test except
P-GUARD-LOAD, whose gap you already accepted with its reason recorded (C52), and the one
promise the feature deferred (P-C2-OPTIN, C7) is recorded as deferred rather than
delivered. Re-steering would mean defining "rated load" and building a load test, which is
a plan of its own, not a slice of this one.

Slice 4 of 4, complete, proof line recorded in `00-status.md`.

Sources: `docs/plans/decision-log-delivery/evidence/slice-4.md`; `04-slices.md` (slice 4
row, `### Slice 4 interfaces`, C50/C51/C52); `00-status.md` (slice 4 proof line, slices
1–3); `01-product.md` (P-WHEN, P-GUARD-LOAD).

Complete the feature, or re-steer?
