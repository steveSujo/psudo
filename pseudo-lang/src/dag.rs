use crate::vfs::WorkspaceCapsule;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

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

        let mut queue = VecDeque::new();
        for (id, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(id.clone());
            }
        }

        let mut sorted_queue: Vec<String> = queue.into();
        sorted_queue.sort_by(|a, b| {
            let pa = self.tasks.get(a).map(|t| t.priority).unwrap_or(0);
            let pb = self.tasks.get(b).map(|t| t.priority).unwrap_or(0);
            pb.cmp(&pa)
        });
        let mut queue: VecDeque<String> = sorted_queue.into();

        let mut result = Vec::new();

        while let Some(curr) = queue.pop_front() {
            result.push(curr.clone());
            if let Some(neighbors) = adj.get(&curr) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor.clone());
                    }
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
        sorted_tasks.sort_by_key(|t| std::cmp::Reverse(t.priority));

        for task in &sorted_tasks {
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
        for task in self.tasks.values() {
            out.push_str(&format!(
                "    \"{}\" [label=\"{}\\nPriority: {}\"];\n",
                task.id, task.id, task.priority
            ));
            for prereq in &task.prerequisites {
                out.push_str(&format!("    \"{}\" -> \"{}\";\n", prereq, task.id));
            }
        }
        out.push_str("}\n");
        out
    }

    pub fn scan_from_capsule(capsule: &WorkspaceCapsule) -> Self {
        let mut dag = Self::new();
        for content in capsule.virtual_files.values() {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("fn ") {
                    if let Some(name_end) = trimmed[3..].find('(') {
                        let task_id = trimmed[3..3 + name_end].trim();
                        dag.add_task(task_id, &[], 5, 300, 1);
                    }
                }
            }
        }
        dag
    }
}
