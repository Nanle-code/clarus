/// Represents a single node in the Clarity AST

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ClarityNode {
    /// A list of nodes — e.g. (map-set balances tx-sender amount)
    List(Vec<ClarityNode>, usize),  // usize = line number

    /// An atomic value — a symbol, number, string, or keyword
    Atom(String, usize),  // usize = line number
}

#[allow(dead_code)]
impl ClarityNode {
    /// Get the line number of this node
    pub fn line(&self) -> usize {
        match self {
            ClarityNode::List(_, line) => *line,
            ClarityNode::Atom(_, line) => *line,
        }
    }

    /// Get the atom value if this node is an Atom
    pub fn as_atom(&self) -> Option<&str> {
        match self {
            ClarityNode::Atom(val, _) => Some(val.as_str()),
            _ => None,
        }
    }

    /// Get the children if this node is a List
    pub fn as_list(&self) -> Option<&Vec<ClarityNode>> {
        match self {
            ClarityNode::List(children, _) => Some(children),
            _ => None,
        }
    }

    /// Check if this node is a List whose first element matches a given name
    /// e.g. node.is_form("define-public") 
    pub fn is_form(&self, name: &str) -> bool {
        match self.as_list() {
            Some(children) => {
                children.first()
                    .and_then(|n| n.as_atom())
                    .map(|s| s == name)
                    .unwrap_or(false)
            }
            None => false,
        }
    }
}