---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# Container image tags: rolling major / major.minor, immutable patch, never `latest`

## Context and Problem Statement

sbd is distributed as a container image that **users pull directly**. That makes consumer-convenience tags (`latest`, `1`, `1.2`) meaningful — a user wants to pin to a track and pick up updates without editing a digest. Which tags should we publish?

## Decision Drivers

* Convenience: let a user pin to a track (`:1`, `:1.2`) and pick up patches without editing a pin.
* Reproducibility: an exact, immutable tag must always exist.
* Footgun avoidance: `latest` silently changes across *major* versions.

## Considered Options

* **A. Immutable `X.Y.Z` only** — users always pin an exact version.
* **B. Immutable `X.Y.Z` + rolling `X` and `X.Y`, no `latest`.**
* **C. B + `latest`.**

## Decision Outcome

Chosen option: **B**. Publish the immutable `X.Y.Z` (plus `X.Y.Z-<buildstamp>` primary and `X.Y.Z-<gitref>`) **and** rolling `X` (major) and `X.Y` (major.minor) tags. **Never publish `latest`.**

`latest` is the classic footgun: it's implicit, unpinned, and rolls across *major* versions, so a `docker run …:latest` silently adopts breaking changes. An explicit `:1` is strictly better — it gives the same "just track updates" convenience while bounding the blast radius to a compatible major. Users who want reproducibility use `:X.Y.Z`.

### Consequences

* Good, because users get both a track-following pin (`:1`/`:1.2`) and a reproducible pin (`:1.2.3`).
* Good, because omitting `latest` removes the most common "it changed under me across a major" surprise.
* Neutral, because rolling tags are mutable by design — acceptable since the immutable tag is always available alongside.

### Confirmation

`Makefile`'s `CONTAINER_ALT_TAGS` emits `X.Y.Z`, `X.Y`, `X`, and `X.Y.Z-<gitref>`; `publish-container` pushes exactly those; no target ever produces `latest`.

## More Information

By convention git tags carry a `v` prefix (`v1.2.3`) while image tags do not (`1.2.3`); the version is read from `Cargo.toml`.
