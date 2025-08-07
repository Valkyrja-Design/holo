mod common;

use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
fn clock() {
    // base directory containing the test inputs and expected outputs
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_files")
        .join("native");
    let path = base_dir.join("clock.holo");

    let mut output_stream: Vec<u8> = Vec::new();
    let mut err_stream: Vec<u8> = Vec::new();

    // run the interpreter
    common::interpret(path.clone(), &mut output_stream, &mut err_stream);

    let errors = String::from_utf8(err_stream).unwrap();
    let output = String::from_utf8(output_stream).unwrap();

    // there should be no errors
    assert_eq!(errors.trim(), "");

    // and output should not be empty
    assert_ne!(output.trim(), "");
}

#[test]
fn clock_error() {
    // base directory containing the test inputs and expected outputs
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_files")
        .join("native");
    let expected_dir = base_dir.join("expected");
    let path = base_dir.join("clock_error.holo");

    let mut output_stream: Vec<u8> = Vec::new();
    let mut err_stream: Vec<u8> = Vec::new();

    // run the interpreter
    common::interpret(path.clone(), &mut output_stream, &mut err_stream);

    let errors = String::from_utf8(err_stream).unwrap();
    let output = String::from_utf8(output_stream).unwrap();

    // there should be no output
    assert_eq!(output.trim(), "");

    // load the expected output
    let test_name = path.file_name().unwrap().to_string_lossy().to_string();
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
