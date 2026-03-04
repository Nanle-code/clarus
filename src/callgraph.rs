use std::collections::{HashMap, HashSet};
use crate::ast::ClarityNode;
use crate::registry::Registry;

/// A single edge in the call graph
/// represents one function calling another
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallEdge {
    pub caller_contract: String,
    pub caller_function: String,
    pub callee_contract: String,
    pub callee_function: String,
    pub line: usize,
}

/// The full call graph for a project
#[derive(Debug, Default)]
pub struct CallGraph {
    /// All edges in the graph
    pub edges: Vec<CallEdge>,

    /// Quick lookup: given a (contract, function) what does it call?
    pub outgoing: HashMap<(String, String), Vec<CallEdge>>,

    /// Quick lookup: given a (contract, function) who calls it?
    pub incoming: HashMap<(String, String), Vec<CallEdge>>,
}

#[allow(dead_code)]
impl CallGraph {
    pub fn new() -> Self {
        CallGraph {
            edges: vec![],
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
        }
    }

    /// Add an edge to the graph
    fn add_edge(&mut self, edge: CallEdge) {
        let caller_key = (edge.caller_contract.clone(), edge.caller_function.clone());
        let callee_key = (edge.callee_contract.clone(), edge.callee_function.clone());

        self.outgoing
            .entry(caller_key)
            .or_default()
            .push(edge.clone());

        self.incoming
            .entry(callee_key)
            .or_default()
            .push(edge.clone());

        self.edges.push(edge);
    }

    /// Get all functions that a given function calls
    pub fn calls_made_by(&self, contract: &str, function: &str) -> Vec<&CallEdge> {
        let key = (contract.to_string(), function.to_string());
        self.outgoing.get(&key)
            .map(|edges| edges.iter().collect())
            .unwrap_or_default()
    }

    /// Get all functions that call a given function
    pub fn callers_of(&self, contract: &str, function: &str) -> Vec<&CallEdge> {
        let key = (contract.to_string(), function.to_string());
        self.incoming.get(&key)
            .map(|edges| edges.iter().collect())
            .unwrap_or_default()
    }

    /// Detect cycles in the call graph — circular calls are dangerous
    pub fn find_cycles(&self) -> Vec<Vec<(String, String)>> {
        let mut cycles = vec![];
        let mut visited = HashSet::new();
        let mut stack = vec![];

        // collect all unique nodes
        let nodes: HashSet<(String, String)> = self.edges.iter()
            .flat_map(|e| vec![
                (e.caller_contract.clone(), e.caller_function.clone()),
                (e.callee_contract.clone(), e.callee_function.clone()),
            ])
            .collect();

        for node in &nodes {
            if !visited.contains(node) {
                self.dfs_cycles(node, &mut visited, &mut stack, &mut cycles);
            }
        }

        cycles
    }

    /// Depth first search to find cycles
    fn dfs_cycles(
        &self,
        node: &(String, String),
        visited: &mut HashSet<(String, String)>,
        stack: &mut Vec<(String, String)>,
        cycles: &mut Vec<Vec<(String, String)>>,
    ) {
        visited.insert(node.clone());
        stack.push(node.clone());

        let key = node.clone();
        if let Some(edges) = self.outgoing.get(&key) {
            for edge in edges {
                let callee = (edge.callee_contract.clone(), edge.callee_function.clone());

                if !visited.contains(&callee) {
                    self.dfs_cycles(&callee, visited, stack, cycles);
                } else if stack.contains(&callee) {
                    // found a cycle — extract it from the stack
                    let cycle_start = stack.iter().position(|n| n == &callee).unwrap();
                    let cycle = stack[cycle_start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }

        stack.pop();
    }

    /// Get a summary of the call graph for display
    pub fn summary(&self) -> CallGraphSummary {
        let contracts: HashSet<String> = self.edges.iter()
            .flat_map(|e| vec![
                e.caller_contract.clone(),
                e.callee_contract.clone(),
            ])
            .collect();

        let functions: HashSet<(String, String)> = self.edges.iter()
            .flat_map(|e| vec![
                (e.caller_contract.clone(), e.caller_function.clone()),
                (e.callee_contract.clone(), e.callee_function.clone()),
            ])
            .collect();

        CallGraphSummary {
            total_contracts: contracts.len(),
            total_functions: functions.len(),
            total_edges: self.edges.len(),
            cycles: self.find_cycles(),
        }
    }
}

#[derive(Debug)]
pub struct CallGraphSummary {
    pub total_contracts: usize,
    pub total_functions: usize,
    pub total_edges: usize,
    pub cycles: Vec<Vec<(String, String)>>,
}

/// Build a call graph from the registry
pub fn build(registry: &Registry) -> CallGraph {
    let mut graph = CallGraph::new();

    for contract in registry.all() {
        for node in &contract.ast {
            if node.is_form("define-public")
                || node.is_form("define-private")
                || node.is_form("define-read-only")
            {
                if let Some(func_name) = extract_function_name(node) {
                    extract_calls(
                        node,
                        &contract.name,
                        &func_name,
                        &mut graph,
                    );
                }
            }
        }
    }

    graph
}

/// Walk a function AST and extract all contract-call? expressions
fn extract_calls(
    node: &ClarityNode,
    caller_contract: &str,
    caller_function: &str,
    graph: &mut CallGraph,
) {
    if let ClarityNode::List(children, line) = node {
        let head = children.first().and_then(|n| n.as_atom());

        if head == Some("contract-call?") {
            // (contract-call? .contract-name function-name ...args)
            if let (Some(callee_contract), Some(callee_function)) =
                (children.get(1), children.get(2))
            {
                let callee_contract_name = callee_contract
                    .as_atom()
                    .unwrap_or("unknown")
                    .trim_start_matches('.')
                    .to_string();

                let callee_function_name = callee_function
                    .as_atom()
                    .unwrap_or("unknown")
                    .to_string();

                graph.add_edge(CallEdge {
                    caller_contract: caller_contract.to_string(),
                    caller_function: caller_function.to_string(),
                    callee_contract: callee_contract_name,
                    callee_function: callee_function_name,
                    line: *line,
                });
            }
        }

        // recurse into all children
        for child in children {
            extract_calls(child, caller_contract, caller_function, graph);
        }
    }
}

/// Extract function name from a define-public/private/read-only node
fn extract_function_name(node: &ClarityNode) -> Option<String> {
    let children = node.as_list()?;
    let sig = children.get(1)?;

    match sig {
        ClarityNode::List(sig_children, _) => {
            sig_children.first()?.as_atom().map(|s| s.to_string())
        }
        ClarityNode::Atom(name, _) => Some(name.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_contracts;
    use crate::registry::Registry;
    use std::path::Path;

    fn build_graph() -> CallGraph {
        let sources = load_contracts(Path::new("tests/contracts")).unwrap();
        let registry = Registry::build(sources).unwrap();
        build(&registry)
    }

    #[test]
    fn builds_without_panic() {
        let graph = build_graph();
        // vulnerable.clar and safe.clar both have contract-call? expressions
        // so we should have edges
        println!("Edges found: {}", graph.edges.len());
    }

    #[test]
    fn summary_has_correct_structure() {
        let graph = build_graph();
        let summary = graph.summary();

        println!("Contracts : {}", summary.total_contracts);
        println!("Functions : {}", summary.total_functions);
        println!("Edges     : {}", summary.total_edges);
        println!("Cycles    : {}", summary.cycles.len());
    }
}