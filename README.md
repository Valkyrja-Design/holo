## Holo

Holo is a small dynamically-typed and interpreted language heavily inspired from [Lox](https://craftinginterpreters.com), written in Rust

## TODO

- [ ] refactor and document code
- [ ] handle multi line expressions like the following properly
```
print -
    true;

// should show error at line 1 instead of line 2
```
- [ ] impl Display trait for objects
- [ ] put limit on number of nested functions
- [ ] test with string hash for interning in the metadata itself
- [ ] fix error token in `emit_opcode_with*` functions
- [ ] add long jump instructions
- [ ] try more specialized instructions
- [ ] const vars
- [ ] multi-pass compilation
- [ ] string interpolation
- [ ] Better error handling/messages
- [ ] standard library
- [x] build a symbol table for globals
- [x] multi-line strings
