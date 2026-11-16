use pseudo_transpiler::{
    psudo_to_ir, ir_to_rust, rust_to_ir, ir_to_psudo,
    transpile_psudo_to_rust, transpile_rust_to_psudo,
};

#[test]
fn test_ir_s_expression_format() {
    let source = "fn add(a, b)\n    return a + b\n";
    let ir = psudo_to_ir(source).unwrap();
    let sexpr = ir.to_sexpr();
    assert!(sexpr.contains("source_file"));
    assert!(sexpr.contains("function_item"));
}

#[test]
fn test_forward_transpile_psudo_to_rust() {
    let source = "fn calc(x)\n    let y = x + 10\n    print y\n";
    let rust_code = transpile_psudo_to_rust(source).unwrap();
    assert!(rust_code.contains("pub fn calc(x: PsudoVal)"));
    assert!(rust_code.contains("println!"));
}

#[test]
fn test_reverse_transpile_rust_to_psudo() {
    let rust_code = "pub fn add(a: PsudoVal) -> PsudoVal {\n    let mut x = a + 10;\n    println!(\"{}\", x);\n}\n";
    let psudo_code = transpile_rust_to_psudo(rust_code).unwrap();
    assert!(psudo_code.contains("fn add(a)"));
    assert!(psudo_code.contains("let x ="));
}
