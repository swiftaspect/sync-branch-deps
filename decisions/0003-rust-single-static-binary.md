---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# Implement sbd in Rust, distributed as a single static binary

## Context and Problem Statement

sbd must work across multiple package/artifact ecosystems and be runnable anywhere — ideally with **no language runtime or toolchain** installed on the host that runs it. What language should it be written in, and how should it be distributed?

## Decision Drivers

* One self-contained artifact: no interpreter/runtime dependency on the host.
* Cross-platform, small, easy to distribute (download-and-run plus a tiny container).
* Enough type safety and testing rigor for the manifest parsing and rewriting.

## Considered Options

* **An interpreted / runtime language.** Requires that runtime present on every host that runs the tool.
* **Go** — a single static binary.
* **Rust** — a single static binary.

## Decision Outcome

Chosen option: **Rust**, distributed as a static (musl) binary plus a container image.

A runtime language would force its interpreter onto every consumer just to resolve dependencies — directly against the "runnable anywhere" goal. Go and Rust both yield a single static binary that depends on nothing; **Rust** was chosen for team fit and for a type-system/test story that suits the manifest parsing and rewriting. The logic is compact, so either compiled option would have worked — this is a deliberate, low-regret choice.

### Consequences

* Good, because the tool is a single self-contained artifact — nothing needs to be installed to run it.
* Good, because the strong type system and inline unit tests keep the registry/rewrite logic honest.
* Neutral, because the logic is compact regardless of language.
* Bad, because a Rust + musl + multi-arch container build is more involved than a scripting-language image (mitigated: see [0006](0006-distroless-multi-arch-image.md)).

### Confirmation

`cargo` builds the `sbd` binary; the container ships it on a minimal base; CI runs `make check` (fmt + clippy + tests).
