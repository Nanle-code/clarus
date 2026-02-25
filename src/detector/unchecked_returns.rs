use crate::ast::ClarityNode;
use crate::analyzer::{Finding, Severity};

const EXTERNAL_CALLS: &[&str] = &[
    "contract-call?",
    "stx-transfer?",
    "ft-transfer?",
    "nft-transfer?",
];

const SAFE_WRAPPERS: &[&str] = &[
    "try!", "unwrap!", "unwrap-panic", "unwrap-err!", "match",
];

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

    let mut findings = vec![];
    walk_body(body, &func_name, false, &mut findings);

    Some(findings)
}

/// inside_wrapper = true means this node is already inside try!/unwrap!/etc
fn walk_body(
    nodes: &[ClarityNode],
    func_name: &str,
    inside_wrapper: bool,
    findings: &mut Vec<Finding>,
) {
    for node in nodes {
        if let ClarityNode::List(children, line) = node {
            let head = children.first().and_then(|n| n.as_atom());

            match head {
                // safe wrapper — recurse with flag set so inner calls are skipped
                Some(op) if SAFE_WRAPPERS.contains(&op) => {
                    walk_body(&children[1..], func_name, true, findings);
                }

                // external call — only flag if NOT inside a safe wrapper
                Some(op) if EXTERNAL_CALLS.contains(&op) => {
                    if !inside_wrapper {
                        findings.push(Finding {
                            severity: Severity::Medium,
                            kind: "Unchecked Return Value".to_string(),
                            function_name: func_name.to_string(),
                            line: *line,
                            message: format!(
                                "'{}' return value is not checked — errors will be silently ignored",
                                op
                            ),
                            fix: format!("Wrap with try! — (try! ({} ...))", op),
                        });
                    }
                    // don't recurse into the call arguments
                }

                _ => {
                    // reset inside_wrapper to false for sibling nodes
                    walk_body(children, func_name, false, findings);
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