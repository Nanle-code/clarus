use colored::*;
use crate::analyzer::{Finding, Severity, ProjectFinding};

pub struct Report {
    pub filename: String,
    pub findings: Vec<Finding>,
}

pub struct ProjectReport {
    pub directory: String,
    pub findings: Vec<ProjectFinding>,
    pub contract_count: usize,
}

impl Report {
    pub fn new(filename: &str, findings: Vec<Finding>) -> Self {
        Report { filename: filename.to_string(), findings }
    }

    pub fn print(&self) {
        self.print_header();
        if self.findings.is_empty() {
            self.print_clean();
            return;
        }
        for (i, finding) in self.findings.iter().enumerate() {
            self.print_finding(i + 1, finding);
        }
        self.print_summary();
    }

    fn print_header(&self) {
        println!();
        println!("{} {}", "Clarus".bold().cyan(), "— Clarity Smart Contract Analyzer".dimmed());
        println!("{} {}", "Analyzing:".dimmed(), self.filename.bold());
        println!("{}", "─".repeat(60).dimmed());
    }

    fn print_clean(&self) {
        println!();
        println!("  {} No issues found in {}", "✓".bold().green(), self.filename.bold());
        println!();
        println!("{}", "─".repeat(60).dimmed());
        println!("  {} {}", "Result:".dimmed(), "Clean".bold().green());
        println!();
    }

    fn print_finding(&self, index: usize, finding: &Finding) {
        let severity_colored = match finding.severity {
            Severity::Critical => finding.severity.as_str().bold().red(),
            Severity::High     => finding.severity.as_str().bold().red(),
            Severity::Medium   => finding.severity.as_str().bold().yellow(),
            Severity::Low      => finding.severity.as_str().bold().blue(),
        };
        println!();
        println!("  {} [{}] {}", format!("[{}]", index).bold(), severity_colored, finding.kind.bold());
        println!();
        println!("    {}  {}", "Function :".dimmed(), finding.function_name.bold().yellow());
        println!("    {}  Line {}", "Location :".dimmed(), finding.line.to_string().bold());
        println!("    {}  {}", "Issue    :".dimmed(), finding.message.red());
        println!("    {}  {}", "Fix      :".dimmed(), finding.fix.cyan());
        println!();
        println!("{}", "─".repeat(60).dimmed());
    }

    fn print_summary(&self) {
        let critical = self.findings.iter().filter(|f| matches!(f.severity, Severity::Critical)).count();
        let high     = self.findings.iter().filter(|f| matches!(f.severity, Severity::High)).count();
        let medium   = self.findings.iter().filter(|f| matches!(f.severity, Severity::Medium)).count();
        let low      = self.findings.iter().filter(|f| matches!(f.severity, Severity::Low)).count();

        println!();
        println!("  {} {} total findings", "Result :".dimmed(), self.findings.len().to_string().bold().red());
        if critical > 0 { println!("         {} critical", critical.to_string().bold().red()); }
        if high > 0     { println!("         {} high",     high.to_string().bold().red()); }
        if medium > 0   { println!("         {} medium",   medium.to_string().bold().yellow()); }
        if low > 0      { println!("         {} low",      low.to_string().bold().blue()); }
        println!();
    }

    pub fn print_json(&self) {
        use serde_json::{json, Value};
        let findings: Vec<Value> = self.findings.iter().map(|f| {
            json!({
                "severity": f.severity.as_str(),
                "kind": f.kind,
                "function": f.function_name,
                "line": f.line,
                "message": f.message,
                "fix": f.fix,
            })
        }).collect();
        let report = json!({
            "file": self.filename,
            "total_findings": self.findings.len(),
            "findings": findings
        });
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    }
}

impl ProjectReport {
    pub fn new(directory: &str, findings: Vec<ProjectFinding>, contract_count: usize) -> Self {
        ProjectReport {
            directory: directory.to_string(),
            findings,
            contract_count,
        }
    }

    pub fn print(&self) {
        println!();
        println!("{} {}", "Clarus".bold().cyan(), "— Cross-Contract Analysis".dimmed());
        println!("{} {}", "Directory:".dimmed(), self.directory.bold());
        println!("{}", "─".repeat(60).dimmed());

        if self.findings.is_empty() {
            println!();
            println!("  {} No issues found across {} contracts",
                "✓".bold().green(),
                self.contract_count
            );
            println!();
            return;
        }

        for (i, pf) in self.findings.iter().enumerate() {
            let severity_colored = match pf.finding.severity {
                Severity::Critical => pf.finding.severity.as_str().bold().red(),
                Severity::High     => pf.finding.severity.as_str().bold().red(),
                Severity::Medium   => pf.finding.severity.as_str().bold().yellow(),
                Severity::Low      => pf.finding.severity.as_str().bold().blue(),
            };

            println!();
            println!(
                "  {} [{}] {}",
                format!("[{}]", i + 1).bold(),
                severity_colored,
                pf.finding.kind.bold()
            );
            println!();
            println!("    {}  {}", "Contract :".dimmed(), pf.contract_name.bold().yellow());
            println!("    {}  {}", "Function :".dimmed(), pf.finding.function_name.bold().yellow());
            println!("    {}  Line {}", "Location :".dimmed(), pf.finding.line.to_string().bold());
            println!("    {}  {}", "Issue    :".dimmed(), pf.finding.message.red());
            println!("    {}  {}", "Fix      :".dimmed(), pf.finding.fix.cyan());
            println!();
            println!("{}", "─".repeat(60).dimmed());
        }

        self.print_summary();
    }

    fn print_summary(&self) {
        let critical = self.findings.iter().filter(|f| matches!(f.finding.severity, Severity::Critical)).count();
        let high     = self.findings.iter().filter(|f| matches!(f.finding.severity, Severity::High)).count();
        let medium   = self.findings.iter().filter(|f| matches!(f.finding.severity, Severity::Medium)).count();
        let low      = self.findings.iter().filter(|f| matches!(f.finding.severity, Severity::Low)).count();

        println!();
        println!(
            "  {} {} total findings across {} contracts",
            "Result :".dimmed(),
            self.findings.len().to_string().bold().red(),
            self.contract_count
        );
        if critical > 0 { println!("         {} critical", critical.to_string().bold().red()); }
        if high > 0     { println!("         {} high",     high.to_string().bold().red()); }
        if medium > 0   { println!("         {} medium",   medium.to_string().bold().yellow()); }
        if low > 0      { println!("         {} low",      low.to_string().bold().blue()); }
        println!();
    }

    pub fn print_json(&self) {
        use serde_json::{json, Value};

        let findings: Vec<Value> = self.findings.iter().map(|f| {
            json!({
                "contract": f.contract_name,
                "severity": f.finding.severity.as_str(),
                "kind": f.finding.kind,
                "function": f.finding.function_name,
                "line": f.finding.line,
                "message": f.finding.message,
                "fix": f.finding.fix,
            })
        }).collect();

        let report = json!({
            "directory": self.directory,
            "contracts_analyzed": self.contract_count,
            "total_findings": self.findings.len(),
            "findings": findings
        });

        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    }
}