//! Naming an unsupported-but-recognisable image container from its leading
//! bytes.
//!
//! Deliberately tiny: this exists to turn a confusing decoder error into an
//! actionable one, not to be a format registry. Every format this project
//! supports decodes, so anything reaching here has already failed — the only
//! question is whether the message can say something useful.
//!
//! Byte-slice input rather than a path, because this crate compiles to
//! `wasm32-unknown-unknown` where there is no filesystem. The native
//! path-taking wrapper lives in `synthpass_ocr::decode_image`; the browser
//! reads the same bytes from a `Blob` and gets the same answer from the same
//! table.

/// How many leading bytes [`unsupported_format`] needs. Reading more is
/// harmless; reading fewer degrades gracefully (a short slice simply matches
/// nothing).
pub const SNIFF_BYTES: usize = 12;

/// Names an unsupported container from `head`, the leading bytes of a file, or
/// `None` when the bytes are not one this can identify.
///
/// Returning `None` for unrecognised bytes is the point: an invented
/// diagnosis is worse than none, because it sends the reader after the wrong
/// problem. There is a test pinning that.
///
/// ```
/// use synthpass_imageprep::sniff::unsupported_format;
///
/// assert_eq!(unsupported_format(b"%PDF-1.7\n%\xe2\xe3"), Some("a PDF"));
/// assert_eq!(unsupported_format(b"\0\0\0\x18ftypheic"), Some("HEIC/HEIF"));
/// assert_eq!(unsupported_format(b"\xff\xd8\xff\xe0JFIF"), None); // a JPEG decodes fine
/// ```
pub fn unsupported_format(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(b"%PDF-") {
        return Some("a PDF");
    }
    // ISO-BMFF: bytes 4..8 are `ftyp`, and the brand that follows says which
    // flavour. HEIC/HEIF is the one worth naming — it is the iPhone default,
    // and the browser's `createImageBitmap` rejects it in most engines.
    if head.len() >= SNIFF_BYTES && &head[4..8] == b"ftyp" {
        return match &head[8..12] {
            b"heic" | b"heix" | b"hevc" | b"heim" | b"heis" | b"mif1" | b"msf1" => {
                Some("HEIC/HEIF")
            }
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_a_pdf() {
        assert_eq!(unsupported_format(b"%PDF-1.4 whatever"), Some("a PDF"));
    }

    #[test]
    fn names_every_heif_brand_we_claim() {
        for brand in [
            &b"heic"[..],
            b"heix",
            b"hevc",
            b"heim",
            b"heis",
            b"mif1",
            b"msf1",
        ] {
            let mut head = vec![0u8, 0, 0, 0x18];
            head.extend_from_slice(b"ftyp");
            head.extend_from_slice(brand);
            assert_eq!(
                unsupported_format(&head),
                Some("HEIC/HEIF"),
                "brand {:?} should be named",
                std::str::from_utf8(brand),
            );
        }
    }

    #[test]
    fn an_isobmff_that_is_not_heif_gets_no_name() {
        // An MP4 is ISO-BMFF too. It is not HEIC, and saying so would send the
        // reader after the wrong problem.
        let mut head = vec![0u8, 0, 0, 0x18];
        head.extend_from_slice(b"ftypisom");
        assert_eq!(unsupported_format(&head), None);
    }

    #[test]
    fn unrecognised_bytes_get_no_invented_diagnosis() {
        assert_eq!(unsupported_format(b"not any known header"), None);
        assert_eq!(unsupported_format(&[0xde, 0xad, 0xbe, 0xef]), None);
    }

    #[test]
    fn a_short_slice_does_not_panic() {
        for n in 0..SNIFF_BYTES {
            assert_eq!(unsupported_format(&vec![0u8; n]), None);
        }
        // `%PDF-` is only 5 bytes, so it must still match on a short read.
        assert_eq!(unsupported_format(b"%PDF-"), Some("a PDF"));
    }
}
