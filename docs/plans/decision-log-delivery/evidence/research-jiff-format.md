# jiff 0.2.37 timestamp formatting (C49 evidence, 2026-09-22)

Question: how does `jiff::Timestamp` render as text, and can `from_microsecond` fail?

Answer, from two sources that agree:

1. **Crate source.** Read at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/jiff-0.2.37/src/timestamp.rs`:
   - `:728` has `pub fn from_microsecond(microsecond: i64) -> Result<Timestamp, Error>`, so it is fallible.
   - `:2293-2336` is the `Display` documentation. `Formatter::precision` controls how many fractional-second digits appear. With no precision set, the minimum precision is used. Its doctest asserts `format!("{ts:.6}") == "2005-08-07T23:19:49.123000Z"`.
2. **Execution.** A scratch crate pinned to `jiff = "=0.2.37"` and run with `cargo run --offline` printed:

   | micros | `{t}` | `{t:.6}` |
   |---|---|---|
   | 1790000000000000 | `2026-09-21T14:13:20Z` | `2026-09-21T14:13:20.000000Z` |
   | 1790000000123400 | `2026-09-21T14:13:20.1234Z` | `2026-09-21T14:13:20.123400Z` |
   | 1790000000123456 | `2026-09-21T14:13:20.123456Z` | `2026-09-21T14:13:20.123456Z` |

   `from_microsecond(i64::MAX).is_err()` returned `true`.

Consequence: C49 uses `{:.6}` for fixed-width output, and handles the `Err` case with an explicit fallback.
`Cargo.toml:52` declares `jiff = "0.2.37"` with no features, so `serde` is not enabled.
