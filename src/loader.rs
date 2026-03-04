use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ContractSource {
    pub name: String,      // contract name e.g. "vault"
    pub path: PathBuf,     // full path to the file
    pub source: String,    // raw source code
}

/// Find and load all .clar files in a directory
pub fn load_contracts(dir: &Path) -> Result<Vec<ContractSource>, String> {
    if !dir.exists() {
        return Err(format!("Directory '{}' does not exist", dir.display()));
    }

    if !dir.is_dir() {
        return Err(format!("'{}' is not a directory", dir.display()));
    }

    let mut contracts = vec![];

    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        // only process .clar files
        if path.extension().and_then(|e| e.to_str()) != Some("clar") {
            continue;
        }

        let source = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;

        // derive contract name from filename without extension
        // e.g. "vault.clar" -> "vault"
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        contracts.push(ContractSource { name, path, source });
    }

    if contracts.is_empty() {
        return Err(format!("No .clar files found in '{}'", dir.display()));
    }

    // sort by name for deterministic ordering
    contracts.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(contracts)
}

/// Load a single .clar file — keeps Phase 1 behavior intact
pub fn load_single(path: &Path) -> Result<ContractSource, String> {
    if !path.exists() {
        return Err(format!("File '{}' does not exist", path.display()));
    }

    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(ContractSource {
        name,
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn loads_clar_files_from_directory() {
        let path = Path::new("tests/contracts");
        let contracts = load_contracts(path).unwrap();

        // we have at least vulnerable.clar, safe.clar, bug.clar
        assert!(contracts.len() >= 3);
        assert!(contracts.iter().all(|c| c.path.extension().unwrap() == "clar"));
    }

    #[test]
    fn contract_names_derived_from_filenames() {
        let path = Path::new("tests/contracts");
        let contracts = load_contracts(path).unwrap();

        // names should not contain .clar extension
        assert!(contracts.iter().all(|c| !c.name.contains(".clar")));
    }

    #[test]
    fn loads_single_file() {
        let path = Path::new("tests/contracts/vulnerable.clar");
        let contract = load_single(path).unwrap();

        assert_eq!(contract.name, "vulnerable");
        assert!(!contract.source.is_empty());
    }

    #[test]
    fn errors_on_missing_directory() {
        let path = Path::new("tests/nonexistent");
        let result = load_contracts(path);

        assert!(result.is_err());
    }
}