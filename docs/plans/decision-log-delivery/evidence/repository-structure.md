# Repository grounding — osprey, for Gate 2 of Decision log delivery

Produced by an isolated `sf-repo-view` dispatch, read-only, against Git base
`0f50cec6d526773abec4b4897f64afc10992a8ae` on branch `package-firewall-mvp`. Working tree clean
apart from untracked `docs/plans/decision-log-delivery/`. Every claim below carries a
`path:line` reference from that tree.

## 1. Logging construction — `src/main.rs:58-63`

```rust
tracing_subscriber::fmt()
    .json()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .init();
```

One global subscriber, JSON formatter, no `Layer` composition. There is no `.with_writer(...)`
call, which is what makes it stdout-only. There is no `tracing_appender` non-blocking writer and
no retained `WorkerGuard` — the `.init()` return value is discarded. This runs once in `main()`
before dispatch to `serve` / `check_config` / `check_blocklist`.

## 2. Decision record emission — `src/http/logging.rs`

- Call site: `tracing::info!(...)` at `src/http/logging.rs:172-186`, message `"request decided"`.
- Enclosing function: `pub async fn decide(State(app): State<Arc<App>>, request: Request, next: Next) -> Response` at `src/http/logging.rs:134`.
- Field set confirmed as claimed: `request_id, method, ecosystem, package, version, status, result, reason, blocklist_revision, cache, duration_micros, bytes`.
- `decide` is Axum middleware and, per its doc comment at `src/http/logging.rs:128-133`, "is the outermost layer, so the line it writes describes the response that actually left the process, including one produced by the router itself".
- The design rule "Nothing a client sends reaches a log line" is confirmed verbatim at `src/http/logging.rs:14`. It is enforced by `Target::of` / `loggable` at `src/http/logging.rs:198-263`, which bounds and escapes every route component before logging (`MAX_LOGGED_TARGET = 256` at `src/http/logging.rs:35`).

UNKNOWN: the exact `.layer(axum::middleware::from_fn_with_state(...))` registration line was not
read (expected in `src/http/mod.rs`).

## 3. Counter summary — `src/http/logging.rs:270-332`

There is **no background task**. `summarise()` (`src/http/logging.rs:294`) is called synchronously
at the end of every `decide()` invocation (`src/http/logging.rs:188`, immediately after the
decision line). It locks `static COUNTERS: LazyLock<Mutex<Counters>>`
(`src/http/logging.rs:284`), accumulates `requests / errors / bytes / micros`, and emits and
resets only when the window has elapsed (`SUMMARY_MILLIS = 60_000` at `src/http/logging.rs:38`,
overridable only under `#[cfg(feature = "test-support")] set_summary_window` at
`src/http/logging.rs:341`). Poison recovery at `src/http/logging.rs:300`. The doc comment at
`src/http/logging.rs:270-275` states the intent: "The window is closed by the request that notices
it has run out, so there is no background task to start, stop or leak."

## 4. Configuration — `src/config.rs` and `config.sample.toml`

- Validated public struct `pub struct Config` at `src/config.rs:21`, typed fields (`SocketAddr`, `Url`, `PathBuf`, `NonZeroU64` / `NonZeroU32` where zero is invalid, plain `u64` where zero is meaningful).
- Wire struct `struct RawConfig` at `src/config.rs:62` with `#[derive(Deserialize)]` and `#[serde(deny_unknown_fields)]` at `src/config.rs:61`. Two fields carry `#[serde(default = "fn")]` (`src/config.rs:69-70`, `src/config.rs:80-81`).
- Load path: `Config::load(path)` at `src/config.rs:129` → `Config::from_toml_str` at `src/config.rs:120`, which parses three times over: `toml::from_str::<toml::Table>` for syntax (`src/config.rs:123`), then `check_keys()` (`src/config.rs:240`) against the `REQUIRED_KEYS` / `OPTIONAL_KEYS` const slices (`src/config.rs:98-117`), then `toml::from_str::<RawConfig>` (`src/config.rs:125`) followed by `RawConfig::validate()` (`src/config.rs:139`) for cross-field checks and `NonZero*` conversion.
- Errors: `enum ConfigError` at `src/config.rs:268` — `Read{path,source}`, `Syntax(toml::de::Error)`, `UnknownKey(String)`, `MissingKey(&'static str)`, `Invalid{key,reason}` — each naming the exact key in its `Display` impl at `src/config.rs:285`. Surfaced by the `check_config` CLI command at `src/main.rs:80-96`, which prints `path: err` and exits `ExitCode::FAILURE`.
- **Adding a key is a two-place change**: the typed field on `Config` (`src/config.rs:21-46`) and the raw field on `RawConfig`, *plus* the key name in `REQUIRED_KEYS` (`src/config.rs:98`) or `OPTIONAL_KEYS` (`src/config.rs:117`), *plus* any conversion or cross-field rule in `RawConfig::validate()` (`src/config.rs:139-235`). `deny_unknown_fields` means an unregistered key is rejected outright.
- `config.sample.toml` is 35 lines, **one flat TOML table with no nested `[section]` anywhere**. A nested table would be the first in this file; there is no existing example to imitate.

