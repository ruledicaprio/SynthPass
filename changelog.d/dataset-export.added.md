- **`synthpass export` — turn a synthetic corpus into a training dataset.** A new
  `crates/synthpass-export` crate and CLI subcommand generate a deterministic `synthpass-gen`
  corpus and write it as JSONL or a Hugging Face `datasets`-loadable directory, in the
  DeepSeek-OCR / Unlimited-OCR convention fixed by
  `knowledge/decisions/ADR-0007-dataset-export-format.md`: field boxes normalised to integer
  0–1000 of the image, each block's box and content concatenated into an end-to-end
  `ground_truth` string, `<page>` separating packed documents (`--pack-pages N`). Rows carry
  the 100%-accurate-by-construction labels for all five MRZ formats plus the rendered PNGs and
  a `manifest.json` that records the exact command to reproduce the export. `--format hf` also
  writes `dataset_infos.json` and a dataset card — always to local disk, never a Hub push.
  v1 is `--profile clean` only; COCO / YOLO are deferred (`knowledge/EXPORTS.md`).
- **New license feature `export`.** `synthpass export` is gated on it — bulk dataset production
  is a "higher-capacity generation" surface per `knowledge/BRANDING.md` §5, the same boundary
  `batch` sits behind; a single `synthpass generate` stays free. `export` is in the Pro and
  Enterprise tier presets. `SYNTHPASS_LICENSE_SKIP=1` bypasses it for local development.
