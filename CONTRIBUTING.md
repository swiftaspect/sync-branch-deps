# Contributing

Thanks for your interest in sync-branch-deps! This is a small, focused tool — contributions and issues are welcome.

## Development

The build is **container-first**: every `cargo` invocation runs inside a pinned Rust container, so the only tool you need locally is a container engine (podman or docker).

```console
$ make check     # format check + clippy + tests (the CI gate)
$ make fmt        # format the code
$ make test       # tests only
$ make build      # release binary at target/release/sbd
$ make help       # list all targets
```

Prefer a native toolchain? `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` work directly if you have Rust installed.

## Tests

- **Unit tests** live inline in each module (`#[cfg(test)] mod tests`), next to the code they cover — this is the idiomatic Rust layout and lets them exercise private helpers.
- **Integration tests** live in `tests/` and drive the public API only.

Every change should keep `make check` green, and new behavior should come with tests.

## Architecture

The code is organized so each axis of the tool extends by adding **one file**:

- `resolvers/` — "does a branch artifact exist for this coordinate?", one file per artifact kind (npm, oci, …).
- `rewriters/` — "pin the reference wherever it lives", one file per file kind (package.json, compose, …).
- `reporters/` — output formats (plain, github, json, quiet, …).

Adding an ecosystem, a reference location, or an output format is a new file in the matching directory plus one line in that module's dispatch.

## Commits & pull requests

- Commit messages follow **[Conventional Commits](https://www.conventionalcommits.org/)** (`feat:`, `fix:`, `docs:`, `chore:`, …). Releases and the changelog are generated from them, so the prefix matters.
- Open PRs against `main`. CI runs `make check`; keep it green.

## Decision records

Notable technical decisions are recorded as [MADR](https://adr.github.io/madr/) files under [`decisions/`](decisions/). If a change makes a non-obvious architectural choice, add one (copy `decisions/template.md`).

## License

By contributing you agree that your contributions are licensed under the project's [Apache-2.0](LICENSE) license.
