# Code positions: where the MRZ characters live in the code tables

Doc 9303 wants MRZ data passed on "in a standard protocol (e.g. ASCII)"
([Part 3 §4.2](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#42-properties-of-the-mrz)).
The code-set standards confirm that nothing below the ASCII level is at stake. The tags are
defined in [`README.md`](README.md#sources).

## ISO 2033 (1983) and ECMA-19 (1969)

- **ISO 2033** assigns bit patterns to the characters that MICR and OCR readers recognise, so
  that reader output can be interchanged. It is based on the 7-bit code of ISO 646 and on its
  8-bit extension under ISO 2022. It explicitly does **not** define the character set to be read
  (ISO 2033 §1-§2, p. 1 [R via text layer, prose only]).
- **Table 8** gives the OCR-B assignments (p. 8 [R]):
  - digits 0-9 → 3/0-3/9;
  - capitals A-Z → 4/1-5/10;
  - **LESS THAN SIGN (ref. 79) → 3/12**;
  - GREATER THAN → 3/14;
  - PLUS → 2/11.

  These are the ISO 646 / ASCII positions: `<` is 0x3C.
- **2nd-edition change.** Alternative OCR-B assignments at 2/3 and 2/4 were removed to align with
  ISO 646's International Reference Version (ISO 2033 annex, p. 11 [R via text layer]). The
  MRZ's 37 characters were never affected.
- **ECMA-19 (June 1969)** is the ECMA ancestor. It gives a 7-bit coding and, where appropriate, a
  4-bit coding for CMC7, OCR-A and OCR-B. For OCR-B it covers 10 numerals, 26 capitals and 26
  small letters plus symbols, and it leaves code-extension rules to other standards (ECMA-19 §1-§2,
  p. 3 [R]).

## ISO 646 / ECMA-6, and the code-structure standards

- ECMA-6 (6th ed., 1991) is identical to ISO 646. In every version of that code, digits, capitals
  and LESS-THAN SIGN have **unique, invariant allocations** (ECMA-6 §6.4.1 [T]). The national
  variant positions are 2/3, 2/4, 4/0, 5/11-5/14, 6/0 and 7/11-7/14 (§6.4.2-§6.4.3 [T]).
  None holds an MRZ character.
- ECMA-35 (= ISO/IEC 2022), ECMA-43 (= ISO 4873) and ECMA-48 (= ISO/IEC 6429) cover code
  extension, 8-bit structure and control functions. They matter here only as the lineage of the
  code positions above. **None of them says anything about glyph geometry or print.**

**Bears on SynthPass:** nothing. The MRZ alphabet (`crates/mrz`'s `MRZ_ALPHABET`, in
[`crates/mrz/src/repair.rs`](../../crates/mrz/src/repair.rs)) is plain ASCII in every relevant
standard. No national variant of ISO 646 remaps any of the 37. The code-set standards are
recorded for completeness, and they do not need transcribing for Tier-1 work.
