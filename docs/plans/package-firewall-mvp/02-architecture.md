# Architecture: Package Firewall MVP

Sources: `SPEC.md` revision 2 (sections 3, 4, 8–11, 13, 15), `01-product.md` (approved 2026-09-17), and the repository briefing `docs/codebase-overview.md` (current at commit `1acede7`). Every "exists" claim below is repository evidence; every other claim is a proposal taken from `SPEC.md` and labelled with its section.

## Fit
Greenfield. The repository at `1acede7` holds `README.md`, `SPEC.md`, `LICENSE` (Apache-2.0), and `.gitignore` only — no manifest, source, tests, CI, or container build (briefing, confirmed by the user 2026-09-17). Nothing existing is touched; everything below is new.

Shape (SPEC §3): one Rust binary `package-firewall`, one Cargo package at the repository root, one process, one local data directory, loopback listener behind the operator's own TLS reverse proxy. Eight modules with one-way dependencies:

| Module | Owns | May depend on |
| --- | --- | --- |
| `config` | Parse and validate the TOML file at startup; the three CLI commands' shared validation. | — |
| `policy` | Pure eligibility decision (`UNAVAILABLE / DENY / HOLD(until) / ALLOW`) and immutable blocklist snapshots. No I/O, no clock reads — `now` is passed in. | — |
| `store` | The single storage task owning the one embedded Turso connection; bounded memory caches. | `policy` (snapshot types) |
| `upstream` | Pooled HTTPS client, timeouts, size bounds, fixed-origin and address validation. The test seam: transport and origin set are injected through the application constructor. | `config` |
| `npm` | npm names, package documents, version ordering, tags, tarball references, filtered rendering. | `policy`, `store`, `upstream` |
| `pypi` | Project-name normalisation, filename/PEP 440 parsing, Simple API HTML and JSON rendering. | `policy`, `store`, `upstream` |
| `artifacts` | Reference lookup, download coalescing, hashing, digest pins, content cache files, eviction, response streaming. | `policy`, `store`, `upstream` |
| `http` | Axum routes, error bodies, health, request IDs, structured logs, overload limits. | all of the above |

Background work inside the same process: the blocklist poller (SPEC §8) and batched cache maintenance (access times, eviction, checkpoint; SPEC §10).

## Endpoints
All from SPEC §11. Explicit routes only; no catch-all proxying. Every response carries `Cache-Control: no-store`.

- `GET /npm/{package}` — filtered full or abbreviated package document (scoped names, including the percent-encoded form).
- `GET /npm/{package}/{version-or-tag}` — policy-checked single-version document.
- `GET /npm/-/ping` — npm connectivity response.
- `GET /pypi/simple/` — index of projects this instance already knows.
- `GET /pypi/simple/{project}/` — filtered file listing, HTML or JSON by content negotiation; local trailing-slash redirect.
- `GET, HEAD /artifacts/{reference_id}/{filename}` — verified artifact delivery, single byte range supported.
- `GET /health/live` — process liveness.
- `GET /health/ready` — valid unexpired blocklist and usable storage; no live upstream probe.

Failure mapping (SPEC §11): `403` held or blocked (JSON `error`, `reason`, `request_id`, and for cooldown `eligible_at` plus numeric `Retry-After`); `404` unknown or removed upstream; `400` invalid input; `404/405/406` unsupported route, method, representation; `503` blocklist missing or expired, capacity exhausted, overloaded, storage unusable; `502` upstream failure, invalid upstream metadata, integrity mismatch; `504` upstream timeout.

Operator commands (SPEC §4), not endpoints: `package-firewall serve --config <path>`, `package-firewall check-config --config <path>`, `package-firewall check-blocklist <path>`. The two check commands exit nonzero on failure and write no state.

## Data
One embedded Turso database at `<data_dir>/state/firewall.db`, WAL, `PRAGMA synchronous=FULL` verified at startup; content files under `<data_dir>` keyed by content SHA-256; an exclusive data-directory process lock (SPEC §3, §10). Timestamps are UTC microseconds since the Unix epoch.

