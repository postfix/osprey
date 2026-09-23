# Show me: Slice 2 — decision records reach the SIEM collector

## Delivered

Outcome shipped (`04-slices.md`, Slice 2): an operator who sets `siem_url` gets decision
records batched and `POST`ed to that collector as newline-delimited JSON, authenticated from
`OSPREY_SIEM_AUTH` when it is set, never following a redirect, never blocking a request, and
never hanging shutdown when the collector goes silent; a drop is counted per sink and readable
out of the file sink's own output.

New module, `src/delivery/siem.rs` — batching, retry and the fixed drop-counting guard:

```rust
const BATCH_RECORDS: usize = 256;
const BATCH_INTERVAL: Duration = Duration::from_secs(2);
const BACKOFF: [Duration; 3] = [
    Duration::from_millis(100),
    Duration::from_millis(500),
    Duration::from_secs(2),
];
const DRAIN_DEADLINE: Duration = Duration::from_secs(5);

async fn send(
    client: &Client,
    url: &Url,
    auth: Option<&(HeaderName, HeaderValue)>,
    batch: &mut Vec<Record>,
    drops: &AtomicU64,
) {
    let count = batch.len() as u64;
    let mut body = String::new();
    for record in batch.drain(..) {
        if let Ok(line) = serde_json::to_string(&record) {
            body.push_str(&line);
            body.push('\n');
        }
    }
    // From here the records exist nowhere else: they are off the queue and out of
    // `batch`, so the caller's own `batch.len() + rx.len()` can no longer see them and
    // this guard is the only thing that can still count them. It matters because this
    // whole future is awaited inside the drain deadline's `timeout`, and a future cut
    // off at an await point is dropped rather than resumed — `Drop` runs, so
    // cancellation is counted by construction rather than by remembering to.
    let mut unsent = Unsent { count, drops };
    ...
}

struct Unsent<'a> {
    count: u64,
    drops: &'a AtomicU64,
}

impl Unsent<'_> {
    fn delivered(&mut self) {
        self.count = 0;
    }
}

impl Drop for Unsent<'_> {
    fn drop(&mut self) {
        if self.count != 0 {
            self.drops.fetch_add(self.count, Ordering::Relaxed);
        }
    }
}
```

`delivery::build` (`src/delivery/mod.rs`) wires the client and the credential:

```rust
let client = reqwest::Client::builder()
    // Gate 2 C4: a collector that redirects must not be able to point
    // this process — credential attached — at a host nobody configured.
    .redirect(redirect::Policy::none())
    .timeout(SIEM_REQUEST_TIMEOUT)      // 3s
    .connect_timeout(SIEM_CONNECT_TIMEOUT) // 1s
    .build()
    ...
```

```rust
fn siem_auth(name: &HeaderName) -> Result<Option<(HeaderName, HeaderValue)>, StartupError> {
    let value = match std::env::var(SIEM_AUTH_ENV) { ... };
    let mut value =
        HeaderValue::from_str(&value).map_err(|_| StartupError::Delivery(SIEM_AUTH_REJECTED))?;
    // Redacts it in any `Debug` rendering, including reqwest's own.
    value.set_sensitive(true);
    Ok(Some((name.clone(), value)))
}
```

`StartupError::Delivery` on the crate-level enum (`src/lib.rs:322-335`), a `&'static str` so no
part of a rejected credential can be interpolated into it:

```rust
pub enum StartupError {
    DataDir(store::startup::StartupError),
    Bind { addr: SocketAddr, source: std::io::Error },
    Serve(std::io::Error),
    /// Decision-log delivery could not be set up. The text is a fixed `&'static str`
    /// on purpose: it is printed by `check_config` (`src/main.rs`) and by the startup
    /// log, and the rejection it most often reports is a malformed `OSPREY_SIEM_AUTH`.
    /// Nothing that could carry part of that value can be put here.
    Delivery(&'static str),
}
```

Changed files for this slice: `src/delivery/siem.rs` (new), `src/delivery/mod.rs`,
`src/lib.rs`, `src/config.rs`, `tests/decision_log_delivery.rs`, `tests/config_validation.rs`,
`config.sample.toml`, `docs/operations.md`.

## Proof

Commands re-run by the main agent after the fix, not taken on the implementer's report:

| Command | Result |
|---|---|
| `cargo test --test decision_log_delivery` | 13 passed, 0 failed: tp1, tp2, tp2b, tp3, tp7, tp8, tp9, tp10, tp11a, tp11b, tp13, tp15, tp15b |
| `cargo test --test config_validation` | 20 passed, 0 failed, incl. `tp5b_siem_auth_header_requires_siem_url` and `tp6_siem_url_scheme_rule` |
| `cargo clippy --all-targets -- -D warnings` | exit 0, no warnings |
| `cargo test` (full suite) | 294 passed, 0 failed |

Reviews, each a fresh isolated subagent against the approved Gate 3 row set:

| Review | First pass | After fix |
|---|---|---|
| `sf-security-review` | — | **CLEAR** |
| `sf-code-review` | FIX FIRST | **CLEAR** on recheck |
| `sf-adversarial-testing` | FIX | **PASS** on recheck |

`sf-security-review`'s findings: credential read once in `siem_auth()` (`src/delivery/mod.rs:254-268`),
marked `set_sensitive(true)`; `Config.siem_auth_header` is typed `HeaderName`, so the credential
value cannot reach the `Debug`-deriving `Config` structurally; `StartupError::Delivery` is
`&'static str` (`src/lib.rs:335`), so no rejected value can be interpolated; client built with
`redirect::Policy::none()` and 3s/1s timeouts (`mod.rs:212-214`).

