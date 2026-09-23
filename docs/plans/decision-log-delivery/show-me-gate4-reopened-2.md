## What problem do we have?
Two gaps were found after Slice 3 (consumer identification) shipped once and passed its Security
review's `FIX FIRST` follow-up: the file sink can silently lose queued decision records when
shutdown's 5-second drain deadline cuts it off mid-flush (nothing increments the drop counter, so
an operator sees no evidence a record was ever lost), and the SIEM client honours `HTTP_PROXY`/
`HTTPS_PROXY` environment variables while the product's other outbound HTTP client
(`src/upstream/reqwest_transport.rs:101`) explicitly disables that behaviour with `.no_proxy()` —
an inconsistency nobody had decided was intentional. Both were raised as open items in Slice 2's
proof line on 2026-09-21 and left unresolved until now. Gate 3 was reopened on 2026-09-22 and
re-approved for both (C42, C43); this Gate 4 revision assigns each to Slice 3's row so the same
code change that is already reopening that slice also closes these two gaps.

## How will we solve it?
Slices 1 and 2 (file sink, SIEM sink) are already implemented and shipped in the working tree —
they are history here, not a decision. Slice 3 is the live row, already reopened for three
Security findings from 2026-09-21; this revision adds two more changes to the same slice:

| Slice | Outcome | APIs used/changed | Available dependency | Deciding check | Temporary limit |
|---|---|---|---|---|---|
| 1 (shipped) | Decisions land in an operator-named file, bounded and drained at shutdown. | `delivery::{Record,Sinks,build}`, `delivery::file::run` | tokio, serde_json | 6 tests in `tests/decision_log_delivery.rs` + 2 in `config_validation.rs`, clippy, TP-14 | none |
| 2 (shipped) | Decisions batch-POST to a SIEM collector over HTTPS. | `delivery::siem::run`, `delivery::build` SIEM arm | reqwest 0.13.5, wiremock | 13 tests + 2 config tests, clippy | none |
| 3 (live, this revision) | Opt-in `log_consumer_identification` records the peer IP; file sink now probes at startup and stops losing un-drained records uncounted; SIEM client stops honouring the proxy environment. | `http::logging::decide` (ConnectInfo read), `delivery::build` (startup probe, `.no_proxy()`), `src/delivery/file.rs` (`.mode(0o600)`, new private `drain()`), `src/config.rs:295` (reason string) | axum `ConnectInfo`, both retained `.smtc` specs already authored | See "How will we confirm it" below | C43 (`.no_proxy()`) has no execution witness — code review only, as Gate 3 itself decided |

What changed today (2026-09-22, C44/C45), added to Slice 3's already-expanded file list — no new
files, since `src/delivery/file.rs` and `src/delivery/mod.rs` were already in scope from the
2026-09-21 reopening:
- **`src/delivery/file.rs`**: the drain loop moves into a private `drain(writer, rx, drops,
  deadline)`. When the 5-second `tokio::time::timeout` cuts it off, `rx.len() as u64 + 1` is added
  to the drop counter instead of being silently discarded. New unit test
  `tp19_file_drain_deadline_counts_cut_off_records` (C42, TP-19).
- **`delivery::build` in `src/delivery/mod.rs`**: add `.no_proxy()` to the SIEM
  `reqwest::Client::builder()` chain, alongside the existing `redirect::Policy::none()` and the
  two timeouts, matching `src/upstream/reqwest_transport.rs:101` (C43). No test — `sf-code-review`
  compares the two builder chains and reports whether `.no_proxy()` is present.
- **`### Slice 3 interfaces`, `.mode` attribution corrected (C45)**: `Writer::open` gets its file
  mode from `tokio::fs::OpenOptions`'s own inherent `.mode` (not `OpenOptionsExt`, which that type
  doesn't implement); only the separate startup probe in `build` uses
  `std::os::unix::fs::OpenOptionsExt`. This corrects an earlier misattribution that had put
  `.no_proxy()` in `src/delivery/siem.rs` instead of `delivery::build` — all four places that name
  it (C43's row, C44's row, the reopening bullet, and `### Slice 3 interfaces`) now agree.

