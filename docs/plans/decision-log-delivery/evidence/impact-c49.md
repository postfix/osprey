# Impact: C49 decision timestamp (sf-impact, 2026-09-22)

Subject: working tree on `package-firewall-mvp` (base `0f50cec` plus uncommitted slices 1-3).

The text search was complete: 7 sites in all, one of them test-only. Function call sites are **Structural** (call graph). Struct literal sites are **Manual**: the graph has no struct-literal edges and cannot tell `delivery::Decision` apart from `policy::Decision`.

| Symbol | Verdict | Sites (one-based) | Test-only |
|---|---|---|---|
| `delivery::Decision` (src/delivery/mod.rs:52) | UNKNOWN (graph); Manual site list | src/http/logging.rs:206 (`decide`) | no |
| `delivery::Summary` (src/delivery/mod.rs:76) | UNKNOWN (graph); Manual site list | src/http/logging.rs:406 (`close_window`) | no |
| | | src/delivery/file.rs:236 (`#[cfg(test)] mod tests`, `tp19_file_drain_deadline_counts_cut_off_records`) | yes |
| `http::logging::summarise` | WIDE by entry-point rule; 1 site | src/http/logging.rs:243 (`decide`) | no |
| `http::logging::flush_summary` | WIDE (cross-module); 1 site | src/lib.rs:321 (`Running::shutdown`) | no |
| `http::logging::close_window` | WIDE by rule; 2 sites | src/http/logging.rs:366, :379 | no |

- No integration test calls any of these items; they are `pub(crate)` or private.
- A grep for exact serialised-JSON assertions found 0 matches, so moving `timestamp` to the front changes no pinned string. Assertions that parse JSON are not ruled out.
- The compiler (E0063 missing field, arity errors) is the Decision-tier check after the edit.
- Consequence for C49: the TP-19 unit test's `Summary` literal at `src/delivery/file.rs:236` must gain `timestamp`. That is a mechanical test edit forced by the struct change, not a change to what TP-19 asserts.
