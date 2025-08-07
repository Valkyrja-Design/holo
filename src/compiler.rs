use super::scanner;

pub struct Compiler<'a> {
    source: &'a str,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        Compiler { source }
    }

    pub fn compile(&mut self) {}
}
