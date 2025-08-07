## Holo

Holo is a small dynamically-typed and interpreted language inspired from [Lox](https://craftinginterpreters.com) and written in Rust

## TODO

- [ ] store initializer in a separate field in the class object - might speed up execution
- [ ] optimize field accesses
- [ ] more efficient marking of GC pointers
- [ ] more robust allocation size tracking in GC
- [ ] remove unnecessary checks
- [ ] put all errors in an enum or something
- [ ] make new loop variable in every iteration (for closing over it)
- [ ] think about removing limit on locals and upvalues
- [ ] maybe impl `Deref` for `Closure` to `Function`
- [ ] fix indices in `Closure` instruction
- [ ] refactor and document code
- [ ] report max argument error at argument token and not `,`, see `method/too_many_arguments.holo`
- [ ] handle multi line expressions like the following properly
```
print -
    true;

// should show error at line 1 instead of line 2
```
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
- [x] impl Display trait for objects
- [x] the first slot of locals reserved for `this`
- [x] upvalues behavior with `continue`
- [x] build a symbol table for globals
- [x] multi-line strings