mod common;

use std::env;
use std::fs;
use std::path::PathBuf;

// Benchmarks have non-deterministic timing output, so we only smoke-test that
// each one runs to completion without errors. Ignored by default because they
// are slow; run with `cargo test --test benchmark -- --ignored`.
#[test]
#[ignore]
fn benchmarks_run_without_errors() {
    let _ = env_logger::builder().is_test(true).try_init();

    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_files")
        .join("benchmark");

    for entry in fs::read_dir(&base_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            continue;
        }

        println!("Running benchmark: {}", path.display());

        let mut output_stream: Vec<u8> = Vec::new();
        let mut err_stream: Vec<u8> = Vec::new();

        common::interpret(path.clone(), &mut output_stream, &mut err_stream);

        let errors = String::from_utf8(err_stream).unwrap();
        let output = String::from_utf8(output_stream).unwrap();

        assert!(
            errors.trim().is_empty(),
            "Benchmark `{}` produced errors:\n{}",
            path.display(),
            errors,
        );
        assert!(
            !output.trim().is_empty(),
            "Benchmark `{}` produced no output",
            path.display(),
        );
    }
}
