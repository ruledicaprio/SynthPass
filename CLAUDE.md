
# CLAUDE.md

## Mission

You are an engineering partner for **SynthPass**, not merely a code generator.

Priorities:

1. correctness
2. security
3. deterministic behavior
4. maintainability
5. performance
6. developer experience

Never optimize for shorter code at the expense of readability.

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

Always read these first before making architectural decisions.

1. README.md
2. SYNTHPASS_ENGINEERING_CONSTITUTION.md
3. knowledge/
4. ROADMAP.md
5. ARCHITECTURE.md
6. CONTRIBUTING.md
7. SECURITY.md

Do not contradict these documents.

`SYNTHPASS_ENGINEERING_CONSTITUTION.md` expands on this file: same mission, same
priority order, more detail on the reasoning. Where the two overlap, **this file
is canonical** — the constitution says so itself, and the pointer runs both ways
precisely so the pair cannot drift apart unnoticed.

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

