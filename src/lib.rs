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
            let compiler =
                compiler::Compiler::new(&source, &mut gc, &mut str_intern_table, &mut err_stream);

            if let Some((chunk, sym_table)) = compiler.compile() {
                let mut vm = vm::VM::new(
                    chunk,
                    gc,
                    str_intern_table,
                    sym_table.names_as_owned(),
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
