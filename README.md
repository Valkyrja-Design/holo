# Holo

Holo is a small dynamically-typed, interpreted language inspired by
[Lox](https://craftinginterpreters.com) and written in Rust. It compiles source
to bytecode and executes it on a stack-based virtual machine with a
mark-and-sweep garbage collector.

## Features

- Dynamic typing with numbers, booleans, strings, and `nil`
- First-class functions and closures
- Classes with methods, single inheritance
- Control flow: `if`/`else`, `while`, `for`, `break`, and `continue`
- A handful of native functions (e.g. `clock`)

## Building

Holo builds with a stable Rust toolchain via Cargo:

```sh
cargo build --release
```

## Running

Pass a Holo source file to the interpreter:

```sh
cargo run -- path/to/program.holo
```

Or, after building, run the binary directly:

```sh
./target/release/holo path/to/program.holo
```

## Examples

Example programs live under [`tests/test_files`](tests/test_files), grouped by
language feature. A small taste:

```
fun fib(n) {
  if (n < 2) return n;
  return fib(n - 2) + fib(n - 1);
}

print fib(10);
```

## Testing

```sh
cargo test
```

Benchmark programs are smoke-tested but ignored by default; run them with:

```sh
cargo test --test benchmark -- --ignored
```
