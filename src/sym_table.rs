use std::collections::HashMap;

pub struct SymbolTable<'a> {
    symbols: HashMap<&'a str, usize>,
    /// owned names in insertion order, index → name
    names: Vec<String>,
}

impl<'a> SymbolTable<'a> {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            names: Vec::new(),
        }
    }

    /// Add a global (or return existing index) and store its owned name
    pub fn add(&mut self, name: &'a str) -> usize {
        if let Some(&idx) = self.symbols.get(name) {
            idx
        } else {
            let idx = self.names.len();
            self.names.push(name.to_owned());
            self.symbols.insert(name, idx);
            idx
        }
    }

    /// Resolve a variable name to its index (declares if missing)
    pub fn get(&mut self, name: &'a str) -> usize {
        self.add(name)
    }

    /// Number of globals
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// returns the internal list of variable names
    pub fn names_as_owned(self) -> Vec<String> {
        self.names
    }
}
