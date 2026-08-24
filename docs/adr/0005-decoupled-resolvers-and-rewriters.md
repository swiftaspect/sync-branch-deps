---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# Decouple resolvers (by artifact kind) from rewriters (by file kind)

## Context and Problem Statement

Two concerns look similar but are independent: *does a branch artifact exist for this coordinate?* (a registry question, per artifact kind) and *where is that coordinate referenced, and how do we pin it?* (a file question, per file format). A container image, for instance, can be referenced in a Compose file, a Kubernetes manifest, a `Containerfile` `FROM`, a Helm values file… How should the code model this so it stays extensible as both axes grow?

## Decision Drivers

* Extensibility along *two* axes: more artifact kinds (npm, OCI, PyPI, crates, …) and more reference locations (Compose, Kubernetes, Dockerfile, …).
* Avoid coupling a file format to an artifact kind (e.g. baking "Compose" into "OCI").
* Adding support should be a small, local change.

## Considered Options

* **A. One `Ecosystem` trait** that both resolves and pins — each ecosystem owns a fixed rewrite target.
* **B. Two traits:** a `Resolver` (per artifact kind) and a `Rewriter` (per file kind), **matched by coordinate kind**.

## Decision Outcome

Chosen option: **B**. `Resolver`s answer existence per artifact kind; `Rewriter`s pin per file kind; the orchestrator hands a resolved coordinate to **every** rewriter registered for that kind.

With one combined trait, "images live in Compose" gets hard-coded into the OCI logic, and supporting Kubernetes manifests means editing the OCI ecosystem. Splitting the traits makes the two axes orthogonal: adding an artifact kind is one new `resolvers/` file; adding a reference location is one new `rewriters/` file; neither touches the other.

### Consequences

* Good, because Compose is just *one* `oci` rewriter — Kubernetes/Dockerfile/etc. drop in beside it without touching resolution.
* Good, because each axis extends independently with a single new file.
* Neutral, because it is slightly more indirection than a single trait — justified by the two independent growth axes.

### Confirmation

`resolvers::for_key` selects a resolver by config key; `rewriters::for_kind` gathers every rewriter for the resolver's coordinate kind; the orchestrator loops the two. No resolver references a file format; no rewriter performs a registry lookup.
