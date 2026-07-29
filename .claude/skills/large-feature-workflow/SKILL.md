---
name: large-feature-workflow
description: Four-phase Claude Web/CLI workflow for planning and shipping a large SynthPass feature (Web plans, CLI implements, Web reviews the diff, CLI applies review comments). Use when starting work on a large or multi-step feature.
---

# Expected workflow

For large features:

Phase 1

Claude Web

* read Issues
* inspect recent commits
* inspect ROADMAP
* inspect ARCHITECTURE
* inspect knowledge/

Produce an implementation plan.

Phase 2

Claude CLI

Implement the approved plan.

Phase 3

Claude Web

Review the resulting Git diff.

Suggest improvements.

Phase 4

Claude CLI

Apply review comments.

Run tests.
