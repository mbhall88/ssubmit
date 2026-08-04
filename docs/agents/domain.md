# Domain documentation

How engineering skills should consume this repository’s domain documentation.

## Before exploring

Read:

- `CONTEXT.md` at the repository root, if present;
- relevant ADRs under `docs/adr/`, if present.

If these files do not exist, proceed silently. Do not propose creating them pre-emptively. Domain-modelling workflows create them when terminology or decisions are actually resolved.

## File structure

This is a single-context repository:

```text
/
├── CONTEXT.md
├── docs/
│   └── adr/
│       ├── 0001-example-decision.md
│       └── 0002-another-decision.md
└── src/
```

## Use the glossary vocabulary

When output names a domain concept—in an issue title, proposal, hypothesis or test—use the term defined in `CONTEXT.md`. Do not drift to synonyms that the glossary explicitly avoids.

If a required concept is absent, reconsider whether the term belongs to the project. If it represents a real gap, note it for the domain-modelling workflow.

## Flag ADR conflicts

If proposed work contradicts an existing ADR, surface the conflict explicitly instead of silently overriding the decision.
