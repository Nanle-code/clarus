use crate::ast::ClarityNode;

pub struct Parser {
    source: Vec<char>,
    pos: usize,
    line: usize,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        Parser {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
        }
    }

    /// Main entry point — parse the entire source file
    /// Returns a list of top-level nodes
    pub fn parse(&mut self) -> Result<Vec<ClarityNode>, String> {
        let mut nodes = vec![];

        loop {
            self.skip_whitespace_and_comments();

            if self.pos >= self.source.len() {
                break;
            }

            let node = self.parse_node()?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    /// Parse a single node — either a List or an Atom
    fn parse_node(&mut self) -> Result<ClarityNode, String> {
        self.skip_whitespace_and_comments();

        if self.pos >= self.source.len() {
            return Err("Unexpected end of input".to_string());
        }

        match self.current_char() {
            '(' => self.parse_list(),
            '"' => self.parse_string(),
            _   => self.parse_atom(),
        }
    }

    /// Parse a list — everything between ( and )
    fn parse_list(&mut self) -> Result<ClarityNode, String> {
        let line = self.line;
        self.advance(); // consume '('

        let mut children = vec![];

        loop {
            self.skip_whitespace_and_comments();

            if self.pos >= self.source.len() {
                return Err(format!("Unclosed parenthesis opened at line {}", line));
            }

            if self.current_char() == ')' {
                self.advance(); // consume ')'
                break;
            }

            let child = self.parse_node()?;
            children.push(child);
        }

        Ok(ClarityNode::List(children, line))
    }

    /// Parse an atom — any token that isn't a list or string
    fn parse_atom(&mut self) -> Result<ClarityNode, String> {
        let line = self.line;
        let mut value = String::new();

        while self.pos < self.source.len() {
            let ch = self.current_char();

            // atoms end at whitespace, parens, or end of input
            if ch.is_whitespace() || ch == '(' || ch == ')' {
                break;
            }

            value.push(ch);
            self.advance();
        }

        if value.is_empty() {
            return Err(format!("Empty atom at line {}", line));
        }

        Ok(ClarityNode::Atom(value, line))
    }

    /// Parse a string literal — handles escaped quotes
    fn parse_string(&mut self) -> Result<ClarityNode, String> {
        let line = self.line;
        self.advance(); // consume opening '"'

        let mut value = String::from("\"");

        while self.pos < self.source.len() {
            let ch = self.current_char();
            self.advance();

            if ch == '\\' && self.pos < self.source.len() {
                // escaped character
                value.push(ch);
                value.push(self.current_char());
                self.advance();
                continue;
            }

            value.push(ch);

            if ch == '"' {
                break; // closing quote
            }
        }

        Ok(ClarityNode::Atom(value, line))
    }

    /// Skip whitespace and Clarity comments (;; ...)
    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.source.len() {
            let ch = self.current_char();

            if ch == '\n' {
                self.line += 1;
                self.pos += 1;
            } else if ch.is_whitespace() {
                self.pos += 1;
            } else if ch == ';' {
                // Clarity comments start with ;; — skip to end of line
                while self.pos < self.source.len() && self.current_char() != '\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn current_char(&self) -> char {
        self.source[self.pos]
    }

    fn advance(&mut self) {
        self.pos += 1;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_list() {
        let source = "(map-set balances tx-sender u100)";
        let mut parser = Parser::new(source);
        let nodes = parser.parse().unwrap();

        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].is_form("map-set"));
    }

    #[test]
    fn test_parse_ignores_comments() {
        let source = "
            ;; this is a comment
            (define-public (foo) ;; inline comment
                (ok true)
            )
        ";
        let mut parser = Parser::new(source);
        let nodes = parser.parse().unwrap();

        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].is_form("define-public"));
    }

    #[test]
    fn test_line_numbers_tracked() {
        let source = "\n\n(map-set balances tx-sender u100)";
        let mut parser = Parser::new(source);
        let nodes = parser.parse().unwrap();

        assert_eq!(nodes[0].line(), 3);
    }
}