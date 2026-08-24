---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# Ship the static binary on distroless, as a multi-arch image

## Context and Problem Statement

sbd builds a single static binary ([0003](0003-rust-single-static-binary.md)). What runtime base should the container image use, and which architectures should it target?

## Decision Drivers

* Minimal image size and CVE/attack surface — a static binary needs no OS libraries.
* Native support for common architectures (amd64 and arm64 — Apple Silicon, ARM servers/CI).
* Reproducible, pinnable bases (dependabot-updatable).

## Considered Options

* **A full-OS base** (e.g. a minimal distro or vendor UBI image) — familiar, but carries a shell, package manager, and userland a static binary never uses.
* **A slim distro base** — smaller, still a full userland.
* **distroless/static** — CA certificates and a nonroot user, and nothing else (~a couple MB).

Architecture: a single native arch, versus a multi-arch manifest (amd64 + arm64).

## Decision Outcome

Chosen option: a **static (musl) binary on `gcr.io/distroless/static-debian12:nonroot`**, published as a **multi-arch (amd64 + arm64) manifest**. Base images are pinned by digest.

A static binary needs no shell, package manager, or shared libraries, so distroless/static is the smallest correct base — it still provides CA certificates (for the registry HTTPS calls) and a nonroot user. A multi-arch manifest means ARM users get a native image rather than emulation.

### Consequences

* Good, because the image is a couple MB with a minimal attack surface (no shell/package manager), and runs natively on amd64 and arm64.
* Good, because digest-pinned bases are reproducible and dependabot-updatable.
* Bad, because building both arches via emulation is slow; cross-compilation (e.g. `cargo-zigbuild`) is a future speed optimization, and the CI publish path needs a multi-arch-capable builder.

### Confirmation

The `Containerfile` selects the arch's musl target from `uname -m` and copies the binary onto distroless; `make build-container` builds a `--platform linux/amd64,linux/arm64` manifest; `publish-container` pushes the full manifest. Verified: the built manifest carries both `amd64` and `arm64` entries.
