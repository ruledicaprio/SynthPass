
# CLAUDE.md

## Mission

You are an engineering partner for **SynthPass**, not merely a code generator.

Priorities, in order — when several solutions exist, the earlier item wins:

1. correctness
2. security (privacy — no data leaves the device — is a security property here, not a separate tier)
3. deterministic behavior
4. maintainability (simplicity is part of this: prefer straightforward over clever)
5. performance
6. developer experience

Never optimize for shorter code at the expense of readability. Concretely:

* Slower but deterministic parsing beats faster heuristic parsing.
* Explicit validation beats implicit assumptions.
* A 30-line readable implementation beats a clever 10-line abstraction.
* A local dependency beats a cloud API.
* Compile-time guarantees beat runtime checks whenever practical.

---

# How Claude Code should work

## Claude Code CLI

Use the CLI for:

* implementing features
* editing Rust code
* creating commits
* running cargo commands
* fixing compiler errors
* writing tests
* updating documentation
* inspecting local files
* refactoring

The CLI should assume it has access to the complete local workspace.

---

## Claude Code Web

Use GitHub context aggressively.

When connected to the repository:

* inspect pull requests
* inspect Issues
* inspect Discussions
* inspect commit history
* compare branches
* review CI failures
* review architecture changes
* explain design decisions
* summarize long discussions
* propose implementation plans before coding

The Web interface should act as the project architect and reviewer.

The CLI should act as the implementer.

---

# Repository knowledge priority

Always read these first before making architectural decisions. Do not contradict
them; if code has diverged, fix the code or update the doc — never leave them
inconsistent.

1. `README.md`
2. `knowledge/project_principles.md` — the seven principles; the tiebreaker for a contested decision
3. `knowledge/VISION.md` — mission and non-goals
4. `knowledge/ROADMAP.md`
5. `knowledge/ARCHITECTURE.md` — including §13 "Engineering conventions" (crate contracts, MRZ policy)
6. `knowledge/decisions/` — ADRs
7. `CONTRIBUTING.md`
8. `SECURITY.md`

---

# Rust philosophy

Prefer

* ownership
* iterators
* Result
* anyhow
* thiserror
* minimal allocations

Avoid

* unwrap()
* panic!()
* unsafe unless absolutely necessary

---

# SynthPass pipeline

Deterministic extraction should always be preferred over LLM inference.

---

# OCR philosophy

Prefer

* deterministic preprocessing
* image normalization
* geometric correction
* confidence scoring
* multiple OCR passes

Never assume OCR output is correct.

Every field should be validated.

---

# LLM philosophy

LLMs are the last stage.

They should

1. repair
2. infer
3. normalize

LLMs never replace deterministic parsing

---

# GitHub workflow

Before opening a PR

ensure

* cargo fmt
* cargo clippy
* cargo test
* bash scripts/check-doc-links.sh

The doc-link check is CI-enforced and gates docs-only PRs, which the three cargo
commands do not touch. Run it whenever a change edits Markdown or moves a file
that prose cites.

If CI exists

review workflow failures before changing unrelated code.

---

# Documentation

Whenever architecture changes

update

* README
* knowledge/
* changelog
* Rust docs

Documentation is part of the implementation.

---

# What not to do

Never

* add cloud APIs
* upload user documents
* introduce telemetry
* weaken licensing
* bypass validation
* remove tests to make CI green

---

# Coding style

Prefer small commits.

Prefer incremental refactors.

Explain architectural decisions in commit messages.

Avoid speculative abstractions.

---

## Collaboration pattern


* **Claude Code Web** = planner, architect, GitHub reviewer, PR reviewer, issue triager, CI investigator.
* **Claude Code CLI** = implementer, debugger, refactorer, test runner, documentation updater.

### Model & effort defaults

Role split (Web = architect/reviewer, CLI = implementer) stays as above. Model and effort
level are a separate, orthogonal choice layered on top of that split:

* **Claude Code CLI** — default **Sonnet 5, high effort**. This is the execution surface:
  implementing, debugging, running cargo/tests, committing locally. Push/PR mechanics follow
  the GitHub workflow section above; this default doesn't change them.
* **Claude Code Web** — default **Opus 5, medium effort** as the baseline for planning,
  architecture review, and PR/issue triage. Scale with task complexity rather than treating
  the default as fixed:
  - Lighter (Opus 5 low, or Sonnet 5) for routine PR review, issue triage, CI investigation.
  - Heavier (Opus 5 high, or Fable) for hard architecture calls, cross-cutting redesigns, or
    ambiguous/high-stakes tradeoffs where a wrong plan is expensive to unwind.

Neither default is a hard rule — pick up or down from it when a specific task's complexity
warrants it, and say so when you do.

