- **The OCR-dump destination guard refuses `..` past the last existing directory on every
  operating system.** The OCR dump flags of `provider-bench` judge where the dump will land
  before writing document OCR anywhere. A `..` in the part of the output directory that does
  not exist yet was refused on Linux and let through on Windows, where `..` is collapsed
  lexically before the guard could see it, by the path API for a plain path and by
  `PathBuf::push` for a canonical one; the guard's test failed on every Windows checkout. The
  guard now classifies each component of the requested directory before joining it, so the
  answer no longer depends on the operating system; a `..` through directories that exist is
  still resolved by the filesystem. The Windows answer had matched where the file would have
  been written, so no dump landed anywhere the guard had not checked (#619).
