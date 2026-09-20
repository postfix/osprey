# Codebase overview

## Purpose
Osprey is planned as an "Open-Source Package Firewall for npm and PyPI" (manual, README.md:2). SPEC.md — titled "Package Firewall — Rust MVP Specification," status "reviewed implementation specification; runtime validation pending," dated 2026-09-17, revision 2 (manual, SPEC.md:1-6) — describes a self-hosted registry proxy for public npm and PyPI packages that hides releases younger than a configurable cooldown (default 24h) and known-malicious packages/artifacts, while leaving dependency resolution to npm/pip (manual, SPEC.md:8-16).

## Start here
There is no source code to start from yet. The repository currently contains only prose and metadata:
- `/home/john/go/src/github.com/postfix/osprey/README.md` — one-line project description (manual).
- `/home/john/go/src/github.com/postfix/osprey/SPEC.md` — the 452-line implementation specification; this is the de facto design document and closest thing to an entry point for a future implementer (manual).
- `/home/john/go/src/github.com/postfix/osprey/LICENSE` — Apache License 2.0 (manual, LICENSE:1-2).
- `/home/john/go/src/github.com/postfix/osprey/.gitignore` — a Go-flavored ignore template (ignores `go.work`, `go.work.sum`, Go test binaries) (manual, .gitignore:23-25), which is notable because it contradicts SPEC.md's Rust-based design (see Unknowns).

There is no `src/`, `Cargo.toml`, `Cargo.lock`, `go.mod`, `package.json`, or any other build manifest anywhere in the tree (structural: `smtc orient`/`smtc freshness` report `file_count: 0`, `analysis_depth`/status `not_built`→`cached` with no analyzable source; confirmed manually via `find . -iname 'Cargo.toml' -o -iname 'go.mod' -o -iname 'package.json'` returning nothing).

## Shape
As-exists: flat repository with 4 tracked files (README.md, SPEC.md, LICENSE, .gitignore) plus `.git/` and a local `.smtc-cache/` directory. No modules, packages, or runnable code exist (structural, via `smtc orient --with-repo-map`: repo-map fallback fired because there is nothing to map — `fallback_reasons: ["get_repo_map: tool returned None"]`; manual `find -maxdepth 3` confirms the same file list).

As-specified (SPEC.md §3, lines 35-58; not yet built): one Rust binary + container image, single process, single local data directory, loopback listener behind an external reverse proxy for TLS. Proposed module layout inside one Cargo package: `config`, `policy`, `npm`, `pypi`, `upstream`, `artifacts`, `store`, `http`. Proposed stack: Tokio, Axum, Reqwest+rustls, Serde, TOML, the embedded `turso` crate (Rust-native, MIT-licensed, local-only — explicitly not the older libSQL engine or hosted Turso), SHA-2 hashing, tracing (manual, SPEC.md:56-67).

## Build and test
No build or test commands exist because no manifest or automation exists (verified: no Cargo.toml/Makefile/CI config found). SPEC.md proposes future CLI commands that are not yet implemented (manual, SPEC.md:97-101):
```
package-firewall serve --config /etc/package-firewall/config.toml
package-firewall check-config --config /etc/package-firewall/config.toml
package-firewall check-blocklist /etc/package-firewall/blocklist.json
```
SPEC.md §13 (lines 383-408) also specifies an extensive future test plan (fake registry with deterministic clock control, pure-policy tests, protocol tests, real package-manager installation tests, benchmark harness) and states the MVP is "complete when these tests pass" — none of this exists yet. Do not infer any command from ecosystem convention; there is nothing to run today.

## Conventions
No code conventions can be observed because no code exists. Documentation conventions actually in use:
- SPEC.md is written as a single authoritative, versioned specification with an explicit "Status," "Revision," and a review/changelog section (§15) recording findings (STATE-01, REL-01, FLOW-01, PERF-01, TEST-01, OPS-01) and a decision ledger — this is a deliberate, unusually rigorous spec-first process (manual, SPEC.md:422-451).
- The license is Apache-2.0, chosen already even before implementation (manual, LICENSE:1-2), though SPEC.md §13 line 406 separately says "Select an explicit open-source license before public release" — slightly inconsistent phrasing since a license is already present at the root.

