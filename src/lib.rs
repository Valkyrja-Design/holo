pub mod chunk;
pub mod compiler;
pub mod disassembler;
pub mod gc;
pub mod intern_table;
pub mod object;
pub mod scanner;
pub mod token;
pub mod value;
pub mod vm;

use std::fs;

pub fn interpret(path: &str) -> vm::InterpretResult {
    match fs::read_to_string(path) {
        Ok(source) => {
            let mut gc = gc::GC::new();
            let mut str_intern_table = intern_table::StringInternTable::new();
            let compiler = compiler::Compiler::new(&source, &mut gc, &mut str_intern_table);

            if let Some(chunk) = compiler.compile() {
                let mut vm = vm::VM::new(chunk, &mut gc, str_intern_table);
                vm.run()
            } else {
                vm::InterpretResult::CompileError
            }
        }
        Err(err) => {
            eprintln!("{err}");
            vm::InterpretResult::CompileError
        }
    }
    // vm::InterpretResult::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        let path = "./tests/expressions/arithmetic.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn ternary() {
        let path = "./tests/expressions/ternary.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn ternary_error() {
        let path = "./tests/expressions/ternary_error.holo";

        assert_eq!(interpret(path), vm::InterpretResult::RuntimeError);
    }

    #[test]
    fn logical() {
        let path = "./tests/expressions/logical.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn string() {
        let path = "./tests/expressions/string.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn string_concate() {
        let path = "./tests/expressions/string_concate.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn string_concate_error() {
        let path = "./tests/expressions/string_concate_error.holo";

        assert_eq!(interpret(path), vm::InterpretResult::RuntimeError);
    }

    #[test]
    fn string_interning() {
        let path = "./tests/expressions/string_interning.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn print_err() {
        let path = "./tests/print/print_err.holo";

        assert_eq!(interpret(path), vm::InterpretResult::CompileError);
    }
}