## 5. Async runtime and shutdown

- Runtime: `#[tokio::main] async fn main()` at `src/main.rs:54`. `Cargo.toml` declares `tokio = { version = "1.53.1", features = ["rt-multi-thread", "macros", "fs", "net", "sync", "time", "signal"] }` — multi-thread.
- Tasks spawned today: the HTTP server at `src/lib.rs:170` (`axum::serve(...).with_graceful_shutdown(...)` inside `App::start`); `blocklist_poller::run(...)` at `src/tasks/mod.rs:32-36`; `maintenance::run(...)` at `src/tasks/mod.rs:37`; and a conditional store task from `store::spawn(...)` at `src/lib.rs:122`, kept as `Option<JoinHandle<()>>` named `store_task` at `src/lib.rs:239`.
- Shutdown signal: one `tokio_util::sync::CancellationToken` created at `src/lib.rs:166` (named `shutdown`), cloned into `tasks::spawn(Arc::clone(&app), shutdown.clone(), watcher)` at `src/lib.rs:167`, and awaited by the server's `with_graceful_shutdown` closure at `src/lib.rs:174`. Triggered by `tokio::signal::ctrl_c()` at `src/main.rs:148`, then `running.shutdown().await` at `src/main.rs:154`.
- **Drain order, `Running::shutdown()` at `src/lib.rs:255-271`, in order**: (1) `self.shutdown.cancel()` (`src/lib.rs:256`); (2) await the HTTP server join handle (`src/lib.rs:257`), which is where in-flight requests finish; (3) `self.tasks.join().await` (`src/lib.rs:264`), which just awaits each `JoinHandle` (`Tasks::join`, `src/tasks/mod.rs:46-52`); (4) `drop(self.app)` (`src/lib.rs:265`), closing storage queues by dropping the last `Arc<App>`; (5) await `store_task` if present (`src/lib.rs:266-267`).
- A new background task spawned inside `tasks::spawn` (`src/tasks/mod.rs:26-41`), with its handle pushed into `Tasks.handles` (`src/tasks/mod.rs:20`, `src/tasks/mod.rs:40`), is already awaited by the existing drain — no new hook point is needed. **Note the ordering consequence:** cancellation fires at step 1, before in-flight requests have finished at step 2, and `Arc<App>` is not dropped until step 4, after `tasks.join()` at step 3.

## 6. Dependencies already present

| Crate | Present | Version | Note |
|---|---|---|---|
| `tracing-appender` | absent | — | no rotation or non-blocking writer anywhere |
| `reqwest` | direct | 0.13.5 | features `["stream","gzip","brotli","zstd"]`; 0.13 defaults to rustls per Cargo.toml comment |
| `hyper` | transitive | 1.11.1 | via reqwest |
| `rustls` | transitive | 0.23.45 | via reqwest |
| `serde` | direct | 1.0.229 | feature `derive` |
| `serde_json` | direct | 1.0.151 | |
| `toml` | direct | 1.1.6 | used by `config.rs` |
| `tokio` | direct | 1.53.1 | feature `sync` enabled → `tokio::sync::mpsc` available with no new dependency |
| `tokio-util` | direct | 0.7.19 | `CancellationToken`, the confirmed background-coordination primitive |
| `arc-swap` | direct | 1.9.2 | lock-free publication |

No raw-socket or syslog crate beyond tokio's `net` feature and reqwest's HTTP client.

Consequence, stated as fact rather than recommendation: local file writing and background-channel
mechanics are available with zero new Cargo dependency; an outbound HTTP delivery path can reuse
the present rustls-backed `reqwest` stack with zero new dependency; log rotation has no existing
crate.

UNKNOWN: whether `tokio::sync::mpsc` is already used elsewhere in the tree — not searched.

## 7. Counters and metrics surface

Entirely inside `src/http/logging.rs`. `struct Counters { requests, errors, bytes, micros, opened }`
at `src/http/logging.rs:276-282`; `static COUNTERS: LazyLock<Mutex<Counters>>` at
`src/http/logging.rs:284`; incremented under that lock in `summarise()` at
`src/http/logging.rs:300-304`; reset and emitted at `src/http/logging.rs:317-321`; rendered by the
summary `tracing::info!` at `src/http/logging.rs:324-331`. There is **no separate metrics module** —
the module list at `src/lib.rs:7-17` has none.

Adding a dropped-records counter in the existing idiom means: a field on `Counters`
(`src/http/logging.rs:276-282`), initialisation in the `LazyLock::new` block
(`src/http/logging.rs:285-291`), reset alongside the others (`src/http/logging.rs:317-321`), and a
named field on the summary call (`src/http/logging.rs:324-331`) — exactly how `errors` sits beside
`requests`.

