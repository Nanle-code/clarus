use crate::ast::ClarityNode;
use crate::analyzer::{Finding, Severity};

/// State mutations we care about
const STATE_MUTATORS: &[&str] = &[
    "map-set",
    "map-delete",
    "map-insert",
    "var-set",
];

/// External calls that can trigger reentrancy
const EXTERNAL_CALLS: &[&str] = &[
    "contract-call?",
    "stx-transfer?",
    "stx-burn?",
    "ft-transfer?",
    "nft-transfer?",
    "ft-mint?",
    "nft-mint?",
    "nft-burn?",
];

/// Analyze all top-level nodes and return findings
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

/// Analyze a single define-public function for reentrancy
fn analyze_function(node: &ClarityNode) -> Option<Vec<Finding>> {
    let children = node.as_list()?;

    let func_name = extract_function_name(children.get(1)?)?;
    let body = &children[2..];

    let mut findings = vec![];
    let mut seen_call = false;
    let mut call_line = 0;

    // walk the entire function body recursively
    walk_body(body, &func_name, &mut seen_call, &mut call_line, &mut findings);

    Some(findings)
}

/// Recursively walk a list of nodes tracking call/mutation order
fn walk_body(
    nodes: &[ClarityNode],
    func_name: &str,
    seen_call: &mut bool,
    call_line: &mut usize,
    findings: &mut Vec<Finding>,
) {
    for node in nodes {
        match node {
            ClarityNode::Atom(_, _) => {}

            ClarityNode::List(children, line) => {
                let head = children.first().and_then(|n| n.as_atom());

                match head {
                    Some(op) if EXTERNAL_CALLS.contains(&op) => {
                        *seen_call = true;
                        *call_line = *line;
                    }

                    Some("try!") | Some("unwrap!") | Some("unwrap-panic") => {
                        // check if inner expression is an external call
                        if let Some(inner) = children.get(1) {
                            if let Some(inner_children) = inner.as_list() {
                                let inner_head = inner_children
                                    .first()
                                    .and_then(|n| n.as_atom());
                                if let Some(op) = inner_head {
                                    if EXTERNAL_CALLS.contains(&op) {
                                        *seen_call = true;
                                        *call_line = *line;
                                    }
                                }
                            }
                        }
                        // still recurse in case its nested deeper
                        walk_body(&children[1..], func_name, seen_call, call_line, findings);
                    }

                    Some(op) if STATE_MUTATORS.contains(&op) => {
                        if *seen_call {
                            findings.push(Finding {
                                severity: Severity::Critical,
                                kind: "Reentrancy".to_string(),
                                function_name: func_name.to_string(),
                                line: *line,
                                message: format!(
                                    "'{}' called after external interaction on line {}",
                                    op, call_line
                                ),
                                fix: format!(
                                    "Move '{}' to before the external call on line {}",
                                    op, call_line
                                ),
                            });
                        }
                    }

                    Some("let") | Some("let*") => {
                        // let has shape: (let ((var val) ...) body...)
                        // we need to walk bindings first, THEN body
                        // carrying seen_call across both so a call in
                        // bindings is visible to mutations in the body
                        if children.len() > 1 {
                            // walk binding expressions
                            if let Some(bindings) = children.get(1) {
                                if let Some(binding_list) = bindings.as_list() {
                                    for binding in binding_list {
                                        if let Some(binding_children) = binding.as_list() {
                                            // binding is (var-name expr)
                                            // walk the expr part
                                            if binding_children.len() > 1 {
                                                walk_body(
                                                    &binding_children[1..],
                                                    func_name,
                                                    seen_call,
                                                    call_line,
                                                    findings,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            // walk body expressions after bindings
                            // seen_call carries over from bindings
                            if children.len() > 2 {
                                walk_body(
                                    &children[2..],
                                    func_name,
                                    seen_call,
                                    call_line,
                                    findings,
                                );
                            }
                        }
                    }

                    Some("begin") | Some("if") | Some("match")
                    | Some("and") | Some("or") => {
                        walk_body(&children[1..], func_name, seen_call, call_line, findings);
                    }

                    _ => {
                        walk_body(children, func_name, seen_call, call_line, findings);
                    }
                }
            }
        }
    }
}

/// Extract function name from signature node
/// e.g. (withdraw (amount uint)) -> "withdraw"
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
    use crate::parser::Parser;

    fn parse(source: &str) -> Vec<ClarityNode> {
        Parser::new(source).parse().unwrap()
    }

    #[test]
    fn detects_basic_reentrancy() {
        let source = r#"
            (define-public (withdraw (amount uint))
                (begin
                    (contract-call? .token transfer tx-sender amount)
                    (map-set balances tx-sender u0)
                )
            )
        "#;

        let nodes = parse(source);
        let findings = detect(&nodes);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].function_name, "withdraw");
        assert!(findings[0].message.contains("map-set"));
    }

    #[test]
    fn no_finding_when_mutation_before_call() {
        let source = r#"
            (define-public (withdraw (amount uint))
                (begin
                    (map-set balances tx-sender u0)
                    (contract-call? .token transfer tx-sender amount)
                )
            )
        "#;

        let nodes = parse(source);
        let findings = detect(&nodes);

        assert!(findings.is_empty());
    }

    #[test]
    fn detects_var_set_after_call() {
        let source = r#"
            (define-public (claim)
                (begin
                    (contract-call? .rewards distribute tx-sender)
                    (var-set total-claimed u100)
                )
            )
        "#;

        let nodes = parse(source);
        let findings = detect(&nodes);

        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("var-set"));
    }

    #[test]
    fn ignores_private_functions() {
        let source = r#"
            (define-private (internal-withdraw (amount uint))
                (begin
                    (contract-call? .token transfer tx-sender amount)
                    (map-set balances tx-sender u0)
                )
            )
        "#;

        let nodes = parse(source);
        let findings = detect(&nodes);

        assert!(findings.is_empty());
    }

    #[test]
    fn detects_nested_mutation_in_if_block() {
        let source = r#"
            (define-public (withdraw (amount uint))
                (begin
                    (contract-call? .token transfer tx-sender amount)
                    (if (> amount u0)
                        (map-set balances tx-sender u0)
                        (ok false)
                    )
                )
            )
        "#;

        let nodes = parse(source);
        let findings = detect(&nodes);

        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn detects_reentrancy_inside_let_block() {
        let source = r#"
            (define-public (withdraw (amount uint))
                (let ((bal (default-to u0 (map-get? balances tx-sender))))
                    (try! (stx-transfer? amount (as-contract tx-sender) tx-sender))
                    (map-set balances tx-sender u0)
                )
            )
        "#;

        let nodes = parse(source);
        let findings = detect(&nodes);

        assert_eq!(findings.len(), 1);
        assert!(findings[0].kind == "Reentrancy");
    }
}