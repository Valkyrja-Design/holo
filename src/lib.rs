pub mod chunk;
pub mod compiler;
pub mod disassembler;
pub mod gc;
pub mod object;
pub mod scanner;
pub mod sym_table;
pub mod table;
pub mod token;
pub mod value;
pub mod vm;

use std::fs;

pub fn interpret(path: &str) -> vm::InterpretResult {
    match fs::read_to_string(path) {
        Ok(source) => {
            let mut gc = gc::GC::new();
            let mut str_intern_table = table::StringInternTable::new();
            let compiler = compiler::Compiler::new(&source, &mut gc, &mut str_intern_table);

            if let Some((chunk, sym_table)) = compiler.compile() {
                let mut vm = vm::VM::new(chunk, gc, str_intern_table, sym_table.names_as_owned());
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

    #[test]
    fn globals() {
        let path = "./tests/variable/globals.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn redeclare_global() {
        let path = "./tests/variable/redeclare_global.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn redefine_global() {
        let path = "./tests/variable/redefine_global.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn undefined_global() {
        let path = "./tests/variable/undefined_global.holo";

        assert_eq!(interpret(path), vm::InterpretResult::RuntimeError);
    }

    #[test]
    fn uninitialized() {
        let path = "./tests/variable/uninitialized.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn use_global_in_initializer() {
        let path = "./tests/variable/use_global_in_initializer.holo";

        assert_eq!(interpret(path), vm::InterpretResult::RuntimeError);
    }

    #[test]
    fn assignment_global() {
        let path = "./tests/assignment/global.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn assignment_associativity() {
        let path = "./tests/assignment/associativity.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn assignment_grouping() {
        let path = "./tests/assignment/grouping.holo";

        assert_eq!(interpret(path), vm::InterpretResult::CompileError);
    }

    #[test]
    fn assignment_infix_operator() {
        let path = "./tests/assignment/infix_operator.holo";

        assert_eq!(interpret(path), vm::InterpretResult::CompileError);
    }

    #[test]
    fn assignment_prefix_operator() {
        let path = "./tests/assignment/prefix_operator.holo";

        assert_eq!(interpret(path), vm::InterpretResult::CompileError);
    }

    #[test]
    fn assignment_syntax() {
        let path = "./tests/assignment/syntax.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn assignment_undefined() {
        let path = "./tests/assignment/undefined.holo";

        assert_eq!(interpret(path), vm::InterpretResult::RuntimeError);
    }

    #[test]
    fn assignment_local() {
        let path = "./tests/assignment/local.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn duplicate_local() {
        let path = "./tests/variable/duplicate_local.holo";

        assert_eq!(interpret(path), vm::InterpretResult::CompileError);
    }

    #[test]
    fn in_middle_of_block() {
        let path = "./tests/variable/in_middle_of_block.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn in_nested_block() {
        let path = "./tests/variable/in_nested_block.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn scope_reuse_in_different_blocks() {
        let path = "./tests/variable/scope_reuse_in_different_blocks.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn shadow_and_local() {
        let path = "./tests/variable/shadow_and_local.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn shadow_global() {
        let path = "./tests/variable/shadow_global.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn shadow_local() {
        let path = "./tests/variable/shadow_local.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }

    #[test]
    fn undefined_local() {
        let path = "./tests/variable/undefined_local.holo";

        assert_eq!(interpret(path), vm::InterpretResult::RuntimeError);
    }

    #[test]
    fn use_false_as_var() {
        let path = "./tests/variable/use_false_as_var.holo";

        assert_eq!(interpret(path), vm::InterpretResult::CompileError);
    }

    #[test]
    fn use_local_in_initializer() {
        let path = "./tests/variable/use_local_in_initializer.holo";

        assert_eq!(interpret(path), vm::InterpretResult::CompileError);
    }

    #[test]
    fn use_nil_as_var() {
        let path = "./tests/variable/use_nil_as_var.holo";

        assert_eq!(interpret(path), vm::InterpretResult::CompileError);
    }

    #[test]
    fn use_this_as_var() {
        let path = "./tests/variable/use_this_as_var.holo";

        assert_eq!(interpret(path), vm::InterpretResult::CompileError);
    }

    #[test]
    fn scope() {
        let path = "./tests/block/scope.holo";

        assert_eq!(interpret(path), vm::InterpretResult::Ok);
    }
}
