use crate::vfs::WorkspaceCapsule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskMetadata {
    pub id: String,
    pub priority: usize,
    pub timeout_sec: u64,
    pub resource_cpu: usize,
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleError {
    pub cycle_nodes: Vec<String>,
}

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DAG cycle detected among tasks: {:?}", self.cycle_nodes)
    }
}

impl std::error::Error for CycleError {}

// Extracting call edges into a topological DAG lets me synthesize and verify leaf stubs
// before callers, so broken dependencies don't cascade into multi-stub failures.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskDAG {
    pub tasks: HashMap<String, TaskMetadata>,
}

impl TaskDAG {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn add_task(
        &mut self,
        id: &str,
        prerequisites: &[&str],
        priority: usize,
        timeout_sec: u64,
        resource_cpu: usize,
    ) {
        let task = TaskMetadata {
            id: id.to_string(),
            priority,
            timeout_sec,
            resource_cpu,
            prerequisites: prerequisites.iter().map(|s| s.to_string()).collect(),
        };
        self.tasks.insert(id.to_string(), task);
    }

    pub fn topological_sort(&self) -> Result<Vec<String>, CycleError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();

        for id in self.tasks.keys() {
            in_degree.insert(id.clone(), 0);
            adj.insert(id.clone(), Vec::new());
        }

        for (id, task) in &self.tasks {
            for prereq in &task.prerequisites {
                if self.tasks.contains_key(prereq) {
                    adj.entry(prereq.clone()).or_default().push(id.clone());
                    *in_degree.entry(id.clone()).or_default() += 1;
                }
            }
        }

        for list in adj.values_mut() {
            list.sort();
        }

        let mut ready: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let sort_ready = |vec: &mut Vec<String>| {
            vec.sort_by(|a, b| {
                let pa = self.tasks.get(a).map(|t| t.priority).unwrap_or(0);
                let pb = self.tasks.get(b).map(|t| t.priority).unwrap_or(0);
                pb.cmp(&pa).then_with(|| a.cmp(b))
            });
        };
        sort_ready(&mut ready);

        let mut result = Vec::new();

