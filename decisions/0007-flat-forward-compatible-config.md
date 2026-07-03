---
status: superseded by ADR-0009
date: 2026-07-03
decision-makers: [Team]
---

# `.sync-branch-deps.yaml`: a flat, ecosystem-keyed map; unknown keys warn, not fail

> Superseded by [0009](0009-config-ignores-unusable-values.md): the flat map is
> kept, but non-list values are set aside and warned instead of hard-failing the
> parse — closing a forward-compat gap this decision left open.

## Context and Problem Statement

The config declares which coordinates sbd should resolve, across potentially many ecosystems. How should it be shaped — and what should happen when a config uses an ecosystem key a given binary doesn't know (e.g. a newer config read by an older binary)?

## Decision Drivers

* Extensibility: adding an ecosystem shouldn't require a schema change.
* Forward/backward compatibility: an older binary should tolerate a newer config.
* Explicitness: nothing is auto-discovered — a repo declares exactly what it consumes.

## Considered Options

* **A. Typed fields**, one per known ecosystem (`npm:`, `oci:`, …).
* **B. A flat map** from ecosystem key → list of coordinates, with unknown keys handled at dispatch time.

## Decision Outcome

Chosen option: **B**. The config is a flat map `{ <ecosystem>: [<coordinate>, …] }`. Parsing accepts any keys; the orchestrator looks up a resolver per key and, if none handles it, emits a **warning and skips** — rather than failing to parse.

A typed schema would reject any config that mentions an ecosystem a given binary version doesn't support, coupling the config format to the binary version. The flat map plus warn-on-unknown keeps configs forward-compatible: adding an ecosystem is a code change, not a schema break, and an older binary degrades gracefully instead of erroring.

### Consequences

* Good, because adding an ecosystem needs no config-schema change, and an older binary can still read a newer config.
* Good, because it's explicit — only declared coordinates are ever touched.
* Neutral, because a key's validity is decided at dispatch (a warning), not at parse time.

### Confirmation

`Config` deserializes to a map of key → list; `run` warns on any key with no matching resolver and continues.
