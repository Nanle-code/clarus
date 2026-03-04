use crate::ast::ClarityNode;
use crate::analyzer::{Finding, Severity};

/// Detect functions that accept trait parameters and mutate state
/// around those calls — any conforming contract could be passed in
pub fn detect(nodes: &[ClarityNode], contract_name: &str) -> Vec<Finding> {
    let mut findings = vec![];

    for node in nodes {
        if node.is_form("define-public") || node.is_form("define-private") {
            if let Some(func_findings) = analyze_function(node, contract_name) {
                findings.extend(func_findings);
            }
        }
    }

    findings
}

fn analyze_function(node: &ClarityNode, contract_name: &str) -> Option<Vec<Finding>> {
    let children = node.as_list()?;
    let sig = children.get(1)?;
    let func_name = extract_function_name(sig)?;
    let trait_params = extract_trait_params(sig);

    if trait_params.is_empty() {
        return Some(vec![]);
    }

    let body = &children[2..];
    let mut findings = vec![];

    // check if any trait parameter is called and state is mutated around it
    let mut seen_trait_call = false;
    let mut call_line = 0;

    walk_body(
        body,
        &func_name,
        contract_name,
        &trait_params,
        &mut seen_trait_call,
        &mut call_line,
        &mut findings,
    );

    Some(findings)
}

/// Extract trait parameter names from a function signature
/// e.g. (swap (token <ft-trait>) (amount uint)) -> ["token"]
fn extract_trait_params(sig: &ClarityNode) -> Vec<String> {
    let mut trait_params = vec![];

    if let ClarityNode::List(children, _) = sig {
        // skip first child which is the function name
        for param in children.iter().skip(1) {
            if let ClarityNode::List(param_children, _) = param {
                // param is (name type)
                // if type contains < > it is a trait
                if param_children.len() >= 2 {
                    let param_name = param_children.first()
                        .and_then(|n| n.as_atom())
                        .map(|s| s.to_string());

                    let param_type = param_children.get(1)
                        .and_then(|n| n.as_atom())
                        .unwrap_or("");

                    if param_type.starts_with('<') && param_type.ends_with('>') {
                        if let Some(name) = param_name {
                            trait_params.push(name);
                        }
                    }
                }
            }
        }
    }

    trait_params
}

fn walk_body(
    nodes: &[ClarityNode],
    func_name: &str,
    contract_name: &str,
    trait_params: &[String],
    seen_trait_call: &mut bool,
    call_line: &mut usize,
    findings: &mut Vec<Finding>,
) {
    const STATE_MUTATORS: &[&str] = &["map-set", "map-delete", "map-insert", "var-set"];

    for node in nodes {
        if let ClarityNode::List(children, line) = node {
            let head = children.first().and_then(|n| n.as_atom());

            match head {
                Some("contract-call?") => {
                    // check if the callee is one of our trait params
                    if let Some(callee) = children.get(1) {
                        if let Some(callee_name) = callee.as_atom() {
                            if trait_params.contains(&callee_name.to_string()) {
                                *seen_trait_call = true;
                                *call_line = *line;
                            }
                        }
                    }
                }

                Some(op) if STATE_MUTATORS.contains(&op) => {
                    if *seen_trait_call {
                        findings.push(Finding {
                            severity: Severity::Critical,
                            kind: "Trait Dispatch Reentrancy".to_string(),
                            function_name: func_name.to_string(),
                            line: *line,
                            message: format!(
                                "'{}' called after trait dispatch on line {} — any conforming contract could be passed as the trait argument",
                                op, call_line
                            ),
                            fix: format!(
                                "Move '{}' to before the trait-based contract-call? on line {}. Validate the trait implementor if possible.",
                                op, call_line
                            ),
                        });
                    }
                }

                Some("begin") | Some("let") | Some("let*")
                | Some("if") | Some("match") | Some("and") | Some("or") => {
                    walk_body(
                        &children[1..],
                        func_name,
                        contract_name,
                        trait_params,
                        seen_trait_call,
                        call_line,
                        findings,
                    );
                }

                _ => {
                    walk_body(
                        children,
                        func_name,
                        contract_name,
                        trait_params,
                        seen_trait_call,
                        call_line,
                        findings,
                    );
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