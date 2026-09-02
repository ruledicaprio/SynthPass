- **Images are decoded by what their bytes are, not by what their filename claims.** `image::open`
  picks a decoder from the file extension, so a JPEG named `.png` fails with `Invalid PNG signature`
  even though the format is fully supported — and the error names the format the *name* implied, so
  it reads as a corrupt file rather than a mislabelled one. New `synthpass_ocr::decode_image` calls
  `with_guessed_format()` first, which is what the browser demo's `createImageBitmap` has always
  done.

  This was found the hard way: three specimens in this repo's own corpus are JPEGs carrying `.png`
  and `.webp` names, and all three were **silently skipped** by every OCR walk in the tree — the
  manifest generator, both surveys, `mrz_corpus` and `synthpass-bench` — until a magic-byte sweep
  found them. One of them is `Spain_Passport_Specimen_P0_2022_mrz`, the specimen
  `crates/mrz/tests/line1_nonconformance.rs` pins as having no issuing state, which had therefore
  never once been read by the engine that pins it.

  The mistake was repo-wide rather than engine-local, which is why the decoder is public and shared.
  Four call sites reached for `image::open` independently. The worst was
  `synthpass_bench::load_specimen`, which maps a decode failure to `None`: a mislabelled file
  quietly shrank the benchmark corpus with no message at all.

- **A PDF or HEIC file renamed to `.jpg` now says so.** `synthpass-pipeline` already rejects both by
  *extension* with an actionable message, but that check is exactly the one a wrong extension
  defeats. When decoding fails, the leading bytes are consulted (`%PDF-`, and ISO-BMFF `ftyp` brands
  for HEIC/HEIF — the iPhone default) so the message names the real format instead of reporting a
  decoder error about a format the file never was. Unrecognised bytes get no invented diagnosis.
