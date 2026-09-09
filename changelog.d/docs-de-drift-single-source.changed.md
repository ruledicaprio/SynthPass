- **`README.md` now states the measured Tier-1 hit rate, and CI keeps it that way.** The
  Accuracy section advertised `~42–46%` on real specimens against a CI-measured **119/229 =
  52.0%**, and named `checksum_failed` (66) as the largest miss when `no_mrz_found` (85) had
  overtaken it 3.5:1 — a reader was handed both a stale number and the wrong bottleneck. The
  cause was structural: the same figures were restated in four documents with nothing keeping
  them in step.

  `knowledge/benchmarks/README.md` gains a **Current headline numbers** section and is now the
  single source for every measured figure; every other document states at most one and links
  there. New `scripts/check-headline-numbers.sh` reads the CI-written
  `real-specimen-mrz-baseline.json` and fails the build if `README.md` disagrees — including if
  it names the wrong dominant miss, which is the specific way this went wrong. Wired into
  `ci.yml` beside `check-doc-links.sh`.

- **`ADR-0008` succeeds the closed sequence-completeness track with MRZ detection.** M6's stated
  lead item finished, and closing it inverted the miss distribution: `mrz` 0.7.0 stopped
  accepting structurally implausible readings, reclassifying ~38 phantom `checksum_failed` into
  the `no_mrz_found` they had always been, with no Tier-1 HIT movement. Detection, not parsing,
  is now the bottleneck at 85 of 229 scored documents. The ADR mandates a *measurement* as its
  first chunk — explain why the browser demo's tesseract.js out-reads the native `ocrs`/`rten`
  pipeline (64.2% vs 59.5% on the same corpus) before touching a detector. `ROADMAP.md`'s M6
  table row, section body, suggested order and Measured-accuracy section all follow.

- **Docs de-drifted against v1.4.0.** `SECURITY.md`'s supported-version table (1.3.x → 1.4.x);
  `ARCHITECTURE.md`'s title no longer reads as a v1.2.0 document while citing 2026-09
  measurements; stale `v1.3.0` citations in `ROADMAP.md`, `hardware/README.md` and
  `vision/README.md`; `BRANDING.md`'s repository rename recorded as done rather than intended;
  `EXPORTS.md`'s status corrected from "pending implementation" to JSONL/HF shipped; the corpus
  badge 57 → 58 countries; `VIZ_TIER2_DESIGN.md`'s "no code has been written" banner, false
  since its holdout mode shipped; and `V2-DESIGN.md`'s "v2.0.0" disambiguated as the schema
  generation rather than a workspace release that is not planned.

- **`DOCUMENT_INTELLIGENCE_ENGINE.md` archived; `MRZ_SEQUENCE_COMPLETENESS.md` deliberately not.**
  The first is superseded design conversation whose duplicate was already archived and whose
  argument M7 shipped. The second is *finished*, not superseded — seventeen source files across
  `crates/mrz`, `crates/synthpass-bench` and `crates/synthpass-die` cite it by path as the
  rationale of record, so it stays in `knowledge/` with its status and its now-historical
  numbers marked as such. `knowledge/README.md` also indexes four root documents it had been
  silently omitting.

- **`technical_debt.md` records three debts the code carries and the docs did not.** The
  licensing public key is still the documented placeholder, so no real license can be issued
  against a shipped binary; the machine fingerprint is real only on Linux and binds nothing on
  Windows; and `synthpass-ocr`'s 18 `unsafe` blocks — the workspace's largest concentration —
  sit on the untrusted-image ingest path the fuzz targets exist to defend.
