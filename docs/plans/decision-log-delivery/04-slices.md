# Vertical Slices: Decision log delivery

Build order: one sink end to end, then the second sink, then the opt-in that reverses the
"nothing a client sends reaches a log line" rule, then the `timestamp` on every delivered record
(slice 4, `TP-20`). Every check named in
`03-program-design.md` `## Test plan` is assigned below; nothing is left for Engineering to
choose.

Repository facts used here were read at Git base `0f50cec6d526773abec4b4897f64afc10992a8ae`:
`tests/config_validation.rs` exists and `tests/decision_log_delivery.rs` does not, so every
slice-1 witness fails today; `cargo`, `cargo-clippy` and `smtc` all resolve on PATH.

**Revised on 2026-09-21, after slice 3 was implemented and Gates 3 and 4 were reopened.** Slices 1
and 2 are shipped in the working tree, uncommitted on that same base, so the first two rows below
are history rather than instructions. Slice 3's row is the live one: it was implemented once, its
witness passed, and its Security review then returned `FIX FIRST` with three findings the
pre-implementation threat model did not contain. The row now carries the expanded file list and the
red checks those findings force (Gate 3 C27–C40, Gate 4 C32–C34). A Red Team review of the
revised Gate 3 and a Gate QA review then forced further changes, including a Gate 2 correction. `jq` is a fourth
witness tool as of this revision and resolves at `/usr/bin/jq`.

`sf-red-team: not triggered` — this gate adds no new decision with material uncertainty: it
slices an architecture and design that were already red-teamed, and the security surface was
threat-modelled inside that Gate 3 review (`03-program-design.md` `## Threat model`). The five
choices recorded below are sequencing and check-assignment, each with its own witness.

`sf-red-team: not triggered` for the 2026-09-21 revision either, for a stated reason rather than by
default: the reopening's decisions were all made at Gate 3, where an isolated Red Team with
`sf-threat-model` reviewed them and returned six findings, all resolved there (Gate 3 C35–C39).
This gate's revision only assigns those decisions to slice 3's files, witness steps and reviews
(C32–C34); it adds no decision of its own that Red Team has not already attacked.

**Revised again on 2026-09-22, after Gate 3 was reopened and re-approved for C42 and C43.** Slice 3
now also carries the file sink's drain-deadline accounting (Gate 3 C42, witness `TP-19`) and the
SIEM client's `.no_proxy()` (Gate 3 C43), which lands in `delivery::build` in `src/delivery/mod.rs`,
already in slice 3's `Exact files` (C44). The `.mode` attribution for `Writer::open` is corrected to match Gate 3 C27 (C45).
`sf-red-team: not triggered` for this revision: Gate 3 recorded that C42 and C43 need no further Red
Team round (`03-program-design.md` `## Threat model`), and this revision only assigns them to slice
3's files, witness steps and reviews.

**Revised a third time on 2026-09-22, after Gate 3 was reopened and re-approved for C46.** The
approved `TP-19` could not fail, so witness step 6 is rewritten in the C46 form and gains a
mutant run that proves it discriminates (C47). `sf-red-team: not triggered` for this revision:
Gate 3 recorded that C46 changes only a test's form, and this revision only makes step 6 match it.

**Revised a fourth time on 2026-09-22, after Gate 1 was reopened (C8) and Gates 2 and 3 re-approved
for the delivered-record timestamp (Gate 2 C48, Gate 3 C49).** Slices 1–3 are complete and stay
checked. This revision adds slice 4, which is only the timestamp (C50). `sf-red-team: not
triggered` for this revision: Gate 3 recorded that C49 adds no entry point, trust boundary,
destination or input (`03-program-design.md` `## Threat model`). This revision only assigns C49's
seven edit sites (`evidence/impact-c49.md`) and `TP-20` to one slice.

## Clarifications and decisions

