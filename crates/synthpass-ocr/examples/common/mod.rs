//! Shared helpers for this crate's example binaries — dev tooling, not part
//! of the public library API.
//!
//! Included via `#[path = "common/mod.rs"] mod common;` in each example that
//! needs it, rather than published as an example of its own: Cargo's example
//! auto-discovery picks up `examples/*.rs` and `examples/*/main.rs`, and a
//! bare `examples/common/mod.rs` matches neither.
//!
//! `mine_country_vocab.rs` and `visual_zone_survey.rs` each carried their own
//! copy of a workspace-root lookup and a recursive image-file walker, and the
//! two walkers' extension lists had drifted apart — five entries
//! (`jpg`/`jpeg`/`png`/`webp`/`gif`) here, eight
//! (`+ bmp`/`tif`/`tiff`) in `synthpass-bench`'s own copy for the same job. A
//! new image format landing in the corpus would silently keep scanning fewer
//! formats in one tool than the others with no error. One copy here closes
//! that for this crate's own examples.

use std::path::{Path, PathBuf};

/// Matches `synthpass_bench::find_image_files`'s own list — kept in sync by
/// hand, since these are example/dev-tool extension lists in two different
/// crates, neither of which exports one for the other to import.
const IMAGE_EXTENSIONS: [&str; 8] = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"];

/// The workspace root: two levels up from this crate's own manifest
/// directory (`crates/synthpass-ocr` -> `crates` -> the workspace root).
///
/// Returns an error rather than panicking so a caller running this from an
/// unexpected working directory or crate layout gets a clean diagnostic
/// instead of a raw panic and backtrace.
pub fn repo_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            "could not find the workspace root two levels above this crate's manifest \
             directory — run this example from within the repository"
                .to_string()
        })
}

/// Recursively collect every image file under `dir`, matching
/// [`IMAGE_EXTENSIONS`] case-insensitively and skipping anything that is not
/// a regular file (a directory whose name happens to end in an image
/// extension, a symlink loop's non-file entries, etc.).
pub fn walk_images(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_images(&path, out);
        } else if path.is_file()
            && path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
        {
            out.push(path);
        }
    }
}