What Slice 3 still carries from the 2026-09-21 reopening (unchanged by today's revision, restated
for completeness since the row is the same one): `.mode(0o600)` on every file open that can create
the decision log (C27); a startup probe in `delivery::build` that opens the path and fails startup
only when the opt-in is on (C28, C36, C39); the corrected `log_consumer_identification` reason
string at `src/config.rs:295` (C35); `tp16_unopenable_log_path_fails_startup` and
`tp17_log_file_mode_is_0600` (C33); the `tp4` strengthening to a distinct bound address plus
forwarding-header case (C31, C38); and the second retained `.smtc` spec covering the
`request.extensions()` read route (C29).

## How will we confirm it is solved?
Slice 3 runs six witness steps in order, all from the repository root:

| Step | Scenario | Expected result | Status |
|---|---|---|---|
| 1 | `cargo test --test decision_log_delivery` (`tp4`, `tp13`, `tp16`, `tp17` + all Slice 1/2 tests) | all `ok` | Red today: `tp16`/`tp17` don't exist |
| 2 | `cargo test --test config_validation` (`tp5c`, full reason string) | passes | Red today: old reason string doesn't match |
| 3 | `cargo clippy --all-targets -- -D warnings` | exit 0, no warnings | — |
| 4 | `TP-14` (3 runs: product root, control, non-vacuity) | 0 findings / fires / non-zero | Passes today; re-run confirms slice 3 added no `headers()`/`body()`-style read |
| 5 | `TP-18`: `smtc spec run … consumer-identity-extension-read.yaml \| jq -e '.ok==true and findings length==1 and file endswith logging.rs'` | exit 0 | Passes today against the spec; confirms the `extensions()` route stays the one approved read site |
| 6 | `cargo test --lib tp19_file_drain_deadline_counts_cut_off_records` | `1 passed` | **Red today: test doesn't exist, so `0 passed`** (C42, C44) |

C43 (`.no_proxy()`) has no step of its own by design — Gate 3 decided code review is sufficient
since there's no behavioral difference to assert without a live proxy.

Reviews, each gated on a re-run because Slice 3 was implemented once already:
- `sf-security-review` — re-confirms the three 2026-09-21 findings (T-LOGFILE-WORLD-READABLE,
  T-SILENT-DESTINATIONLESS-COLLECTION, T-EXTENSION-ROUTE) stay closed.
- `sf-code-review` — re-confirms its two prior minor findings are addressed, and newly compares
  the SIEM client builder in `delivery::build` against `src/upstream/reqwest_transport.rs:101` for
  `.no_proxy()` (C43), and checks that `drain` counts `rx.len() + 1` on cutoff (C42).
- `sf-verification` — last slice; checks the feature's promised outcomes hold, including the
  `docs/operations.md` prose items (a)-(f) that have no executable witness.

Gate QA ran fresh and returned READY (`docs/plans/decision-log-delivery/gate-4-qa.md`): every
Gate 3 decision (C27-C40, C42, C43) is traced to an exact file, witness step or named review in
`04-slices.md`; the `.mode`/`.no_proxy()` misattribution from the earlier revision is confirmed
gone from all four places; the claimed-red witnesses (TP-16, TP-17, TP-19, `tp5c`'s full reason
string) were independently checked against current code and confirmed to fail today as stated;
`cargo`, `cargo-clippy`, `smtc`, `jq` all resolve on PATH.

Recommendation: approve. The revision closes both 2026-09-22 reopening items (C42, C43) inside the
slice that was already being reopened for the 2026-09-21 findings, without adding files or
inventing new scope, and Gate QA found no unresolved decision, contradiction or stale witness
claim.

Limits: C43 (`.no_proxy()`) has no execution witness by Gate 3's own design — it is verified by
code review only, not by a test. Gate QA did not re-run the full Slice 1-2 test suites or the
TP-14/TP-18 `smtc` witnesses itself; it relied on grep/read checks against current source plus two
independently-reproduced prior command outputs (the `tp19` compile-and-run and the `no_proxy`
grep). `01-product.md` was not re-read line-by-line this round; only its cited IDs were
cross-checked.

Gate 4 (Slice plan) is in progress; Gates 1-3 are approved (Gate 3 re-approved 2026-09-22 for C42,
C43).

Sources: `docs/plans/decision-log-delivery/04-slices.md`, `03-program-design.md`,
`02-architecture.md`, `00-status.md` (Slices section, Notes), `gate-4-qa.md`.

Approve Gate 4, or what should change?
