mod ast;
mod parser;
mod analyzer;
mod reporter;
mod detector;
mod loader;
mod registry;
mod callgraph;

use clap::{Parser, Subcommand};
use reporter::Report;
use std::path::Path;
use std::process;
use colored::Colorize;

#[derive(Parser, Debug)]
#[command(
    name = "clarus",
    about = "Static analyzer for Clarity smart contracts",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyze a single Clarity contract file
    Check {
        /// Path to the .cla file
        file: String,
        
        #[arg(long, default_value_t = false)]
        json: bool,

        #[arg(long, default_value_t = false)]
        strict: bool,
    },

    /// Analyze all Clarity contracts in a directory
    Scan {
        /// Path to the contracts directory
        dir: String,

        #[arg(long, default_value_t = false)]
        json: bool,

        #[arg(long, default_value_t = false)]
        strict: bool,
    },

    /// Show the call graph for all contracts in a directory
    Graph {
        /// Path to the contracts directory
        dir: String,

        /// Show only cross-contract edges
        #[arg(long, default_value_t = false)]
        cross_only: bool,
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file, json, strict } => {
            run_check(&file, json, strict);
        }
        Commands::Scan { dir, json, strict } => {
            run_scan(&dir, json, strict);
        }
        Commands::Graph { dir, cross_only} => {
        run_graph(&dir, cross_only);
    }
    }
}

fn run_check(file: &str, json: bool, strict: bool) {
    let path = Path::new(file);

    let contract = match loader::load_single(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let mut clarity_parser = parser::Parser::new(&contract.source);
    let nodes = match clarity_parser.parse() {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("Parse error in '{}': {}", file, e);
            process::exit(1);
        }
    };

    let findings = analyzer::analyze(&nodes);
    let has_findings = !findings.is_empty();

    let report = Report::new(file, findings);

    if json { report.print_json(); } else { report.print(); }

    if strict && has_findings {
        process::exit(1);
    }
}

fn run_scan(dir: &str, json: bool, strict: bool) {
    let path = Path::new(dir);

    let sources = match loader::load_contracts(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let registry = match registry::Registry::build(sources) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error building registry: {}", e);
            process::exit(1);
        }
    };

    println!();
    println!("  {} {} — Clarity Smart Contract Analyzer",
        "Clarus".bold().cyan(),
        "v0.2.0".dimmed()
    );
    println!("  {} {} contracts in {}",
        "Scanning:".dimmed(),
        registry.len().to_string().bold(),
        dir.bold()
    );
    println!("{}", "─".repeat(60).dimmed());

    let mut total_findings = 0;
    let mut has_findings = false;

    for contract in registry.all() {
        let findings = analyzer::analyze(&contract.ast);

        if !findings.is_empty() {
            has_findings = true;
            total_findings += findings.len();
        }

        let filename = contract.source.path.to_string_lossy().to_string();
        let report = Report::new(&filename, findings);

        if json { report.print_json(); } else { report.print(); }
    }

    if !json {
        println!();
        println!("  {} {} total findings across {} contracts",
            "Summary:".dimmed(),
            total_findings.to_string().bold().red(),
            registry.len()
        );
        println!();
    }

    if strict && has_findings {
        process::exit(1);
    }
}


fn run_graph(dir: &str, cross_only: bool) {
    use colored::Colorize;

    let path = Path::new(dir);

    let sources = match loader::load_contracts(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let registry = match registry::Registry::build(sources) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let graph = callgraph::build(&registry);
    let summary = graph.summary();

    println!();
    println!("{} {}", "Clarus".bold().cyan(), "— Call Graph".dimmed());
    println!("{} {}", "Directory:".dimmed(), dir.bold());
    println!("{}", "─".repeat(60).dimmed());
    println!();
    println!("  {}  {}", "Contracts :".dimmed(), summary.total_contracts.to_string().bold());
    println!("  {}  {}", "Functions :".dimmed(), summary.total_functions.to_string().bold());
    println!("  {}      {}", "Edges :".dimmed(), summary.total_edges.to_string().bold());
    println!();

    if graph.edges.is_empty() {
        println!("  {} No cross-contract calls found", "✓".green().bold());
        println!();
        return;
    }

    println!("{}", "─".repeat(60).dimmed());
    println!("  {}", "Call Edges:".bold());
    println!();

    for edge in &graph.edges {
        // skip same-contract calls if cross_only flag is set
        if cross_only && edge.caller_contract == edge.callee_contract {
            continue;
        }

        let arrow = if edge.caller_contract != edge.callee_contract {
            "──▶".bold().red().to_string()  // cross-contract call
        } else {
            "──▶".dimmed().to_string()       // same-contract call
        };

        println!(
            "  {}.{}  {}  {}.{}  {}",
            edge.caller_contract.bold().yellow(),
            edge.caller_function.cyan(),
            arrow,
            edge.callee_contract.bold().yellow(),
            edge.callee_function.cyan(),
            format!("(line {})", edge.line).dimmed()
        );
    }

    println!();

    // warn about cycles
    if !summary.cycles.is_empty() {
        println!("{}", "─".repeat(60).dimmed());
        println!("  {} {} circular call path(s) detected",
            "⚠ WARNING:".bold().red(),
            summary.cycles.len()
        );
        for cycle in &summary.cycles {
            let path: Vec<String> = cycle.iter()
                .map(|(c, f)| format!("{}.{}", c, f))
                .collect();
            println!("  {}", path.join(" → ").red());
        }
        println!();
    }

    println!("{}", "─".repeat(60).dimmed());
    println!("  {} Red arrows indicate cross-contract calls",
        "Legend:".dimmed()
    );
    println!();
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