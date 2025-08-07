use holo::{compiler::Compiler, gc::GC, table::StringInternTable, vm::VM};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn interpret<T: Write, U: Write>(path: PathBuf, output_stream: &mut T, err_stream: &mut U) {
    match fs::read_to_string(path) {
        Ok(source) => {
            let mut gc = GC::new();
            let mut str_intern_table = StringInternTable::new();
            let compiler = Compiler::new(&source, &mut gc, &mut str_intern_table, err_stream);

            if let Some((chunk, sym_table)) = compiler.compile() {
                let mut vm = VM::new(
                    chunk,
                    gc,
                    str_intern_table,
                    sym_table.names_as_owned(),
                    output_stream,
                    err_stream,
                );

                let _ = vm.run();
            }
        }
        Err(err) => {
            let _ = writeln!(err_stream, "{}", err);
        }
    }
}
