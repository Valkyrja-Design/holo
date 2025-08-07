use holo::{
    compiler::Compiler, gc::GC, object, sym_table::SymbolTable, table::StringInternTable, vm::VM,
};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn interpret<T: Write, U: Write>(path: PathBuf, output_stream: &mut T, err_stream: &mut U) {
    match fs::read_to_string(path) {
        Ok(source) => {
            let mut gc = GC::new();
            let mut str_intern_table = StringInternTable::new();
            let (global_var_names, compiled_function) = {
                let mut sym_table = SymbolTable::new();
                let compiler = Compiler::new(
                    &source,
                    "<main>",
                    &mut gc,
                    &mut str_intern_table,
                    &mut sym_table,
                    err_stream,
                );
                let compiled_function = compiler.compile();

                (sym_table.names_as_owned(), compiled_function)
            };

            if let Some(function) = compiled_function {
                let main_func_ptr = gc.alloc(object::Object::Func(function));
                let mut vm = VM::new(
                    main_func_ptr,
                    gc,
                    str_intern_table,
                    global_var_names,
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
