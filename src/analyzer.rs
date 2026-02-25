use crate::ast::ClarityNode;
use crate::detector::{reentrancy, integer_overflow, access_control, unchecked_returns};

#[derive(Debug, Clone)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn as_str(&self) -> &str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::High     => "HIGH",
            Severity::Medium   => "MEDIUM",
            Severity::Low      => "LOW",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub kind: String,        // "Reentrancy", "Integer Underflow", etc
    pub function_name: String,
    pub line: usize,
    pub message: String,     // what the problem is
    pub fix: String,         // how to fix it
}

/// Run all detectors against the parsed AST
pub fn analyze(nodes: &[ClarityNode]) -> Vec<Finding> {
    let mut findings = vec![];

    findings.extend(reentrancy::detect(nodes));
    findings.extend(integer_overflow::detect(nodes));
    findings.extend(access_control::detect(nodes));
    findings.extend(unchecked_returns::detect(nodes));

    findings
}