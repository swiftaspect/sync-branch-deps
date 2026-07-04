---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# OCI resolution authenticates from the standard credential sources

## Context and Problem Statement

The OCI resolver checks whether a branch tag exists with a manifest `HEAD`, following the registry's `WWW-Authenticate: Bearer` challenge for a token. The first implementation requested that token *anonymously* — a plain GET to the authorization realm with no credentials. That works for public images, but a branch-dependency workflow exists to coordinate a team's own repos, whose artifacts are almost always **private**. Against a private repository the anonymous token carries no `pull` scope, so the follow-up `HEAD` returns `401` and the resolver reports a hard lookup failure — for *every* image, on *every* branch, including a plain miss. In practice that made the OCI half of the tool unusable for its primary audience.

So the resolver has to present real credentials. The question is *where they come from* — without inventing a bespoke mechanism, and without coupling to any one container engine.

## Decision Drivers

* The private-registry case is the common case, not an edge case — it has to work by default.
* Stay engine-agnostic: no assumption of Docker, podman, or any specific runtime.
* Reuse what operators already have. "Logged in" should be enough locally; CI should have a standard knob.
* No new tool-specific environment variables to learn or document.
* The resolver only reads registries — credential handling must not grow into managing a package manager or a login flow (see [0004](0004-resolve-and-pin-only.md)).

## Considered Options

* **A. Anonymous only** (status quo) — cannot see private images.
* **B. A bespoke sbd env var** (e.g. `SBD_REGISTRY_TOKEN` [+ user]) — one knob, but invents a convention, has to base64-encode a Basic value itself, and is naturally single-registry.
* **C. The standard OCI credential sources** — the auth-config document (`{"auths":{"<host>":{"auth":"<base64 user:pass>"}}}`) that every registry-login tool reads and writes, discovered via the conventional env vars and file locations: `$REGISTRY_AUTH_FILE`, then inline `$DOCKER_AUTH_CONFIG`, then the default files (`$XDG_RUNTIME_DIR/containers/auth.json`, `$DOCKER_CONFIG/config.json`, `~/.config/containers/auth.json`, `~/.docker/config.json`).

## Decision Outcome

Chosen option: **C**. When following the Bearer challenge, the resolver looks up a per-host credential from the standard sources in that precedence order and, if found, sends it as HTTP Basic to the authorization realm so the issued token is scoped. With no credential configured it stays anonymous — public images still resolve, so the change is strictly additive.

The stored `auth` value is already base64(`user:pass`), i.e. exactly an HTTP Basic credential, so nothing is decoded or re-encoded — the blob is passed straight through as `Authorization: Basic <blob>`. This is why B's self-encoding is unnecessary work: the ecosystem already stores the value in the form we need.

These env vars and paths are cross-tool conventions (their names are historical, not owned by any single engine), which keeps the tool engine-agnostic while making "just log in" — or, in CI, "set `DOCKER_AUTH_CONFIG`" — sufficient. The credential logic lives in `resolvers/oci.rs` alongside the resolver it serves, mirroring how the npm resolver reads its own `.npmrc` auth in `resolvers/npm.rs`; each ecosystem owns its registry-auth story rather than sharing a misleadingly-named "registry auth" module.

### Consequences

* Good, because private images — the common case — resolve with no sbd-specific configuration: a prior registry login is enough locally, and `DOCKER_AUTH_CONFIG` is the standard CI path.
* Good, because it's engine-agnostic and adds no new convention to learn.
* Good, because reusing the base64 `auth` blob as the Basic value keeps the code small and dependency-free (no base64 codec).
* Neutral, because `DOCKER_AUTH_CONFIG` applies to whatever host is queried; a stored file is per-host, but the inline document is trusted as given.
* Bad (minor), because the Basic-then-token flow assumes a Bearer-challenge registry (the Distribution norm — ghcr, Docker Hub, Quay, GitLab). A registry that answers with a `Basic` challenge directly is not yet handled; it can be added if one shows up.
