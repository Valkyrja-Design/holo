## Holo

Holo is a small dynamically-typed and interpreted language inspired from [Lox](https://craftinginterpreters.com) and written in Rust

## TODO

- [ ] Maybe impl `Deref` for `Closure` to `Function`
- [ ] Fix indices in `Closure` instruction
- [ ] Beware of GC
- [ ] the first slot of locals reserved for?
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
