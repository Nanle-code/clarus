use crate::ast::ClarityNode;
use crate::callgraph::{CallGraph};
use crate::registry::Registry;
use crate::analyzer::{Finding, Severity};

const STATE_MUTATORS: &[&str] = &[
    "map-set", "map-delete", "map-insert", "var-set",
];


/// Detect cross-contract reentrancy using the call graph
pub fn detect(registry: &Registry, graph: &CallGraph) -> Vec<Finding> {
    let mut findings = vec![];

    for contract in registry.all() {
        for node in &contract.ast {
            if node.is_form("define-public") {
                if let Some(func_findings) = analyze_function(
                    node,
                    &contract.name,
                    graph,
                ) {
                    findings.extend(func_findings);
                }
            }
        }
    }

    findings
}

fn analyze_function(
    node: &ClarityNode,
    contract_name: &str,
    graph: &CallGraph,
) -> Option<Vec<Finding>> {
    let children = node.as_list()?;
    let func_name = extract_function_name(children.get(1)?)?;
    let body = &children[2..];

    let mut findings = vec![];
    let mut seen_external_call = false;
    let mut call_line = 0;
    let mut callee_contract = String::new();
    let mut callee_function = String::new();

    walk_body(
        body,
        contract_name,
        &func_name,
        graph,
        &mut seen_external_call,
        &mut call_line,
        &mut callee_contract,
        &mut callee_function,
        &mut findings,
    );

    Some(findings)
}

fn walk_body(
    nodes: &[ClarityNode],
    contract_name: &str,
    func_name: &str,
    graph: &CallGraph,
    seen_external_call: &mut bool,
    call_line: &mut usize,
    callee_contract: &mut String,
    callee_function: &mut String,
    findings: &mut Vec<Finding>,
) {
    for node in nodes {
        if let ClarityNode::List(children, line) = node {
            let head = children.first().and_then(|n| n.as_atom());

            match head {
                Some("contract-call?") => {
                    // extract the target contract and function
                    let target_contract = children.get(1)
                        .and_then(|n| n.as_atom())
                        .unwrap_or("unknown")
                        .trim_start_matches('.')
                        .to_string();

                    let target_function = children.get(2)
                        .and_then(|n| n.as_atom())
                        .unwrap_or("unknown")
                        .to_string();

                    // only flag cross-contract calls
                    if target_contract != contract_name {
                        *seen_external_call = true;
                        *call_line = *line;
                        *callee_contract = target_contract;
                        *callee_function = target_function;
                    }
                }

                Some("try!") | Some("unwrap!") | Some("unwrap-panic") => {
                    // check if inner expression is a cross-contract call
                    if let Some(inner) = children.get(1) {
                        if let Some(inner_children) = inner.as_list() {
                            let inner_head = inner_children
                                .first()
                                .and_then(|n| n.as_atom());

                            if inner_head == Some("contract-call?") {
                                let target_contract = inner_children.get(1)
                                    .and_then(|n| n.as_atom())
                                    .unwrap_or("unknown")
                                    .trim_start_matches('.')
                                    .to_string();

                                let target_function = inner_children.get(2)
                                    .and_then(|n| n.as_atom())
                                    .unwrap_or("unknown")
                                    .to_string();

                                if target_contract != contract_name {
                                    *seen_external_call = true;
                                    *call_line = *line;
                                    *callee_contract = target_contract;
                                    *callee_function = target_function;
                                }
                            }
                        }
                    }
                    walk_body(
                        &children[1..],
                        contract_name,
                        func_name,
                        graph,
                        seen_external_call,
                        call_line,
                        callee_contract,
                        callee_function,
                        findings,
                    );
                }

                Some(op) if STATE_MUTATORS.contains(&op) => {
                    if *seen_external_call {
                        findings.push(Finding {
                            severity: Severity::Critical,
                            kind: "Cross-Contract Reentrancy".to_string(),
                            function_name: func_name.to_string(),
                            line: *line,
                            message: format!(
                                "'{}' mutated after calling {}.{} on line {} — re-entry possible before state is updated",
                                op,
                                callee_contract,
                                callee_function,
                                call_line,
                            ),
                            fix: format!(
                                "Move '{}' to before the contract-call? to {}.{} on line {}",
                                op,
                                callee_contract,
                                callee_function,
                                call_line,
                            ),
                        });
                    }
                }

                Some("let") | Some("let*") => {
                    if children.len() > 1 {
                        // walk bindings
                        if let Some(bindings) = children.get(1) {
                            if let Some(binding_list) = bindings.as_list() {
                                for binding in binding_list {
                                    if let Some(bc) = binding.as_list() {
                                        if bc.len() > 1 {
                                            walk_body(
                                                &bc[1..],
                                                contract_name,
                                                func_name,
                                                graph,
                                                seen_external_call,
                                                call_line,
                                                callee_contract,
                                                callee_function,
                                                findings,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        // walk body
                        if children.len() > 2 {
                            walk_body(
                                &children[2..],
                                contract_name,
                                func_name,
                                graph,
                                seen_external_call,
                                call_line,
                                callee_contract,
                                callee_function,
                                findings,
                            );
                        }
                    }
                }

                Some("begin") | Some("if") | Some("match")
                | Some("and") | Some("or") => {
                    walk_body(
                        &children[1..],
                        contract_name,
                        func_name,
                        graph,
                        seen_external_call,
                        call_line,
                        callee_contract,
                        callee_function,
                        findings,
                    );
                }

                _ => {
                    walk_body(
                        children,
                        contract_name,
                        func_name,
                        graph,
                        seen_external_call,
                        call_line,
                        callee_contract,
                        callee_function,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_contracts;
    use crate::registry::Registry;
    use crate::callgraph;
    use std::path::Path;

    #[test]
    fn detects_cross_contract_reentrancy_in_multicontract() {
        let sources = load_contracts(
            Path::new("tests/multicontract")
        ).unwrap();
        let registry = Registry::build(sources).unwrap();
        let graph = callgraph::build(&registry);
        let findings = detect(&registry, &graph);

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.kind == "Cross-Contract Reentrancy"));
        assert!(findings.iter().any(|f| f.function_name == "withdraw"));
    }

    #[test]
    fn finding_names_callee_contract() {
        let sources = load_contracts(
            Path::new("tests/multicontract")
        ).unwrap();
        let registry = Registry::build(sources).unwrap();
        let graph = callgraph::build(&registry);
        let findings = detect(&registry, &graph);

        // message should name the external contract being called
        let reentrancy = findings.iter()
            .find(|f| f.kind == "Cross-Contract Reentrancy")
            .unwrap();

        assert!(reentrancy.message.contains("token") || 
                reentrancy.message.contains("rewards"));
    }
}