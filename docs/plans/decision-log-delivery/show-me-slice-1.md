## Delivered

An operator who sets `log_file_path` now gets every decision the firewall makes appended to that
file as one NDJSON object per line, bounded at twice `log_file_max_bytes` with a single `.1`
rollover, including records for requests still in flight when the process shuts down. An operator
who sets nothing sees no file, no background task, and a console line byte-identical to before.

This is slice 1 of 3 in "Decision log delivery" (`04-slices.md`): the file sink, tracer-bullet
end to end. Slice 2 adds the SIEM (HTTPS) sink; slice 3 adds opt-in peer-IP consumer
identification. Neither exists yet.

New: `src/delivery/mod.rs`, `src/delivery/file.rs`, `tests/decision_log_delivery.rs`.
Changed: `src/http/logging.rs`, `src/config.rs`, `src/lib.rs`, `src/tasks/mod.rs`,
`tests/config_validation.rs`, `config.sample.toml`, `docs/operations.md`. Nothing is committed;
the changes sit in the working tree on branch `package-firewall-mvp`.

Two new configuration keys, `log_file_path` and `log_file_max_bytes`, are registered in
`RawConfig`, `Config` and `OPTIONAL_KEYS`, and rejected together: `log_file_max_bytes` without
`log_file_path`, and a zero value, both fail config validation at startup.

Signatures otherwise unchanged from the approved Gate 3 design (`03-program-design.md`
`## Modules and interfaces`): `Sinks::offer`, `delivery::build`, `file::run` all match as
declared, minus `Decision.consumer` and `siem`, both deferred to their own slices per the
plan's C24/C25.

## Proof

All commands below were run and their output read directly by the main agent, not taken on a
subagent's report.

- **Promise:** the four file-sink tests named in `04-slices.md` Slice 1's direct witness pass.
  **Witness:** `cargo test --test decision_log_delivery`. **Result:** 6 passed —
  `tp1_file_sink_appends_ndjson`, `tp2_file_bounded_at_twice_cap`, `tp2b_restart_does_not_overshoot_cap`,
  `tp3_default_config_opens_nothing`, `tp10_in_flight_record_survives_shutdown`,
  `tp13_stdout_decision_key_set`.
- **Promise:** the two new cross-field config rejections (TP-5a) exist and hold.
  **Witness:** `cargo test --test config_validation`. **Result:** 18 passed, including
  `tp5a_log_file_max_bytes_requires_path` and `tp5a_log_file_max_bytes_rejects_zero`.
- **Promise:** no new lint debt (TP-12). **Witness:** `cargo clippy --all-targets -- -D warnings`.
  **Result:** exit 0, no warning lines.
- **Promise:** nothing else in the crate broke. **Witness:** full `cargo test`. **Result:** 285
  passed, 0 failed, 15 pre-existing ignored.
- **Promise (TP-14):** the invariant "nothing a client sends reaches a log line" still holds, and
  the check that verifies it is a live search, not a dead one. **Witness, three required runs:**
  product root scan → **0 findings**. Unsafe control crate (a two-file crate built solely to
  contain a `request.headers()` value flowing into an `offer()`-shaped call, per the exact
  heredoc in `04-slices.md`) → **fires**, at `main.rs:13`. Non-vacuity run (same spec with the
  `receiver_type` filter deleted, run against the product root) → **49 call sites matched**,
  proving the product-root zero reflects the filter working, not a search that can never fire.
  The control crate was deleted afterward, per the plan's cleanup step.
- **Promise:** the change survives review. **Witness:** `sf-code-review`. **Result:** first pass
  FIX FIRST with one Minor defect (below); after the fix, a bounded recheck returned CLOSED with
  no new defect.

## The defect found and fixed

`Writer::append` checked the rollover threshold *before* `open()` restored the byte tally from
the file's on-disk length. On a freshly constructed writer the tally starts at 0, so the first
record appended after a process restart skipped the size check entirely — even when the live
file was already sitting just under the cap — and could push it over. The same ordering bug also
affected the path after a write I/O error, since an error also nulls the handle and resets the
tally. Fixed by calling `open()` first and checking the threshold after.

Its regression witness, `tp2b_restart_does_not_overshoot_cap`, fails against the pre-fix code at
4422 bytes against a 4096-byte cap.

