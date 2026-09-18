# Phase D provider-gap measurement setup (2026-09-17)

**Date:** 2026-09-17 · **MAIN:** not stated · **DATA:** not stated · **Evidence:** unlabelled (pre-standard) · **Status:** superseded by [`phase-d-native-vs-browser-2026-09-18.md`](phase-d-native-vs-browser-2026-09-18.md)

**Amended 2026-09-18 — the measurement ran.** `web-ocr.yml` run **35169813105** (dispatched
2026-09-17T01:14:59Z with `reference_tag=v1.5.0`, MAIN `b2a0afd`, DATA `469a4ee`) completed
successfully and produced all three artifacts. The status line immediately below was accurate
when this file was written and is kept, unedited, as the record of that moment; the result is in
[`phase-d-native-vs-browser-2026-09-18.md`](phase-d-native-vs-browser-2026-09-18.md).

Status (as of 2026-09-17, superseded): **Prepared; comparison measurement not run.**

This branch prepares a same-population native/browser measurement for ADR-0008. The
workflow resolves the samples_data_sha recorded in the committed real-specimen
baseline, verifies that pinned object, runs the identity auditor, and records MAIN,
DATA, and baseline provenance in the native report before the browser sweep.

Each browser result joins to the native result by the samples-relative asset_id.
The native report records the selected `pass-NN` retry identifier only when a pass
validates, the retry stop reason, and whether the 52-second retry budget was reached.
It also carries the ADR-0013 name fields on this branch. The report provenance records
the resolved pass, seconds, order, texture, skew, and rotate configuration because
pass numbers depend on those settings.

No comparison numbers are reported here. The first authoritative run must use a
MAIN revision based on the v1.5.0 tag and must cite that tag, the exact DATA commit,
the baseline hash, and the provider configuration. It must localize native/browser
differences by asset before any OCR change is proposed.

The protected benchmark summary documents and historical baselines are unchanged
by this setup.
