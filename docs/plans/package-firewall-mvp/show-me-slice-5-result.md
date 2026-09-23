## What problem do we have?

Slice 5 made `GET /npm/{package}` and `GET /npm/{package}/{version-or-tag}` real: held and blocked versions are removed, the `latest` tag falls back correctly, other tags on excluded versions are dropped rather than guessed, tarball URLs are rewritten, and a warm repeat request costs no database query and no upstream call. That work is done, reviewed, and independently re-witnessed — it is not in question.

What is in question is three obligations the approved program design placed on this feature that no slice row in the approved plan currently owns: coalescing concurrent metadata refreshes for one package, honoring an upstream "not modified" answer instead of treating it as a failure, and four named tests covering package removal, revalidation, and clock movement. All three were written into the design before the slice table was drawn, and the split that produced the ten-row table dropped them on the floor. Today, 25 simultaneous first-time requests for one uncached package produce 21 separate upstream calls instead of one — a duplicated-traffic and burst-amplification risk, not a data-correctness one, because the storage layer still keeps only correct data through a single-writer design.

## How will we solve it?

Reopen Gate 4 before dispatching slice 6, and reconcile the approved slice table against the approved program design so every design-level obligation has a named row and owner — including the three items above, and the same authoring gap that has forced four consecutive slices to edit files outside their approved list because rows name files but not the call sites those files force. Slice 5 itself needs no rework and its proof line stands; reopening the gate does not touch it.

## How will we confirm it is solved?

- The gate reopens and the reconciled slice table names an owning slice for request coalescing (program design line 685), for upstream revalidation handling, and for each of the four missing tests (program design line 813): `upstream_removal_becomes_404`, `revalidation_after_ttl`, `projection_expires_at_next_hold_release`, `backward_clock_jump_recomputes_projections`.
- Slice 5's own witnesses, already independently re-run by the main agent and not taken from the implementer's report: `cargo clippy --all-targets -- -D warnings` exit 0 clean; `cargo test --test npm_metadata` ok, 20 passed including all eight row-named tests; `cargo test --test blocklist_snapshot` ok, 5 passed including `expiry_stops_delivery_including_warm_hits`; `cargo test --test persistence_projects` ok, 5 passed for exactly the five named tests, including `first_seen_survives_restart` and `upstream_timestamp_supersedes_first_seen` — the "a restart does not reset the clock" product promise.
- sf-code-review SHIP with no defects, sf-adversarial-testing PASS with no defects reproduced across eight attacks, sf-verification VERIFIED.

## Decisions

- Slice 5 is complete and correctly proven: filtering, tag fallback, URL rewriting, and warm-cache behavior all work as specified, confirmed by an independent re-run of every named test.
- Request coalescing (SPEC §10: "Coalesce concurrent metadata refreshes for the same project") does not exist yet. Measured effect: 21 of 25 simultaneous first-time requests independently called upstream. Consequence is duplicated upstream traffic and burst risk, not data corruption.
- Upstream-304 handling (SPEC §10: "An upstream 304 renews upstream freshness") is unimplemented; a not-modified answer is today treated as an upstream failure.
- Four tests named in the approved program design appear in no slice row and exist nowhere in the codebase.
- All three gaps trace to the same root cause: the design-to-slice split named files but not the call sites those files force, which is also why four consecutive slices had to edit out-of-list files.
- Fixture and probe caveats are test evidence, not product behavior: the oversized-reference fixture used 40 versions against a 24-version test-set cap rather than a full-scale document, and reporting a block ahead of a hold when nothing is eligible is a deliberate implementation choice, not a spec requirement.
- Recommendation: re-steer — reopen Gate 4, assign owners for all three items, and only then dispatch slice 6. Continuing to slice 6 now defers three known obligations and, on the plan's own trajectory, surfaces the rest of the authoring gap around slice 9.

Full slice table: `docs/plans/package-firewall-mvp/04-slices.md`

Continue to slice 6, or re-steer?
