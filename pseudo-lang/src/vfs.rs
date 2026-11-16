use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::context::{analyze_psudo_source, ModuleContextDigest};
use crate::crdt::SymbolMapCRDT;
use crate::zettel::ZettelStore;

use crate::user_docs::starter_helloWorld;

const MAGIC: &[u8; 9] = b"PSUDOBIN\x02";
const VERSION: u32 = 2;
const XOR_MASK: u8 = 0x5B; // '['
const SIGNATURE_SALT: &[u8] = b"PSEUDO_EDGEML_CAPSULE_SIG_2026";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubDefinition {
    pub name: String,
    pub params: Vec<String>,
    pub directive: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StubTraceRecord {
    pub symbol: String,
    pub status: String,
    pub directive: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceCapsule {
    pub project_name: String,
    pub entrypoint: String,
    pub virtual_files: BTreeMap<String, String>,
    #[serde(default)]
    pub crdt: SymbolMapCRDT,
    #[serde(default)]
    pub zettels: ZettelStore,
    #[serde(default)]
    pub stub_traces: BTreeMap<String, Vec<StubTraceRecord>>,

    pub context_cache: Option<ModuleContextDigest>,
    pub revision: u64,
}

impl WorkspaceCapsule {
    pub fn new_empty(project_name: &str) -> Self {
        Self {
            project_name: project_name.to_string(),
            entrypoint: "src/main.psudo".to_string(),
            virtual_files: BTreeMap::new(),
            crdt: SymbolMapCRDT::default(),
            zettels: ZettelStore::new_with_core_defaults(),
            stub_traces: BTreeMap::new(),

            context_cache: None,
            revision: 1,
        }
    }

    pub fn new(project_name: &str) -> crate::utils::Result<Self> {
        let mut capsule = Self::new_empty(project_name);
        // the main file of the pseudo
        let main_src = starter_helloWorld();

        capsule
            .virtual_files
            .insert(String::from("src/main.psudo"), main_src.to_string());

        // the config file toml
        let config_toml = format!(
            r#"[workspace]
name = "{}"
version = "0.1.0"

[sandbox.fs]
root = ".pseudo/sandbox"
allow_external_read = []
deny_write = []
"#,
            project_name
        );
        capsule
            .virtual_files
            .insert("psudo.toml".to_string(), config_toml.to_string());

        capsule.recompute_context()?;

        Ok(capsule)
    }
    // DEPCREATED: only the sensable defaults, only the empty with a working hello world
    //     pub fn new_with_starter(project_name: &str, _template: &str) -> Self {
    //         let mut capsule = Self::new_empty(project_name);
    //         let main_src = starter_blank_psudo();
    //         capsule
    //             .virtual_files
    //             .insert("src/main.psudo".to_string(), main_src.to_string());
    //
    //         let config_toml = r#"[workspace]
    // name = "app"
    // version = "0.1.0"
    //
    // [sandbox.fs]
    // root = ".pseudo/sandbox"
    // allow_external_read = []
    // deny_write = []
    // "#;
    //         capsule
    //             .virtual_files
    //             .insert("psudo.toml".to_string(), config_toml.to_string());
    //
    //         let _ = capsule.recompute_context();
    //         capsule
    //     }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let file_path = if path.is_dir() {
            path.join("psudo.bin")
        } else {
            path.to_path_buf()
        };

        if !file_path.exists() {
            return Err(format!(
                "Workspace capsule file not found: {}",
                file_path.display()
            ));
        }

        let mut file = fs::File::open(&file_path)
            .map_err(|e| format!("Failed to open capsule file {}: {}", file_path.display(), e))?;

        let mut magic_buf = [0u8; 9];
        file.read_exact(&mut magic_buf)
            .map_err(|e| format!("Failed to read capsule magic bytes: {}", e))?;
        let is_v2 = &magic_buf == MAGIC;
        let is_v1 = &magic_buf == b"PSUDOBIN\x01";
        if !is_v2 && !is_v1 {
            return Err("Invalid capsule magic header (not a psudo.bin file)".to_string());
        }

        let mut ver_buf = [0u8; 4];
        file.read_exact(&mut ver_buf)
            .map_err(|e| format!("Failed to read capsule version: {}", e))?;
        let ver = u32::from_le_bytes(ver_buf);

        let mut len_buf = [0u8; 8];
        file.read_exact(&mut len_buf)
            .map_err(|e| format!("Failed to read payload length: {}", e))?;
        let payload_len = u64::from_le_bytes(len_buf) as usize;

        let mut checksum_buf = [0u8; 8];
        file.read_exact(&mut checksum_buf)
            .map_err(|e| format!("Failed to read checksum/signature: {}", e))?;
        let expected_signature = u64::from_le_bytes(checksum_buf);

        let mut encrypted_payload = vec![0u8; payload_len];
        file.read_exact(&mut encrypted_payload)
            .map_err(|e| format!("Failed to read capsule payload: {}", e))?;

        // Unmask XOR
        let decrypted_payload: Vec<u8> = encrypted_payload
            .into_iter()
            .map(|b| b ^ XOR_MASK)
            .collect();

        // Verify cryptographic signature / checksum
        let computed_signature = if ver >= 2 {
            compute_crypto_signature(&decrypted_payload)
        } else {
            compute_simple_checksum(&decrypted_payload)
        };

        if computed_signature != expected_signature {
            return Err(
                "Capsule signature verification failed (corrupted or tampered psudo.bin)"
                    .to_string(),
            );
        }

        let mut capsule: WorkspaceCapsule = serde_json::from_slice(&decrypted_payload)
            .map_err(|e| format!("Failed to deserialize workspace capsule JSON: {}", e))?;

        // Invariant: documentation files must NOT reside inside the virtual capsule
        capsule.virtual_files.retain(|k, _| !k.ends_with(".md"));

        Ok(capsule)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let file_path = if path.is_dir() {
            path.join("psudo.bin")
        } else {
            path.to_path_buf()
        };

        if let Some(parent) = file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut clean_self = self.clone();
        clean_self.virtual_files.retain(|k, _| !k.ends_with(".md"));

        let payload_json = serde_json::to_string(&clean_self)
            .map_err(|e| format!("Failed to serialize workspace capsule: {}", e))?;
        let payload_bytes = payload_json.into_bytes();
        let signature = compute_crypto_signature(&payload_bytes);

        // Mask with XOR
        let encrypted_payload: Vec<u8> = payload_bytes.into_iter().map(|b| b ^ XOR_MASK).collect();

        let mut file_bytes = Vec::new();
        file_bytes.extend_from_slice(MAGIC);
        file_bytes.extend_from_slice(&VERSION.to_le_bytes());
        file_bytes.extend_from_slice(&(encrypted_payload.len() as u64).to_le_bytes());
        file_bytes.extend_from_slice(&signature.to_le_bytes());
        file_bytes.extend_from_slice(&encrypted_payload);

        let tmp_path = file_path.with_extension("bin.tmp");
        fs::write(&tmp_path, &file_bytes)
            .map_err(|e| format!("Failed to write temporary capsule file: {}", e))?;
        fs::rename(&tmp_path, &file_path)
            .map_err(|e| format!("Failed to replace capsule file atomically: {}", e))?;

        Ok(())
    }

    pub fn get_file(&self, virtual_path: &str) -> Option<&str> {
        self.virtual_files.get(virtual_path).map(|s| s.as_str())
    }

    pub fn set_file(&mut self, virtual_path: &str, content: String) -> Result<(), String> {
        self.virtual_files.insert(virtual_path.to_string(), content);
        self.revision += 1;
        let _ = self.recompute_context();
        Ok(())
    }

    pub fn remove_file(&mut self, virtual_path: &str) -> bool {
        let removed = self.virtual_files.remove(virtual_path).is_some();
        if removed {
            self.revision += 1;
            let _ = self.recompute_context();
        }
        removed
    }

    pub fn list_files(&self) -> Vec<String> {
        self.virtual_files.keys().cloned().collect()
    }

    pub fn recompute_context(&mut self) -> Result<(), String> {
        let src = self
            .virtual_files
            .get(&self.entrypoint)
            .cloned()
            .or_else(|| self.virtual_files.get("src/main.psudo").cloned())
            .unwrap_or_else(|| "fn main()\n    return 0".to_string());

        match analyze_psudo_source(&src, &self.entrypoint, None) {
            Ok(digest) => {
                self.context_cache = Some(digest);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn extract_stubs(&self, vpath: &str) -> Result<Vec<StubDefinition>, String> {
        let src = self
            .get_file(vpath)
            .ok_or_else(|| format!("Virtual file '{}' not found", vpath))?;
        let mut lexer = crate::lexer::Lexer::new(src.to_string());
        let tokens = lexer
            .scan_tokens()
            .map_err(|e| format!("Lexer error: {}", e))?;
        let mut parser = crate::parser::Parser::new(tokens);
        let statements = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

        let mut stubs = Vec::new();
        for stmt in &statements {
            if let crate::parser::Statment::ExpressionStatement(
                crate::parser::Expression::Assign(name, val),
            ) = stmt
            {
                if let crate::parser::Expression::Function(params, body) = val.as_ref() {
                    let mut directive = None;
                    match body.as_ref() {
                        crate::parser::Statment::BlockStatement(stmts) => {
                            if stmts.len() == 1 {
                                if let Some(dir) = crate::context::check_single_stmt_stub(&stmts[0])
                                {
                                    directive = Some(dir);
                                }
                            }
                        }
                        other => {
                            if let Some(dir) = crate::context::check_single_stmt_stub(other) {
                                directive = Some(dir);
                            }
                        }
                    }
                    if let Some(dir) = directive {
                        stubs.push(StubDefinition {
                            name: name.clone(),
                            params: params.clone(),
                            directive: dir,
                        });
                    }
                }
            }
        }
        Ok(stubs)
    }

    pub fn format_stubs_pir(&self, stubs: &[StubDefinition]) -> String {
        let mut out = String::new();
        for s in stubs {
            out.push_str(&format!(
                "f {} ({}) [ @stub \"{}\" ]\n",
                s.name,
                s.params.join(" "),
                s.directive
            ));
        }
        out.trim_end().to_string()
    }

    pub fn format_stubs_human(&self, stubs: &[StubDefinition]) -> String {
        let mut out = String::new();
        for s in stubs {
            out.push_str(&format!(
                "fn {}({})\n    @stub(\"{}\")\n\n",
                s.name,
                s.params.join(", "),
                s.directive
            ));
        }
        out.trim_end().to_string()
    }

    pub fn record_stub_trace(&mut self, symbol: &str, status: &str, directive: &str) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let record = StubTraceRecord {
            symbol: symbol.to_string(),
            status: status.to_string(),
            directive: directive.to_string(),
            timestamp_ms: now_ms,
        };
        self.stub_traces
            .entry(symbol.to_string())
            .or_default()
            .push(record);
    }

    pub fn upsert_stub(
        &mut self,
        vpath: &str,
        name: &str,
        params: &[String],
        directive: &str,
    ) -> Result<(), String> {
        let mut src = self
            .get_file(vpath)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let param_str = params.join(", ");
        let stub_fn = format!(
            "\nfn {}({})\n    @stub(\"{}\")\n",
            name, param_str, directive
        );
        let sig_prefix = format!("fn {}(", name);

        if let Some(pos) = src.find(&sig_prefix) {
            let rest = &src[pos..];
            let end_pos = rest[4..]
                .find("\nfn ")
                .map(|p| pos + 4 + p)
                .unwrap_or(src.len());
            src.replace_range(pos..end_pos, &stub_fn);
        } else {
            if !src.ends_with('\n') {
                src.push('\n');
            }
            src.push_str(&stub_fn);
        }
        let hash = SymbolMapCRDT::compute_hash(&stub_fn);
        self.crdt.apply_patch(
            name.to_string(),
            self.revision + 1,
            hash,
            vec![],
            stub_fn,
            false,
        );

        self.set_file(vpath, src)?;
        self.record_stub_trace(name, "DECLARED", directive);
        self.revision += 1;
        self.recompute_context()?;
        Ok(())
    }

    pub fn merge_delta(&mut self, other: &WorkspaceCapsule) -> Result<(), String> {
        self.crdt.merge(&other.crdt);
        self.zettels.merge(&other.zettels);

        for (vpath, content) in &other.virtual_files {
            if !self.virtual_files.contains_key(vpath) {
                self.virtual_files.insert(vpath.clone(), content.clone());
            } else if vpath == &self.entrypoint || vpath == "src/main.psudo" {
                let current_content = self.virtual_files.get(vpath).cloned().unwrap_or_default();
                let mut merged = current_content.clone();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("fn ") {
                        let fn_name = trimmed
                            .strip_prefix("fn ")
                            .unwrap_or("")
                            .split('(')
                            .next()
                            .unwrap_or("")
                            .trim();
                        if !fn_name.is_empty()
                            && !current_content.contains(&format!("fn {}(", fn_name))
                        {
                            if let Some(node) = other.crdt.symbols.get(fn_name) {
                                if !node.is_tombstone {
                                    merged.push('\n');
                                    merged.push_str(&node.body);
                                }
                            }
                        }
                    }
                }
                self.virtual_files.insert(vpath.clone(), merged);
            }
        }

        self.revision = std::cmp::max(self.revision, other.revision) + 1;
        let _ = self.recompute_context();
        Ok(())
    }
}

fn compute_simple_checksum(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 14695981039346656037; // FNV offset basis
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn compute_crypto_signature(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for &sb in SIGNATURE_SALT {
        hash ^= sb as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    // Mix with xxhash64
    xxhash_rust::xxh64::xxh64(bytes, hash)
}

/// Resolves a capsule path and virtual path from the given argument.
/// Precedence:
/// 1. Explicit arg (directory with psudo.bin, .bin file, or path::vpath)
/// 2. Saved .pseudo/capsule.path (from `vfs use`)
/// 3. CWD + ancestors search for psudo.bin
/// 4. Error with actionable message
pub fn resolve_capsule_and_vpath(arg: &str) -> Result<(PathBuf, String), String> {
    resolve_capsule_and_vpath_with_cwd(arg, None)
}

/// Internal resolution with optional CWD override for saved path lookup.
/// If `arg` is ".", it's treated as "no explicit arg" unless `explicit_dot` is true.
pub fn resolve_capsule_and_vpath_with_cwd(
    arg: &str,
    cwd_override: Option<&Path>,
) -> Result<(PathBuf, String), String> {
    let path = PathBuf::from(arg);

    // Check if this is an explicit arg (not the default ".")
    let is_explicit = arg != ".";

    // 1. Explicit arg: directory with psudo.bin
    if is_explicit && path.is_dir() {
        let bin_path = path.join("psudo.bin");
        if bin_path.exists() {
            return Ok((bin_path, "src/main.psudo".to_string()));
        }
        return Err(format!(
            "No psudo.bin found in directory {}",
            path.display()
        ));
    }

    // 1. Explicit arg: .bin file
    if is_explicit && arg.ends_with(".bin") {
        if path.exists() {
            return Ok((path, "src/main.psudo".to_string()));
        }
        return Err(format!("Capsule file not found: {}", path.display()));
    }

    // 1. Explicit arg: path::vpath separator
    if is_explicit {
        if let Some(pos) = arg.find("::") {
            let bin_part = &arg[..pos];
            let vpath_part = &arg[pos + 2..];
            let bin_path = PathBuf::from(bin_part);
            if bin_path.exists() {
                return Ok((bin_path, vpath_part.to_string()));
            }
            return Err(format!("Capsule file not found: {}", bin_path.display()));
        }
    }

    // 2. Saved .pseudo/capsule.path (from `vfs use`)
    let cwd = cwd_override.unwrap_or_else(|| Path::new("."));
    if let Some(saved) = load_saved_capsule_path(cwd) {
        return Ok((saved, arg.to_string()));
    }

    // 3. CWD + ancestors search for psudo.bin
    let mut curr = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        let candidate = curr.join("psudo.bin");
        if candidate.exists() {
            return Ok((candidate, arg.to_string()));
        }
        if !curr.pop() {
            break;
        }
    }

    // 4. Error with actionable message (no auto-create)
    Err("No psudo.bin found. Run 'psudo init' to create one, or 'psudo vfs use <PATH>' to point to existing capsule.".to_string())
}

pub fn sanitize_cargo_package_name(raw: &str) -> String {
    let trimmed = raw.trim_start_matches(|c: char| !c.is_alphabetic() && c != '_');
    let sanitized: String = trimmed
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "psudo_app".to_string()
    } else {
        sanitized
    }
}

pub fn pack_directory_to_capsule(
    dir: &Path,
    out_bin: &Path,
    explicit_name: Option<&str>,
) -> Result<(), String> {
    if !dir.is_dir() {
        return Err(format!("{} is not a directory", dir.display()));
    }

    let mut capsule = WorkspaceCapsule::new_empty("app");

    // Recursively walk directory and add non-hidden, non-target files
    fn visit_dirs(dir: &Path, base: &Path, capsule: &mut WorkspaceCapsule) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                visit_dirs(&path, base, capsule)?;
            } else if path.is_file() {
                if let Ok(rel) = path.strip_prefix(base) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    if rel_str != "psudo.bin" {
                        if let Ok(content) = fs::read_to_string(&path) {
                            capsule.virtual_files.insert(rel_str, content);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    visit_dirs(dir, dir, &mut capsule)?;

    let mut pkg_name = None;
    if let Some(toml_content) = capsule.virtual_files.get("psudo.toml") {
        for line in toml_content.lines() {
            let line = line.trim();
            if line.starts_with("name") || line.starts_with("name =") {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    let val = parts[1].trim().trim_matches('"').trim_matches('\'');
                    if !val.is_empty() {
                        pkg_name = Some(val.to_string());
                    }
                }
            }
        }
    }

    let raw_name = pkg_name
        .or_else(|| explicit_name.map(ToString::to_string))
        .unwrap_or_else(|| {
            dir.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("app")
                .to_string()
        });

    capsule.project_name = sanitize_cargo_package_name(&raw_name);

    let _ = capsule.recompute_context();
    capsule.save_to_file(out_bin)?;
    Ok(())
}

pub fn unpack_capsule_to_directory(bin_path: &Path, out_dir: &Path) -> Result<(), String> {
    let capsule = WorkspaceCapsule::load_from_file(bin_path)?;
    if !out_dir.exists() {
        fs::create_dir_all(out_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;
    }

    for (vpath, content) in &capsule.virtual_files {
        let target_path = out_dir.join(vpath);
        if let Some(parent) = target_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&target_path, content).map_err(|e| {
            format!(
                "Failed to write virtual file {}: {}",
                target_path.display(),
                e
            )
        })?;
    }
    Ok(())
}
/// Saves the capsule path to .pseudo/capsule.path relative to the given working directory.
/// The path is stored as a relative path from the working directory to the capsule.
pub fn save_capsule_path(cwd: &Path, capsule_path: &Path) -> Result<(), String> {
    let pseudo_dir = cwd.join(".pseudo");
    if !pseudo_dir.exists() {
        fs::create_dir_all(&pseudo_dir)
            .map_err(|e| format!("Failed to create .pseudo directory: {}", e))?;
    }

    // Normalize paths to avoid Windows extended-length prefix issues
    let cwd_normalized = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    let capsule_normalized =
        std::fs::canonicalize(capsule_path).unwrap_or_else(|_| capsule_path.to_path_buf());

    // Compute relative path from cwd to capsule
    let relative = pathdiff::diff_paths(&capsule_normalized, &cwd_normalized).ok_or_else(|| {
        format!(
            "Cannot compute relative path from {} to {}",
            cwd_normalized.display(),
            capsule_normalized.display()
        )
    })?;

    // Ensure the relative path doesn't have Windows extended prefix
    let relative_str = relative.to_string_lossy();
    let relative_clean = relative_str.strip_prefix(r"\?").unwrap_or(&relative_str);

    let path_file = pseudo_dir.join("capsule.path");
    fs::write(&path_file, relative_clean.as_bytes())
        .map_err(|e| format!("Failed to write capsule.path: {}", e))
}

/// Loads the saved capsule path by searching for .pseudo/capsule.path in CWD and ancestors.
/// Returns the absolute path to the capsule if found and valid.
pub fn load_saved_capsule_path(cwd: &Path) -> Option<PathBuf> {
    let mut curr = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    loop {
        let path_file = curr.join(".pseudo").join("capsule.path");
        if path_file.exists() {
            let relative = fs::read_to_string(&path_file).ok()?;
            let relative = relative.trim();
            if relative.is_empty() {
                return None;
            }
            let capsule_path = curr.join(relative);
            if capsule_path.exists() {
                return Some(capsule_path);
            } else {
                return None;
            }
        }
        if !curr.pop() {
            break;
        }
    }
    None
}
