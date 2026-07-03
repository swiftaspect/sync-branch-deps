---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# Keep `ureq::Error` unboxed in the registry client; suppress `clippy::result_large_err` locally

## Context and Problem Statement

The OCI-registry existence check (`head_manifest` in `src/main.rs`) returns `Result<ureq::Response, ureq::Error>` so the caller can branch on the HTTP status: `404` means "tag not published" (a miss), `401` means "authenticate and retry", anything else is a hard error.

`ureq::Error` is a large enum — its `Status` variant embeds a whole `Response` (~272 bytes). Because a `Result<T, E>` is sized to the larger of `T` and `E`, every value of this result — *including the `Ok` path* — carries that ~272-byte footprint. Clippy's `result_large_err` lint flags this and suggests boxing the error (`Box<ureq::Error>`). Our CI runs `cargo clippy -- -D warnings`, so an unaddressed lint fails the build.

## Decision Drivers

* The status-code `match` in `image_tag_exists` is the clearest expression of the tool's core registry logic and should stay flat and readable.
* `head_manifest` is called a handful of times per run (once per configured image, plus a possible auth retry) — this is a short-lived CLI, not a hot loop.
* This is a public, educational repository: how we handle a lint should model *reasoned* handling, not reflexive silence.

## Considered Options

* **A. Narrowly `#[allow(clippy::result_large_err)]` on `head_manifest`, with an explanatory comment.**
* **B. Box the error** (`Result<ureq::Response, Box<ureq::Error>>`).
* **C. Introduce a small domain enum** (e.g. `Found` / `NotFound` / `Unauthorized`) so `ureq::Error` never leaves the function.

## Decision Outcome

Chosen option: **A — narrow `#[allow]` with a comment**, because the lint's premise (the large `Result` is copied often enough to matter) does not hold for a CLI that makes a few network calls and exits, and keeping the flat status-code `match` is the most legible way to express the resolve logic. The suppression is function-scoped and documented, so the reasoning travels with the code.

### Consequences

* Good, because the caller keeps a single flat `match` on `Status(404)` / `Status(401)` / other — the clearest form of the logic.
* Good, because the `#[allow]` is scoped to one function and carries a comment, so a reviewer learns *why* rather than finding a silent crate-wide suppression.
* Neutral, because the ~272-byte result footprint remains; at this call volume it is immeasurable.
* Bad, because it suppresses a real lint — acceptable only because the suppression is narrow, justified, and revisitable (see Confirmation).

### Confirmation

`cargo clippy -- -D warnings` runs in CI; the only suppression is the single documented `#[allow]` on `head_manifest`. If registry checks ever move into a hot path (e.g. batch scanning many images per invocation), revisit this ADR and prefer option B or C.

## Pros and Cons of the Options

### A. Narrow `#[allow]` with a comment

* Good, because it preserves the flat, readable status-code `match`.
* Good, because scope is one function; a comment records the rationale.
* Neutral, because the large-result footprint stays (irrelevant at this frequency).
* Bad, because it is still a suppressed lint.

### B. Box the error

* Good, because it satisfies clippy with no suppression.
* Bad, because callers can no longer match `Err(ureq::Error::Status(404, _))` directly — they must `Err(e) => match *e { … }`, turning one flat match into a nested one.
* Bad, because it adds a heap allocation on the error path for no real-world benefit here.

### C. Custom domain enum

* Good, because `ureq::Error` never leaks; the return type states exactly the outcomes we care about.
* Neutral, because it is the most "designed" option.
* Bad, because it is more code and indirection than a three-call tool warrants right now.

## More Information

* Clippy lint: <https://rust-lang.github.io/rust-clippy/master/index.html#result_large_err>
* Revisit if the registry client is ever exercised in a hot path; option C would also be the natural home for richer registry-auth behavior (e.g. credentialed pulls of private images).
