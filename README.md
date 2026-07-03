# sync-branch-deps (`sbd`)

**Resolve cross-repo feature-branch dependencies to the pre-release artifacts a sibling repo published for that same branch.**

When a feature spans several repositories, `sbd` lets you build a consumer repo against the *in-flight* branch builds of its siblings — without hand-editing version pins — while a CI gate guarantees none of those branch pins can ever reach `main`.

This is a small, single-binary tool and a **reference implementation of a branch-dependency CI model**. The code is intentionally compact and readable; the model is the interesting part.

---

## The problem

You maintain several repos — a shared library, a couple of services, some container images. A feature touches three of them at once. You want repo B's CI to build against repo A's *branch* work, but:

- you don't want to publish messy pre-releases into your release channels,
- you don't want a half-finished branch pin to accidentally merge to `main`,
- and you don't want a bespoke script in every repo.

## The model

The convention is: **when a feature spans repos, use the same branch name in each.** Then three pieces cooperate:

1. **Publish on branch.** Each repo's CI, on a non-`main` branch, publishes a *branch-tagged pre-release* to a registry — an npm dist-tag named after the branch, and/or a container image tagged with the branch slug. These are registry artifacts only; **no GitHub Release, no git tag**.
2. **Resolve locally (`sbd`).** In a consumer repo on the same branch, you run `sbd`. It reads `.sync-branch-deps.yaml`, checks each declared sibling for a matching branch artifact, and rewrites your `package.json` dep and/or compose image tag to point at it. Missing match → skipped. It never runs in CI — CI just tests whatever is committed.
3. **Gate on merge.** A PR gate rejects any branch/pre-release sibling reference before it can merge to `main`. You revert to released versions; the gate enforces it.

`sbd` is piece #2. (Pieces #1 and #3 live in your CI.)

## Branch → slug

A branch name becomes a registry-safe **slug** by replacing every non-alphanumeric character with `-`:

```
feat/new-types   →  feat-new-types
release/1.2      →  release-1-2
```

This must match how your publish step names artifacts. `sbd` is a *resolver*; it assumes your CI already published under the same slug.

## `.sync-branch-deps.yaml`

Drop this at the consumer repo root. It declares which siblings this repo consumes — nothing is auto-discovered.

```yaml
# npm packages resolved via a branch dist-tag (rewritten in package.json)
npm:
  - "@your-org/shared-lib"

# container image prefixes resolved via a branch tag (rewritten in compose files)
images:
  - ghcr.io/your-org/service
  - quay.io/your-org/other-service
```

A repo that lists only `images:` never needs npm present. Image prefixes work against **any OCI-compliant registry** — ghcr.io, Docker Hub, Quay, GitLab, a private registry — via the standard Distribution auth flow.

## Usage

Run it from a consumer repo's root:

```console
$ sbd
sbd: branch=feat/new-types sanitized=feat-new-types
sbd:   @your-org/shared-lib: pinned to dist-tag 'feat-new-types' (resolved 0.4.0-feat-new-types.7)
sbd:   ghcr.io/your-org/service: no branch tag 'feat-new-types' — skipping
```

- On `main` (or a detached `HEAD`, or with no config) it is a **no-op**.
- A registry *miss* (no artifact for this branch) is skipped quietly.
- A registry *lookup failure* (network/auth error) is a hard error — a miss and a failure are different things.
- If `package.json` changed, it runs `npm install`.

Environment: `CURRENT_BRANCH` overrides branch detection (CI passes it when git isn't available); `DEFAULT_BRANCH` overrides `main`.

## Install

Download the binary for your platform from the [latest release](https://github.com/swiftaspect/sync-branch-deps/releases) and put it on your `PATH` as `sbd`. (A `cargo install` path may follow.)

## Development

Container-first — the only local dependency is a container engine (podman or docker):

```console
$ make check     # fmt check + clippy + tests, all inside a pinned Rust image
$ make build     # release binary at target/release/sbd
$ make help       # list targets
```

Architectural decisions are recorded under [`decisions/`](decisions/) (MADR format).

## License

[Apache-2.0](LICENSE).
