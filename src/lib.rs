pub mod chunk;
pub mod compiler;
pub mod disassembler;
pub mod scanner;
pub mod token;
pub mod value;
pub mod vm;

use std::fs;

pub fn interpret(path: &str) -> vm::InterpretResult {
    match fs::read_to_string(path) {
        Ok(source) => {
            let mut compiler = compiler::Compiler::new(&source);

            if let Some(chunk) = compiler.compile() {
                let mut vm = vm::VM::new(chunk);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_code() {
        let path = "./tests/expressions/evaluate.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }
}
