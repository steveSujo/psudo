use crate::ir::TsNode;
use tree_sitter::Parser;

pub fn rust_to_ir(rust_code: &str) -> Result<TsNode, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::language())
        .map_err(|e| format!("Failed to set tree-sitter-rust language: {}", e))?;

    let tree = parser
        .parse(rust_code.as_bytes(), None)
        .ok_or_else(|| "Failed to parse Rust source code".to_string())?;

    let root_node = tree.root_node();
    let source_bytes = rust_code.as_bytes();
    let ts_node = TsNode::from_tree_sitter_node(root_node, source_bytes);

    Ok(ts_node)
}
