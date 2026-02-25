use crate::ast::ClarityNode;
use crate::analyzer::{Finding, Severity};

const SENSITIVE_MUTATORS: &[&str] = &[
    "map-set", "map-delete", "map-insert", "var-set",
];

pub fn detect(nodes: &[ClarityNode]) -> Vec<Finding> {
    let mut findings = vec![];

    for node in nodes {
        if node.is_form("define-public") {
            if let Some(func_findings) = analyze_function(node) {
                findings.extend(func_findings);
            }
        }
    }

    findings
}

fn analyze_function(node: &ClarityNode) -> Option<Vec<Finding>> {
    let children = node.as_list()?;
    let func_name = extract_function_name(children.get(1)?)?;
    let body = &children[2..];

    let has_auth_check = body_has_real_auth_check(body);
    let has_mutation = body_contains_mutation(body);

    let mut findings = vec![];

    if has_mutation && !has_auth_check {
        findings.push(Finding {
            severity: Severity::High,
            kind: "Missing Access Control".to_string(),
            function_name: func_name.clone(),
            line: node.line(),
            message: format!(
                "Public function '{}' modifies state without access control check",
                func_name
            ),
            fix: "Add (asserts! (is-eq tx-sender contract-owner) (err u403)) at the top of the function".to_string(),
        });
    }

    Some(findings)
}

/// Only count tx-sender/contract-caller as auth checks when they
/// appear inside asserts! or is-eq expressions — not just anywhere
fn body_has_real_auth_check(nodes: &[ClarityNode]) -> bool {
    for node in nodes {
        if let ClarityNode::List(children, _) = node {
            let head = children.first().and_then(|n| n.as_atom());

            match head {
                Some("asserts!") => {
                    // check if the condition references tx-sender or contract-caller
                    if let Some(condition) = children.get(1) {
                        if node_contains_auth_principal(condition) {
                            return true;
                        }
                    }
                }
                Some("is-eq") => {
                    // (is-eq tx-sender contract-owner) pattern
                    if children.iter().any(|n| {
                        n.as_atom().map(|s| s == "tx-sender" || s == "contract-caller")
                            .unwrap_or(false)
                    }) {
                        return true;
                    }
                }
                _ => {
                    if body_has_real_auth_check(children) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if a node or any of its children reference tx-sender/contract-caller
fn node_contains_auth_principal(node: &ClarityNode) -> bool {
    match node {
        ClarityNode::Atom(val, _) => {
            val == "tx-sender" || val == "contract-caller"
        }
        ClarityNode::List(children, _) => {
            children.iter().any(node_contains_auth_principal)
        }
    }
}

fn body_contains_mutation(nodes: &[ClarityNode]) -> bool {
    for node in nodes {
        if let ClarityNode::List(children, _) = node {
            let head = children.first().and_then(|n| n.as_atom());
            if let Some(op) = head {
                if SENSITIVE_MUTATORS.contains(&op) {
                    return true;
                }
            }
            if body_contains_mutation(children) {
                return true;
            }
        }
    }
    false
}

fn extract_function_name(sig: &ClarityNode) -> Option<String> {
    match sig {
        ClarityNode::List(children, _) => {
            children.first()?.as_atom().map(|s| s.to_string())
        }
        ClarityNode::Atom(name, _) => Some(name.clone()),
    }
}