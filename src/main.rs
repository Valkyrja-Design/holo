use holo::vm;
use std::env;
use std::fs;

fn run(path: &str) {
    match fs::read_to_string(path) {
        Ok(source) => {
            let mut vm = vm::VM::new(source);
            let _err = vm.interpret();
        }
        Err(err) => eprintln!("{err}"),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        run(&args[1]);
    } else {
        eprintln!("Usage: holo [path]");
    }
}
