pub mod chunk;
pub mod compiler;
pub mod disassembler;
pub mod gc;
pub mod native;
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
            let mut globals: Vec<Option<value::Value>> = Vec::new();

            let (global_var_names, compiled_function) = {
                let mut sym_table = sym_table::SymbolTable::new();
                let native_funcs = native::get_native_funcs();

                // define native functions as global variables
                for native_func in &native_funcs {
                    sym_table.declare(&native_func.name);
                    globals.push(Some(gc.alloc_native(native_func.clone())));
                }

                let compiler = compiler::Compiler::new(
                    &source,
                    "<main>",
                    &mut gc,
                    &mut str_intern_table,
                    &mut sym_table,
                    &mut err_stream,
                );
                let compiled_function = compiler.compile();
                let global_var_names = sym_table.names_as_owned();

                // we need to push `None` for each global variable that is not a native function
                for _ in &global_var_names[native_funcs.len()..] {
                    globals.push(None);
                }

                (global_var_names, compiled_function)
            };

            if let Some(function) = compiled_function {
                let main_closure = gc.alloc_function_ptr(function);
                let main_closure = gc.alloc_closure_ptr(value::Closure::new(main_closure, 0));

                let mut vm = vm::VM::new(
                    main_closure,
                    gc,
                    str_intern_table,
                    global_var_names,
                    globals,
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
}

#[cfg(test)]
mod tests {
    use crate::interpret;
    use std::io::{stderr, stdout};
}