Worth noting for honesty: the first draft of `tp2b` asserted only that the *live* file stayed
under the cap, and that version passed even against the broken code — because the oversized file
does not stay live, the shutdown summary record rolls it to `.1` right after. The committed test
instead asserts no generation (live or `.1`) ever exceeds the cap, which is what actually catches
the bug.

## Limits

**The TP-14 check is a forbidden-call inventory, not a reachability proof — this is the one limit
to carry into slices 2 and 3.** The Gate 3 design approved TP-14 as a `taint_query` tracing client
input to the sink call. Its first real run against clean source failed: the query's source globs
are bare names with no directionality, so it matched our own outbound `response.headers()`
(computing the `bytes` field), the upstream registry client's `response.headers()`, and
`into_bytes()` on locally rendered markup — none of them client-supplied data. Repair established
that in this SMTC build every discrimination lever on `taint_query` — `receiver_type`, `package`,
`scope.files`, `sanitizers` — is a no-op, confirmed against the engine source and by passing a
filter engineered to match nothing and getting an unchanged finding. Backward taint is
function-granular (it flags `decide()` merely for containing both a `body()` call and a path to
`offer`, anywhere in the function). Forward taint returns a clean zero, but has a proven false
negative on exactly the breach shape this spec exists to catch: a header bound to a named local,
used, then moved into an enum-wrapped `offer()` call — literally the shape `Decision` /
`Record::RequestDecided` has.

So TP-14 is now a `structural_pattern` forbidden-call inventory: it fires if any inbound-request
accessor (`headers`, `body`, `into_body`, `to_bytes`, `from_request*` on a `request`/`req`/`parts`
receiver) exists anywhere in the crate, and it never traces where the value goes. What this gives
up: it does not prove any such call reaches `offer` — an unrelated inbound read anywhere else in
the crate would also trip it. That over-banning bias is deliberate, and it is why the zero and the
49-site non-vacuity run matter together: the zero alone would prove nothing if the search could
never fire. `03-program-design.md` `## Invariants & spec dispositions` and `04-slices.md` were
corrected at this slice to describe this form rather than the approved-but-unachievable one.
Slice 3 is where a peer-IP-derived field first reaches `offer`; TP-14 by construction cannot see
that value (a `ConnectInfo` read is not an inbound-request accessor), so its re-run there confirms
only that slice 3 added no *new inbound accessor* — not that the peer IP itself is handled safely.

**Three test-only seams beyond the design's declared list.** `App::delivery_is_empty`,
`Running::background_task_count`, and `Tasks::count`, all `#[cfg(feature = "test-support")]`, were
added so `tp3_default_config_opens_nothing` could observe `pub(crate)` state. They compile out of
every default build. Code review judged them idiomatic against the existing
`logging::set_summary_window` seam already in the crate.

**Product-level limits already accepted upstream, unchanged by this slice:** consumer
identification does not exist until slice 3 and is off by default even then (`01-product.md`);
delivery drops rather than blocks or fails a request, and every drop is counted (`Summary.dropped_siem`
and `Drops.siem` exist now but stay at zero until slice 2 wires the SIEM sink — `04-slices.md` C25);
one rollover generation only, no retention beyond that (`03-program-design.md` Least confident
decision 6).

## Next

Slice 2: the SIEM (HTTPS) sink — batched, authenticated from `OSPREY_SIEM_AUTH`, never following a
redirect, never blocking a request or hanging shutdown on a wedged collector, with drops counted
per sink. Slice 3 after that: opt-in peer-IP consumer identification, off by default, refused at
startup with no sink configured.

## Recommendation

Continue. All acceptance checks named for this slice passed, the one review defect found was
fixed with its own regression witness, and the one approved-but-unachievable check (TP-14) was
repaired to a form that is honest about what it can and cannot prove — rather than left silently
weaker than the gate documents claimed.

Status: Gates 1–4 approved 2026-09-21; slice 1 of 3 complete, unbuilt: slices 2–3.

Sources: `docs/plans/decision-log-delivery/00-status.md`; `04-slices.md` (Slice 1 row, `### Slice
1 interfaces`, TP-14 witness steps); `01-product.md` (`## Decisions recorded here rather than
left to a later gate`); `03-program-design.md` (`## Invariants & spec dispositions`, `## Threat
model` T-INVARIANT-FIRST-BREACH); `sf-code-review` result recorded in `00-status.md`.

Continue to slice 2, or re-steer?
