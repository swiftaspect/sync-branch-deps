---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# `.sync-branch-deps.yaml`: a flat, ecosystem-keyed map; anything it can't use is warned, not fatal

Supersedes [0007](0007-flat-forward-compatible-config.md).

## Context and Problem Statement

The config declares which coordinates sbd should resolve, across potentially many ecosystems. How should it be shaped, and what should happen when a config carries something a given binary doesn't understand — an ecosystem key it has no resolver for, or (from a newer schema) a value that isn't a list of coordinates? An older binary must tolerate a newer config instead of aborting. 0007 chose the flat map and warn-on-unknown-key, but still deserialized every value straight into a list, so any non-list value hard-failed — breaking the very compatibility it aimed for. This decision replaces it.

## Decision Drivers

* Extensibility: adding an ecosystem shouldn't require a schema change.
* Forward/backward compatibility: an older binary should tolerate a newer config for *any* addition, not just unknown keys.
* Explicitness: nothing is auto-discovered — a repo declares exactly what it consumes.
* Don't silently swallow a genuine mistake (e.g. `npm: mypackage` instead of a one-item list).

## Considered Options

* **A. Typed fields**, one per known ecosystem (`npm:`, `oci:`, …).
* **B. A flat map, values deserialized straight into lists** — unknown keys handled at dispatch, non-list values rejected at parse (the 0007 implementation).
* **C. A flat map that parses leniently** — keep list-valued entries, set aside every other key and warn on it; unknown *list-valued* keys are still warned-and-skipped at dispatch.

## Decision Outcome

Chosen option: **C**. The config is a flat map from ecosystem key to a list of coordinates. `parse` deserializes to `key → YAML value`: entries whose value is a list of strings are kept; every other key is collected into `Config::ignored`, which the orchestrator turns into a `warn` before dispatch. An empty, comment-only, or `null` document is an empty config, not an error. A list-valued key that no resolver handles is still warned-and-skipped at dispatch.

A typed schema (A) would reject any config mentioning an ecosystem a binary version doesn't support, coupling the format to the binary version. B kept the flat map but deserialized each value directly into `Vec<String>`, so a newer config that set an existing key to a scalar or added a scalar schema key such as `version: 2` still hard-failed an older binary. C keeps the run alive on anything an older binary can't act on and surfaces what it skipped.

### Consequences

* Good, because adding an ecosystem needs no config-schema change, and an older binary reads a newer config regardless of the added value's shape.
* Good, because it's explicit — only declared, list-valued coordinates are ever touched.
* Good, because a mistyped value is surfaced as a warning instead of silently dropped or fatally rejected.
* Neutral, because a key's validity is decided at parse/dispatch (a warning), not by a schema; the warning can't tell a forward-compat scalar from a typo, so it fires for both.

### Confirmation

`Config::parse` unit tests cover ecosystem-keyed lists, a scalar-valued known key, an added scalar schema key (both recorded in `ignored`), and empty/comment-only/`null` documents; `run` warns on every ignored key and on any list-valued key with no resolver, then continues. An integration test confirms a non-list value warns and touches nothing.
