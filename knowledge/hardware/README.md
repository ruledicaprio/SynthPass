# hardware/

Memory budgets, CPU-only and consumer-GPU targets, deployment shapes.

## Why this folder exists

The deployment target is not a datacentre. It is a single static binary on a
machine that may have no network, modest RAM, and either no GPU or a consumer one
several generations old. Model choices that ignore that are not choices we can
ship.

## What belongs here

- **Memory budgets** — what a given provider costs resident, measured rather than
  declared. `Capability::estimated_resident_bytes` is the provider's own claim;
  this folder holds what we observed.
- **CPU-only performance** — the default assumption. The architecture is CPU-first.
- **Consumer GPU envelopes** — what actually fits and at what latency.
- **Deployment shapes** — the musl static build, container images, air-gapped
  install and verification.

## What this is not

A hardware-recommendation *feature*. The design notes float a
`synthpass benchmark` command that detects your GPU and star-rates models for it.
That is not implemented and is not in the v1.3.0 scope. If it is ever built, the
numbers behind it come from this folder and from `../benchmarks/`, measured — not
from a table someone typed.
