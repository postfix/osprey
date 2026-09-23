# Gate 4 QA
## Verdict
READY
## Questions and findings
None blocking.

- QA-G4-R4-1 (prior, blocking): CLOSED. `## Slices` header (04-slices.md:73) now matches the template's nine columns, with `Reviews` between `Temporary artifacts` and `Visible review triggers`. Values: slice 1 `code-review`; slice 2 `code-review, security, adversarial`; slice 3 `code-review, security, qa`; slice 4 `code-review, qa`. All are bare router kinds, and each agrees with its row's `Visible review triggers` (slice 4: `sf-code-review`, `sf-verification`; `sf-security-review` omitted with the Gate 3 threat-model reason).
- QA-G4-R4-2 (prior, minor): CLOSED. The build-order sentence (04-slices.md:3-6) names slice 4 and `TP-20`.

## Checked dimensions
- Subject completeness, readability, freshness: PASS. 04-slices.md (fourth revision), approved 01/02/03, the four evidence files, and the relied-on source were tracked, then read. Source claims were re-checked with SMTC on the current working tree. `summarise` has no clock parameter (src/http/logging.rs:349), and neither do `flush_summary` (:376) or `close_window` (:404). The `Decision` literal in `decide` has no `timestamp` field. `flush_summary(&self.app.delivery)` in `Running::shutdown` (src/lib.rs:321). The `tp19` `Summary` literal starts at src/delivery/file.rs:236. `pub clock: Arc<dyn Clock>` on `App` (src/lib.rs:49). `Clock::now_utc_micros` (src/clock.rs:8-12). `jiff = "0.2.37"` (Cargo.toml:52). `TestClock`, `at_rfc3339`, `advance_seconds`, `shared` and `TestServer::start_with(config, clock)` all exist in tests/common/mod.rs with the signatures `TP-20` uses.
- Clarification inventory: PASS. C22–C26, C32–C34, C44, C45, C47, C50 and C51 all have a stable ID, class, `resolved` status, owner 4, target `none`, a selected value and a decision source. No open or deferred row exists in Gates 1–4. Gate 1 C4–C6 are resolved at Gate 2 under their original IDs. C51, an implementation verification, names the selected policy and the red check (step 1), and hides no unselected value.
- Gate 4 vertical slices: PASS. Slice 4 is one working caller outcome: file and SIEM records carry `timestamp`, and stdout does not change. It depends only on slices 1–3 and on existing code. `Exact files` covers every site in evidence/impact-c49.md: logging.rs:206, :243, :366, :379 and :406; lib.rs:321; file.rs:236. It also covers `rfc3339` and the struct fields in src/delivery/mod.rs, `tests/decision_log_delivery.rs` and `docs/operations.md`, which matches the Gate 3 `## Files` entries for C49. It correctly leaves out `config.sample.toml` and `src/config.rs`, because C49 adds no key.
- Falsifiable direct witnesses: PASS. Step 1 is red today because `tp20_delivered_records_carry_timestamp` does not exist, and its assertions match Gate 3 `TP-20` exactly: the injected-clock start, the 60 s advance, the last `request_summary`, and SIEM parity. It discriminates:
  - A `SystemClock` read fails the exact strings.
  - The `.120000` value fails a plain `Display`.
  - The advance fails a summary that copies the decision's time.
  - The "last" `request_summary` makes the check robust to a periodic summary, because process-global `COUNTERS` are shared within the test binary but each record goes to its caller's own sinks. `set_summary_window` is used only in tests/http_contract.rs, a different binary.

  Step 2 (`cargo test`) covers `tp19` in the lib target after the literal edit. evidence/slice-3.md records the full `cargo test` passing in every target, so step 2 is achievable. Step 3 is the typechecker (`TP-12`), required because typed source is in `Exact files`.
- Test prerequisites: PASS. `cargo` and `cargo-clippy` are backticked programs. The helpers are named, and wiremock binds loopback only.
- Resolved prerequisites and no Engineering deferral: PASS. C49's formatter, type, field order, plumbing and doc text are all fixed in `### Slice 4 interfaces`. The `rfc3339` body is given verbatim, and it matches Gate 3 and evidence/research-jiff-format.md.
- Unwitnessed residuals are assigned: PASS. The Gate 3 invariant residual (which clock `summarise` forwards) is assigned to `sf-code-review` in the row and in Stage limits. `docs/operations.md` prose is assigned to `sf-verification`.
- Red Team: PASS. `sf-red-team: not triggered` for the fourth revision, with a stated reason: Gate 3 records that C49 adds no entry point, trust boundary, destination or input (03-program-design.md `## Threat model`). The earlier revisions keep their own recorded reasons.
- Slice history: PASS. Slices 1–3 are marked complete. C50 records why the timestamp is a new slice rather than a reopened slice 3: slice 3's proof line stays valid.
- Gates 1–3 stage-specific dimensions: not applicable (approved upstream; read for traceability only).

## Limitations
- `00-status.md` could not be tracked: `gate-qa track` refuses it as machine state. So the renamed slice-2 review keys (`security=CLEAR, adversarial=PASS`) and the Gate 1 reopening note that C50 cites were not independently read. C50's decision and rationale are recorded in 04-slices.md itself, so no decision depends only on bookkeeping. The main agent reports that `router check` returns no findings.
- `TP-13` checks only the stdout *decision* key set. That the stdout *summary* line (`emit`) leaves out `timestamp` has no execution witness. Gate 3 accepted this, and slice 4's code review does not name it explicitly. It is worth adding to the review prompt, but it does not block readiness.
- Gate 1's rated-load guard (P-GUARD-LOAD) still has no executed load witness. This gap was inherited from approved Gate 3 and slice 3 verification, and this revision does not change it.
- Source evidence is Manual/Structural (SMTC file read/grep). No build or test was executed in this review.
