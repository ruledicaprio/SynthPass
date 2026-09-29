- **`synthpass-bench` records a hash of each rendered document.** Every `results[]` entry carries
  `render_sha256`, the SHA-256 of the image the run feeds OCR (its width and height, then its RGBA8
  pixels). A seed renders the same pixels until the generator or a degrade profile changes, so a
  re-render shows up as a changed hash; the nightly benchmark uses it as its generator fingerprint.
  Report-only: no measured value, gate or other report field changes.
