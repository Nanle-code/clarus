use crate::ast::ClarityNode;
use crate::analyzer::{Finding, Severity};

pub fn detect(nodes: &[ClarityNode]) -> Vec<Finding> {
    let mut findings = vec![];

    for node in nodes {
        if node.is_form("define-public") || node.is_form("define-private") {
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

    // check if function has a bounds check anywhere
    let has_bounds_check = body_has_bounds_check(body);

    let mut findings = vec![];
    walk_body(body, &func_name, has_bounds_check, &mut findings);

    Some(findings)
}

/// Check if the function body contains an asserts! with >= or <=
/// which indicates the developer is doing bounds checking
fn body_has_bounds_check(nodes: &[ClarityNode]) -> bool {
    for node in nodes {
        match node {
            ClarityNode::List(children, _) => {
                let head = children.first().and_then(|n| n.as_atom());

                // look for (asserts! (>= ...) ...) pattern
                if head == Some("asserts!") {
                    if let Some(condition) = children.get(1) {
                        if let Some(cond_children) = condition.as_list() {
                            let cond_head = cond_children.first().and_then(|n| n.as_atom());
                            if matches!(cond_head, Some(">=") | Some("<=") | Some(">") | Some("<")) {
                                return true;
                            }
                        }
                    }
                }

                if body_has_bounds_check(children) {
                    return true;
                }
            }
            ClarityNode::Atom(_, _) => {}
        }
    }
    false
}

fn walk_body(
    nodes: &[ClarityNode],
    func_name: &str,
    has_bounds_check: bool,
    findings: &mut Vec<Finding>,
) {
    for node in nodes {
        if let ClarityNode::List(children, line) = node {
            let head = children.first().and_then(|n| n.as_atom());

            match head {
                Some("-") => {
                    // only flag if there is no bounds check in the function
                    if !has_bounds_check {
                        findings.push(Finding {
                            severity: Severity::High,
                            kind: "Integer Underflow".to_string(),
                            function_name: func_name.to_string(),
                            line: *line,
                            message: "Subtraction may underflow — no bounds check found in function"
                                .to_string(),
                            fix: "Add (asserts! (>= balance amount) (err u1)) before subtracting"
                                .to_string(),
                        });
                    }
                }

                Some("+") => {
                    findings.push(Finding {
                        severity: Severity::Low,
                        kind: "Integer Overflow".to_string(),
                        function_name: func_name.to_string(),
                        line: *line,
                        message: "Addition may overflow uint bounds without upper bound check"
                            .to_string(),
                        fix: "Consider asserting an upper bound before adding".to_string(),
                    });
                }

                _ => {
                    walk_body(children, func_name, has_bounds_check, findings);
                }
            }
        }
    }
}

fn extract_function_name(sig: &ClarityNode) -> Option<String> {
    match sig {
        ClarityNode::List(children, _) => {
            children.first()?.as_atom().map(|s| s.to_string())
        }
        ClarityNode::Atom(name, _) => Some(name.clone()),
    }
}