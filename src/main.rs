use std::env;
use std::io;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        holo::interpret(&args[1], io::stdout(), io::stderr());
    } else {
        eprintln!("Usage: holo [path]");
    }
}
