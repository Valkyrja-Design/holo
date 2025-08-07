# Runs the benchmarks using `hyperfine`

# Build the project in release mode
cargo build --release

if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to build the project" -ForegroundColor Red
    exit 1
}

$iterations = 10  # Number of iterations for the benchmark
$testcases = Get-ChildItem -Path "tests/test_files/benchmark/*.holo"

# Runs all the tests in the `tests/test_files/benchmark` directory
foreach ($testcase in $testcases) {
    if (Test-Path $testcase.FullName) {
        # Run the benchmark using the specified testcase and iterations
        Write-Host "Running benchmark for: $($testcase.Name)" -ForegroundColor Cyan
        hyperfine --warmup 2 --runs $iterations "./target/release/holo $($testcase.FullName)"
    }
}
