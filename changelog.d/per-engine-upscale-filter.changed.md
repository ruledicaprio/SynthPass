- **The band upscale filter is chosen per OCR engine now, because the two engines measurably want
  opposite answers.** `upscale_to_width` hard-coded `Lanczos3`. Testing the alternative on both
  engines — one variable, after confirming the harness is deterministic across two runs of the same
  build — found no single winner:

  | | `Lanczos3` | `Triangle` |
  | :-- | --: | --: |
  | browser (tesseract, OCR-B), 190 docs — checksum-valid | 125 | 125 |
  | browser — reviewed line-1 fields | 143/153 | **150/153** |
  | browser — median latency | 2 285 ms | **2 088 ms** |
  | native (`ocrs`), 20 docs | **18/20** | 17/20 |

  `Lanczos3` sharpens, which helps the detector *find* MRZ-shaped text on a blurred scan but rings
  on high-contrast edges — and MRZ glyph strokes are nothing but high-contrast edges — costing
  *character* accuracy. `Triangle` is the reverse. The clearest evidence it is a real effect rather
  than corpus noise: `Cyprus_Passport_Specimen_P0_CYP_2010` is the one document native *loses* under
  `Triangle` and simultaneously the one where the browser *recovers* line-1 fields under it.

  The browser takes `Triangle` despite the tied hit rate because **line 1 carries no check digit in
  any ICAO format** — for `document_type`, `issuing_country`, `surname` and `given_names` the
  recognizer is the only protection there is, and nothing downstream can catch a misread. Four of
  the recovered fields were wrong under `Lanczos3` *and* under the JavaScript port this replaced.

  `UpscaleFilter` deliberately has **no `Default`**: `synthpass-ocr` passes `Lanczos3`, `mrz-wasm`
  passes `Triangle`, and both name it at the call site. A silent default is how the two engines'
  needs were conflated to begin with. Full measurement in `knowledge/WEB_OCR_BASELINE.md`.
