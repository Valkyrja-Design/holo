## Holo

Holo is a small dynamically-typed and interpreted language heavily inspired from [Lox](https://craftinginterpreters.com), written in Rust

## TODO

- [ ] Handle multi line expressions like the following properly

```
print -
    true;

// should show error at line 1 instead of line 2
```
- [ ] fix error token in `emit_opcode_with*` functions
- [x] build a symbol table for globals
- [ ] multi-line strings
- [ ] multi-pass compilation
- [ ] string interpolation
- [ ] Better error messages
- [ ] standard library
