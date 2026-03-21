use super::{process_dir, process_file, validate_dir, validate_file, Format};
use std::path::{Path, PathBuf};

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn check_or_bless(actual: &str, expected_path: &Path) {
    if std::env::var("WST_BLESS").is_ok() {
        std::fs::write(expected_path, actual)
            .unwrap_or_else(|e| panic!("Failed to bless {:?}: {}", expected_path, e));
        return;
    }
    let expected = std::fs::read_to_string(expected_path)
        .unwrap_or_else(|_| {
            panic!(
                "Expected file not found: {:?}\nRun with WST_BLESS=1 to generate it.",
                expected_path
            )
        });
    assert_eq!(actual, expected, "Fixture mismatch for {:?}", expected_path);
}

// ── Validation ───────────────────────────────────────────────────────────────

#[test]
fn validate_single_valid_file() {
    let input = examples_dir().join("single").join("input.wst");
    validate_file(&input).expect("valid file should parse without error");
}

#[test]
fn validate_single_invalid_file() {
    let tmp = tempfile::NamedTempFile::with_suffix(".wst").unwrap();
    std::fs::write(tmp.path(), "struct Broken { id = #required, };").unwrap();
    let err = validate_file(tmp.path()).unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[test]
fn validate_dir_all_valid() {
    let input = examples_dir().join("directory").join("input");
    validate_dir(&input).expect("all example directory files should be valid");
}

#[test]
fn validate_dir_with_invalid_file() {
    let tmp_dir = tempfile::tempdir().unwrap();
    std::fs::write(tmp_dir.path().join("ok.wst"), "enum Color { Red };").unwrap();
    std::fs::write(tmp_dir.path().join("bad.wst"), "struct { = int64 };").unwrap();
    let err = validate_dir(tmp_dir.path()).unwrap_err();
    assert!(err.to_string().contains("1 file(s) failed validation"));
}

#[test]
fn validate_dir_multiple_invalid() {
    let tmp_dir = tempfile::tempdir().unwrap();
    std::fs::write(tmp_dir.path().join("a.wst"), "struct { = int64 };").unwrap();
    std::fs::write(tmp_dir.path().join("b.wst"), "variant { };").unwrap();
    let err = validate_dir(tmp_dir.path()).unwrap_err();
    assert!(err.to_string().contains("2 file(s) failed validation"));
}

// ── Single-file fixtures ──────────────────────────────────────────────────────

#[test]
fn single_ts() {
    let dir = examples_dir().join("single");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    process_file(&dir.join("input.wst"), tmp.path(), &Format::Ts).unwrap();
    let actual = std::fs::read_to_string(tmp.path()).unwrap();
    check_or_bless(&actual, &dir.join("expected.ts"));
}

#[test]
fn single_rs() {
    let dir = examples_dir().join("single");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    process_file(&dir.join("input.wst"), tmp.path(), &Format::Rust).unwrap();
    let actual = std::fs::read_to_string(tmp.path()).unwrap();
    check_or_bless(&actual, &dir.join("expected.rs"));
}

#[test]
fn single_json() {
    let dir = examples_dir().join("single");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    process_file(&dir.join("input.wst"), tmp.path(), &Format::Json).unwrap();
    let actual = std::fs::read_to_string(tmp.path()).unwrap();
    check_or_bless(&actual, &dir.join("expected.json"));
}

// ── Directory fixtures ────────────────────────────────────────────────────────

fn run_dir_fixture(example: &str, format: &Format, expected_subdir: &str) {
    let base = examples_dir().join(example);
    let input = base.join("input");
    let expected_dir = base.join(expected_subdir);
    let tmp = tempfile::tempdir().unwrap();

    process_dir(&input, tmp.path(), format).unwrap();

    let mut entries: Vec<_> = std::fs::read_dir(&expected_dir)
        .unwrap_or_else(|_| panic!("Expected dir not found: {:?}", expected_dir))
        .map(|e| e.unwrap())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let filename = entry.file_name();
        let actual_path = tmp.path().join(&filename);
        let actual = std::fs::read_to_string(&actual_path)
            .unwrap_or_else(|_| panic!("Output file not generated: {:?}", actual_path));
        check_or_bless(&actual, &expected_dir.join(&filename));
    }
}

fn bless_dir_fixture(example: &str, format: &Format, expected_subdir: &str, ext: &str) {
    let base = examples_dir().join(example);
    let input = base.join("input");
    let expected_dir = base.join(expected_subdir);
    let tmp = tempfile::tempdir().unwrap();

    process_dir(&input, tmp.path(), format).unwrap();
    std::fs::create_dir_all(&expected_dir).unwrap();

    for entry in std::fs::read_dir(tmp.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|e| e.to_str()) == Some(ext) {
            let actual = std::fs::read_to_string(entry.path()).unwrap();
            let dest = expected_dir.join(entry.file_name());
            std::fs::write(&dest, &actual).unwrap();
        }
    }
}

#[test]
fn directory_ts() {
    if std::env::var("WST_BLESS").is_ok() {
        bless_dir_fixture("directory", &Format::Ts, "expected-ts", "ts");
    } else {
        run_dir_fixture("directory", &Format::Ts, "expected-ts");
    }
}

#[test]
fn directory_rs() {
    if std::env::var("WST_BLESS").is_ok() {
        bless_dir_fixture("directory", &Format::Rust, "expected-rs", "rs");
    } else {
        run_dir_fixture("directory", &Format::Rust, "expected-rs");
    }
}
