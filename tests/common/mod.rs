use holo::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn interpret<T: Write, U: Write>(path: PathBuf, output_stream: &mut T, err_stream: &mut U) {
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
                    let native_func_ptr = gc.alloc(object::Object::NativeFunc(native_func.clone()));
                    sym_table.declare(&native_func.name);
                    globals.push(Some(value::Value::Object(native_func_ptr)));
                }
                
                let compiler = compiler::Compiler::new(
                    &source,
                    "<main>",
                    &mut gc,
                    &mut str_intern_table,
                    &mut sym_table,
                    err_stream,
                );
                let compiled_function = compiler.compile();
                let global_var_names = sym_table.names_as_owned();
                
                // we need to push `None` for each global variable that is not a native function
                for _ in  &global_var_names[native_funcs.len()..] {
                    globals.push(None);
                }

                (global_var_names, compiled_function)
            };

            if let Some(function) = compiled_function {
                let main_func_ptr = gc.alloc(object::Object::Func(function));
                let mut vm = vm::VM::new(
                    main_func_ptr,
                    gc,
                    str_intern_table,
                    global_var_names,
                    globals,
                    output_stream,
                    err_stream,
                );
                let _res = vm.run();
            }
        }
        Err(err) => {
            let _ = writeln!(err_stream, "{}", err);
        }
    }
}
