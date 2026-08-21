use std::path::Path;

use colla_conformance::{corpus_dir, fixture_files, run_fixture};

/// Runs every fixture in `corpus/v1` against the Rust reference implementation.
#[test]
fn corpus_v1_conforms() {
    let version_dir = corpus_dir().join("v1");
    let files = fixture_files(&version_dir);
    assert!(
        !files.is_empty(),
        "no fixtures found under {}",
        version_dir.display()
    );

    let mut failures = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        let fixture: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{}: invalid JSON: {e}", path.display()));

        let id = fixture
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("{}: missing `id`", path.display()));
        assert_eq!(
            id,
            expected_id(&version_dir, path),
            "{}: `id` must equal the path relative to corpus/v1 without extension",
            path.display()
        );
        assert_eq!(
            fixture.get("corpusVersion").and_then(|v| v.as_u64()),
            Some(1),
            "{id}: `corpusVersion` must be 1"
        );

        if let Err(reason) = run_fixture(id, &fixture) {
            failures.push(format!("{id}: {reason}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} conformance failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn expected_id(version_dir: &Path, path: &Path) -> String {
    path.strip_prefix(version_dir)
        .unwrap()
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
}
