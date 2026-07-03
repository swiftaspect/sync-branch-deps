---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# Resolve via registry APIs, and only pin — never invoke a package manager

## Context and Problem Statement

To decide whether a branch artifact exists (an npm dist-tag, a container tag, …) and to update a manifest to point at it, sbd could either drive each ecosystem's package manager (`npm`, `pip`, `cargo`, …) or talk to the registries directly. And after rewriting a manifest, it could run the ecosystem's install/lock step — or leave that to the developer.

## Decision Drivers

* Self-containment: the binary should run without any package manager installed.
* Single responsibility: sbd is a *resolver/pinner*, not a build tool.
* Predictability: no surprising side effects (installs, lockfile churn, network beyond a lookup).

## Considered Options

* **Shell out to each package manager** for both the existence check and the install.
* **Query registries directly, then run the ecosystem's install** after a change.
* **Query registries directly, and only rewrite the manifest** — never install.

## Decision Outcome

Chosen option: **query registries directly over their HTTP APIs, and only rewrite manifests — sbd never invokes a package manager or runs an install.**

Shelling out would require every consumer to have each package manager present, defeating the "runnable anywhere" goal. Running installs would pull sbd into build-tool territory with side effects it shouldn't own. Resolution is a registry lookup; pinning is a text rewrite; the subsequent install/lock is the developer's or CI's normal step, unchanged.

### Consequences

* Good, because the binary stays portable — no npm/pip/etc. required to resolve and pin.
* Good, because the tool's job is small and predictable: look up, rewrite, done.
* Neutral, because each ecosystem carries a small direct registry client (e.g. reading `.npmrc` for registry + auth) instead of leaning on its package manager.
* Bad, because after sbd rewrites a manifest the developer must run their own install to refresh the lockfile — this is documented, and it matches how they'd normally proceed.

### Confirmation

No resolver or rewriter spawns a subprocess for a package manager; resolvers make HTTP requests only; rewriters edit files only.
