# Slice 3 evidence — opt-in consumer identification

Recorded 2026-09-22, after the Gate 3/4 C46–C47 re-approval.

## Diff summary (this session's changes on top of the paused slice 3 code)

- `src/delivery/file.rs`: `tp19_file_drain_deadline_counts_cut_off_records` rewritten to its C46 form.
  It queues 10,000 records on a channel sized for all of them and calls `drain(.., Duration::ZERO)`.
  It then asserts `written < 10_000` (message `inconclusive: no cut-off happened`) and
  `written + drops ∈ 10_000..=10_001`. `drain` itself is unchanged.
- `docs/operations.md`: the `log_consumer_identification` table row no longer says records "reach
  a durable destination"; it says only that a destination is configured. A new bullet in "Recording
  who asked" names the three residuals of T-SILENT-DESTINATIONLESS-COLLECTION: a SIEM-only
  deployment is never checked at startup, the file sink can fail after startup, and neither is
  detectable at startup. It points to `dropped_file` and `dropped_siem` as the signal (Security finding 1).
- The rest of slice 3 (probe, 0o600, consumer field, reason string, `.no_proxy()`, drain accounting,
  tp4/tp13/tp16/tp17/tp5c, module doc, operations prose (a)–(f)) was already in the tree from the paused run.

Subject SHA-256 at completion:

```
e846c29e47810dcfbb17e025bef94cbd0db63c5c1882fdea61f45e77987c7cf7  src/http/logging.rs
c891433176041f30b2acfe8656c4f4922e4f620e20c2ed019c0f89997a1a8135  src/delivery/mod.rs
8726b5d05e9a787e342e65022d9a9b2a12d44c8e7103a4570c0cbe24b494b8b5  src/delivery/file.rs
aa1dda40721d3f8e064f223371956a712eb200231181b13f970c4607de16e09c  src/delivery/siem.rs
b6b0c7b6afa101c9a00482d384aa0a3751b065888ba2821a1435f1edaa7aa523  src/config.rs
09c27d0909e3f2e68a9c3f4e94c6934925e15c8e7a6dffe9443efe282670b467  src/lib.rs
35c223b8cd93a4027cda1f07c6fe39dca5ec819b043f4112fed6e1eddbb19321  src/tasks/mod.rs
b068c163234a828d860806dc5215161c5fb679507b37c42121308d04f37d029d  tests/decision_log_delivery.rs
914c5deff612e7f4129ab2c51cc7c0597fb7d53400aac21c9770cd3e2088362a  tests/config_validation.rs
30c3fc7a679ba4580fd851f16363e0076152591f3beb370b805d94e4bf490bbc  config.sample.toml
a45cd2498c92388c9704017e442875f9b0dd99a908ee42b2df4bf5052661d21f  docs/operations.md
```

## Direct witness (run by the main agent)

1. `cargo test --test decision_log_delivery` → `test result: ok. 16 passed; 0 failed`
   (tp4, tp13, tp16, tp17 and every slice 1/2 test `ok`).
2. `cargo test --test config_validation` → `test result: ok. 21 passed`, `tp5c_consumer_identification_requires_a_sink ... ok`.
   Re-run by Engineering after the operations.md fix: 21 passed.
3. `cargo clippy --all-targets -- -D warnings` → exit 0.
4. TP-14:
   - product root: `spec client-data-reaches-delivery-sink: 0 finding(s)`, not truncated;
   - control: 1 finding at `/tmp/osprey-tp14-control/src/main.rs` line 13 (the `request.headers()` call);
   - non-vacuity with the receiver filter removed: `50 finding(s)`.
   - The first jq read reported 0 for the non-vacuity run because the finding list was truncated behind a ref; the summary line is the count.
5. TP-18 jq → `true`, exit 0.
6. TP-19:
   - 6a: `test delivery::file::tests::tp19_file_drain_deadline_counts_cut_off_records ... ok`, `1 passed`.
   - 6b: printed `TP19-DISCRIMINATES`, then `test result: FAILED. 0 passed; 1 failed`. The panic was at `src/delivery/file.rs:258:9`, the conservation assertion, not the inconclusive one.

## tdd lines

- tdd: tp19_file_drain_deadline_counts_cut_off_records — red: panicked at src/delivery/file.rs:258:9: every queued record is either in the file or counted as dropped: 0 written, 0 dropped (mutant with the accounting branch deleted) — green: cargo test --lib tp19_file_drain_deadline_counts_cut_off_records passed
- tp4 red (supplied by sf-code-review): a mutant recording a constant `127.0.0.1` failed at `tests/decision_log_delivery.rs:369:5`, `0 passed; 1 failed`. It passes on the real tree.
- tp13 red (supplied by sf-code-review): a mutant with `consumer = …` removed from the stdout `tracing::info!` failed at `tests/decision_log_delivery.rs:609`, `0 passed; 1 failed`.
- tp16, tp17, tp5c: their reds are recorded in 04-slices.md C34.

## Review verdicts

- **sf-code-review: SHIP.**
  - `.no_proxy()` is at `src/delivery/mod.rs:230`, matching `reqwest_transport.rs:101` (C43).
  - `drain` adds `rx.len() + 1` at `file.rs:74-76` (C42), and tp19 is not tautological.
  - Both earlier minor findings are closed.
  - `build` performs the startup probe, as its Effects contract says.
- **sf-security-review: FIX FIRST → CLEAR.**
  - F1 (0600), F2 (startup probe, fatal with the opt-in on), F3 (retained extension spec) and C43 are all closed in code.
  - Its one finding was the doc gap on T-SILENT-DESTINATIONLESS-COLLECTION's residuals. It was fixed in `docs/operations.md`, and the re-check returned CLEAR. No unmodeled surface.
- **sf-verification: FAILED at feature level; every slice 3 truth VERIFIED.**
  - Verified: P-FILE, P-SIEM, P-C3, P-C1, the slice 3 refusals and 0600, the default-off non-goal, TP-19, operations prose (a)–(f), the module doc, and config.sample.toml.
  - Full `cargo test` passes in every target.
  - Failed, as gaps in Gates 2–4 and not in slice 3's row:
    - **P-WHEN:** delivered records carry no time of decision. Gate 1 promises "what was served and when" and "with timestamps".
    - **P-C2-OPTIN:** the refuse-rather-than-serve-unlogged setting was never designed.
  - Uncertain: **P-GUARD-LOAD** has no executed load witness.
  - Disposition, 2026-09-22: the user deferred the refuse setting (Gate 1 grilling Q1). Gate 1 is reopened to record that. The timestamp returns through Gates 3 and 4.
