## What problem do we have?

Gates 1-3 approved *what* to build: optional delivery of the existing per-decision log record to
a local file and to the operator's SIEM, plus an off-by-default opt-in that records the peer IP
(reversing `src/http/logging.rs:14`'s "nothing a client sends reaches a log line" rule only for
that opt-in). No implementation code exists yet. Gate 4 decides *how Engineering builds it*: the
order of three vertical slices, which check first proves each Gate 3 promise, and five sequencing
questions Gate 3 explicitly left open (`03-program-design.md` states "Gate 4 supplies that
command" for one of them).

## How will we solve it?

Build order: one sink end to end (a genuine tracer bullet, not a mock), then the second sink,
then the opt-in that reverses the client-data rule. Every `03-program-design.md` `## Test plan`
row is assigned to exactly one first-proving slice; nothing is left for Engineering to choose.

| Slice | Caller outcome | APIs used/changed | Available dependency | Deciding check | Temporary limit / removal |
|---|---|---|---|---|---|
| 1 — file sink | An operator who sets `log_file_path` gets every decision appended to that file as one NDJSON object per line, bounded at twice `log_file_max_bytes` with a single `.1` rollover, including records of requests still in flight at shutdown; an operator who sets nothing gets no file, no task, no console change. | New `src/delivery/{mod,file}.rs` (`Record`, `Decision`, `Summary`, `Drops`, `Sinks`, `build`, `Sinks::offer`); changed `http::logging::decide`/`summarise`, `src/lib.rs` shutdown splice, `tasks::spawn`, `src/config.rs`. | `tokio` (`fs` feature already on), `tokio::sync::mpsc`, `tokio_util::sync::CancellationToken`, `serde_json` — all already in the tree; zero new Cargo dependency. | `cargo test --test decision_log_delivery` (TP-1, TP-2, TP-3, TP-10, TP-13) + `cargo test --test config_validation` (TP-5a) + clippy (TP-12) + the retained analyzer spec against a deleted unsafe control (TP-14). All fail or don't exist today — `tests/decision_log_delivery.rs` doesn't exist. | `Summary.dropped_siem`/`Drops.siem` exist and are always zero (C25). Removal point: slice 2. |
| 2 — SIEM sink | An operator who sets `siem_url` gets decision records batched and POSTed as NDJSON, credentialed from `OSPREY_SIEM_AUTH`, never following a redirect, never blocking a request, never hanging shutdown against a silent collector; a drop is counted per sink and readable back out of the file sink's own output. | New `src/delivery/siem.rs`; changed `delivery::build` (SIEM arm, client with `redirect::Policy::none()`), new `StartupError::Delivery` on the crate-level enum at `src/lib.rs:274-283` (C26), changed `src/config.rs`. | `reqwest 0.13.5` (rustls) — already a direct dependency; `wiremock` — already a dev-dependency, used directly rather than the `tests/common` registry helper (wrong shape here). | `cargo test --test decision_log_delivery` (TP-7 through TP-11b, TP-15) + `config_validation` (TP-5b, TP-6) + clippy (TP-12). TP-15 must return inside the ~5s drain deadline — a hang is the failure. TP-7 reads the drop count back out of the file sink's NDJSON, not stdout. | None. This slice removes slice 1's temporary zero (fills `dropped_siem` for real). |
| 3 — consumer opt-in | An operator who sets `log_consumer_identification = true` alongside at least one sink gets the peer IP of the asking connection on every record, and is refused at startup with nowhere for it to go; with the key absent (default), no `consumer` field and the server is built exactly as today. | Changed `delivery::Decision` (new optional `consumer: Option<IpAddr>` field, C24), `http::logging::decide` reads `ConnectInfo<SocketAddr>` before `next.run(request)` consumes the request, conditional `axum::serve` branch in `src/lib.rs`, `src/config.rs` new key with its own startup rejection. | `axum 0.8.9` `ConnectInfo` / `into_make_service_with_connect_info`, used for the first time here — availability confirmed only by clippy compiling clean (TP-12, also settles Gate 3 C17). | `cargo test --test decision_log_delivery` (TP-4, TP-13 in its twelve-or-thirteen-key form) + `config_validation` (TP-5c) + clippy (TP-12) + TP-14 re-run against the product root — "the run that matters," the first time a per-connection value reaches `offer`. | None — but the peer IP is the only consumer identity this feature will ever record; `X-Forwarded-For`/`User-Agent` stay rejected (Gate 2 C7) and the retained analyzer spec fires if either is ever wired in. |

**The five Gate 4 clarifications (C22-C26), all resolved, none deferred:**
- **C22** — Gate 3 required `TP-14`'s unsafe control but left the command unspecified. Slice 1
  (and slice 3's re-run) create it with an exact `cat`-heredoc, outside the product root at
  `/tmp/osprey-tp14-control`, and delete it in the same witness step — nothing unsafe is
  committed.
- **C23** — `TP-5` covers three cross-key rejections whose keys arrive in three different slices,
  so it splits by the slice that introduces the key: `TP-5a` (slice 1), `TP-5b` (slice 2), `TP-5c`
  (slice 3). Together they are `TP-5`; none is dropped.
- **C24** — `Decision.consumer` lands in slice 3, not as a permanently-`None` field in slice 1:
  `serde(skip_serializing_if)` makes a `None` field invisible on the wire, so putting it in slice
  1 would be a field no check could observe or caller could set.
- **C25** — `Summary.dropped_siem`/`Drops.siem` exist from slice 1 (always zero) rather than
  appearing only in slice 2, because both fields are non-optional and always serialised — leaving
  them out in slice 1 would change the summary's key set twice and make `TP-7` a moving target.
- **C26** — `StartupError::Delivery` goes on the crate-level enum at `src/lib.rs:274-283` (the one
  `App::start` returns), not the unrelated `src/store/startup.rs:179-182` enum of the same name —
  confirmed by symbol search against the repository, not assumed.

`sf-red-team: not triggered` — these five are sequencing/check-assignment decisions over an
architecture and design already red-teamed and threat-modelled at Gate 3; no slice opens a new
trust boundary beyond what that threat model scoped.

## How will we confirm it is solved?

| Scenario | Expected result | Check |
|---|---|---|
| Run slice 1's witness today | `cargo test --test decision_log_delivery` fails: `error: no test target named \`decision_log_delivery\`` | Confirmed against the working tree at Git base `0f50cec6d526773abec4b4897f64afc10992a8ae` |
| Slice 1 complete | TP-1, TP-2, TP-3, TP-5a, TP-10, TP-12, TP-13, TP-14 all pass; default config opens no file, spawns no task | `## Slices` row 1, `Acceptance` |
| Slice 2 complete | TP-5b, TP-6, TP-7, TP-8, TP-9, TP-11a, TP-11b, TP-12, TP-15 all pass; every slice-1 check still passes | `## Slices` row 2, `Acceptance` |
| Slice 3 complete | TP-4, TP-5c, TP-12, TP-13 (13-key form), TP-14 (re-run, non-vacuous) all pass; every slice-1/2 check still passes | `## Slices` row 3, `Acceptance` |
| Every Gate 3 Test plan check (TP-1..TP-15) has exactly one first-proving slice | No orphaned or double-owned promise | Gate 4 QA, "Every Gate 3 Test plan check is assigned to exactly one first-proving slice" |
| Every architecture Fit-row file (`src/delivery/{mod,file,siem}.rs`, `http/logging.rs`, `config.rs`, `lib.rs`, `tasks/mod.rs`, `config.sample.toml`, `docs/operations.md`) is delivered by some slice | No orphaned file | Gate 4 QA, reference-closure check against `02-architecture.md` |

Recommendation: Approve — every Gate 3 promise and Test plan row has exactly one first-proving
slice with a witness that fails today for a stated reason, slice 1 is a genuine tracer bullet
(not layer-only or mocked), zero new Cargo dependencies are introduced, and all five sequencing
clarifications are resolved with a decision source rather than deferred to Engineering.

Limits: three deciding limits an approver should weigh, all already approved at Gate 3 and
reiterated here at the slice that makes them concrete, not new gaps Gate 4 introduces.
1. **Slice 2's whole-batch loss.** A non-retryable `4xx` from the SIEM collector drops and counts
   the *entire* 256-record batch, not just the offending record — one bad record can cost up to
   255 good ones beside it on a recurring basis. This is the approved Gate 3 C21 accept, owner
   product, not a gap for Engineering to close; the file sink is the answer for an operator who
   needs completeness.
2. **The shutdown drop tail reaches the console only.** `flush_drop_tail` runs after every sink
   task has returned, so a SIEM-only operator whose SIEM was down learns of the last drain
   window's drops from stdout or not at all (Gate 3 C20).
3. **Behind a proxy, NAT or load balancer, the recorded peer IP is that hop**, not the machine
   that made the original request — stated as an operator-facing fact slice 3 adds to
   `docs/operations.md`, not a defect to fix.

Gate 4 QA verdict: READY (fresh, `router gate-qa verify` confirms). No blocking findings; the two
non-blocking observations concern why `TP-12`/`TP-13`/`TP-14` legitimately recur across slices
(each rerun is a distinct, correctly-scoped assertion, not duplicate ownership) and a pre-existing
Gate 3 cross-reference that Gate 4 does not repeat. READY permits presentation only; it does not
constitute approval.

Sources: `docs/plans/decision-log-delivery/04-slices.md` (`## Clarifications and decisions`,
`## Slices`, `## Slice interface details`); `03-program-design.md` (`## Test plan`, `## Threat
model`); `docs/plans/decision-log-delivery/evidence/repository-structure.md`;
`.smtc/analyzers/client-data-reaches-delivery-sink.yaml`;
`docs/plans/decision-log-delivery/gate-4-qa.md`.

Approve Gate 4, or what should change?