## 8. Request-path hot spots

`decide()` (`src/http/logging.rs:134-190`) wraps every request. Per-request work already on that
path: `Instant::now()` (`src/http/logging.rs:135`); `Target::of()` splitting the URI path
(`src/http/logging.rs:136`) and `loggable()` allocating a bounded `String` via
`.chars().take(256).collect()` (`src/http/logging.rs:261`); an `Arc::new(RequestContext{...})`
(`src/http/logging.rs:139`); a `tokio::task_local!` `CONTEXT.scope(...)` wrapping
`next.run(request)` (`src/http/logging.rs:144-146`); a `CONTENT_LENGTH` header lookup with a
`size_hint()` fallback (`src/http/logging.rs:151-157`); the `tracing::info!` JSON serialisation
(`src/http/logging.rs:172-186`); and `summarise()` taking a **synchronous
`std::sync::Mutex`** (`src/http/logging.rs:300`) on every single request, held only for integer
arithmetic.

The relevant tension: the existing counter mechanism deliberately avoids a background task by
doing this inline, which is the opposite of the "never slow a request" requirement this feature
must satisfy for delivery.

## 9. Tests and checks

- `tests/` — 20 files, flat, named by concern, plus `tests/common/mod.rs` for shared helpers: `blocklist_revocation.rs`, `artifacts_concurrency.rs`, `artifacts_verification.rs`, `tracer.rs`, `origin_guard.rs`, `pypi_metadata.rs`, `config_validation.rs`, `blocklist_snapshot.rs`, `persistence_recovery.rs`, `slice11_adversarial_singleflight.rs`, `http_contract.rs`, `e2e_pip.rs`, `blocklist_reload.rs`, `warm_artifact_budget.rs`, `persistence_projects.rs`, `e2e_npm.rs`, `npm_metadata.rs`, `persistence_content.rs`.
- Unit tests are embedded in source, e.g. `#[cfg(test)] mod tests` at `src/http/logging.rs:345-379`.
- `benches/` — 3 files declared as `[[bench]]` with `harness = false`; criterion is explicitly absent per a Cargo.toml comment.
- Commands, from `README.md:23`, `README.md:71-75` and `docs/operations.md:62,457,584-585`:
  - `cargo build --release --locked`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test` (offline, no public network)
  - `cargo test --test e2e_npm -- --ignored` (drives a real npm client)
  - `cargo test --test e2e_pip -- --ignored` (drives a real pip client)
  - `cargo bench`
- Toolchain pinned by `rust-toolchain.toml` to channel `1.96.0`, components `clippy, rustfmt`; `Cargo.toml` declares `rust-version = "1.96.0"`.
- **No CI configuration exists anywhere**: no `.github/workflows/*`, no `.gitlab-ci.yml`, no `Makefile`, no `justfile`.
- `docs/operations.md:717-736` self-reports that there is no `#![deny(warnings)]` in the tree and that `cargo test --release` "has never been green", while debug `cargo test` passes "276 passed, 0 failed, 15 ignored". This is the document's own claim, not an independently executed result.

UNKNOWN: whether rustfmt is enforced by any command or hook — no invoking command was found
despite the component being pinned.

## Documentation freshness

- **`docs/codebase-overview.md` is entirely stale and must not be trusted.** It states "There is no source code to start from yet", that no `src/`, `Cargo.toml` or `Cargo.lock` exists "anywhere in the tree", that no build or test commands exist, and describes the module layout as merely "Proposed" (`docs/codebase-overview.md:6`, `:8-11`, `:18`, `:23-24`). All of that is false against commit `0f50cec`, which has a built `src/`, a `Cargo.toml` and `Cargo.lock`, and the exact module layout the document calls proposed, plus `clock`, `concurrency` and `tasks`. The document predates the MVP commit and was never updated.
- **`docs/operations.md` is current for the areas checked.** Its "## 8. Logs" section (`docs/operations.md:409-428`) matches `src/http/logging.rs` exactly: the example JSON field names match the emission at `src/http/logging.rs:172-186`, the `RUST_LOG`-with-`info`-default claim matches `src/main.rs:60-61`, and its "no metrics service… counters and timing summaries are emitted to stdout periodically" claim matches the inline `summarise()` design. Only the Logs section, the build-command references and the limitations list were checked against source; the remaining sections of its 741 lines were not.

## Files read

`src/main.rs`, `src/http/logging.rs`, `src/config.rs`, `src/lib.rs`, `src/tasks/mod.rs`,
`Cargo.toml`, `Cargo.lock` (searched), `config.sample.toml`, `rust-toolchain.toml`,
`docs/codebase-overview.md`, `docs/operations.md` (partial), `README.md` (searched).

Not read: `src/http/mod.rs`, `src/tasks/blocklist_poller.rs`, `src/tasks/maintenance.rs`,
`src/store/*.rs`.
