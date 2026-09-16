# Phase D provider-gap measurement setup (2026-09-17)

Status: **Prepared; comparison measurement not run.**

This branch prepares a same-population native/browser measurement for ADR-0008. The
workflow resolves the samples_data_sha recorded in the committed real-specimen
baseline, verifies that pinned object, runs the identity auditor, and records MAIN,
DATA, and baseline provenance in the native report before the browser sweep.

Each browser result joins to the native result by the samples-relative asset_id.
The native report also records the stable retry variant identifier and whether the
52-second retry budget was reached. Optional ADR-0013 name fields are carried when
the name-accuracy change is present on the measured MAIN revision.

No comparison numbers are reported here. The first authoritative run must use a
MAIN revision based on the v1.5.0 tag and must cite that tag, the exact DATA commit,
the baseline hash, and the provider configuration. It must localize native/browser
differences by asset before any OCR change is proposed.

The protected benchmark summary documents and historical baselines are unchanged
by this setup.
