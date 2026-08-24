# Claude Code Instructions

Follow the conventions in [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for architecture, the container-first dev loop, and the commit/PR process.

## Task Completion Checklist

Before considering a task complete:

1. `make check` passes (format check + clippy with warnings denied + tests).
2. New behavior is covered by tests (unit tests inline; integration tests in `tests/`).
3. Commit messages follow Conventional Commits (releases + changelog are generated from them).
4. A non-obvious architectural choice is recorded as a MADR file under `docs/adr/`.

## Notes for Claude

- **This is a public repository.** Keep it self-contained: do not reference private repositories, internal org conventions, or the tool's history in code, docs, or decision records. Describe sbd on its own terms.
- Extension points are one-file-each: a new artifact kind is a `resolvers/` file, a new reference location is a `rewriters/` file, a new output format is a `reporters/` file — plus one line in that module's dispatch.
- The tool only **resolves and rewrites** — it never invokes a package manager or runs an install (see `docs/adr/0004`).

## Agent skills

Repo-specific configuration for the engineering skills:

- Issue tracker: read `docs/agents/issue-tracker.md`
- Domain docs and ADRs: read `docs/agents/domain.md`

When asking the user questions (grilling), keep each question short and in literal plain language. Prefer fewer, sharper questions over long lists.
