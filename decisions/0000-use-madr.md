---
status: accepted
date: 2026-07-03
decision-makers: [Team]
---

# Use MADR Format for Decision Records

## Context and Problem Statement

We need a standard way to document architectural and technical decisions in this repository, so future contributors (and external readers — this repo is public) can understand why a choice was made and revisit it when context changes.

## Decision Drivers

* Need for a consistent documentation format
* Desire for a lightweight, Markdown-based approach
* Integration with the existing Git-based workflow
* A widely-adopted, tool-agnostic format that needs no dedicated tooling

## Considered Options

* MADR (Markdown Any Decision Records)
* Y-Statements
* Custom format
* No formal decision records

## Decision Outcome

Chosen option: "MADR", because it is a well-structured, widely-adopted, Markdown-based template with all the sections needed for comprehensive decision documentation.

### Consequences

* Good, because decisions are consistently documented and version-controlled
* Good, because new and external contributors can understand historical context
* Neutral, because it requires discipline to maintain
* Bad, because it adds some overhead to the decision-making process

### Confirmation

Significant technical decisions are documented as numbered ADRs in `decisions/`. PRs introducing notable changes should include a corresponding decision record.

## More Information

* [MADR GitHub Repository](https://github.com/adr/madr)
* [ADR GitHub Organization](https://adr.github.io/)
