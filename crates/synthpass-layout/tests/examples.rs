//! Every committed example layout loads (ADR-0022 Decision 8), covers all five
//! formats, and differs from its format's built-in.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use synthpass_layout::{builtin, identity, load_path};

#[test]
fn every_example_loads_and_every_format_has_one() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{dir:?}: {e}"))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no example layouts in {dir:?}");

    let mut formats = BTreeSet::new();
    for path in &files {
        let loaded = load_path(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let format = loaded.layout().format();
        assert_ne!(
            identity(loaded.layout()),
            identity(builtin(format).layout()),
            "{}: an example must differ from its built-in",
            path.display()
        );
        formats.insert(format.as_str());
    }
    assert_eq!(
        formats.into_iter().collect::<Vec<_>>(),
        ["MRVA", "MRVB", "TD1", "TD2", "TD3"],
        "every format needs at least one example"
    );
}
