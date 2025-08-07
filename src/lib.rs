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
use std::io::Write;

pub fn interpret<T, U>(path: &str, mut output_stream: T, mut err_stream: U)
where
    T: Write,
    U: Write,
{
    match fs::read_to_string(path) {
        Ok(source) => {
            let mut gc = gc::GC::new();
            let mut str_intern_table = table::StringInternTable::new();
            let (global_var_names, compiled_function) = {
                let mut sym_table = sym_table::SymbolTable::new();
                let compiler = compiler::Compiler::new(
                    &source,
                    "<main>",
                    &mut gc,
                    &mut str_intern_table,
                    &mut sym_table,
                    &mut err_stream,
                );
                let compiled_function = compiler.compile();

                (sym_table.names_as_owned(), compiled_function)
            };

            if let Some(function) = compiled_function {
                let main_func_ptr = gc.alloc(object::Object::Func(function));
                let mut vm = vm::VM::new(
                    main_func_ptr,
                    gc,
                    str_intern_table,
                    global_var_names,
                    &mut output_stream,
                    &mut err_stream,
                );
                let _res = vm.run();
            }
        }
        Err(err) => {
            let _ = writeln!(err_stream, "{err}");
        }
    }
    // vm::InterpretResult::Ok
}

#[cfg(test)]
mod tests {
    use std::io::{stderr, stdout};
}
