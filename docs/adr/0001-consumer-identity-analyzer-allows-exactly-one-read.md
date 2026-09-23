# 0001 — The consumer-identity analyzer allows exactly one read, not zero

Date: 2026-09-23. Status: accepted. Source: decision-log-delivery Gate 3 C29, raised by the
slice 3 security review (F3).

## Context

`src/http/logging.rs` deliberately reads one inbound-request accessor,
`request.extensions()`, to obtain the peer address for opt-in consumer identification. The
retained analyzer spec that guards "nothing a client sends reaches a record" matches receiver
idioms by call name, and its call-name list cannot see `extensions`. Its pass condition was
zero findings across the product root, and its own prose claimed it fails the moment any
inbound-request accessor call site exists — which was no longer true.

## Decision

A second retained spec, `.smtc/analyzers/consumer-identity-extension-read.yaml`, covers
`extensions` with the same `request|req|parts` receiver idiom. Its pass condition is **exactly
one finding, at `src/http/logging.rs`**, not zero. `extensions` was deliberately not added to
the existing spec.

## Why

Adding `extensions` to the existing spec would fire on the one approved read and destroy that
spec's zero-findings pass condition, leaving no guard at all. Putting the exception in the pass
condition keeps it in the spec rather than in an engine suppression feature this SMTC build is
not confirmed to have. A count moving off 1 is as loud a signal as a zero becoming 1.
