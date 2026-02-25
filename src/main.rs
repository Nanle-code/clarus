mod ast;
mod parser;
mod analyzer;
mod reporter;
mod detector;

use clap::Parser;
use reporter::Report;
use std::fs;
use std::process;

#[derive(Parser, Debug)]
#[command(
    name = "clarus",
    about = "Static analyzer for Clarity smart contracts",
    version = "0.1.0"
)]
struct Cli {
    /// Path to the Clarity contract file to analyze
    file: String,

    /// Output results as JSON
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Exit with code 1 if issues are found (useful for CI/CD)
    #[arg(long, default_value_t = false)]
    strict: bool,
}

fn main() {
    let cli = Cli::parse();

    // read the contract file
    let source = match fs::read_to_string(&cli.file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", cli.file, e);
            process::exit(1);
        }
    };

    // parse into AST — use full module path to avoid conflict with clap::Parser
    let mut clarity_parser = parser::Parser::new(&source);
    let nodes = match clarity_parser.parse() {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("Parse error in '{}': {}", cli.file, e);
            process::exit(1);
        }
    };

    // run reentrancy detector
    let findings = analyzer::analyze(&nodes);
    let has_findings = !findings.is_empty();

    // build and print report
    let report = Report::new(&cli.file, findings);

    if cli.json {
        report.print_json();
    } else {
        report.print();
    }

    // exit with code 1 in strict mode if issues found
    if cli.strict && has_findings {
        process::exit(1);
    }
}




#[cfg(test)]
mod integration_tests {
    use crate::{parser, analyzer};

    fn run(source: &str) -> Vec<crate::analyzer::Finding> {
        let mut p = parser::Parser::new(source);
        let nodes = p.parse().unwrap();
        analyzer::analyze(&nodes)
    }

    #[test]
    fn detects_reentrancy_stx_transfer() {
        let source = r#"
            (define-public (withdraw (amount uint))
                (let ((bal (default-to u0 (map-get? balances tx-sender))))
                    (try! (stx-transfer? amount (as-contract tx-sender) tx-sender))
                    (map-set balances tx-sender u0)
                ))
        "#;
        let findings = run(source);
        assert!(findings.iter().any(|f| f.kind == "Reentrancy"));
    }

    #[test]
    fn detects_unchecked_stx_transfer() {
        let source = r#"
            (define-public (withdraw-all)
                (let ((bal (default-to u0 (map-get? balances tx-sender))))
                    (stx-transfer? bal (as-contract tx-sender) tx-sender)
                    (map-delete balances tx-sender)
                    (ok true)))
        "#;
        let findings = run(source);
        assert!(findings.iter().any(|f| f.kind == "Unchecked Return Value"));
        assert!(findings.iter().any(|f| f.kind == "Reentrancy"));
    }

    #[test]
    fn detects_missing_access_control() {
        let source = r#"
            (define-public (deposit)
                (let ((amount (stx-get-balance tx-sender)))
                    (map-set balances tx-sender amount)
                    (ok true)))
        "#;
        let findings = run(source);
        assert!(findings.iter().any(|f| f.kind == "Missing Access Control"));
    }

    #[test]
    fn detects_integer_underflow_without_bounds_check() {
        let source = r#"
            (define-public (transfer-from (owner principal) (amount uint))
                (let ((owner-balance (default-to u0 (map-get? balances owner))))
                    (map-set balances owner (- owner-balance amount))
                    (ok true)))
        "#;
        let findings = run(source);
        assert!(findings.iter().any(|f| f.kind == "Integer Underflow"));
    }

    #[test]
    fn no_underflow_when_asserts_present() {
        let source = r#"
            (define-public (withdraw (amount uint))
                (let ((bal (default-to u0 (map-get? balances tx-sender))))
                    (asserts! (>= bal amount) (err u1))
                    (map-set balances tx-sender (- bal amount))
                    (ok true)))
        "#;
        let findings = run(source);
        assert!(!findings.iter().any(|f| f.kind == "Integer Underflow"));
    }

    #[test]
    fn clean_contract_has_no_findings() {
        let source = r#"
            (define-public (withdraw (amount uint))
                (let ((bal (default-to u0 (map-get? balances tx-sender))))
                    (asserts! (>= bal amount) (err u1))
                    (asserts! (is-eq tx-sender contract-caller) (err u2))
                    (map-set balances tx-sender (- bal amount))
                    (try! (stx-transfer? amount (as-contract tx-sender) tx-sender))
                    (ok true)))
        "#;
        let findings = run(source);
        assert!(findings.iter().all(|f| f.kind != "Reentrancy"));
        assert!(findings.iter().all(|f| f.kind != "Integer Underflow"));
        assert!(findings.iter().all(|f| f.kind != "Unchecked Return Value"));
    }
}