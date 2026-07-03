---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# CLI shape: explicit `sync` / `verify` subcommands; bare invocation is an error

## Context and Problem Statement

sbd pins branch-matched pre-release references into a repo's manifests. The inverse — asserting that *no* such branch reference remains — is the "PR gate" that keeps in-flight pins from reaching `main`. Both operations need the same per-ecosystem knowledge of what a branch reference looks like. Should that gate be a separate workflow, and how should the CLI expose these behaviors?

## Decision Drivers

* The gate is the *read-side* of pinning — the tool already knows the convention it writes.
* Portability: a gate that runs via `make`, any CI, a git hook, or by hand — not locked to one CI's action model.
* Language-agnostic: one gate across every ecosystem sbd supports, not a per-language grep.
* Explicit intent for a tool that edits files.

## Considered Options

* **A. Keep the gate as a per-language CI step** (a `make` grep target / workflow).
* **B. Gate in the tool as a subcommand; bare `sbd` defaults to sync.**
* **C. Gate in the tool as a subcommand; bare `sbd` errors with usage; behaviors are explicit.**

## Decision Outcome

Chosen option: **C**. sbd grows subcommands:

- `sbd sync` — resolve + pin (with `--dry-run` to preview without writing).
- `sbd verify` — the gate: scan the manifests and exit non-zero if any declared coordinate is pinned to a branch / pre-release reference.
- bare `sbd` — print usage and **exit non-zero**; there is no implicit default for a tool that edits files.

The gate belongs in the tool because it is the *same narrow domain* as pinning — branch-matched cross-repo references — just read instead of written. sbd already understands each ecosystem's reference shape, so this is language-agnostic in one place (replacing per-language grep gates) and maximally portable: runnable via `make`, any CI, a pre-commit hook, or by hand, with no CI lock-in.

The verb is **`verify`, not `check`**: "check" reads like a preview/dry-run, which is a genuinely different operation. Previewing what `sync` would do is `sync --dry-run`; `verify` asserts the *current* file state is clean.

### Consequences

* Good, because there is one portable, language-agnostic gate with no CI lock-in, and the tool owns the whole manifest half of the model (`sync` + `verify`).
* Good, because `--dry-run` gives a safe preview of pinning.
* Good, because bare-errors-with-usage prevents an accidental mutation.
* Neutral, because it introduces a small subcommand parser and dispatch.
* Bad, because sbd is no longer a single-action binary — kept in check by limiting the surface to two verbs plus `--dry-run`.

### Confirmation

`sbd` with no subcommand exits non-zero with usage; `sbd verify` exits non-zero when a branch pin is present and zero when clean; `sbd sync --dry-run` writes nothing. Covered by parser unit tests and `verify` integration tests.

## More Information

publish-on-branch and registry cleanup stay `make` / CI concerns (registry-side, per-repo) — not tool subcommands. sbd remains scoped to the manifest half of the model.
