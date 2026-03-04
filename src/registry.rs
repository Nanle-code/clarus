use std::collections::HashMap;
use crate::ast::ClarityNode;
use crate::parser::Parser;
use crate::loader::ContractSource;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ParsedContract {
    pub name: String,
    pub source: ContractSource,
    pub ast: Vec<ClarityNode>,
}

#[derive(Debug, Default)]
pub struct Registry {
    contracts: HashMap<String, ParsedContract>,
}

#[allow(dead_code)]
impl Registry {
    pub fn new() -> Self {
        Registry {
            contracts: HashMap::new(),
        }
    }

    /// Build a registry from a list of loaded contract sources
    pub fn build(sources: Vec<ContractSource>) -> Result<Self, String> {
        let mut registry = Registry::new();

        for source in sources {
            let mut parser = Parser::new(&source.source.clone());
            let ast = parser.parse()
                .map_err(|e| format!(
                    "Parse error in '{}': {}",
                    source.name, e
                ))?;

            let name = source.name.clone();
            let parsed = ParsedContract { name: name.clone(), source, ast };
            registry.contracts.insert(name, parsed);
        }

        Ok(registry)
    }

    /// Look up a contract by name
    pub fn get(&self, name: &str) -> Option<&ParsedContract> {
        // try exact match first
        if let Some(contract) = self.contracts.get(name) {
            return Some(contract);
        }

        // try with leading dot stripped e.g. ".vault" -> "vault"
        let stripped = name.trim_start_matches('.');
        self.contracts.get(stripped)
    }

    /// Get all contracts in the registry
    pub fn all(&self) -> Vec<&ParsedContract> {
        let mut contracts: Vec<&ParsedContract> = self.contracts.values().collect();
        contracts.sort_by(|a, b| a.name.cmp(&b.name));
        contracts
    }

    /// Get total number of contracts
    pub fn len(&self) -> usize {
        self.contracts.len()
    }

    /// Get all contract names
    pub fn names(&self) -> Vec<&str> {
        self.contracts.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_contracts;
    use std::path::Path;

    #[test]
    fn builds_registry_from_directory() {
        let sources = load_contracts(Path::new("tests/contracts")).unwrap();
        let registry = Registry::build(sources).unwrap();

        assert!(registry.len() >= 3);
    }

    #[test]
    fn lookup_by_name() {
        let sources = load_contracts(Path::new("tests/contracts")).unwrap();
        let registry = Registry::build(sources).unwrap();

        let contract = registry.get("vulnerable");
        assert!(contract.is_some());
        assert_eq!(contract.unwrap().name, "vulnerable");
    }

    #[test]
    fn lookup_with_leading_dot() {
        let sources = load_contracts(Path::new("tests/contracts")).unwrap();
        let registry = Registry::build(sources).unwrap();

        // .vault style references should resolve to vault
        let contract = registry.get(".vulnerable");
        assert!(contract.is_some());
    }
}