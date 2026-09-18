# M6 checksum-failed miss mechanisms

**Date:** 2026-09-18 · **MAIN:** `d2d5855` · **DATA:** `5988c7bd878e58cf0279bbe078b0e6a679ac46af` · **Evidence:** Observed (native runner and committed visual-zone survey) · **Status:** current

This note closes the six previously unattributed frozen M6 `checksum_failed` documents under ADR-0011's attribution criterion. The verification used the default `mrz` provider with `--real-specimens --mrz-only --dump-ocr` on the native corpus: 257 documents were visited and 10 rows were `checksum_failed`; all six frozen targets were present in the dump. The recorded fixtures are checksum-valid, so each is a recognition miss rather than an expected refusal.

The descriptions below use only field-level and layout evidence. They do not reproduce personal names or MRZ contents.

## Afghanistan passport specimen

**Observed:** The dump has a three-line TD3 read and fails `document_number` plus the composite check. The document-number error is a single `O`/`0` confusion at the first character; the surrounding line is otherwise aligned. The visual survey records 7 noise lines out of 39 (0.1944).

**Mechanism:** OCR-B glyph confusion at the document-number boundary, amplified by the specimen's background/noise. This is a localized recognition error, not a missing-MRZ or line-selection failure.

## Belgium identity-card specimen

**Observed:** The dump finds three TD1 lines and fails only the composite check. The band-first path recovers the expected `UTO` token, while the default path documented in the order-band benchmark shows the same `O`/`0` confusable in that token; the stored line also has a filler-run/alignment difference. The visual survey records 4 noise lines out of 13 (0.4). The browser/native note independently reports that the nine reviewed fields agree even though the stored `mrz_line` differs.

**Mechanism:** The miss is driven by OCR-B `O`/`0` ambiguity and filler-run alignment at the line boundary. It is a representation/checksum failure after field recovery, rather than evidence that the MRZ zone was absent.

## Czech identity-card specimen

**Observed:** The dump finds three TD1 lines and fails `personal_number` plus the composite check. One filler position is emitted as `Z`; the personal-number tail is shifted and contains a digit substitution, with the filler run no longer aligned. The visual survey records 6 noise lines out of 42 (0.1765).

**Mechanism:** A mixed glyph/filler recognition error (`<` versus a letter-like glyph, followed by digit/filler alignment drift) propagates into the personal-number and composite checks. The dump does not support a stronger claim about which print feature caused the initial substitution.

## Romania passport specimen

**Observed:** The dump finds three TD3 lines and fails `personal_number` plus the composite check. The name/filler portion of line 1 collapses separators and filler into a misaligned run; the final composite area also contains a filler-versus-digit error. The visual survey records 19 noise lines out of 59 (0.3585).

**Mechanism:** Recognition is degraded across the name/filler and check-digit regions by a noisy, crowded zone, producing alignment drift. The dump does not determine whether the initiating defect is print, security texture, or perspective.

## Russian Federation passport specimen

**Observed:** The dump returns a malformed three-line TD1 split and fails `document_number`, `date_of_birth`, and the composite check. The recovered lines have no trustworthy field-level correspondence to the fixture. The visual survey records 21 noise lines out of 47 (0.4468), and records zero MRZ lines for this image.

**Mechanism:** This is a zone segmentation/read failure with no reliable character alignment. The evidence does not determine whether the initiating cause is print quality, security background, perspective, or another image condition; no narrower attribution is justified.

## Sweden identity-card specimen

**Observed:** The dump finds three TD1 lines and fails `date_of_expiry` plus the composite check. Line 1 is aligned, while line 2 has an expiry-date substitution/shift and line 3 is replaced by unrelated body-text material. The visual survey contains separate Sweden specimens with substantial non-MRZ noise; it does not identify the exact image-level cause for this frozen target.

**Mechanism:** The expiry error is a local OCR/date alignment failure, compounded by a line-selection error that admits non-MRZ body text as line 3. The available dump and survey do not determine whether print, security texture, or perspective initiated the confusion.

## Closure and follow-up context

These six entries satisfy ADR-0011's criterion because each records the observed failing fields, the narrowest mechanism supported by the run, and an explicit uncertainty boundary where the dump cannot identify an image-level cause. They are all native `checksum_failed` misses in the same 257-document run; no data, manifest, or sample files were changed.

The unmerged `codex/bench-identity-followups` branch remains preserved for its separate discrepancy: relative to `origin/main`, it still differs only in `crates/synthpass-bench/src/lib.rs` and `crates/synthpass-bench/src/provider_bench.rs` (16 deleted lines), with no changelog fragment. This note does not resolve or modify that branch.

