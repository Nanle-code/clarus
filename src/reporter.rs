use colored::*;
use crate::analyzer::{Finding, Severity};

pub struct Report {
    pub filename: String,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn new(filename: &str, findings: Vec<Finding>) -> Self {
        Report {
            filename: filename.to_string(),
            findings,
        }
    }

    /// Print a human-readable report to the terminal
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
        println!(
            "  {} [{}] {}",
            format!("[{}]", index).bold(),
            severity_colored,
            finding.kind.bold()
        );
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

 /// Output findings as JSON (for CI/CD integration)
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