| Table | Holds | Queries that hit it |
| --- | --- | --- |
| `schema_meta` | Schema version; Turso crate version of the build. | Read once at startup; refuse to run on mismatch. |
| `project_snapshot` | Ecosystem, normalised name, upstream payload, upstream validators, last successful validation time, generation. | Point read by (ecosystem, name) on memory-cache miss; upsert once per refresh; list names for `/pypi/simple/`. |
| `artifact_reference` | Reference ID, project, version, filename, upstream URL, expected digests, publication or first-seen time, pinned computed SHA-256/SHA-512, optional content key. | Point read by reference ID on artifact request; batch upsert inside the project-refresh transaction; update pins once after first verified download; clear content key on eviction. |
| `content` | SHA-256 key, SHA-512, size, created and last-access time. | Insert after file rename and directory sync; batched access-time updates; oldest-first scan for eviction; startup reconcile against files on disk. |
| `blocklist` | Last accepted revision, timestamps, the complete validated snapshot. | Read once at startup; replaced in one transaction per accepted revision, with queue priority over maintenance. |

Write rules that the design depends on (SPEC §10): a project snapshot, its reference upserts, and new first-seen values commit together before the new in-memory generation is published; a blocklist commits before its memory snapshot is published; a content file is flushed, synced, renamed, and its directory synced before its mapping commits; a database error blocks the dependent response. Warm requests issue no database query.

## Flow
Metadata request, npm or PyPI (SPEC §6, §7, §10):
1. `http` validates and normalises the route, takes an active-request permit, assigns a request ID.
2. Load the current policy snapshot. Missing or expired → `503`.
3. Look up the rendered response in the memory cache; reuse only if project generation, blocklist revision, that project's digest generation, representation, and deadline all match. Hit → respond.
4. Miss → `npm` or `pypi` asks `store` for the project snapshot; if its TTL has passed, `upstream` revalidates it (one coalesced refresh per project).
5. `store` commits snapshot + references + first-seen times in one transaction, then publishes the new generation.
6. `policy` evaluates every candidate with one `now`; the ecosystem module filters, applies tag rules (npm) or per-file rules (PyPI), rewrites artifact URLs to `/artifacts/{reference_id}/{filename}`, renders, caches with deadline = earliest of metadata TTL, blocklist expiry, next hold release.
7. Respond, log the decision.

Artifact request (SPEC §9):
1. `http` → `artifacts`: look up the reference by ID; unknown → `404`.
2. Locally conclusive denial first: expired policy → `503`; known package or digest block → `403`, without touching upstream.
3. Ensure fresh project metadata (step 4–5 above); the reference must still belong to it, else `404`.
4. `policy` evaluates with advertised and pinned digests.
5. Verified content cached → open and pin the file. Otherwise join or start the single shared download: reserve capacity, stream to a temp file with content decoding off, compute SHA-256 and SHA-512 in one pass, verify upstream integrity and size, compare with existing pins, persist computed digests (even if now blocked), re-evaluate policy, publish the file durably.
6. Revalidate membership if metadata expired meanwhile; final policy check immediately before the response is created — this is the revocation boundary.
7. Stream the verified local file; a newly discovered digest block invalidates that project's rendered metadata.

Blocklist reload (SPEC §8): poller detects a change → read within the size limit → validate the whole candidate off the request path → require a strictly higher revision and a valid time window → commit → atomically swap the in-memory snapshot. A bad candidate keeps the last good snapshot and logs an error.

Startup: parse config → take the data-directory lock → open the database and let it recover → verify `synchronous=FULL` and the schema row → remove temp files, clear mappings to missing files, reclaim unreferenced content → load the persisted blocklist if still valid → listen. Failed recovery keeps readiness false and never recreates the database.

