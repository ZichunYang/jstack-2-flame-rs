use std::collections::HashMap;
use std::process::{Command, Output};
use std::sync::RwLock;

lazy_static::lazy_static! {
    static ref GLOBAL_NODE: RwLock<FlameGraphNode> = RwLock::new(FlameGraphNode::new("Root", 0));
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Process {
    pub pid: u32,
    pub name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct FlameGraphNode {
    pub name: String,
    pub num: u32,
    pub children: HashMap<String, FlameGraphNode>,
}

impl FlameGraphNode {
    fn new(name: &str, num: u32) -> Self {
        FlameGraphNode {
            name: name.to_string(),
            num,
            children: HashMap::new(),
        }
    }

    fn get_or_insert(&mut self, name: &str) -> &mut Self {
        self.children
            .entry(name.to_string())
            .or_insert_with(|| Self::new(name, 1))
    }

    pub fn merge(&mut self, other: FlameGraphNode) {
        self.num += other.num;

        for (name, other_child) in other.children {
            self.children
                .entry(name)
                .and_modify(|e| e.merge(other_child.clone()))
                .or_insert(other_child);
        }
    }
}

pub fn get_java_processes() -> Result<Vec<Process>, String> {
    let output = Command::new("jps")
        .arg("-l")
        .output()
        .map_err(|e| format!("Failed to execute jps command: {}", e))?;

    if !output.status.success() {
        return Err(format!("jps command failed with exit code: {}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let processes: Result<Vec<Process>, String> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                Some(
                    parts[0]
                        .parse::<u32>()
                        .map(|pid| Process {
                            pid,
                            name: parts[1].to_string(),
                        })
                        .map_err(|e| format!("Failed to parse PID: {}", e)),
                )
            } else {
                None
            }
        })
        .collect();

    processes
}


fn parse_stack_trace(lines: &[&str]) -> FlameGraphNode {
    let mut current_node = FlameGraphNode::new("Root", 0);
    let name_parts: Vec<&str> = lines[0].split("\"").collect();
    if let Some(name) = name_parts.get(1) {
        if lines.len() <= 1 {
            return current_node;
        }
        for line in lines {
            if line.contains("CompilerThread") || line.contains("GC Thread") || line.contains("VM Thread") {
                return current_node;
            }
        }
        current_node.num += 1;
        let mut parent_node = current_node.get_or_insert(name);
        for &line in lines[1..].iter().rev() {
            if !line.is_empty() {
                let trimmed = line.trim_start().replace("\t", "");
                if !trimmed.starts_with("at") && !trimmed.starts_with("-") {
                    continue;
                }
                let trimmed = trimmed.strip_prefix("-").unwrap_or(&trimmed).trim();
                let trimmed = trimmed.strip_prefix("at").unwrap_or(&trimmed).trim();

                parent_node = parent_node.get_or_insert(trimmed);
            }
        }
    }
    current_node
}


pub fn clear_jstack_info() {
    *GLOBAL_NODE.write().unwrap() = FlameGraphNode::new("Root", 0);
}

pub fn parse_jstack_info(pid: &str) -> Result<FlameGraphNode, Box<dyn std::error::Error>> {
    let mut root = GLOBAL_NODE.write().unwrap();

    let output: Output = Command::new("jstack")
        .arg(pid)
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("jstack command failed: {}", err).into());
    }

    let jstack_info = String::from_utf8_lossy(&output.stdout);

    for stack_trace in jstack_info.split("\n\n") {
        let lines: Vec<&str> = stack_trace.lines().collect();
        let lines: Vec<&str> = lines
            .iter()
            .filter(|&&line| !line.trim().is_empty())
            .cloned()
            .collect();
        if !lines.is_empty() {
            let parsed = parse_stack_trace(&lines);
            root.merge(parsed);
        }
    }

    Ok(root.clone())
}
