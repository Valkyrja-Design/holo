#! /usr/bin/bash

# Runs the benchmarks using `hyperfine`

# Build the project in release mode
cargo build --release

if [ $? -ne 0 ]; then
    echo "Failed to build the project"
    exit 1
fi

iterations=10  # Number of iterations for the benchmark
testcases=tests/test_files/benchmark/*.holo

# Runs all the tests in the `tests/test_files/benchmark` directory
for testcase in $testcases; do
    if [ -f $testcase ]; then
        # Run the benchmark using the specified testcase and iterations
        hyperfine --warmup 2 --runs $iterations "./target/release/holo $testcase"
    fi
done
