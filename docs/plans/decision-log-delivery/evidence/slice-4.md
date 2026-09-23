# Slice 4 — every record delivered to the file or the SIEM states when it was made

Date: 2026-09-23. Git base `0f50cec6d526773abec4b4897f64afc10992a8ae`; the feature is
uncommitted working-tree change.

## Diff summary

The seven C49 edit sites, and nothing else:

- `src/delivery/mod.rs` — new `pub(crate) fn rfc3339(utc_micros: i64) -> String` (`:91-95`),
  exactly the C49 expression (rustfmt wrapped the chain across three lines). `pub timestamp:
  String` is the first field of `Decision` (`:53`) and of `Summary` (`:78`). The `Decision` doc
  comment (`:48-52`) was corrected after verification: it had claimed the struct holds exactly the
  stdout line's fields, which `timestamp` made untrue.
- `src/http/logging.rs` — `decide` sets `timestamp: delivery::rfc3339(app.clock.now_utc_micros())`
  (`:208`), read after `next.run(request)` returns (`:177`), and passes `app.clock.as_ref()` to
  `summarise` (`:245-251`). `summarise` (`:357`), `flush_summary` (`:384`) and `close_window`
  (`:415`) each gained a final `clock: &dyn Clock`; both callers forward their own parameter
  (`:374`, `:387`). `close_window:423` is the only place a summary reads the time. Neither
  `tracing::info!` names `timestamp` (`:223-238`, `:440-452`).
- `src/lib.rs:321` — `http::logging::flush_summary(&self.app.delivery, self.app.clock.as_ref());`,
  still after the server join and before the drain token is cancelled.
- `src/delivery/file.rs:237` — the `tp19` test's `Summary` literal gains `timestamp:
  String::new()`. Its assertions are unchanged.
- `tests/decision_log_delivery.rs:620-683` — new `tp20_delivered_records_carry_timestamp`.
- `docs/operations.md:453-459`, `:512-514` — the operator-facing text for the field.

SHA-256 after the slice: `src/delivery/mod.rs`
`6aab1cbe82b449287ebe50cb2b943203004ca111f8435a2c779c610508a418d9`; `src/delivery/file.rs`
`fb0245e88d374220cb6be0a041462890bcc9f18b14648bc14bac38a34d24bf3d`; `src/http/logging.rs`
`78edf6b4136e1283da7a88c00dbdfcdb941b26e1daa3e9ae21364d4b5b4957ec`; `src/lib.rs`
`524b5bf6bd473dc79a8de7657f6bc1f7054c955053e9a88cf36ff1d24be1b68a`;
`tests/decision_log_delivery.rs`
`d96e074de51402d67c8ca2453a3c7d6a1423e70b00c9f3e8ce7e5a60b89983d0`; `docs/operations.md`
`a74817397bdd9000ee99a1f27842af920673ddeb1da361ddc53082aa5f7a9538`.

## Witness

Run by the main agent from the repository root, after the doc-comment correction:

1. `cargo test --test decision_log_delivery` → `test result: ok. 17 passed; 0 failed`, with
   `test tp20_delivered_records_carry_timestamp ... ok` and `test tp13_stdout_decision_key_set
   ... ok`, and every slice 1–3 test `ok`.
2. `cargo test` → all 22 targets report `0 failed`, including the lib target that holds `tp19`.
3. `cargo clippy --all-targets -- -D warnings` → exit 0, no warning lines.

## TDD

tdd: tp20_delivered_records_carry_timestamp — red: `assertion left == right failed: the decision
carries the injected clock's time in fixed-width UTC RFC 3339: {..."event":"request_decided",...}
left: Null  right: "2026-09-21T14:13:20.120000Z"` — green: `cargo test --test
decision_log_delivery` passed.

After the product change `tp20` failed intermittently in the whole-file run with `OSPREY_SIEM_AUTH
is not a valid HTTP header value`, because it started a SIEM sink without holding the env lock
that `tp11b` uses. Fixed in the test with `let _env = AuthEnv::set(None).await;`, as every other
SIEM test in the file does. Product code unchanged; the file then passed 3 runs of 3 plus the
witness run.

## Reviews

- `sf-code-review`: **SHIP**, no findings. Checks (a)–(f) all pass: both summary paths forward
  their own `clock` to `close_window`; `rfc3339`'s body is the C49 expression; the `file.rs`
  edit only adds the field; `timestamp` is first in both structs and named in neither
  `tracing::info!`; `decide` reads the clock after `next.run`; `tp20` is not tautological (its
  expected values are literals, the collector is an external wiremock). It also checked
  `Running::shutdown`'s ordering against the contract. Limitations: the periodic
  `summarise → close_window` path is read, not executed (as Gate 3 intends), and `file.rs` is
  untracked so "only adds the field" is a reading, not a diff.
- `sf-verification`: **UNCERTAIN**, on P-GUARD-LOAD alone. **P-WHEN is VERIFIED** — `tp20` plus
  the reviewer's own scratch-tree test, whose collector received
  `{"event":"request_decided","timestamp":"2026-09-21T14:13:20.000001Z",…,"consumer":"127.0.0.3"}`.
  P-FILE, P-SIEM, P-C1, P-DROP, P-C3, the default-off non-goal, C5 and C6 are VERIFIED by
  executed tests, and P-C2-OPTIN is confirmed recorded as deferred rather than delivered. The
  `docs/operations.md` timestamp text is CURRENT against the code.
- `sf-security-review` was not named for this slice: Gate 3 recorded that C49 adds no entry
  point, trust boundary, destination or input.

## Accepted limitations

- **P-GUARD-LOAD has no executed witness (C52, user decision 2026-09-23).** The counted half
  holds: a record the queue cannot take is counted and reported as `dropped_file` /
  `dropped_siem` (`tp7`, `tp15b`). The no-loss half has no load test, because "rated load" is
  not defined anywhere in the firewall. Accepted rather than guessing a number; an operator
  learns about loss from the drop counters. This is the feature's one accepted verification gap.
- The periodic `summarise → close_window` clock pass has no execution witness; code review is
  its check, and it passed.
- The stdout *summary* line's omission of `timestamp` is checked by reading `emit`, not by a
  test; `TP-13` covers the stdout decision line.
- Verification noted, outside this slice: the delivered `package` value is escaped as
  `"\"left-pad\""`, so a collector query must match that form. Pre-existing, same as stdout.
