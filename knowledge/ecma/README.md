# ecma/

The Ecma International OCR standards that stand behind the MRZ's typeface, converted to Markdown.
Doc 9303 Part 3 §4 requires OCR-B from **ISO 1073-2** and print quality from **ISO 1831**. Both are
ISO documents that cannot be reproduced here. ECMA-11 and ECMA-15 are their Ecma twins, and
ECMA-18, ECMA-21 and ECMA-30 complete the family.
[`../docs9303/`](../docs9303/README.md) is the MRZ *spec*. This tree is the *typeface and print
spec* underneath it, and [`../ocrb/`](../ocrb/README.md) is where its facts are distilled, cited,
and tied to SynthPass code and measurements.

## Contents

| File | Standard | Relevance to SynthPass |
|---|---|---|
| `ECMA-11_OCR-B_Alphanumeric_Character_Set_1976.md` | ECMA-11, 3rd ed. (1976): the OCR-B alphanumeric character set | The nominal shapes and sizes of every OCR-B character: centrelines, stroke width, sizes I/III/IV, and the letterpress vs constant-strokewidth styles. It is the Ecma twin of ISO 1073-2 and the ground truth for the glyph geometry `synthpass-ocr`'s chargrid and the generator's font assume. See [`../ocrb/glyph-dimensions.md`](../ocrb/glyph-dimensions.md). |
| `ECMA-15_Printing_Specifications_for_OCR_1968.md` | ECMA-15 (1968): printing specifications for OCR | Print quality: reflectance, spectral bands, stroke-width tolerances, voids and spots, line separation. It is the Ecma counterpart of ISO 1831, which Doc 9303 cites for the B900 band and range X. See [`../ocrb/print-quality-iso1831.md`](../ocrb/print-quality-iso1831.md). |
| `ECMA-18_Printing_Line_Position_on_OCR_Single-Line_Documents_1977.md` | ECMA-18, 2nd ed. (1977) | Where a printed OCR line may sit on a single-line document: the reference edges and tolerances. It is background for MRZ line-position priors. |
| `ECMA-21_Character_Positioning_on_OCR_Journal_Tape_1969.md` | ECMA-21 (1969) | Character positioning (pitch, alignment, skew) on journal tape. It is the positioning rules in their simplest setting, and background for the chargrid's pitch and alignment priors. |
| `ECMA-30_OCR-B_Sub-Sets_for_Numeric_Applications_1976.md` | ECMA-30, 2nd ed. (1976) | The OCR-B numeric sub-sets. It explains the `OCRBIII`-style subset fonts, and the closed alphabets a numeric field can be checked against. |

The rest of the family is not reproduced here:

- **ISO 1073-2, ISO 1831 and ISO 2033** are © ISO and are not reproduced anywhere in this
  repository. Facts from them appear only as cited paraphrase in [`../ocrb/`](../ocrb/README.md).
- **The code-set standards** (ECMA-6, ECMA-35, ECMA-43, ECMA-48) are not transcribed here.
  They matter for the character repertoire, not for glyph geometry; see
  [`../ocrb/code-positions.md`](../ocrb/code-positions.md).

## How these were made

- The originals are scans with no usable text layer. Each page was rendered and read as an
  image, and transcribed to Markdown following the standard's own clause numbering.
- `<!-- page N -->` marks each PDF page.
- Text is copied **as printed**: the source's own typos are kept, and an uncertain character is
  marked `[?]`. Every table or figure that deserves a second look carries a `<!-- VERIFY -->`
  comment. Each file ends with its *Transcription notes*.
- Figures (glyph drawings, dimension diagrams) are **not reproduced**. Each is replaced by a
  pointer to its page in the original, and its caption and any legible dimension values are kept
  as text.
- When a transcription and the original disagree, the original wins. Correct the file and record
  the correction in its notes.

## Provenance

© Ecma International. Ecma publishes its standards free of charge at
<https://ecma-international.org/publications-and-standards/standards/>, and these editions say so
themselves ("available free of charge"). The transcriptions are kept here, as Doc 9303's are in
[`../docs9303/`](../docs9303/README.md), so that SynthPass's geometry and print-quality
assumptions can be checked against the text they come from.
