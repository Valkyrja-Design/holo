# Holo

Holo is a small dynamically-typed, interpreted language inspired by
[Lox](https://craftinginterpreters.com) and written in Rust. It compiles source
to bytecode and executes it on a stack-based virtual machine with a
mark-and-sweep garbage collector.

## A taste of Holo

### Functions and recursion

```
fun fib(n) {
  if (n < 2) return n;
  return fib(n - 2) + fib(n - 1);
}

for (var i = 0; i < 10; i = i + 1) print fib(i);
```

### Closures

Functions are first-class and capture their surrounding variables:

```
fun makeCounter() {
  var count = 0;
  fun next() {
    count = count + 1;
    return count;
  }
  return next;
}

var counter = makeCounter();
print counter(); // 1
print counter(); // 2
```

### Classes and inheritance

```
class Animal {
  init(name) { this.name = name; }
  speak() { print this.name + " makes a sound"; }
}

class Dog : Animal {
  speak() { print this.name + " barks"; }
}

Dog("Rex").speak(); // Rex barks
```

### String interpolation

Embed any expression in a string literal with `{ }`:

```
var name = "Rex";
print "Hello, {name}! 1 + 2 = {1 + 2}"; // Hello, Rex! 1 + 2 = 3
```

## Helpful error messages

When something goes wrong at compile time, Holo points at the exact span with a
Rust-style diagnostic instead of a bare line number:

```
error: expected expression
 --> line 1:10
  |
1 | print 1 +;
  |          ^
```

```
error: a class cannot inherit from itself
 --> line 1:13
  |
1 | class Foo : Foo {}
  |             ^^^
```

Runtime errors carry a call-stack trace:

```
Runtime error: Incorrect number of arguments: expected 2, got 1
[line 5] in <main>
```

## How it works

Holo runs in a single pass from source to bytecode, then executes that bytecode
on a virtual machine.

1. The **scanner** turns source text into tokens, tracking line and column for
   diagnostics.
2. The **compiler** is a single-pass Pratt parser that consumes tokens and emits
   bytecode directly into a chunk.
3. The **VM** is a stack-based interpreter that executes the bytecode, with call
   frames for functions and closures.
4. The **garbage collector** reclaims unused objects with a mark-and-sweep
   collector, triggered as the live object count grows.

Strings are interned so identical literals share one allocation, and globals are
resolved through a symbol table built during compilation.

## Features

- Dynamic typing with numbers, booleans, strings, and `nil`
- First-class functions and closures
- Classes with methods, single inheritance
- Control flow: `if`/`else`, `while`, `for`, `break`, and `continue`
- String interpolation with embedded expressions (`"sum: {a + b}"`)
- Rust-style compile diagnostics with line, column, and caret spans
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

More example programs live under
[`tests/test_files`](tests/test_files), grouped by language feature.

## Testing

```sh
cargo test
```

Benchmark programs are smoke-tested but ignored by default; run them with:

```sh
cargo test --test benchmark -- --ignored
```
