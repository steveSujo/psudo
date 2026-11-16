use std::path::Path;

pub mod ir;
pub mod from_psudo;
pub mod to_rust;
pub mod from_rust;
pub mod to_psudo;

pub use ir::TsNode;
pub use from_psudo::psudo_to_ir;
pub use to_rust::ir_to_rust;
pub use from_rust::rust_to_ir;
pub use to_psudo::ir_to_psudo;

pub use psudo::blackbox::*;
pub use psudo::codegen::*;
pub use psudo::context::*;

pub fn transpile_psudo_to_rust(source: &str) -> Result<String, String> {
    let ir = psudo_to_ir(source)?;
    ir_to_rust(&ir)
}

pub fn transpile_rust_to_psudo(source: &str) -> Result<String, String> {
    let ir = rust_to_ir(source)?;
    ir_to_psudo(&ir)
}

pub fn roundtrip_psudo(source: &str) -> Result<String, String> {
    let ir = psudo_to_ir(source)?;
    ir_to_psudo(&ir)
}

pub fn roundtrip_rust(source: &str) -> Result<String, String> {
    let ir = rust_to_ir(source)?;
    ir_to_rust(&ir)
}

pub fn compile_psudo_file_to_binary(
    input: &Path,
    out_dir: &Path,
    release: bool,
) -> Result<RustArtifactReport, String> {
    build_rust_binary_artifact(input, out_dir, release, false)
}