## Constraints
1. **Language is Rust.** The checkout sits under a Go-style path and `.gitignore` is GitHub's Go template, but `SPEC.md` (title, §1, §3, §15 ledger "explicit user decision … native Rust engine") fixes Rust. Decision: Rust; `.gitignore` is replaced with Rust entries plus `.smtc-cache/` in the first slice. The Go path has no effect on a Cargo build.
2. **The embedded Turso crate is pre-1.0** (SPEC §3, §13). The architecture is invalid if the pinned release cannot do WAL with `synchronous=FULL`, explicit transactions, crash recovery, and a supported checkpoint. Therefore all SQL stays inside `store` behind one interface, the crate version and `Cargo.lock` are pinned before any persistence code is trusted, and a persistence-and-recovery slice comes early in Gate 4, directly after the pure-policy slice. The engine choice is a user decision and is not reopened here.
3. **One storage task, one connection, bounded queue.** Anything that needs a database round trip on a warm request breaks the speed bar; the memory caches exist to prevent that.
4. **No release-binary bypass.** The fake registry used by tests reaches the application only through the constructor-injected transport and origin set (SPEC §13, finding TEST-01). No config key, flag, or environment variable may relax origin or private-address checks.
5. **Filesystem.** Temp files and the content cache share one filesystem so rename is atomic; the data directory is service-owned and trusted; the host clock must be accurate, and a detected backward jump forces projections to be recomputed.
6. **Bounded everything.** Semaphores and bounded queues for active requests, upstream requests, and downloads; overload is refused, never queued without limit. The artifact-cache budget does not cap the state database or WAL (finding OPS-01) — that is an operations-README item, not a code guarantee.
7. **Review findings carried forward.** STATE-01 (permanent digest pins), REL-01 (sync and commit order), FLOW-01 (waiter cancellation and timeouts), PERF-01 (local denial before network), TEST-01, OPS-01 are all adopted as written in SPEC §15; Gate 3 must give each a test name.
8. **Licence.** `LICENSE` already commits Apache-2.0; SPEC §13's "select a licence before public release" is treated as satisfied by it. All proposed dependencies are MIT or Apache-2.0 compatible per SPEC §3; Gate 3 records the licence of each pinned crate.
9. **No build or CI exists.** Gate 4's first slice must create `Cargo.toml`, `Cargo.lock`, and the commands every later witness runs (`cargo build`, `cargo test`, `cargo clippy`).
10. `docs/` is untracked in Git today; the plan documents should be committed so a fresh clone can resume. That is the user's call and changes no architecture.

`sf-red-team: not triggered` — `SPEC.md` revision 2 already carries a recorded adversarial review with six resolved findings (§15); the one material remaining uncertainty (pinned Turso behaviour) is empirical and is answered by the early persistence slice, not by further plan review. The security surface (untrusted upstream data, outbound-request restrictions) triggers `sf-threat-model` at Gate 3, where the types and paths it needs will exist.

## External
- `https://registry.npmjs.org` — npm package documents and tarballs.
- `https://pypi.org` — Simple API JSON project documents.
- `https://files.pythonhosted.org` — PyPI artifact downloads.
- These three origins are fixed in the binary (SPEC §4). HTTPS only; cross-origin redirects, URL credentials, unexpected ports, and loopback, private, or link-local resolutions are rejected; client authorization, cookies, and proxy credentials are never forwarded (SPEC §11).
- Local inputs: the TOML config file (path from `--config`) and the JSON blocklist file (path from config key `blocklist_file`), written only by trusted operators or the external blocklist producer.
- Environment variables: none. Webhooks: none. No hosted database, account, or telemetry service.
- Operator-provided, outside this binary: the TLS reverse proxy, network policy forcing clients through the firewall, and backups of `state/`.
- Rust dependencies (SPEC §3, versions pinned at Gate 3): Tokio, Axum, Reqwest with rustls, Serde, a TOML parser, the embedded `turso` crate, SHA-2, tracing, ArcSwap, and maintained parsers for URLs, timestamps, npm version ordering, Python distribution filenames and PEP 440, and SRI.
