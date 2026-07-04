---
status: accepted
date: 2026-07-04
decision-makers: [Team]
---

# Detect the branch in-container by reading `.git/HEAD`, git binary preferred

## Context and Problem Statement

`sbd sync` resolves artifacts for the *current branch*. It detects that branch, falling back to `$CURRENT_BRANCH` when git isn't available. The tool now runs primarily as a container (`sbd sync` in the distroless image, the consumer repo mounted at the working directory) — and distroless has no `git` binary, so the binary-based detection fails there. That pushed every consumer's `make sync-branch-deps` to compute the branch on the host and inject `-e CURRENT_BRANCH=$(git rev-parse …)`.

That injection is a smell: the branch is *already in the mounted repo* at `.git/HEAD`. Making each caller detect and pass it means the tool can't stand on its own — a wrapper has to feed it something it could read itself. Auth is genuinely external (host credentials, legitimately injected); the branch is not. How should the container learn its branch without every caller doing work to tell it?

## Decision Drivers

* The tool should be self-sufficient in a mounted repo — no required `CURRENT_BRANCH` plumbing in every consumer.
* Keep the image minimal (single static binary on distroless — [0003](0003-rust-single-static-binary.md), [0006](0006-distroless-multi-arch-image.md)).
* Long-term durability against git's evolving ref storage (the `reftable` backend).
* An explicit override must still win (CI, detached checkouts).

## Considered Options

* **A. Add `git` to the image.** Authoritative everywhere, including future ref backends. But distroless/static can't install packages, and `git` isn't a single static binary (it dynamically links libcurl/pcre2/zlib/openssl and shells out to helpers) — this means a glibc base plus ~10–15 MB of git and dependencies, taking the image from ~6 MB to ~25 MB and reintroducing a dynamic base, undoing 0003/0006. Large CVE surface, all to run one `rev-parse`.
* **B. Parse `.git/HEAD` directly, as a fallback.** ~15 lines, no image change, no dependency. Reads the classic "files" layout; a repo on the `reftable` backend leaves a `refs/heads/.invalid` sentinel there instead.
* **C. Keep injecting `CURRENT_BRANCH`** from each caller. No tool change, but the smell this decision exists to remove.

## Decision Outcome

Chosen option: **B**, ordered by authority: `$CURRENT_BRANCH` → the `git` binary → `.git/HEAD` → the default branch. The `git` binary stays the preferred detector wherever it's on `PATH` (every host) because it is authoritative and agnostic to the ref-storage backend; reading `.git/HEAD` is strictly the fallback for a minimal environment with no git — the container. A leading-dot branch (e.g. reftable's `.invalid` sentinel) is rejected — `git check-ref-format` forbids leading-dot refnames, so it can only be a sentinel — and detection falls through to the default branch, a clean no-op rather than a bogus pin.

This confines the one non-future-proof piece (text-parsing HEAD) to the single environment that forces it, and even there its worst case is a no-op, never a wrong rewrite. Adding `git` to the image (A) pays a large, permanent size/security cost to make that fallback authoritative for a backend that is opt-in today and unlikely in a mounted consumer checkout. The consumer targets drop their `-e CURRENT_BRANCH=…` plumbing entirely.

### Consequences

* Good, because `sbd sync` is self-sufficient in a mounted repo — no consumer needs to compute or inject the branch.
* Good, because the image stays a ~6 MB static distroless artifact; 0003/0006 hold.
* Good, because the authoritative git binary is still preferred wherever it exists, so host runs are backend-agnostic and future-proof.
* Neutral, because in-container detection assumes the "files" ref layout; a `reftable` checkout degrades to the default-branch no-op (revisit only if reftable-in-container becomes real).
* Neutral, because `.git` as a worktree/submodule pointer file (`gitdir: …`) is followed, but more exotic layouts fall through to the default.
