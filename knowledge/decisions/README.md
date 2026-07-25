# Architecture Decision Records

Document *why*, not *what*. The code shows what was built; six months from now the
question is always "why didn't we just do the obvious thing?" — and the answer
should already be written down.

## When to write one

When a decision is (a) hard to reverse, (b) contested or non-obvious, or (c) rules
out an approach a reasonable person would try. Not for routine choices.

A rejected alternative recorded with its reason is worth more than the decision
itself. That is the part nobody remembers.

## Format

Numbered `ADR-NNNN-kebab-title.md`, with:

```
Title, Status (Proposed | Accepted | Superseded by ADR-NNNN), Date
Context      — what forced the decision, with evidence
Decision     — what we chose, stated plainly
Alternatives — what we rejected and why (the load-bearing section)
Consequences — positive and negative, honestly
```

Never delete an ADR. Supersede it and link both ways: the wrong turns are part of
the record.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](ADR-0001-knowledge-tree.md) | Adopt `knowledge/` as the documentation root | Accepted |
| [0002](ADR-0002-provider-model-before-layout-plugins.md) | Build the provider model (M7) ahead of M6 | Accepted |