        while !ready.is_empty() {
            let curr = ready.remove(0);
            result.push(curr.clone());
            if let Some(neighbors) = adj.get(&curr) {
                let mut unlocked = false;
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        ready.push(neighbor.clone());
                        unlocked = true;
                    }
                }
                if unlocked {
                    sort_ready(&mut ready);
                }
            }
        }

        if result.len() != self.tasks.len() {
            let cycle_nodes: Vec<String> = in_degree
                .into_iter()
                .filter(|(_, deg)| *deg > 0)
                .map(|(id, _)| id)
                .collect();
            return Err(CycleError { cycle_nodes });
        }

        Ok(result)
    }

    pub fn render_mermaid(&self) -> String {
        let mut out = String::from("graph TD\n");
        let mut sorted_tasks: Vec<&TaskMetadata> = self.tasks.values().collect();
        sorted_tasks.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));

        let mut has_incoming_or_outgoing: std::collections::HashSet<&str> =
            std::collections::HashSet::new();
        for task in &sorted_tasks {
            if !task.prerequisites.is_empty() {
                has_incoming_or_outgoing.insert(task.id.as_str());
                for prereq in &task.prerequisites {
                    has_incoming_or_outgoing.insert(prereq.as_str());
                }
            }
        }

        for task in &sorted_tasks {
            if !has_incoming_or_outgoing.contains(task.id.as_str()) {
                out.push_str(&format!("    {}\n", task.id));
            }
            for prereq in &task.prerequisites {
                out.push_str(&format!("    {} --> {}\n", prereq, task.id));
            }
        }
        out
    }

    pub fn render_dot(&self) -> String {
        let mut out = String::from(
            "digraph TaskDAG {\n    rankdir=TD;\n    node [shape=box, style=rounded];\n",
        );
        let mut sorted_tasks: Vec<&TaskMetadata> = self.tasks.values().collect();
        sorted_tasks.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
        for task in sorted_tasks {
            out.push_str(&format!(
                "    \"{}\" [label=\"{}\\nPriority: {}\"];\n",
                task.id, task.id, task.priority
            ));
            let mut prereqs = task.prerequisites.clone();
            prereqs.sort();
            for prereq in prereqs {
                out.push_str(&format!("    \"{}\" -> \"{}\";\n", prereq, task.id));
            }
        }
        out.push_str("}\n");
        out
    }

    pub fn scan_from_capsule(capsule: &WorkspaceCapsule) -> Self {
        let mut dag = Self::new();
        let mut fn_bodies: HashMap<String, Vec<String>> = HashMap::new();

        for (vpath, content) in &capsule.virtual_files {
            if !vpath.ends_with(".psudo") && !vpath.ends_with(".pseudo") {
                continue;
            }
            let mut current_fn: Option<String> = None;
            for line in content.lines() {
                let trimmed = line.trim();
                if !line.starts_with(' ') && !line.starts_with('\t') {
                    if let Some(after_fn) = trimmed.strip_prefix("fn ") {
                        if let Some(name_end) = after_fn.find('(') {
                            let task_id = after_fn[..name_end].trim().to_string();
                            fn_bodies.entry(task_id.clone()).or_default();
                            current_fn = Some(task_id);
                            continue;
                        }
                    }
                    if !trimmed.is_empty() {
                        current_fn = None;
                    }
                } else if let Some(ref fn_name) = current_fn {
                    fn_bodies
                        .entry(fn_name.clone())
                        .or_default()
                        .push(trimmed.to_string());
                }
            }
        }

        let all_fns: Vec<String> = fn_bodies.keys().cloned().collect();
        for (fn_name, body_lines) in &fn_bodies {
            let mut prereqs: Vec<String> = Vec::new();
            let mut search_texts = body_lines.clone();
            if let Some(traces) = capsule.stub_traces.get(fn_name) {
                for tr in traces {
                    if !tr.directive.is_empty() {
                        search_texts.push(tr.directive.clone());
                    }
                }
            }
            for candidate in &all_fns {
                if candidate == fn_name {
                    continue;
                }
                let call_pat = format!("{}(", candidate);
                let found = search_texts.iter().any(|t| {
                    t.match_indices(&call_pat).any(|(idx, _)| {
                        idx == 0
                            || !t.as_bytes()[idx - 1].is_ascii_alphanumeric()
                                && t.as_bytes()[idx - 1] != b'_'
                    })
                });
                if found && !prereqs.contains(candidate) {
                    prereqs.push(candidate.clone());
                }
            }
            prereqs.sort();
            let prereq_refs: Vec<&str> = prereqs.iter().map(String::as_str).collect();
            dag.add_task(fn_name, &prereq_refs, 5, 300, 1);
        }
        dag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_scan_extracts_call_edges_and_renders_mermaid() {
        let mut cap = WorkspaceCapsule::new_empty("csv-stats");
        cap.virtual_files.insert(
            "src/main.psudo".to_string(),
            r#"fn min_val(a, b)
    return a

fn max_val(a, b)
    return b

fn spread(a, b)
    return max_val(a, b) - min_val(a, b)
"#
            .to_string(),
        );
        let dag = TaskDAG::scan_from_capsule(&cap);
        let spread = dag.tasks.get("spread").unwrap();
        assert_eq!(spread.prerequisites, vec!["max_val", "min_val"]);
        let mermaid = dag.render_mermaid();
        assert!(mermaid.contains("max_val --> spread"));
        assert!(mermaid.contains("min_val --> spread"));
        let dot = dag.render_dot();
        assert!(dot.contains("\"max_val\" -> \"spread\";"));
        assert!(dot.contains("\"min_val\" -> \"spread\";"));
    }
}
