# Slice 14 result — artifacts under the ecosystem root

## Delivered

Package files now live under each ecosystem's own address — `/npm/artifacts/{reference_id}/{filename}` and `/pypi/artifacts/{reference_id}/{filename}` — replacing the single shared `/artifacts/{reference_id}/{filename}`.

```diff
- GET /artifacts/{reference_id}/{filename}
+ GET /npm/artifacts/{reference_id}/{filename}
+ GET /pypi/artifacts/{reference_id}/{filename}
```

Nothing migrated: the reference id is a fingerprint of the file's own identity (ecosystem, name, version, filename, upstream URL, sorted digests) and never of its address, so no id rotated, no stored record was invalidated, and no prior trust decision moved. The move also adds a check that was not previously possible: the handler now compares the URL's ecosystem segment against the reference row's own stored ecosystem and refuses a mismatch, because a reference id is public and computable and was never a secret — without the check, an npm file requested under the PyPI address would have been served and logged under the wrong ecosystem.

Changed surface: `src/http/mod.rs` (route registration), `src/http/artifact_routes.rs`, `src/artifacts/mod.rs` (`serve_artifact`, ecosystem-mismatch check), `src/npm/render.rs` (`artifact_url`), `src/pypi/mod.rs`, `src/http/logging.rs` (`Target::of`), plus seven test files carrying forced path-literal or positional-parse edits, `SPEC.md` (:228-229, :339-340), and `docs/operations.md`'s npm-12 subsection, fully rewritten.

## Proof

All counts below are DEBUG builds.

- `cargo clippy --all-targets -- -D warnings`: exit 0, no warnings.
- Full suite: **253 passed, 0 failed, 15 ignored** (up from 248/0/14 before this slice).
- `http_contract`: 14 passed, including `an_npm_reference_id_under_the_pypi_artifact_root_is_404`, `a_pypi_reference_id_under_the_npm_artifact_root_is_404`, and `the_artifact_decision_line_logs_the_ecosystem_of_the_reference_row`.
- `npm_metadata`: 34 passed. `pypi_metadata`: 27 passed.
- `e2e_npm -- --ignored`: 7 passed, **including `npm_12_installs_without_allow_remote`** — a real npm 12.0.2 install under nvm Node v24.19.0, with no `.npmrc` and no override flag. This is the decisive result: it failed before this slice with npm's own `EALLOWREMOTE` refusal, and the test asserts no config file exists in either location, so the rejected `allow-remote=all` workaround cannot be what made it pass.
- `e2e_pip -- --ignored`: 7 passed, across both pip 26.2.1 (current) and pip 25.0.1 (previous), against the new address.
- Re-executed independently by the main agent, by `sf-code-review`, and (the `e2e_pip` leg) by `sf-verification`.
- Three named reviews, all clean: `code-review=SHIP`, `security-review=CLEAR`, `adversarial-testing=PASS`, then `verification=VERIFIED`.

## Limits

- Neither new address existed before this slice, so both would already have returned "not found" through the generic router fallback for the wrong reason — a status-code-only test would have passed vacuously. Each cross-root test therefore first proves it reached the real artifact handler (a route-registration probe expecting `400 INVALID_INPUT`, which only that handler produces), and the routes were additionally landed WITHOUT the ecosystem check in a staged experiment, which returned `502` (request reached upstream) — isolating the refusal to the ecosystem comparison rather than a missing route.
- The approved plan's own file list — presented in the program-design document as a verified trace — was one site short: a sixth positional `.nth(2)` path parse in `tests/slice11_adversarial_singleflight.rs`, reached only through a helper and invisible to literal search. Found and fixed by the implementer. This is the second time in this plan this class of miss has occurred.
- After the three reviews passed, the adversarial pass found that the test for a package literally named `artifacts` only checked the rendered URL's shape and never downloaded from it. That gap was closed: the test now issues the GET and asserts the delivered bytes, proved non-vacuous by a negative control (wrong upstream bytes left the status check passing and failed the content check).
- All test counts are from a debug build. One unrelated timing test from an earlier slice still fails in release mode on a margin optimized code cannot satisfy; independently confirmed pre-existing, and recorded as a follow-up nobody owns yet.
- The equivalent byte-level download test was deliberately not written for PyPI. Verification ruled the omission justified: the address collision that makes the npm case delicate (a route-precedence question) does not exist in PyPI's address shape, and the download path itself is shared code, already exercised.

## Next

Slice 15 — a configurable maximum age for cached package information, approved at Gate 4. Slices 15 and 10 remain; slice 15 is next.

## Recommendation

Slice 14 is complete, witnessed, reviewed by three named reviews, and independently verified. Recommend continuing to slice 15.

Document: docs/plans/package-firewall-mvp/00-status.md

Continue to slice 15, or re-steer?
