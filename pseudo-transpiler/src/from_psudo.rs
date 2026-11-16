use crate::ir::TsNode;
use psudo::lexer::Lexer;
use psudo::parser::{Expression, Parser, Statment};
use psudo::tokens::TokenType;

pub fn psudo_to_ir(source: &str) -> Result<TsNode, String> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().map_err(|e| format!("Lexer error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let statements = parser.parse().map_err(|e| format!("Parser error: {:?}", e))?;

    let mut root = TsNode::new("source_file");
    for stmt in statements {
        root = root.push_child(statement_to_ir(&stmt));
    }
    Ok(root)
}

fn statement_to_ir(stmt: &Statment) -> TsNode {
    match stmt {
        Statment::ExpressionStatement(expr) => match expr {
            Expression::Function(params, body) => {
                let func_node = TsNode::new("function_item");
                let name_node = TsNode::new("identifier").with_value("anon");
                let mut func_node = func_node.push_child_with_field("name", name_node);

                let mut params_node = TsNode::new("parameters");
                for p in params {
                    params_node = params_node.push_child(
                        TsNode::new("parameter")
                            .push_child(TsNode::new("identifier").with_value(p))
                    );
                }
                func_node = func_node.push_child_with_field("parameters", params_node);
                func_node = func_node.push_child_with_field("body", statement_to_ir(body));
                func_node
            }
            Expression::Assign(name, boxed_expr) if matches!(**boxed_expr, Expression::Function(..)) => {
                if let Expression::Function(params, body) = &**boxed_expr {
                    let func_node = TsNode::new("function_item");
                    let name_node = TsNode::new("identifier").with_value(name);
                    let mut func_node = func_node.push_child_with_field("name", name_node);

                    let mut params_node = TsNode::new("parameters");
                    for p in params {
                        params_node = params_node.push_child(
                            TsNode::new("parameter")
                                .push_child(TsNode::new("identifier").with_value(p))
                        );
                    }
                    func_node = func_node.push_child_with_field("parameters", params_node);
                    func_node = func_node.push_child_with_field("body", statement_to_ir(body));
                    func_node
                } else {
                    TsNode::new("expression_statement").push_child(expression_to_ir(expr))
                }
            }
            _ => TsNode::new("expression_statement")
                .push_child(expression_to_ir(expr)),
        },
        Statment::PrintStatement(expr) => {
            TsNode::new("macro_invocation")
                .with_value("println!")
                .push_child(TsNode::new("arguments").push_child(expression_to_ir(expr)))
        }
        Statment::LetStatement(token, expr) => {
            let let_node = TsNode::new("let_declaration");
            let pat_node = TsNode::new("identifier").with_value(&token.text);
            let val_node = expression_to_ir(expr);
            let let_node = let_node.push_child_with_field("pattern", pat_node);
            let_node.push_child_with_field("value", val_node)
        }
        Statment::IfStatement(cond, then_b, else_b) => {
            let if_node = TsNode::new("if_expression");
            let if_node = if_node.push_child_with_field("condition", expression_to_ir(cond));
            let if_node = if_node.push_child_with_field("consequence", statement_to_ir(then_b));
            if let Some(e) = else_b {
                if_node.push_child_with_field("alternative", statement_to_ir(e))
            } else {
                if_node
            }
        }
        Statment::WhileStatement(cond, body) => {
            let while_node = TsNode::new("while_expression");
            let while_node = while_node.push_child_with_field("condition", expression_to_ir(cond));
            while_node.push_child_with_field("body", statement_to_ir(body))
        }
        Statment::BlockStatement(stmts) => {
            let mut block = TsNode::new("block");
            for s in stmts {
                block = block.push_child(statement_to_ir(s));
            }
            block
        }
        Statment::TestStatement(name, body) => {
            let test_node = TsNode::new("test_item").with_value(name);
            test_node.push_child_with_field("body", statement_to_ir(body))
        }
        _ => TsNode::new("expression_statement"),
    }
}

fn expression_to_ir(expr: &Expression) -> TsNode {
    match expr {
        Expression::Binary(left, op, right) => {
            let bin_node = TsNode::new("binary_expression").with_value(&op.text);
            let left_node = expression_to_ir(left);
            let right_node = expression_to_ir(right);
            let bin_node = bin_node.push_child_with_field("left", left_node);
            bin_node.push_child_with_field("right", right_node)
        }
        Expression::Literal(token) => {
            let kind = match token.token_type {
                TokenType::NUMBER => "integer_literal",
                TokenType::STRING => "string_literal",
                TokenType::TRUE | TokenType::FALSE => "boolean_literal",
                _ => "identifier",
            };
            TsNode::new(kind).with_value(&token.text)
        }
        Expression::Call(name, args) => {
            let call = TsNode::new("call_expression");
            let func_node = TsNode::new("identifier").with_value(name);
            let call = call.push_child_with_field("function", func_node);
            let mut args_node = TsNode::new("arguments");
            for arg in args {
                args_node = args_node.push_child(expression_to_ir(arg));
            }
            call.push_child(args_node)
        }
        Expression::Return(Some(val)) => {
            TsNode::new("return_expression")
                .push_child(expression_to_ir(val))
        }
        Expression::Return(None) => {
            TsNode::new("return_expression")
        }
        _ => TsNode::new("identifier").with_value("unknown"),
    }
}