| ID | Class | Status | Owning Gate | Target Gate | Question or disposition | Selected value or policy | Decision source |
|---|---|---|---|---|---|---|---|
| C22 | current-Gate decision | resolved | 4 | none | Gate 3 states that `TP-14`'s unsafe control must be materialised by the slice rather than committed, but does not give the command. | Slice 1's witness creates a dependency-free two-file Rust crate at `/tmp/osprey-tp14-control` — outside the product root, as the playbook requires — in which a `headers()` result reaches an `offer()` call. The exact `cat`-heredoc command is written out in `### Slice 1 interfaces`, `Acceptance`. It is deleted by the same slice's cleanup line, so nothing deliberately unsafe is committed. | `### Slice 1 interfaces` |
| C23 | current-Gate decision | resolved | 4 | none | `TP-5` covers three cross-key rejections whose keys arrive in three different slices, so it cannot be one check in one slice. | Split by the slice that introduces the key: `TP-5a` (`log_file_max_bytes` without `log_file_path`, and zero) in slice 1, `TP-5b` (`siem_auth_header` without `siem_url`) in slice 2, `TP-5c` (`log_consumer_identification` with no sink configured) in slice 3. The three together are `TP-5`; no rejection is dropped. | Slice rows 1–3, `Acceptance` |
| C24 | current-Gate decision | resolved | 4 | none | `Decision.consumer` is declared in the Gate 3 struct. Does it land in slice 1 as a permanently-`None` field, or in slice 3 with the opt-in that fills it? | Slice 3. `serde(skip_serializing_if = "Option::is_none")` means a `None` field is invisible on the wire, so adding it in slice 1 would be a field no check could observe and no caller could set. `TP-13` therefore asserts exactly twelve keys in slices 1–2 and twelve-or-thirteen in slice 3, which is what Gate 2 C9 promises an operator. | `### Slice 3 interfaces` |
| C25 | current-Gate decision | resolved | 4 | none | `Summary.dropped_siem` and `Drops.siem` are in the approved Gate 3 types, but the SIEM sink does not exist until slice 2. | Both fields exist from slice 1 and are always zero until slice 2 fills them. Unlike `consumer` they are non-optional and always serialised, so omitting them in slice 1 would change the summary key set twice and make `TP-7`'s reading of the summary out of the file sink a moving target. Listed as temporary behaviour on slice 1 with slice 2 as its removal point. | `### Slice 1 interfaces`, `Temporary behavior` |
| C26 | implementation verification | resolved | 4 | none | `StartupError::Delivery` is a **proposed variant on the existing `StartupError`** — but this repository declares two enums with that name. | The variant goes on the crate-level enum at `src/lib.rs:274-283`, which is the one `App::start` returns; the unrelated `src/store/startup.rs:179-182` enum is not touched. Confirmed by symbol search at the Git base above. The later check is slice 2's `TP-12`, which fails to compile if the wrong enum is extended. | Slice 2 `Exact files`; `### Slice 2 interfaces` |
| C32 | current-Gate decision | resolved | 4 | none | Gate 3 C30 puts C27's and C28's fixes inside a reopened slice 3, but both live in `src/delivery/file.rs` and `src/delivery/mod.rs` — slice 1 files. Does slice 3's `Exact files` grow, and does the implementer get write access to the two retained analyzer specs? | `Exact files` gains `src/delivery/file.rs` and `src/delivery/mod.rs`. It does **not** gain either `.smtc/analyzers/*.yaml`: both specs already exist on disk, authored and validated by an isolated `sf-spec-authoring` dispatch during this Gate 3 revision, so they are planning artifacts the slice consumes rather than files it writes. The implementer runs them and must not edit them — a witness an implementer can edit is not a witness. | Slice 3 row, `Exact files` and `Access` |
| C33 | current-Gate decision | resolved | 4 | none | Which test file owns `TP-16` (unopenable path fails startup) and `TP-17` (file mode `0o600`), given `tests/config_validation.rs` owns configuration rejection and `tests/decision_log_delivery.rs` owns delivery behaviour? | Both go in `tests/decision_log_delivery.rs`. Neither is a configuration-validation check: `TP-16`'s rejection happens in `delivery::build` **after** `RawConfig::validate` has accepted the configuration — that separation is the whole of C28 — and `TP-17` asserts a property of a file the sink produced. Putting either in `config_validation.rs` would imply the key-level validator catches them, which is exactly the false belief C28 exists to correct. | Slice 3 row, `Direct witness` steps 1 |
| C34 | implementation verification | resolved | 4 | none | Does the revised slice 3 witness still fail against the code as it stands, given that `TP-4` strengthened, `TP-14` and `TP-18` all pass today? | Yes, on three checks, as of the 2026-09-21 revision. **Superseded for the current tree:** slice 3's paused code now holds `tp16`, `tp17`, `tp5c` and the new reason string, so all three are regression checks, and step 6b is the slice's only red check (C47). As written on 2026-09-21: `TP-17` fails today — the file is created `0o666 & ~umask`, and nothing creates it before the first request; `TP-16` fails today, because an unopenable path currently starts cleanly; and `tp5c` fails today on its full reason-string assertion, which the old string does not match (Gate 3 C35). The other three are regression and guard checks that are expected to pass before and after, and are labelled as such rather than presented as red tests: `TP-4` strengthened is a discrimination fix to a check that already passes against correct code (Gate 3 C31), and `TP-14` and `TP-18` are the retained specs' guard runs. | Slice 3 row, `Direct witness` |
| C44 | current-Gate decision | resolved | 4 | none | Gate 3 C42 and C43 were reopened into slice 3. Where do they land in the row, and which check owns each? | C42 → `src/delivery/file.rs` (already in slice 3's files), witnessed by `TP-19` as a `#[cfg(test)]` unit test named `tp19_file_drain_deadline_counts_cut_off_records` inside that file, because `drain` is private. It runs as its own witness step through `cargo test --lib`, since `cargo test --test decision_log_delivery` never compiles unit tests. It is a fourth red check: today the test does not exist, and the step requires `1 passed`. **Superseded by C47:** the three-record test now exists and passes against the pre-fix code; step 6 is red through its mutant run 6b. C43 → `delivery::build` in `src/delivery/mod.rs`, where the SIEM `reqwest::Client` is built (Gate 3 `#### build` Effects); that file is already in `Exact files` since C32, so no file is added. It gets no execution witness (Gate 3 C43); `sf-code-review` must compare the SIEM client builder in `src/delivery/mod.rs` against `src/upstream/reqwest_transport.rs:101` and report whether `.no_proxy()` is present. | Slice 3 row, `Exact files`, `Direct witness` step 6, `Visible review triggers`; `### Slice 3 interfaces` |
| C45 | current-Gate decision | resolved | 4 | none | `### Slice 3 interfaces` said `Writer::open` gets `.mode(0o600)` from `std::os::unix::fs::OpenOptionsExt`. `Writer::open` uses `tokio::fs::OpenOptions`, which does not implement that trait. | The mode comes from different APIs at the two call sites, as Gate 3 C27 states: `Writer::open` uses `tokio::fs::OpenOptions`'s own inherent `.mode`, and the startup probe in `build` uses `std::fs::OpenOptions` with `OpenOptionsExt`. `TP-12` (clippy with `-D warnings`) catches an unused trait import. | `### Slice 3 interfaces` |
| C47 | current-Gate decision | resolved | 4 | none | Gate 3 C46 rewrites `TP-19`. The tree now holds slice 3's paused code: `drain` with its `rx.len() + 1` accounting, and a three-record `tp19` test. C44's "red today: the test does not exist" is false, and a `1 passed` run against code that already has the fix proves nothing. How does step 6 fail today and prove the test discriminates? | Step 6 has two parts. (a) The rewritten test passes on the real tree. (b) The same test runs on a copy of the tree where `sed` deletes the three-line `if drained.is_err() { .. }` branch from `drain`, and must fail on assertion (2), not on the `inconclusive` message. The command prints `TP19-DISCRIMINATES` only in that case. Red today: run by the main agent on 2026-09-22, part (b) against the current three-record test printed `test result: ok. 1 passed` and no `TP19-DISCRIMINATES`. That is the same false pass C46 records. The `sed` was dry-run on a copy of `src/delivery/file.rs`: it removes exactly lines 74–76 and nothing else. | Slice 3 row `Direct witness` step 6, `Temporary artifacts`; `### Slice 3 interfaces` |
| C50 | current-Gate decision | resolved | 4 | none | Gate 3 C49 was designed after slice 3 was proven. Does the timestamp reopen slice 3, or go in a new slice? | A new slice 4. The Gate 1 reopening note in `00-status.md` records that the timestamp returns "as a new slice 4" and that slices 1–3 stay checked. Slice 3's proof line covers its own row, and this change invalidates none of it. Slice 4's `Exact files` are the files holding the seven sites in `evidence/impact-c49.md`, plus `tests/decision_log_delivery.rs` for `TP-20` and `docs/operations.md` for the field's documentation. `config.sample.toml` and `src/config.rs` are not in the list: C49 adds no key. | Slice 4 row; `### Slice 4 interfaces` |
| C51 | implementation verification | resolved | 4 | none | Does slice 4's witness fail against the code as it stands? | Yes. On 2026-09-22 the main agent ran an SMTC grep over the working tree. It found no `pub timestamp` field and no `fn rfc3339` in `src/`. `summarise` (`src/http/logging.rs:349`), `flush_summary` (`:376`) and `close_window` (`:404`) have no `clock` parameter. `tp20_delivered_records_carry_timestamp` does not exist, so step 1 cannot name it as `ok`. The helpers `TP-20` needs already exist in `tests/common/mod.rs`: `TestClock` (`:49`), `TestClock::at_rfc3339` (`:71`), `advance_seconds` (`:79`), `shared` (`:95`) and `TestServer::start_with` (`:146`). The slice adds no test helper. | Slice 4 row `Direct witness`, `Test prerequisites` |
| C52 | user clarification | resolved | 4 | none | Slice 4's final `sf-verification` returned `UNCERTAIN` on one promise only: P-GUARD-LOAD (`01-product.md:38`, "no decision record is lost while the firewall is serving at its rated load"). No gate ever gave "rated load" a number, and no load test exists. Only the second half of the promise has an executed witness: a dropped record is counted and reported (`tp7`, `tp15b`). Slices 1–3 carried the same gap. How is it closed before slice 4's proof line? | Accepted as a stated limitation, no slice 5 and no Gate 1 rewording. The delivered half stands and is witnessed: a record the queue cannot take is counted and reported through `dropped_file` / `dropped_siem` (`tp7`, `tp15b`). The no-loss half keeps no executed witness, because "rated load" is not defined anywhere in the firewall and a number chosen now would be a guess. An operator learns about loss from the drop counters. This limitation is recorded in `evidence/slice-4.md` and stands as the feature's one accepted verification gap. | user answer, Gate 4 grilling Q1, 2026-09-23 |

## Slices

| Slice | Outcome | Dependencies | Exact files | Direct witness | Test prerequisites | Temporary artifacts | Reviews | Visible review triggers |
|---|---|---|---|---|---|---|---|---|
| 1 | An operator who sets `log_file_path` gets every decision appended to that file as one NDJSON object per line, bounded at twice `log_file_max_bytes` with a single `.1` rollover, including the records of requests still in flight when the process shuts down; an operator who sets nothing sees no file, no task and no change to the console line. Details: [Slice 1](#slice-1-interfaces) | Everything it needs exists at the Git base: `tokio 1.53.1` with the `fs` feature, `tokio::sync::mpsc`, `tokio_util::sync::CancellationToken`, `serde_json`, the `Config`/`RawConfig`/`OPTIONAL_KEYS` validation idiom at `src/config.rs:117,139-199,240`, the `Tasks` handle set at `src/tasks/mod.rs:26-52`, and the `Running::shutdown` order at `src/lib.rs:255-271`. No earlier slice. | `src/delivery/mod.rs` (new), `src/delivery/file.rs` (new), `src/http/logging.rs`, `src/config.rs`, `src/lib.rs`, `src/tasks/mod.rs`, `tests/decision_log_delivery.rs` (new), `tests/config_validation.rs`, `config.sample.toml`, `docs/operations.md` | Run all four, in this order, from the repository root:<br>1. `cargo test --test decision_log_delivery` → passes, and its output names `tp1_file_sink_appends_ndjson`, `tp2_file_bounded_at_twice_cap`, `tp3_default_config_opens_nothing`, `tp10_in_flight_record_survives_shutdown`, `tp13_stdout_decision_key_set` as `ok`. Today this fails with `error: no test target named `decision_log_delivery``.<br>2. `cargo test --test config_validation` → passes including `tp5a_log_file_max_bytes_requires_path` and `tp5a_log_file_max_bytes_rejects_zero`. Today those two do not exist.<br>3. `cargo clippy --all-targets -- -D warnings` → exits 0 with no warning lines (`TP-12`).<br>4. `TP-14`, three runs, all required. Create the control with the heredoc in `### Slice 1 interfaces`, then run exactly:<br>`smtc spec run --root /home/john/go/src/github.com/postfix/osprey --spec-file /home/john/go/src/github.com/postfix/osprey/.smtc/analyzers/client-data-reaches-delivery-sink.yaml --format json --max-tokens 2048 --session-id tp14` → **0 findings**.<br>`smtc spec run --root /tmp/osprey-tp14-control --spec-file /home/john/go/src/github.com/postfix/osprey/.smtc/analyzers/client-data-reaches-delivery-sink.yaml --format json --max-tokens 2048 --session-id tp14` → **fires** at the `request.headers()` call site.<br>Then the non-vacuity run: copy the spec, delete the `receiver_type:` line, and run it against the product root → must match a **non-zero** number of call sites. A zero there means the search is dead and the product zero proves nothing — that is `UNKNOWN`, not a pass. Then `rm -rf /tmp/osprey-tp14-control`. | `cargo`, `cargo-clippy`, `smtc` — all resolve on PATH at this base. No network: every test drives the in-process HTTP surface through the existing tests/common helpers. The TP-14 control crate is created by the witness itself, by the heredoc below. | The `/tmp/osprey-tp14-control` crate, deleted by witness step 4. Nothing else. | code-review | `sf-code-review` — a new module plus the first change to the per-request path since the MVP shipped. |
| 2 | An operator who sets `siem_url` gets decision records batched and `POST`ed to that collector as newline-delimited JSON, authenticated from `OSPREY_SIEM_AUTH` when it is set, never following a redirect, never blocking a request, and never hanging shutdown when the collector goes silent; a drop is counted per sink and readable out of the file sink's own output. Details: [Slice 2](#slice-2-interfaces) | Slice 1 supplies `Record`, `Sinks`, `Sinks::offer`, `Sinks::drops`, `delivery::build`, the drain token and the `Running::shutdown` order. `reqwest 0.13.5` (rustls) is a direct dependency and `wiremock` is already a dev-dependency (`Cargo.toml:87`), used today at `tests/common/mod.rs:900-923`. | `src/delivery/siem.rs` (new), `src/delivery/mod.rs`, `src/lib.rs`, `src/config.rs`, `tests/decision_log_delivery.rs`, `tests/config_validation.rs`, `config.sample.toml`, `docs/operations.md` | 1. `cargo test --test decision_log_delivery` → passes, output naming `tp7_drop_is_counted_and_read_back_from_file`, `tp8_batch_shape_and_optional_credential`, `tp9_redirect_is_not_followed`, `tp11a_credential_absent_from_runtime_output`, `tp11b_credential_absent_from_startup_rejection`, `tp15_shutdown_bounded_against_wedged_collector` as `ok`, plus every slice-1 test still `ok`. `tp15` must return within roughly the five-second drain deadline; a hang **is** the failure.<br>2. `cargo test --test config_validation` → passes including `tp5b_siem_auth_header_requires_siem_url` and `tp6_siem_url_scheme_rule` (four URLs: `https://…`, `http://127.0.0.1…`, `http://localhost…` accepted, `http://collector.example…` rejected as `ConfigError::Invalid { key: "siem_url" }`).<br>3. `cargo clippy --all-targets -- -D warnings` → exits 0 with no warning lines (`TP-12`). | `cargo`, `cargo-clippy`. The wiremock mock server binds loopback only — no outbound network. The tp15 test creates its own never-responding tokio TCP listener inside the test. The tp11a and tp11b tests set the environment variable named OSPREY_SIEM_AUTH inside the test process to a sentinel value; no credential is read from the developer's environment. | none | code-review, security, adversarial | `sf-security-review` — the credential, the first outbound egress from this process, and redirect policy. `sf-adversarial-testing` — retry ladder, batch loss and the wedged-collector deadline. `sf-code-review`. |
| 3 | An operator who sets `log_consumer_identification = true` alongside at least one sink gets the peer IP of the asking connection on every delivered record, written to a file only they can read, and is refused at startup if they turn it on with nowhere for the data to go **or with a destination that cannot actually be opened**; with the key absent — the default — no record carries a `consumer` field and the server is built exactly as it is today. Details: [Slice 3](#slice-3-interfaces) | Slices 1 and 2 supply both sinks, the `Decision` record and the `Config` surface. `axum 0.8.9`'s `ConnectInfo` / `into_make_service_with_connect_info` are used for the first time here; `TP-12` is what confirms they are available under the declared features (Gate 3 C17). Both retained analyzer specs already exist on disk, authored and validated during the Gate 3 revision (C32). | `src/http/logging.rs`, `src/delivery/mod.rs`, `src/delivery/file.rs`, `src/config.rs`, `src/lib.rs`, `tests/decision_log_delivery.rs`, `tests/config_validation.rs`, `config.sample.toml`, `docs/operations.md`. **Read-only, never edited by the implementer:** `.smtc/analyzers/client-data-reaches-delivery-sink.yaml`, `.smtc/analyzers/consumer-identity-extension-read.yaml` (C32) | Run all six, in this order, from the repository root:<br>1. `cargo test --test decision_log_delivery` → passes, output naming `tp4_consumer_off_by_default_and_recorded_when_on`, `tp13_stdout_decision_key_set`, `tp16_unopenable_log_path_fails_startup` and `tp17_log_file_mode_is_0600` as `ok`, plus every slice-1 and slice-2 test still `ok`. **`tp16` and `tp17` were the two red checks of the first 2026-09-21 slice 3 implementation (C34); they now exist in the tree and run here as regression checks.** `tp16`, two cases with `log_file_path` inside a directory that does not exist: opt-in on → `App::start` returns `StartupError::Delivery`, nothing serves, and the error text carries no peer address; opt-in off → `App::start` succeeds, a request is served, and exactly one `tracing::error!` names the key (Gate 3 C39). `tp17`, four cases: after one request, `metadata(path).permissions().mode() & 0o777 == 0o600` for the live file; after a forced rollover, the same for the `.1` generation; a file pre-created `0o644` before start keeps `0o644` and produces exactly one `tracing::warn!` naming the path and mode (Gate 3 C37); and, with **no request sent and before shutdown**, the file already exists at `0o600` once `App::start` returns — the only case that proves the startup probe created it, since `Writer::open` also creates it lazily on the first record (Gate 3 C36). `tp4` runs the app twice: opt-in off → the record has no `consumer` key at all; opt-in on → the client binds `127.0.0.2` via `reqwest::ClientBuilder::local_address`, sends `X-Forwarded-For: 203.0.113.9` and a `Forwarded:` header, and `consumer` equals `127.0.0.2` with no port while neither header value appears in the record or the stdout line — the distinct source address makes the first half discriminating, and the forwarding headers give the "nothing the caller supplies" promise its first witness (Gate 3 C31, C38). `tp13` asserts exactly twelve keys with the opt-in off and exactly those twelve plus `consumer` with it on.<br>2. `cargo test --test config_validation` → passes including `tp5c_consumer_identification_requires_a_sink`, asserting `ConfigError::Invalid { key: "log_consumer_identification" }` **and** the full reason `requires log_file_path or siem_url, so recorded peer addresses reach a durable destination the operator chose` (Gate 3 C35). This check failed on the reason before the first slice 3 implementation and is now a regression check (C34); `tp16` is deliberately **not** here, because its rejection happens in `delivery::build` after validation accepted the configuration (C33).<br>3. `cargo clippy --all-targets -- -D warnings` → exits 0 with no warning lines (`TP-12`, and with it Gate 3 C17).<br>4. `TP-14` re-run, same three runs and same control as slice 1 → product root still **0 findings**, control still fires at the `request.headers()` call site, receiver-filter-removed non-vacuity run still non-zero. Slice 1's claim that this run confirms "slice 3 added no inbound accessor" was **wrong** and is corrected: slice 3 added `request.extensions()`, which this spec's call-name list does not contain. What the run actually confirms is that no `headers`/`body`/`into_body`/`to_bytes`/`from_request*` call site exists. The `extensions` route is step 5's job (Gate 3 C29).<br>5. `TP-18`, the second retained spec, which must **exit 0** — note its pass condition is exactly one finding, not zero:<br>`smtc spec run --root /home/john/go/src/github.com/postfix/osprey --spec-file /home/john/go/src/github.com/postfix/osprey/.smtc/analyzers/consumer-identity-extension-read.yaml --format json --max-tokens 2048 --session-id tp18 \| jq -e '.ok == true and (.data.findings \| length) == 1 and (.data.findings[0].file \| endswith("/src/http/logging.rs"))'`<br>All three conjuncts are load-bearing: `.ok` catches a refused run reported as an empty finding list, the count catches a second inbound-extension read appearing, and the filename catches deleting the approved read and adding one in a different file, which leaves the count at 1. Assert no line number — the spec has no line predicate, so a line assertion would invent a maintenance trap that does not exist.<br>6. `TP-19` in its Gate 3 C46 form, two parts:<br>6a. `cargo test --lib tp19_file_drain_deadline_counts_cut_off_records` → output names `tp19_file_drain_deadline_counts_cut_off_records` as `ok` and reports `1 passed`.<br>6b. The same test against the pre-fix `drain`, on a copy of the tree with the accounting branch deleted:<br>`rm -rf /tmp/osprey-tp19-prefix && rsync -a --exclude target ./ /tmp/osprey-tp19-prefix/ && sed -i '/if drained.is_err() {/,/^    }$/d' /tmp/osprey-tp19-prefix/src/delivery/file.rs && test "$(grep -c drained.is_err /tmp/osprey-tp19-prefix/src/delivery/file.rs)" = 0 && (cd /tmp/osprey-tp19-prefix && CARGO_TARGET_DIR=/tmp/osprey-tp19-target cargo test --lib tp19_file_drain_deadline_counts_cut_off_records 2>&1) > /tmp/osprey-tp19.out; grep -q '1 failed' /tmp/osprey-tp19.out && ! grep -q inconclusive /tmp/osprey-tp19.out && echo TP19-DISCRIMINATES; grep 'test result' /tmp/osprey-tp19.out; rm -rf /tmp/osprey-tp19-prefix /tmp/osprey-tp19-target /tmp/osprey-tp19.out`<br>→ prints `TP19-DISCRIMINATES`, then `test result: FAILED. 0 passed; 1 failed`. If the output says `inconclusive: no cut-off happened`, `TP19-DISCRIMINATES` is not printed and step 6 fails. **Red today:** the tree holds the three-record test, and 6b printed `test result: ok. 1 passed` with no `TP19-DISCRIMINATES` when the main agent ran it on 2026-09-22 (C47). | `cargo`, `cargo-clippy`, `smtc`, `jq` — all four resolve on PATH; step 6b also uses `rsync`, `sed`, `grep`, all present and exercised by the C47 run; `jq` is at `/usr/bin/jq` and the step-5 command was executed against the current tree and exited 0 before this gate was written. The TP-14 control crate is recreated by the same heredoc and deleted after. `tp4` needs `reqwest`'s `local_address` on the test client; `tp16` needs only a path inside a directory it does not create; `tp17` needs `std::os::unix::fs::PermissionsExt`. | The `/tmp/osprey-tp14-control` crate again, deleted after witness step 4. The `/tmp/osprey-tp19-prefix` tree copy, `/tmp/osprey-tp19-target` and `/tmp/osprey-tp19.out`, deleted by the last command of step 6b. | code-review, security, qa | `sf-security-review` — this reverses the design rule at `src/http/logging.rs:14`, exports a privacy-sensitive value off the machine, and its **previous** run on this slice returned `FIX FIRST`; this re-run must confirm F1, F2 and F3 are closed. `sf-code-review` — its previous run returned two minor findings, both of which this revision addresses. It must also compare the SIEM client builder in `delivery::build` (`src/delivery/mod.rs`) against `src/upstream/reqwest_transport.rs:101` and report whether `.no_proxy()` is present (Gate 3 C43), and check that `drain` counts `rx.len() + 1` on a cut-off (Gate 3 C42). `sf-verification` — last slice; the feature's promised outcomes are all supposed to hold here. |
| 4 | Every decision record and summary record that reaches the log file or the SIEM says when it was made, as a `timestamp` in UTC RFC 3339 with six fractional digits, taken from the application's injected clock. The console line stays exactly as it is. Details: [Slice 4](#slice-4-interfaces) | Slices 1–3 supply `Decision`, `Summary`, both sinks, `summarise`, `flush_summary`, `close_window` and `Running::shutdown`. `jiff 0.2.37` is already a dependency (`Cargo.toml:52`, used by `SystemClock` at `src/clock.rs:22`). `App.clock: Arc<dyn Clock>` already exists. `wiremock` is a dev-dependency. The test helpers are listed under C51. | `src/delivery/mod.rs`, `src/delivery/file.rs` (test-only `Summary` literal at `:236`), `src/http/logging.rs`, `src/lib.rs`, `tests/decision_log_delivery.rs`, `docs/operations.md` | Run all three, in this order, from the repository root:<br>1. `cargo test --test decision_log_delivery` → passes, and the output names `tp20_delivered_records_carry_timestamp` and `tp13_stdout_decision_key_set` as `ok`, plus every slice 1–3 test still `ok`. **Red today:** `tp20` does not exist (C51). `tp20` is `TP-20` exactly as written in `03-program-design.md` `## Test plan`. Start with `TestServer::start_with(config, clock.shared())`, where `clock = TestClock::at_rfc3339("2026-09-21T14:13:20.120000Z")`, `log_file_path` is set, and `siem_url` points at a `wiremock::MockServer` that answers `200`. Send one request, call `clock.advance_seconds(60)`, then shut down. Then assert three things. The file's `request_decided` record has `timestamp == "2026-09-21T14:13:20.120000Z"`. The file's **last** `request_summary` record has `timestamp == "2026-09-21T14:14:20.120000Z"`. The `request_decided` record in the collector's received body has the same `timestamp` as the file's. `tp13` runs unchanged, so a `timestamp` key on stdout fails it.<br>2. `cargo test` → every target passes and the output reports `0 failed`. This covers `tp19` in `cargo test --lib` after its `Summary` literal gains `timestamp`.<br>3. `cargo clippy --all-targets -- -D warnings` → exits 0 with no warning lines (`TP-12`). | `cargo`, `cargo-clippy`. The helpers are listed under C51. The wiremock server binds loopback only, so the test needs no outbound network. | none | code-review, qa | `sf-code-review`: confirm `summarise` and `flush_summary` pass their own `clock` parameter to `close_window`, not `&SystemClock` or another clock. The periodic path has no execution witness, so this review is the check (`03-program-design.md` `## Invariants & spec dispositions`). Also confirm `rfc3339`'s body is exactly the C49 expression, and that the `file.rs:236` edit only adds the field. `sf-verification`: this is the last slice again. Confirm the feature's promised outcomes hold, including P-WHEN, which failed slice 3's run. Check the `docs/operations.md` timestamp text against the code. `sf-security-review` is not named: Gate 3 recorded that C49 adds no entry point, trust boundary, destination or input. |

## Slice interface details

### Slice 1 interfaces

Uses: `Config::load` and the `RawConfig::validate` cross-field idiom (`src/config.rs:129,139-199`);
`OPTIONAL_KEYS` (`src/config.rs:117`) and `check_keys()` (`:240`), which reject any key not
registered in all three places; `tokio::sync::mpsc::channel` in the same
sender-on-a-handle / receiver-into-the-task shape as `store::spawn`
(`src/store/mod.rs:590-612`); `tokio::fs` + `AsyncWriteExt` (the `fs` feature is already on);
`Tasks`/`Tasks::join` (`src/tasks/mod.rs:26-52`); the existing `Running::shutdown` order
(`src/lib.rs:255-271`); the existing `tracing::info!` decision emission
(`src/http/logging.rs:172-186`) and `Target::of` / `loggable` bounding (`:198-263`).

Changes:
- **New** `delivery::Record`, `Decision`, `Summary`, `Drops`, `Sinks`, `build`, `Sinks::offer`,
  `Sinks::drops`, `Sinks::is_empty`, exactly as declared in `03-program-design.md`
  `## Modules and interfaces`, minus `Decision.consumer` (C24). `build` handles the file sink
  only; the SIEM arm arrives in slice 2.
- **New** `delivery::file::run` with the declared signature, including the `.1` rollover built by
  pushing the literal `.1` onto the path's `OsString`, and the drain loop wrapped in
  `tokio::time::timeout(Duration::from_secs(5), ..)`.
- **Changed** `http::logging::decide`: build the `Decision`, render the existing
  `tracing::info!` from it with the identical `"request decided"` message and identical field
  names, then `offer`, then `summarise`.
- **Changed** `http::logging::summarise` gains a `sinks: &Sinks` parameter, adds `dropped_file`
  and `dropped_siem` to the summary line, and offers `Record::RequestSummary`.
- **New** `http::logging::flush_summary` and `http::logging::flush_drop_tail`.
- **Changed** `src/lib.rs`: `App.delivery`, `Running.drain`, the `drain` token created in
  `App::start`, and the two statements added to `Running::shutdown` in exactly the order
  `03-program-design.md` `### src/lib.rs` prints — `flush_summary` and `drain.cancel()` between
  the server join and `tasks.join()`, `flush_drop_tail` between `tasks.join()` and
  `drop(self.app)`.
- **Changed** `tasks::spawn` gains a fourth `delivery: Vec<JoinHandle<()>>` parameter appended to
  the existing `vec![..]`.
- **Changed** `src/config.rs`: `log_file_path` and `log_file_max_bytes` only, registered in
  `RawConfig`, `Config` and `OPTIONAL_KEYS`, with the two `TP-5a` rejections.
- **Changed** `config.sample.toml`: both keys commented out with their defaults.
- **Changed** `docs/operations.md` `## 8. Logs` (`:409-428`): the two keys, the twice-the-cap
  bound, the single `.1` generation, and the two operator obligations Gate 2 fixed — exactly one
  process may write the path, and it must not be handed to an external `logrotate`.

Design: `03-program-design.md` `## Files`; `## Modules and interfaces` (`src/delivery`,
`src/delivery/file.rs`, `src/http/logging.rs`, `src/config.rs`, `src/lib.rs`,
`src/tasks/mod.rs`); `## Call stack`; `## Test plan` rows `TP-1`, `TP-2`, `TP-3`, `TP-5`,
`TP-10`, `TP-12`, `TP-13`, `TP-14`; `## Invariants & spec dispositions`.

Acceptance: `TP-1`, `TP-2`, `TP-3`, `TP-5a`, `TP-10`, `TP-12`, `TP-13`, `TP-14`.

`TP-14`'s unsafe control (C22). Create it with exactly this, then run the verdict, then delete it:

```bash
mkdir -p /tmp/osprey-tp14-control/src
cat > /tmp/osprey-tp14-control/Cargo.toml <<'EOF'
[package]
name = "tp14-control"
version = "0.0.0"
edition = "2021"
EOF
cat > /tmp/osprey-tp14-control/src/main.rs <<'EOF'
struct Request;
impl Request {
    fn headers(&self) -> String {
        String::from("user-agent: whatever the client said")
    }
}

struct Sinks;
impl Sinks {
    fn offer(&self, _record: String) {}
}

fn decide(request: &Request, sinks: &Sinks) {
    let client_supplied = request.headers();
    sinks.offer(client_supplied);
}

fn main() {
    decide(&Request, &Sinks);
}
EOF
```

Stage limits: with no sink configured the default path must open nothing and spawn nothing —
that is `TP-3`, and it is the check that protects the product's "not on by default" non-goal.

Temporary behavior: `Summary.dropped_siem` and `Drops.siem` exist and are always zero until
slice 2 wires the SIEM sink that can increment them (C25). Removal point: slice 2. Nothing else
in this slice is temporary; the file sink is the shipped feature, not a stand-in.

### Slice 2 interfaces

Uses: everything slice 1 created; `reqwest 0.13.5` over rustls; `wiremock::MockServer::start()`
directly rather than the `tests/common/mod.rs:900-923` registry helper, which is bound to
`Transport`/`OriginSet` and is the wrong shape here.

Changes:
- **New** `delivery::siem::run` with the declared signature: batch at 256 records or two seconds,
  `POST` newline-delimited JSON with `Content-Type: application/x-ndjson`, retry a transport
  error / `5xx` / `429` three times at 100 ms / 500 ms / 2 s, drop and count the **whole batch**
  on any other `4xx`, and drain under the same `tokio::time::timeout(Duration::from_secs(5), ..)`.
- **Changed** `delivery::build`: the SIEM arm. The `reqwest::Client` is built with
  `redirect::Policy::none()`, `.timeout(Duration::from_secs(3))` and
  `.connect_timeout(Duration::from_secs(1))`. `OSPREY_SIEM_AUTH` is read here, once, wrapped in a
  `HeaderValue` with `set_sensitive(true)`, and never stored on `Config`.
- **New** `StartupError::Delivery` on the crate-level enum at `src/lib.rs:274-283` — not the
  unrelated `src/store/startup.rs:179-182` enum (C26). **Its text names the environment variable
  and a fixed reason and never contains any part of the rejected value**, because that string
  flows straight into `check_config`'s printed `path: err` (`src/main.rs:80-96`). `TP-11b` is the
  check that fails if a naive `format!("…: {value}")` is used.
- **Changed** `src/config.rs`: `siem_url` and `siem_auth_header`, with the `TP-5b` cross-key
  rejection and the `TP-6` scheme rule (`https` required unless the host is a loopback address or
  `localhost`).
- **Changed** `config.sample.toml` and `docs/operations.md`: both keys, the environment variable
  **name only**, the batch and retry behaviour, the drop counters and where they are delivered,
  and the accepted limitation that one rejected record costs its whole batch (Gate 3 C21) with
  the file sink named as the answer for anyone needing completeness.

Design: `03-program-design.md` `## Modules and interfaces` (`src/delivery/siem.rs`,
`delivery::build`, `src/config.rs`); `## Call stack`; `## Threat model` T-SHUTDOWN-HANG,
T-BATCH-POISON, T-CRED-STARTUP-LEAK, T-CRED-RUNTIME-LEAK, T-CRED-CONFIG-DEBUG, T-REDIRECT-EGRESS;
`## Test plan` rows `TP-5`, `TP-6`, `TP-7`, `TP-8`, `TP-9`, `TP-11a`, `TP-11b`, `TP-12`, `TP-15`.

Acceptance: `TP-5b`, `TP-6`, `TP-7`, `TP-8`, `TP-9`, `TP-11a`, `TP-11b`, `TP-12`, `TP-15`.
`TP-7` must read the summary record **back out of the file sink's NDJSON output**, not out of
captured stdout: the claim under test is that drop counts reach a durable destination, and a
stdout assertion would pass even if that were false.

Stage limits: a batch dropped for a non-retryable `4xx` is counted, not isolated or resent. That
is the approved Gate 3 C21 accept, owner product — not a gap for Engineering to close.

Temporary behavior: none. This slice removes slice 1's (C25) by making `dropped_siem` real.

### Slice 3 interfaces

Uses: both sinks and the whole `Decision` record from slices 1–2; `axum::extract::ConnectInfo`
and `into_make_service_with_connect_info::<SocketAddr>()`.

Changes:
- **Changed** `delivery::Decision` gains the final field
  `#[serde(skip_serializing_if = "Option::is_none")] pub consumer: Option<IpAddr>` (C24).
- **Changed** `http::logging::decide`: read
  `request.extensions().get::<ConnectInfo<SocketAddr>>()` **before** `next.run(request)` at
  `:144-146`, in the same block that already calls `Target::of` — the request is consumed there,
  so reading it afterwards is not possible (Gate 3 C15). Keep `Some(addr.ip())` only when
  `app.config.log_consumer_identification` is true, else `None`. The port is never kept.
- **Changed** `src/lib.rs`: the conditional `axum::serve` at `:170-177`, with each branch owning
  its complete `axum::serve(..).with_graceful_shutdown(..).await` expression, because
  `into_make_service_with_connect_info` changes the service type. Connect-info is enabled only
  when the opt-in is on.
- **Changed** `src/config.rs`: `log_consumer_identification`, defaulting to false, with the
  `TP-5c` rejection — on with neither `log_file_path` nor `siem_url` set is refused at startup,
  naming the key, because otherwise peer IPs start being written to a channel the product
  explicitly calls non-durable, for no stated purpose.
- **Changed** `config.sample.toml` and `docs/operations.md`: the key with its privacy note, and
  the three operator facts Gate 2 fixed — what is recorded (the peer IP of the accepted TCP
  connection and nothing the caller supplies), what it costs behind a proxy, NAT or load
  balancer (that hop, not the machine that ran the install), and that turning it off erases
  nothing already written to the file, its `.1` rollover, or the SIEM.

Added by the Gate 3 reopening, after slice 3's Security review returned `FIX FIRST`. These three
are the reason this slice was reopened rather than closed (Gate 3 C30):

- **Changed** `src/delivery/file.rs` — `.mode(0o600)` on **every** open that may create the file:
  the fresh file opened after a rollover and the reopen after an I/O error, as well as the startup
  probe below (Gate 3 C27). In `Writer::open` this is `tokio::fs::OpenOptions`'s own inherent
  `.mode`; only the startup probe uses `std::os::unix::fs::OpenOptionsExt` (C45). The `.1` generation
  inherits its mode through `rename`. Unix-only, deliberately not `cfg`-guarded. After any open,
  when `mode & 0o077 != 0`, emit **one** `tracing::warn!` naming the path and the mode — do not
  re-chmod a file the process did not create, and do not refuse to run (Gate 3 C37).
- **Changed** `delivery::build` — a startup **probe**:
  `std::fs::OpenOptions::new().create(true).append(true).mode(0o600)` on `log_file_path`, handle
  dropped immediately. It reads no length and touches no tally; `Writer::open` keeps its existing
  length-restoring behaviour untouched, which slice 1's `tp2b` depends on (Gate 3 C36). On failure:
  when `log_consumer_identification` is on, return `StartupError::Delivery` with text naming the
  key and a fixed reason and no peer address; when it is off, emit one `tracing::error!` naming the
  key and continue (Gate 3 C28, C39). This restores the approved Effects contract that slices 1
  and 2 shipped without.
- **Changed** `src/config.rs:295` — the `log_consumer_identification` reason string becomes exactly
  `requires log_file_path or siem_url, so recorded peer addresses reach a durable destination the operator chose`.
  The old one claimed the rejection gives peer addresses "a destination"; they already reach the
  console (Gate 3 C35). `tp5c` asserts the new string in full.
- **Changed** `tests/decision_log_delivery.rs` — `tp16_unopenable_log_path_fails_startup` and
  `tp17_log_file_mode_is_0600` (Gate 4 C33), plus the `tp4` strengthening: the opt-in-on run's
  client binds `127.0.0.2` through `reqwest::ClientBuilder::local_address` and the assertion
  expects `127.0.0.2` (Gate 3 C31).
- **Changed** `docs/operations.md` — in the privacy block: (a) the decision log file is created
  `0o600`; a pre-existing file keeps its mode and the process warns once if it is group- or
  world-accessible; (b) peer addresses also appear on the stdout decision line, which under
  systemd or a container runtime is kept on disk by the host — so the purge bullet names **four**
  places, not three: the log file, its `.1` rollover, the collector, and the host's console log
  (Gate 3 C35); (c) with the opt-in on, an unopenable `log_file_path` stops startup; with it off,
  the process logs one error and keeps serving (C39); (d) `check_config` validates keys, not
  destinations, so it can report a configuration as valid that then fails at boot (C40); (e) the
  file-sink section's "same twelve fields" sentence becomes "twelve, or thirteen with `consumer`
  when the opt-in is on" (code-review finding 2); (f) the sentence "stdout is not durable"
  (`docs/operations.md:432`) is corrected to say stdout is kept by the host under systemd or a
  container runtime. These are prose with no executable witness: `sf-verification` checks each of
  (a)–(f) against the file, and `sf-security-review` checks the privacy statements against the code.

- **Changed** `src/http/logging.rs:14`, the module doc — replace the flat claim "Nothing a client
  sends reaches a log line" with the narrower property that is actually true and actually
  guarded: no client-supplied *header, body or connection* value reaches a log line, noting that
  `package` and `version` have always derived from the client-supplied request path, and naming
  the two retained specs. The gate document is not where a maintainer meets this; this file is.

Added by the 2026-09-22 reopening (Gate 3 C42, C43; Gate 4 C44):

- **Changed** `src/delivery/file.rs` — move the drain loop into a private
  `async fn drain(writer: &mut Writer, rx: &mut mpsc::Receiver<Record>, drops: &AtomicU64, deadline: Duration)`,
  which `run` calls with `DRAIN_DEADLINE`. When `tokio::time::timeout` returns `Err`, add
  `rx.len() as u64 + 1` to `drops`. Add the unit test
  `tp19_file_drain_deadline_counts_cut_off_records` in a `#[cfg(test)]` module in the same file:
  queue **10,000** records on a fresh channel with capacity for all of them, open a `Writer` on a
  temporary path, and await `drain(.., Duration::ZERO)`. Then assert, in this order:
  (1) lines in the file < 10,000, with the message `inconclusive: no cut-off happened`;
  (2) lines + `drops` is 10,000 or 10,001 (Gate 3 C46). The zero deadline fires at tokio's next
  millisecond tick, not on the first poll, so only a large queue gets cut off. Measured before the
  fix: 8 written, 0 dropped, so (2) fails. This test body replaces the three-record version in the
  tree today, which passes against the pre-fix code and is not a witness (C47).
- **Changed** `delivery::build` in `src/delivery/mod.rs` — add `.no_proxy()` to the SIEM `reqwest::Client::builder()` chain,
  next to `redirect::Policy::none()` and the two timeouts, matching
  `src/upstream/reqwest_transport.rs:101`. No test (Gate 3 C43).

**The proof line for this slice carries two separate claims, not one.** The feature — the opt-in
records the peer IP on every delivered record — and the repair — `delivery::build` now probes the
log file at startup, restoring the approved Effects contract slices 1 and 2 shipped without,
witnessed by `TP-16`. The repair must stay legible as a named repair rather than dissolving into
the feature's proof, so that the slice-1 contract-fidelity defect and the process lesson recorded
in `00-status.md` remain readable by whoever inherits this plan.

Not written by this slice, and not editable by its implementer: both `.smtc/analyzers/*.yaml`
specs already exist, authored and validated during the Gate 3 revision (Gate 4 C32). Witness
steps 4 and 5 run them; a witness an implementer can edit is not a witness.

Design: `03-program-design.md` `## Modules and interfaces` (`http::logging::decide`,
`delivery::Decision`, `src/config.rs`, `src/lib.rs`); `#### build` (the startup probe, C28/C36/C39,
and `.no_proxy()`, C43); `## Call stack`; `## Threat model` T-PEER-IP-RETENTION,
T-INVARIANT-FIRST-BREACH, T-LOGFILE-WORLD-READABLE, T-SILENT-DESTINATIONLESS-COLLECTION,
T-EXTENSION-ROUTE, T-CONSOLE-JOURNAL-DESTINATION; `## Test plan` rows `TP-4`, `TP-5`, `TP-12`,
`TP-13`, `TP-14`, `TP-16`, `TP-17`, `TP-18`, `TP-19`; `### src/delivery/file.rs` (Drain);
`## Invariants & spec dispositions`.

Acceptance: `TP-4` in its strengthened form, `TP-5c`, `TP-12` (which also settles Gate 3 C17, the
unwitnessed assumption that `ConnectInfo` is available under axum's declared features), `TP-13` in
its twelve-or-thirteen-key form, `TP-16`, `TP-17`, `TP-18`, `TP-19`, and `TP-14` re-run against the product
root now that a per-connection value reaches `offer`. `TP-14`'s form was corrected at slice 1 — it is a
`structural_pattern` forbidden-call inventory rather than the originally-approved
`taint_query`; see `03-program-design.md` `## Invariants & spec dispositions` for why no
`taint_query` configuration could satisfy both halves of its pass condition.

Stage limits: the peer IP is the only consumer identity this feature will ever record.
`X-Forwarded-For` and `User-Agent` were rejected at Gate 2 C7 and remain rejected. **The claim
that "the retained analyzer spec fires if either is ever wired in" was too strong and is
corrected here**: the sibling spec fires on a direct `headers()`-style read, but a middleware
that parses `X-Forwarded-For` and deposits it in the request extension map, read back with the
shape this slice legitimises, was invisible to it. That route is now covered by the second spec
and witness step 5 (Gate 3 C29). Neither spec proves the invariant; they are tripwires for two
shapes.

Temporary behavior: none.

### Slice 4 interfaces

Uses: `App.clock` (`Arc<dyn Clock>`) and `Clock::now_utc_micros` (`src/clock.rs:8-12`);
`jiff::Timestamp::from_microsecond`; everything slices 1–3 built. The complete edit surface is
the seven sites in `evidence/impact-c49.md`.

Changes:
- **New** `pub(crate) fn rfc3339(utc_micros: i64) -> String` in `src/delivery/mod.rs`, whose body
  is exactly
  `jiff::Timestamp::from_microsecond(utc_micros).map(|t| format!("{t:.6}")).unwrap_or_else(|_| utc_micros.to_string())`.
- **Changed** `delivery::Decision` and `delivery::Summary` get `pub timestamp: String` as their
  **first** field, so it serialises right after the `event` tag.
- **Changed** `http::logging::decide` sets
  `timestamp: delivery::rfc3339(app.clock.now_utc_micros())` on the `Decision`. It reads the clock
  after `next.run(request)` returns. It does not name `timestamp` in the `tracing::info!` call.
- **Changed** `summarise`, `flush_summary` and `close_window` each gain a final
  `clock: &dyn Clock` parameter. `summarise` and `flush_summary` pass theirs unread to
  `close_window`. `close_window` sets `timestamp: rfc3339(clock.now_utc_micros())` on the
  `Summary`, and it is the only place a summary reads the time. `decide` passes
  `app.clock.as_ref()` to `summarise`. `emit` does not name `timestamp`.
- **Changed** `src/lib.rs` `Running::shutdown`:
  `http::logging::flush_summary(&self.app.delivery, self.app.clock.as_ref());`.
- **Changed** `src/delivery/file.rs:236`, the `tp19` test's `Summary` literal, gains a
  `timestamp` value. This edit only makes the literal compile. What `tp19` asserts does not change.
- **Changed** `tests/decision_log_delivery.rs` adds `tp20_delivered_records_carry_timestamp`.
- **Changed** `docs/operations.md` states that file and SIEM records carry `timestamp` as their
  first field, in UTC RFC 3339 with six fractional digits and a `Z` suffix
  (`2026-09-21T14:13:20.000000Z`). The value is the time the record was built. A collector's
  receipt time can lag by the 2 s batch window plus retries. The console line has no `timestamp`,
  because its formatter already stamps the line. The documentation also states that with consumer
  identification on, a delivered record pairs a peer IP with the time of the request.

Design: `03-program-design.md` C49; `delivery::rfc3339`; `## Modules and interfaces` for
`decide`, `summarise` and `flush_summary`; `## Call stack`; `## Test plan` `TP-20`;
`## Invariants & spec dispositions` (clock reads).

Acceptance: `TP-20`, with `TP-13` and `TP-12` unchanged.

Stage limits: `TP-20` covers the decision read and the shutdown summary read, and both summary
paths go through `close_window`. The periodic call from `summarise` to `close_window` has no
execution witness, so `sf-code-review` checks which clock it forwards.

Temporary behavior: none.
