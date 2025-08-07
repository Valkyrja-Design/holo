mod common;

use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
fn field() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Base directory containing the test inputs and expected outputs
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_files")
        .join("field");
    let expected_dir = base_dir.join("expected");

    for entry in fs::read_dir(&base_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        // Skip the `expected` subdirectory
        if path.is_dir() {
            continue;
        }

        println!("Running test: {}", path.as_os_str().to_str().unwrap());

        let test_name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut output_stream: Vec<u8> = Vec::new();
        let mut err_stream: Vec<u8> = Vec::new();

        // Run the interpreter
        common::interpret(path.clone(), &mut output_stream, &mut err_stream);

        let errors = String::from_utf8(err_stream).unwrap();
        let output = String::from_utf8(output_stream).unwrap();

        // Load the expected output
        let expected_path = expected_dir.join(path.file_stem().unwrap());
        let expected = fs::read_to_string(&expected_path).unwrap_or_else(|e| {
            panic!(
                "Could not read expected output file for `{}`: {}",
                test_name, e
            )
        });

        let full_output = errors.trim_end().to_owned() + "\n" + &output;
        let normalized_output = full_output.trim().replace("\r\n", "\n");
        let normalized_expected = expected.trim().replace("\r\n", "\n");

        assert_eq!(
            normalized_output,
            normalized_expected,
            "Output mismatch for test `{}`",
            path.as_os_str().to_str().unwrap(),
        );
    }
}
