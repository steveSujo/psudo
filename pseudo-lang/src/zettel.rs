use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use xxhash_rust::xxh64::xxh64;

// Splitting context into Core, Concept, Symbol, and Episode zettels lets me pin
// language invariants while paging only the relevant symbol notes into a tight context window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZettelKind {
    Core,
    Concept,
    Symbol,
    Episode,
}

impl ZettelKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZettelKind::Core => "CORE",
            ZettelKind::Concept => "CONCEPT",
            ZettelKind::Symbol => "SYMBOL",
            ZettelKind::Episode => "EPISODE",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "CORE" => ZettelKind::Core,
            "CONCEPT" => ZettelKind::Concept,
            "EPISODE" => ZettelKind::Episode,
            _ => ZettelKind::Symbol,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZettelRecord {
    pub id: String,
    pub kind: ZettelKind,
    pub scope: String,
    pub links: Vec<String>,
    pub fact: String,
    pub content_hash: u64,
    pub lamport_tick: u64,
    pub is_tombstone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZettelDeltaOp {
    Noop(String),
    Inserted(ZettelRecord),
    Updated(ZettelRecord),
    Tombstoned(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZettelStore {
    pub records: HashMap<String, ZettelRecord>,
}

impl Default for ZettelStore {
    fn default() -> Self {
        Self::new_with_core_defaults()
    }
}

impl ZettelStore {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    pub fn new_with_core_defaults() -> Self {
        let mut store = Self::new();
        store.insert_raw(ZettelRecord {
            id: "core_sandbox_fs".into(),
            kind: ZettelKind::Core,
            scope: "std.fs".into(),
            links: vec![],
            fact: "All @rd/@wt/@mk operations confine strictly to .pseudo/sandbox/".into(),
            content_hash: xxh64(b"core_sandbox_fs", 0),
            lamport_tick: 0,
            is_tombstone: false,
        });
        store.insert_raw(ZettelRecord {
            id: "core_pir_isomorphism".into(),
            kind: ZettelKind::Core,
            scope: "pIR".into(),
            links: vec![],
            fact: "pIR AST lowering maintains 1:1 whitespace-normalized isomorphism with .psudo"
                .into(),
            content_hash: xxh64(b"core_pir_isomorphism", 0),
            lamport_tick: 0,
            is_tombstone: false,
        });
        store
    }

    pub fn insert_raw(&mut self, record: ZettelRecord) {
        self.records.insert(record.id.clone(), record);
    }

    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(store) = serde_json::from_str::<ZettelStore>(&content) {
                    return store;
                }
            }
        }
        Self::new_with_core_defaults()
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn compute_record_hash(kind: ZettelKind, scope: &str, links: &[String], fact: &str) -> u64 {
        let payload = format!("{:?}|{}|{:?}|{}", kind, scope, links, fact);
        xxh64(payload.as_bytes(), 0)
    }

    pub fn merge(&mut self, other: &ZettelStore) {
        for (id, other_rec) in &other.records {
            if let Some(existing) = self.records.get_mut(id) {
                if other_rec.lamport_tick > existing.lamport_tick
                    || (other_rec.lamport_tick == existing.lamport_tick
                        && other_rec.content_hash > existing.content_hash)
                {
                    *existing = other_rec.clone();
                }
            } else {
                self.records.insert(id.clone(), other_rec.clone());
            }
        }
    }

    pub fn page_tier1_slice(
        &self,
        target_symbol: &str,
        dep_ids: &[String],
        directive: &str,
        token_sub_budget: usize,
    ) -> String {
        let mut scored_records = Vec::new();
        let dir_lower = directive.to_lowercase();
        let target_lower = target_symbol.to_lowercase();

        for record in self.records.values() {
            if record.is_tombstone || record.kind == ZettelKind::Core {
                continue;
            }

            let mut score = 0.0f64;
            let scope_lower = record.scope.to_lowercase();

            // 1. Direct symbol anchor
            if record.scope == target_symbol || dir_lower.contains(&scope_lower) {
                score += 1.0;
            }

            // 2. 1-hop dependencies
            if dep_ids.iter().any(|d| d == &record.scope)
                || record
                    .links
                    .iter()
                    .any(|l| l == target_symbol || dep_ids.contains(l))
            {
                score += 0.7;
            }

            // 3. Keyword overlap
            for word in scope_lower
                .split("::")
                .chain(record.fact.split_whitespace())
            {
                let w_clean = word.trim_matches(|c: char| !c.is_alphanumeric());
                if w_clean.len() >= 4
                    && (dir_lower.contains(w_clean) || target_lower.contains(w_clean))
                {
                    score += 0.5;
                    break;
                }
            }

            if score > 0.0 {
                let recency_multiplier = 1.0 + (record.lamport_tick as f64) * 0.01;
                scored_records.push((score * recency_multiplier, record));
            }
        }

        scored_records.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut out = String::new();
        let mut current_tokens = 0;

        for (_score, record) in scored_records {
            let line = format!(
                "// ZETTEL [{}] {}: {}\n",
                record.kind.as_str(),
                record.scope,
                record.fact
            );
            let tokens = line.len() / 4;
            if current_tokens + tokens <= token_sub_budget {
                out.push_str(&line);
                current_tokens += tokens;
            }
        }

        out
    }
}