## Invariants
Product/behavioral invariants asserted by SPEC.md (not yet enforced by any code — manual, SPEC.md §5, §9, §10):
- Eligibility is never weakened to satisfy a dependency resolution: "Never weaken dependency requirements" (SPEC.md:26) and blocked/held exact versions are refused outright, never substituted (SPEC.md:24).
- Known malware blocks always override age-based cooldown; no implicit exemptions for locked/popular packages (SPEC.md:126).
- Computed artifact digests (SHA-256/SHA-512), once pinned on first verified download, are permanent for that exact reference and every later download must match them, even after the bytes are later blocked by policy; a mismatch is a hard `502 INTEGRITY_MISMATCH` (SPEC.md:260).
- A blocklist snapshot is only accepted with a strictly increasing revision and a valid `generated_at <= now < expires_at` window; rollback and unchanged-revision content changes are rejected; an expired blocklist fails both resolution and cached artifact delivery with `503` (SPEC.md:210-214).
- No artifact body bytes are ever forwarded to a client before verification completes; verification happens fully before any streaming (SPEC.md:262, 394).
- A database/storage error must prevent any dependent state change — never publish a response backed only by uncommitted records (SPEC.md:288).

## Hotspots and landmines
Because no implementation exists, these are risks called out by the spec review itself (SPEC.md §15, lines 422-451) that a future implementer must not overlook:
- **STATE-01 (High, conditional)**: without a strong upstream integrity digest, a refetched artifact for the same reference could silently acquire different bytes unless computed digests are permanently pinned and compared on every refetch (SPEC.md:430).
- **REL-01 (Medium)**: naive rename-then-commit ordering can leave committed identity records pointing at non-durable bytes after a crash; requires explicit file/directory sync ordering and atomic project/reference transaction boundaries (SPEC.md:431).
- **FLOW-01 (Medium)**: coalesced downloads with no defined waiter-cancellation/timeout behavior can leak download slots, disk reservations, and file pins (SPEC.md:432).
- **PERF-01 (Medium)**: the original pseudocode refreshed upstream metadata before checking a locally known malware block, which would make a "fast" denial wait on the network — corrected to check local conclusive blocks first (SPEC.md:433).
- **TEST-01 (Medium)**: production code fixes upstream origins and rejects private/loopback addresses, so a test-only transport/origin injection seam must exist without any release-mode bypass switch (SPEC.md:434).
- **OPS-01 (Medium)**: artifact cache byte budgets must not be confused with total state/WAL storage capacity; an operator misreading this could delete state data thinking it's prunable cache (SPEC.md:435).
- A structural landmine already present in the repo: `.gitignore` assumes a Go project (`go.work`, `go.work.sum`, Go test-binary patterns) while SPEC.md mandates a Rust implementation — this file will likely need to be replaced with a Rust/Cargo-oriented `.gitignore` when implementation starts.

## Unknowns
- Whether "Rust MVP" in the SPEC.md title is final, given the repository lives under a Go-style GOPATH-like path (`.../go/src/github.com/postfix/osprey`) and ships a Go-oriented `.gitignore` — unresolved contradiction between manual evidence sources; not verifiable from the tree alone.
- Whether any implementation work has started outside this checked-in tree (e.g., a local uncommitted branch or another repo) — outside the scope of what git/the filesystem here can show; git status is clean and history has only 3 commits (manual, `git log --oneline`: b3636d2, b46a8f1, 1acede7).
- Whether the "Select an explicit open-source license before public release" note in SPEC.md:406 means the current Apache-2.0 LICENSE file is provisional or already final — not stated anywhere.
- No performance, security, or lifecycle analysis is possible yet since `smtc` reports zero analyzable files; SPEC.md's own performance targets (§12) are explicitly aspirational ("to be measured rather than advertised as achieved," SPEC.md:368) and unverified.

## Evidence freshness
- `smtc orient --root <root> --with-repo-map --with-health --format json --max-tokens 4096 --session-id osprey-onboard`: ok, but `analysis_freshness.status: not_built`, `file_count: 0`, repo-map fallback fired (`get_repo_map: tool returned None`), no recommended starting points returned — structural, run, outer ok / inner reflects "nothing to analyze."
- `smtc freshness --root <root> --format json --max-tokens 2048 --session-id osprey-onboard`: ok, `status: cached`, `file_count: 0`, `possibly_stale: false` — structural, confirms the zero-source state is current, not a stale cache.
- Manual reads: README.md (full, 2 lines), SPEC.md (full, 452 lines), .gitignore (full, 33 lines), LICENSE (header only, confirms Apache-2.0) — manual, all read directly 2026-09-17.
- Manual shell checks: `find . -maxdepth 3 -not -path './.git*' -type f`, `find . -iname 'Cargo.toml' -o -iname 'go.mod' -o -iname 'package.json'` (empty result), `git log --oneline --all` (3 commits, matches task-provided git status) — manual, run 2026-09-17.
- Reviewed at commit `1acede7` on `main`, clean working tree, 2026-09-17.
