use crate::ast::ClarityNode;
use crate::detector::{reentrancy, integer_overflow, access_control, unchecked_returns};
use crate::registry::Registry;
use crate::callgraph::CallGraph;

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

/// Single file analysis entry point
pub fn analyze(nodes: &[ClarityNode]) -> Vec<Finding> {
    let mut findings = vec![];

    findings.extend(reentrancy::detect(nodes));
    findings.extend(integer_overflow::detect(nodes));
    findings.extend(access_control::detect(nodes));
    findings.extend(unchecked_returns::detect(nodes));

    findings
}

/// Project wide analysis — Phase 2 cross-contract detection
pub fn analyze_project(registry: &Registry, graph: &CallGraph) -> Vec<ProjectFinding> {
    use crate::detector::{cross_contract, trait_dispatch};

    let mut findings = vec![];

    // run per-contract detectors first
    for contract in registry.all() {
        let contract_findings = analyze(&contract.ast);

        for f in contract_findings {
            findings.push(ProjectFinding {
                contract_name: contract.name.clone(),
                finding: f,
            });
        }

        // trait dispatch detection needs contract name context
        let trait_findings = trait_dispatch::detect(&contract.ast, &contract.name);
        for f in trait_findings {
            findings.push(ProjectFinding {
                contract_name: contract.name.clone(),
                finding: f,
            });
        }
    }

        // run cross-contract detector across the whole registry
    let cross_findings = cross_contract::detect(registry, graph);
    for f in cross_findings {
        // find which contract this finding belongs to by matching function name
        let contract_name = registry.all().iter()
            .find(|c| c.ast.iter().any(|n| {
                if let Some(children) = n.as_list() {
                    if let Some(sig) = children.get(1) {
                        if let Some(sig_children) = sig.as_list() {
                            return sig_children.first()
                                .and_then(|n| n.as_atom())
                                .map(|name| name == f.function_name)
                                .unwrap_or(false);
                        }
                    }
                }
                false
            }))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        findings.push(ProjectFinding {
            contract_name,
            finding: f,
        });
    }

      // sort by severity then contract name
    findings.sort_by(|a, b| {
        let sev_order = |s: &Severity| match s {
            Severity::Critical => 0,
            Severity::High     => 1,
            Severity::Medium   => 2,
            Severity::Low      => 3,
        };
        sev_order(&a.finding.severity)
            .cmp(&sev_order(&b.finding.severity))
            .then(a.contract_name.cmp(&b.contract_name))
    });

    findings
}

/// A finding with its associated contract name
#[derive(Debug, Clone)]
pub struct ProjectFinding {
    pub contract_name: String,
    pub finding: Finding,
}