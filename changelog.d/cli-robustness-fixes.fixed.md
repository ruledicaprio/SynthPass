- **CLI input handling hardened.** `synthpass`/`synthpass batch` now reject surplus positional
  arguments (an unquoted shell glob expanding to several files used to silently read only the
  first) instead of dropping the rest; a `synthpass batch` glob no longer submits non-image
  files it happens to match; a non-UTF-8 argument reports an error instead of panicking; and
  `synthpass batch <dir>` no longer follows a directory symlink into an infinite loop.
- **No more false "saved to" after a persist failure.** A JSON write failure now suppresses the
  "Pipeline completed... JSON saved to" line (for both the Tier-1 and Tier-2 extraction paths)
  and reports the failure on stderr instead of claiming a save that never happened.
- **`--seed`/`--count` overflow rejected up front.** `synthpass generate` now errors at argument
  parsing time instead of overflowing `u64` partway through a batch.
- **`synthpass generate --help`'s profile list** now always matches the profiles actually
  accepted (it had drifted and omitted `damaged`); the extraction banner also no longer claims
  to be "uploading" a local file this fully offline tool never sends anywhere.
