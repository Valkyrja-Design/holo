use std::env;
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        holo::interpret(&args[1]);
    } else {
        eprintln!("Usage: holo [path]");
    }
}
