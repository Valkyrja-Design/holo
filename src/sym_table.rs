use std::collections::HashMap;

/// A symbol table for managing variable names and their indices.
///
/// This table maintains a bidirectional mapping between variable names
/// and their indices, which are used for efficient variable access in
/// the compiled bytecode.
#[derive(Debug)]
pub struct SymbolTable<'a> {
    symbols: HashMap<&'a str, usize>,
    /// Owned names in insertion order, index → name
    names: Vec<String>,
}

impl<'a> SymbolTable<'a> {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            names: Vec::new(),
        }
    }

    /// Declares a new symbol or returns the existing index if already declared.
    pub fn declare(&mut self, name: &'a str) -> usize {
        if let Some(&idx) = self.symbols.get(name) {
            idx
        } else {
            let idx = self.names.len();
            self.names.push(name.to_owned());
            self.symbols.insert(name, idx);
            idx
        }
    }

    /// Resolve a variable name to its index. This will declare the variable if
    /// it does not exist
    pub fn resolve(&mut self, name: &'a str) -> usize {
        self.declare(name)
    }

    /// Returns the number of symbols in the table.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Returns true if the symbol table is empty.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Consumes the symbol table and returns the owned names.
    pub fn into_names(self) -> Vec<String> {
        self.names
    }
}

impl Default for SymbolTable<'_> {
    fn default() -> Self {
        Self::new()
    }
}
