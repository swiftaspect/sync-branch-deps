---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# Output adapts to its environment; format is auto-detected and overridable

## Context and Problem Statement

sbd runs in three kinds of context: an interactive terminal, a plain pipe, and CI. Each wants different output — color versus none, human-readable lines versus machine-readable annotations. How should the tool decide which to emit?

## Decision Drivers

* Play nicely with local terminals and CI log systems alike.
* Emit machine-readable annotations where the runner understands them (e.g. GitHub Actions).
* No escape-code noise when output is piped or captured.

## Considered Options

* **A. One fixed format.**
* **B. Auto-detect the environment, with an explicit override.**

## Decision Outcome

Chosen option: **B**. A `Reporter` is selected automatically — GitHub Actions workflow commands (`::notice`/`::warning`/`::error`) when `GITHUB_ACTIONS=true`; otherwise plain text, colorized only on a TTY and only when `NO_COLOR` is unset. `--output` / `SBD_OUTPUT` forces a specific format. Supported formats: `plain`, `color`, `github`, `json`, and `quiet` (suppresses `info`/`notice`, surfacing only `warn`/`error`). All progress goes to **stderr** (the tool emits no data on stdout).

### Consequences

* Good, because output is readable locally, annotated in CI, and clean in pipes — with no configuration required.
* Good, because adding a format (GitLab, JSON, …) is a new `reporters/` file plus one match arm.
* Neutral, because each call site chooses a severity (info/notice/warn/error) so every format can render it.

### Confirmation

`reporters::select` maps `--output` / `SBD_OUTPUT` / auto-detection to a `Reporter`; each format is unit-tested through its pure `line()` method.

## More Information

Machine-readable output is a generic newline-delimited **`json`** stream, deliberately **not** a findings/test format (SARIF, JUnit, checkstyle). sbd reports progress and mutates files; it produces neither code findings with locations nor test results, so those formats would misrepresent its output. A JSON line stream is consumable by any CI (a Jenkins shell step, a wrapper script, a log pipeline) without that mismatch, and a purpose-built format can still be added later as its own `reporters/` file if a real need appears.

