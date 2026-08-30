---
status: accepted
date: 2026-08-30
decision-makers: [Team]
---

# The registry client reads HTTP status as data, not as an error

## Context and Problem Statement

The OCI resolver decides whether a branch tag exists by sending a manifest `HEAD`. Three statuses mean three different things: `2xx` is a hit, `404` is a plain miss, and `401` is an instruction to authenticate and retry. The `401` is not merely a signal — its `WWW-Authenticate` header carries the realm, service and scope the client must call to obtain a bearer token, so the response body and headers are load-bearing input, not diagnostics.

The HTTP client's default behaviour turns `4xx` and `5xx` into errors. In ureq 2 that was still workable, because the `Status` variant embedded the whole response, so a `401` could be matched and its headers read straight off the error. ureq 3 replaced that with `StatusCode(u16)`, which carries the number alone. Under the default configuration the challenge is discarded before the client can see which status it received, and there is no way to recover it.

## Decision Drivers

* Private images are the common case, and reaching them depends entirely on the `401` challenge (see [0011](0011-oci-credentials-from-standard-sources.md)).
* A miss and a failure must stay distinguishable: `404` is an ordinary answer, not a fault.
* Transport failures — DNS, TLS, timeouts — are genuine errors and should keep flowing through `Result`.
* The status logic is the core of the resolver and should stay legible as a single flat match.

## Considered Options

* **A. Configure the agent with `http_status_as_error(false)`** so every status arrives as a response.
* **B. Keep the default and pre-flight an unauthenticated request** whose sole purpose is to capture the challenge.
* **C. Keep the default and treat every `401` as "authenticate against a guessed realm"**, deriving the token endpoint from the registry host instead of reading it.

## Decision Outcome

Chosen option: **A**, because it is the only one that keeps the challenge reachable without inventing a second request or a guess. HTTP status becomes ordinary data the resolver matches on, which is what the resolver was always doing; the error channel is left to represent transport failure, which is what it should have represented all along.

### Consequences

* Good, because the `404` / `401` / `2xx` decision reads as one flat match over `resp.status()`, with no error-type destructuring in the middle of it.
* Good, because the error channel now means one thing: the request did not complete. A caller can treat any `Err` as a real failure.
* Good, because it removes the dependency on the error type's shape, which is what broke on the ureq 2 to 3 upgrade in the first place.
* Bad, because success is no longer implied by the absence of an error. Every request site has to test for `2xx` explicitly, and a site that forgets will read an error body as if it were a result.
* Neutral, because the npm resolver keeps the default. It only distinguishes `404` from everything else and never reads an error response, so the configured agent would buy it nothing.

### Confirmation

The token request in `bearer_token` checks `status().is_success()` before parsing the body, and the two `head` call sites in `tag_exists` test `is_success()` rather than assuming it. A reviewer adding a request to this client should look for that check; its absence is the failure mode this decision introduces.

## Pros and Cons of the Options

### A. Configure the agent with `http_status_as_error(false)`

* Good, because the challenge stays reachable with no extra request.
* Good, because the status match sits in one place and reads top to bottom.
* Bad, because it shifts the burden of noticing failure onto each call site.

### B. Pre-flight request to capture the challenge

* Good, because it leaves the default error behaviour alone.
* Bad, because it doubles the request count on the private-image path, which is the common path.
* Bad, because the two requests can disagree: the challenge captured by the first is not necessarily the one the second would have received.

### C. Derive the token endpoint from the host

* Good, because it needs no response body at all.
* Bad, because it hard-codes knowledge of individual registries, which the resolver deliberately avoids.
* Bad, because it breaks on any registry whose realm is not where the guess expects, and the failure looks like a missing tag.

## More Information

* Supersedes the client shape assumed by [0001](0001-unboxed-ureq-error-in-registry-client.md), which is deprecated: its lint suppression existed only because the error variant used to embed a response.
* The credential sources this decision depends on are recorded in [0011](0011-oci-credentials-from-standard-sources.md).
