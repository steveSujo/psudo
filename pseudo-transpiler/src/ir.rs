use serde::{Deserialize, Serialize};

// Using a flat tree-sitter-shaped TsNode lets both Rust and `.psudo` ASTs share one
// round-trip IR without dragging full `syn` / `rustc` parser trees into the loop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TsNode {
    pub kind: String,
    pub field: Option<String>,
    pub value: Option<String>,
    pub children: Vec<TsNode>,
    pub span: (usize, usize),
}

impl TsNode {
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
            field: None,
            value: None,
            children: Vec::new(),
            span: (0, 0),
        }
    }

    pub fn with_field(mut self, field: &str) -> Self {
        self.field = Some(field.to_string());
        self
    }

    pub fn with_value(mut self, val: &str) -> Self {
        self.value = Some(val.to_string());
        self
    }

    pub fn push_child(mut self, child: TsNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn push_child_with_field(mut self, field: &str, mut child: TsNode) -> Self {
        child.field = Some(field.to_string());
        self.children.push(child);
        self
    }

    pub fn from_tree_sitter_node(node: tree_sitter::Node, source_bytes: &[u8]) -> Self {
        let kind = node.kind().to_string();
        let span = (node.start_byte(), node.end_byte());
        let text = if node.child_count() == 0 || node.is_named() {
            node.utf8_text(source_bytes).ok().map(|s| s.to_string())
        } else {
            None
        };

        let mut children = Vec::new();
        let mut cursor = node.walk();
        for (idx, child_node) in node.children(&mut cursor).enumerate() {
            let mut child_ts = Self::from_tree_sitter_node(child_node, source_bytes);
            if let Some(field_name) = node.field_name_for_child(idx as u32) {
                child_ts.field = Some(field_name.to_string());
            }
            children.push(child_ts);
        }

        Self {
            kind,
            field: None,
            value: text,
            children,
            span,
        }
    }

    pub fn to_sexpr(&self) -> String {
        let mut out = String::new();
        self.to_sexpr_fmt(&mut out, 0);
        out
    }

    fn to_sexpr_fmt(&self, out: &mut String, indent: usize) {
        let prefix = "  ".repeat(indent);
        out.push_str(&prefix);
        out.push('(');
        out.push_str(&self.kind);
        if let Some(f) = &self.field {
            out.push_str(" :field ");
            out.push_str(f);
        }
        if let Some(v) = &self.value {
            out.push_str(" :value \"");
            out.push_str(v);
            out.push('"');
        }
        if !self.children.is_empty() {
            out.push('\n');
            for child in &self.children {
                child.to_sexpr_fmt(out, indent + 1);
                out.push('\n');
            }
            out.push_str(&prefix);
        }
        out.push(')');
    }
}