### The blocker found and fixed

Code review and adversarial testing independently found the same defect; adversarial testing
reproduced it against a live `App`. `send()` emptied `batch` with a synchronous `batch.drain(..)`
before its first `.await`. That send is awaited inside the drain deadline's
`tokio::time::timeout`, and a mid-retry cancellation drops the future before its tail
`drops.fetch_add(count, ..)` runs — while `batch` and `rx` are already empty of those records, so
the drain arm's `lost = batch.len() + rx.len()` counts nothing for them. Up to 256 records per
cut-off batch vanished with no increment to any counter, breaking the C20 / T-DROP-BLACKOUT
promise and the claim `docs/operations.md` prints: "A record is never discarded silently."

Reproduction before the fix: shutdown correctly bounded at 5.0019s, `total dropped_siem reported
anywhere = 0`, no drain-tail line at all.

Fix: an `Unsent` RAII guard, armed immediately after the drain and disarmed only on the success
return, so a cancelled future counts by construction rather than by a code path remembering to;
add-only, so it cannot underflow the `swap(0, ..)` reset the summary path uses.

`tp15b_drain_cutoff_records_are_counted` (`tests/decision_log_delivery.rs:688`) is the regression
witness, proven discriminating by reconstructing the pre-fix `send()` in a scratch tree and
running the unmodified test against it: FAILED there, passes here.

## Limits

Two items recorded for a user decision. Neither is implemented; both were deliberately left
outside this slice's approved scope, not overlooked:

1. **`file::run` has the same cancellation-before-accounting shape.** It counts neither the
   in-hand batch nor the queue at its drain deadline. It is a slice-1 sibling of the defect just
   fixed in `siem::run`. Slice 1 is already recorded complete with its reviews closed, so fixing
   this means reopening slice 1's surface, not extending slice 2.
2. **The SIEM client omits `.no_proxy()`,** unlike the upstream registry client at
   `src/upstream/reqwest_transport.rs:95-105`, so it honours `HTTP_PROXY`/`HTTPS_PROXY`. Security
   deliberately did not score this: anyone able to set that variable can already read
   `OSPREY_SIEM_AUTH` from the same process environment, so there is no new privilege crossing.
   It is a one-line consistency gap with the sibling client, not a fixed defect.

Documentation correction, not a wrong decision: the approved Gate 4 row describes this slice as
introducing "the first outbound egress from this process". That is factually wrong —
`src/upstream/reqwest_transport.rs:95-105` has always egressed to npm and PyPI, with
`no_proxy()`, rustls, a `GuardedResolver` against private addresses and a same-origin redirect
policy.

Latent, unreproducible today: a `serde_json::to_string` failure inside `send()` would undercount,
but cannot fail for the current record types, whose fields are all `String`, `u64` and
`&'static str`.

## Next

Slice 3 — the `log_consumer_identification` opt-in that records the peer IP, which is the slice
that reverses the standing "nothing a client sends reaches a log line" rule.

## Recommendation

Continue to slice 3. This slice's outcome is fully delivered and witnessed, its one defect is
fixed with a discriminating regression test, and all three reviews are closed CLEAR/PASS. The two
open items above are genuine user decisions (reopen slice 1's drain accounting? add `.no_proxy()`
for consistency?), not blockers to starting slice 3.

Context indicator: full context available; no summarization occurred while gathering this
evidence.


Continue to slice 3, or re-steer?
